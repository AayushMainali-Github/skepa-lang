mod baseline;
mod cli;
mod workloads;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use skeplib::codegen;
use skeplib::diagnostic::DiagnosticBag;
use skeplib::ir;
use skeplib::parser::Parser;
use skeplib::resolver::resolve_project;
use skeplib::sema::analyze_project_graph_phased;

use crate::baseline::{
    compare_results, default_baseline_path, load_baseline, print_compare_report,
    print_table_report, render_compare_json, render_json_report, write_baseline,
};
use crate::cli::parse_args;
use crate::workloads::{
    BenchWorkspace, src_arith_chain_workload, src_arith_local_const_workload,
    src_arith_local_local_workload, src_arith_workload, src_array_workload,
    src_function_call_chain, src_loop_accumulate, src_string_workload,
    src_struct_complex_method_workload, src_struct_field_workload, src_struct_method_workload,
    workload_config,
};

const DEFAULT_WARMUPS: usize = 4;
const DEFAULT_RUNS: usize = 15;

const LOOP_ITERATIONS: usize = 16_000_000;
const ARITH_ITERATIONS: usize = 10_000_000;
const ARITH_LOCAL_CONST_ITERATIONS: usize = 14_000_000;
const ARITH_LOCAL_LOCAL_ITERATIONS: usize = 12_000_000;
const ARITH_CHAIN_ITERATIONS: usize = 8_000_000;
const CALL_ITERATIONS: usize = 35_000_000;
const ARRAY_ITERATIONS: usize = 10_000_000;
const STRUCT_ITERATIONS: usize = 10_000_000;
const STRUCT_FIELD_ITERATIONS: usize = 14_000_000;
const STRUCT_COMPLEX_METHOD_ITERATIONS: usize = 16_000_000;
const STRING_ITERATIONS: usize = 2_000_000;
const MEDIUM_ACCUMULATE_LIMIT: usize = 160_000;

struct CliOptions {
    warmups: usize,
    runs: usize,
    profile: String,
    filter: Option<String>,
    json: bool,
    save_baseline: bool,
    compare: bool,
    baseline_path: Option<PathBuf>,
}

struct WorkloadConfig {
    loop_iterations: usize,
    arith_iterations: usize,
    arith_local_const_iterations: usize,
    arith_local_local_iterations: usize,
    arith_chain_iterations: usize,
    call_iterations: usize,
    array_iterations: usize,
    struct_iterations: usize,
    struct_field_iterations: usize,
    struct_complex_method_iterations: usize,
    string_iterations: usize,
    medium_accumulate_limit: usize,
}

enum CaseKind {
    Library,
    Cli,
}

struct BenchCase {
    name: &'static str,
    kind: CaseKind,
    runner: Box<dyn FnMut() -> Result<(), String>>,
}

struct BenchStats {
    min: Duration,
    median: Duration,
    max: Duration,
}

enum BenchOutcome {
    Measured(BenchStats),
    Skipped(String),
}

struct BenchRecord {
    name: &'static str,
    kind: &'static str,
    outcome: BenchOutcome,
}

struct BaselineReport {
    warmups: usize,
    runs: usize,
    profile: String,
    results: Vec<BaselineRecord>,
}

struct BaselineRecord {
    case: String,
    kind: String,
    status: String,
    median_ms: Option<f64>,
    min_ms: Option<f64>,
    max_ms: Option<f64>,
    reason: Option<String>,
}

struct CompareRow {
    case: String,
    current_ms: f64,
    baseline_ms: f64,
    delta_ms: f64,
    delta_pct: f64,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let opts = parse_args(env::args().skip(1))?;
    let workloads = workload_config(&opts);
    let workspace =
        BenchWorkspace::create(workloads.medium_accumulate_limit).map_err(|err| err.to_string())?;
    let mut cases = benchmark_cases(&workspace, &opts)?;

    let mut results = Vec::new();

    for case in &mut cases {
        if let Some(filter) = &opts.filter
            && !case.name.contains(filter)
        {
            continue;
        }
        match measure_case(case, opts.warmups, opts.runs) {
            Ok(outcome) => results.push(BenchRecord {
                name: case.name,
                kind: case_kind_label(&case.kind),
                outcome,
            }),
            Err(err) => return Err(format!("benchmark `{}` failed: {err}", case.name)),
        }
    }

    if !opts.json {
        print_table_report(&opts, &results);
    }

    if opts.save_baseline {
        let baseline_path = opts
            .baseline_path
            .clone()
            .unwrap_or_else(|| default_baseline_path(&opts.profile));
        write_baseline(&baseline_path, &opts, &results)?;
        if !opts.json {
            println!("saved baseline to {}", baseline_path.display());
        }
    }

    if opts.compare {
        let baseline_path = opts
            .baseline_path
            .clone()
            .unwrap_or_else(|| default_baseline_path(&opts.profile));
        let baseline = load_baseline(&baseline_path)?;
        let rows = compare_results(&baseline, &results);
        if opts.json {
            println!(
                "{}",
                render_full_json_report(&opts, &results, Some((&baseline_path, &rows)))
            );
        } else {
            print_compare_report(&baseline_path, &rows);
        }
    } else if opts.json {
        println!("{}", render_full_json_report(&opts, &results, None));
    }

    Ok(())
}

fn benchmark_cases(
    workspace: &BenchWorkspace,
    opts: &CliOptions,
) -> Result<Vec<BenchCase>, String> {
    let workloads = workload_config(opts);
    let small_src = fs::read_to_string(&workspace.small_file).map_err(|err| err.to_string())?;
    let small_graph = resolve_project(&workspace.small_file).map_err(format_resolve_errors)?;
    let medium_graph = resolve_project(&workspace.medium_entry).map_err(format_resolve_errors)?;
    let small_graph_for_sema = small_graph.clone();
    let medium_graph_for_sema = medium_graph.clone();

    let loop_src = src_loop_accumulate(workloads.loop_iterations);
    let arith_src = src_arith_workload(workloads.arith_iterations);
    let arith_local_const_src =
        src_arith_local_const_workload(workloads.arith_local_const_iterations);
    let arith_local_local_src =
        src_arith_local_local_workload(workloads.arith_local_local_iterations);
    let arith_chain_src = src_arith_chain_workload(workloads.arith_chain_iterations);
    let call_src = src_function_call_chain(workloads.call_iterations);
    let array_src = src_array_workload(workloads.array_iterations);
    let struct_src = src_struct_method_workload(workloads.struct_iterations);
    let struct_field_src = src_struct_field_workload(workloads.struct_field_iterations);
    let struct_complex_src =
        src_struct_complex_method_workload(workloads.struct_complex_method_iterations);
    let string_src = src_string_workload(workloads.string_iterations);

    let cli_tool = cli_tools(&opts.profile)?;
    let native_exec_cases = if let Some(skepac) = &cli_tool {
        vec![
            native_exec_case(
                "runtime_loop_heavy",
                skepac.clone(),
                &loop_src,
            )?,
            native_exec_case(
                "runtime_arith_heavy",
                skepac.clone(),
                &arith_src,
            )?,
            native_exec_case(
                "runtime_arith_local_const",
                skepac.clone(),
                &arith_local_const_src,
            )?,
            native_exec_case(
                "runtime_arith_local_local",
                skepac.clone(),
                &arith_local_local_src,
            )?,
            native_exec_case(
                "runtime_arith_chain",
                skepac.clone(),
                &arith_chain_src,
            )?,
            native_exec_case(
                "runtime_call_heavy",
                skepac.clone(),
                &call_src,
            )?,
            native_exec_case(
                "runtime_array_heavy",
                skepac.clone(),
                &array_src,
            )?,
            native_exec_case(
                "runtime_struct_heavy",
                skepac.clone(),
                &struct_src,
            )?,
            native_exec_case(
                "runtime_struct_field_heavy",
                skepac.clone(),
                &struct_field_src,
            )?,
            native_exec_case(
                "runtime_struct_method_complex",
                skepac.clone(),
                &struct_complex_src,
            )?,
            native_exec_case(
                "runtime_string_heavy",
                skepac.clone(),
                &string_src,
            )?,
        ]
    } else {
        vec![
            skipped_case(
                "runtime_loop_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_local_const",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_local_local",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_chain",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_call_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_array_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_field_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_method_complex",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_string_heavy",
                "missing skepac binary in selected profile",
            ),
        ]
    };

    let mut cases = vec![
        BenchCase {
            name: "compile_small_parse",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let _ = Parser::parse_source(&small_src);
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || {
                    let _ = resolve_project(&small_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&small_graph_for_sema)
                        .map_err(format_resolve_errors)?;
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_small_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_ir_lowering",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let _ =
                        ir::lowering::compile_source_unoptimized(&source).map_err(format_diags)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_ir_optimize",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let mut program =
                        ir::lowering::compile_source_unoptimized(&source).map_err(format_diags)?;
                    ir::opt::optimize_program(&mut program);
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_object",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let obj = temp_artifact_path("small_obj", object_ext());
                    let result = codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    result
                }
            }),
        },
        BenchCase {
            name: "compile_small_link",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let obj = temp_artifact_path("small_link_obj", object_ext());
                    let exe = temp_artifact_path("small_link_exe", exe_ext());
                    codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string())?;
                    let result = codegen::link_object_file_to_executable(&obj, &exe)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    let _ = fs::remove_file(&exe);
                    result
                }
            }),
        },
        BenchCase {
            name: "compile_medium_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || {
                    let _ = resolve_project(&medium_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&medium_graph_for_sema)
                        .map_err(format_resolve_errors)?;
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_medium_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_medium_ir_lowering",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let _ = ir::lowering::compile_project_entry_unoptimized(&entry)
                        .map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_ir_optimize",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let mut program = ir::lowering::compile_project_entry_unoptimized(&entry)
                        .map_err(format_resolve_errors)?;
                    ir::opt::optimize_program(&mut program);
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let program = ir::lowering::compile_project_entry(&entry)
                        .map_err(format_resolve_errors)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_array_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = array_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_field_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_field_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_method_complex_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_complex_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_string_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = string_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_object",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let program = ir::lowering::compile_project_entry(&entry)
                        .map_err(format_resolve_errors)?;
                    let obj = temp_artifact_path("medium_obj", object_ext());
                    let result = codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    result
                }
            }),
        },
    ];

    cases.extend(native_exec_cases);

    if let Some(skepac) = cli_tool {
        let skepac_small = skepac.clone();
        cases.push(BenchCase {
            name: "cli_small_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_small = skepac_small.clone();
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["check", path_str(&small_path)?])
            }),
        });
        cases.push(BenchCase {
            name: "cli_small_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_small = skepac_small.clone();
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["run", path_str(&small_path)?])
            }),
        });
        let skepac_medium = skepac.clone();
        cases.push(BenchCase {
            name: "cli_medium_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_medium = skepac_medium.clone();
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["check", path_str(&medium_path)?])
            }),
        });
        cases.push(BenchCase {
            name: "cli_medium_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_medium = skepac_medium.clone();
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["run", path_str(&medium_path)?])
            }),
        });
    } else {
        cases.push(skipped_case(
            "cli_small_check",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_small_run",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_check",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_run",
            "missing skepac binary in selected profile",
        ));
    }

    Ok(cases)
}

fn native_exec_case(name: &'static str, skepac: PathBuf, source: &str) -> Result<BenchCase, String> {
    let source_file = TempSourceFile::new(source)?;
    Ok(BenchCase {
        name,
        kind: CaseKind::Library,
        runner: Box::new(move || run_runtime_command(&skepac, &["run", path_str(source_file.path())?])),
    })
}

fn skipped_case(name: &'static str, reason: &'static str) -> BenchCase {
    BenchCase {
        name,
        kind: CaseKind::Cli,
        runner: Box::new(move || Err(format!("SKIP:{reason}"))),
    }
}

fn cli_tools(profile: &str) -> Result<Option<PathBuf>, String> {
    let exe_dir = env::current_exe()
        .map_err(|err| err.to_string())?
        .parent()
        .ok_or_else(|| "failed to locate current executable directory".to_string())?
        .to_path_buf();

    let expected_profile = exe_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_default();
    if expected_profile != profile {
        return Ok(None);
    }

    let skepac = exe_dir.join(exe_name("skepac"));
    if skepac.exists() {
        Ok(Some(skepac))
    } else {
        Ok(None)
    }
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn measure_case(case: &mut BenchCase, warmups: usize, runs: usize) -> Result<BenchOutcome, String> {
    for _ in 0..warmups {
        match (case.runner)() {
            Ok(()) => {}
            Err(err) => {
                if let Some(reason) = err.strip_prefix("SKIP:") {
                    return Ok(BenchOutcome::Skipped(reason.to_string()));
                }
                return Err(err);
            }
        }
    }

    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        match (case.runner)() {
            Ok(()) => samples.push(started.elapsed()),
            Err(err) => {
                if let Some(reason) = err.strip_prefix("SKIP:") {
                    return Ok(BenchOutcome::Skipped(reason.to_string()));
                }
                return Err(err);
            }
        }
    }

    samples.sort();
    let min = samples[0];
    let max = samples[samples.len() - 1];
    let median = samples[samples.len() / 2];
    Ok(BenchOutcome::Measured(BenchStats { min, median, max }))
}

fn case_kind_label(kind: &CaseKind) -> &'static str {
    match kind {
        CaseKind::Library => "lib",
        CaseKind::Cli => "cli",
    }
}

fn run_command(exe: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {}: {err}", exe.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{} {} failed with {}: {}",
            exe.display(),
            args.join(" "),
            output.status,
            stderr.trim()
        ))
    }
}

struct TempSourceFile {
    dir: PathBuf,
    path: PathBuf,
}

impl TempSourceFile {
    fn new(source: &str) -> Result<Self, String> {
        let dir = temp_artifact_dir("bench_src");
        fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let path = dir.join("main.sk");
        fs::write(&path, source).map_err(|err| err.to_string())?;
        Ok(Self { dir, path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSourceFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn temp_artifact_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    env::temp_dir().join(format!("skepabench_{label}_{nanos}"))
}

fn temp_artifact_path(label: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    env::temp_dir().join(format!("skepabench_{label}_{nanos}.{ext}"))
}

fn object_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

fn path_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-utf8 path: {}", path.display()))
}

fn format_resolve_errors(errs: Vec<skeplib::resolver::ResolveError>) -> String {
    errs.into_iter()
        .map(|err| err.message)
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_diags(diags: DiagnosticBag) -> String {
    diags
        .into_vec()
        .into_iter()
        .map(|diag| diag.message)
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_full_json_report(
    opts: &CliOptions,
    results: &[BenchRecord],
    compare: Option<(&Path, &[CompareRow])>,
) -> String {
    let mut out = render_json_report(opts, results);
    if let Some((path, rows)) = compare {
        let compare_json = render_compare_json(path, rows);
        if out.ends_with("\n}") {
            out.truncate(out.len() - 2);
            out.push_str(",\n");
        } else if out.ends_with('}') {
            out.pop();
            out.push_str(",\n");
        } else {
            out.push('\n');
        }
        out.push_str("  \"compare\": ");
        out.push_str(compare_json.trim());
        out.push_str("\n}");
    }
    out
}

fn run_runtime_command(exe: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {}: {err}", exe.display()))?;
    validate_runtime_output(exe, args, &output)
}

fn validate_runtime_output(exe: &Path, args: &[&str], output: &Output) -> Result<(), String> {
    match output.status.code() {
        Some(0) => Ok(()),
        Some(code) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "{} {} exited with {}: {}{}{}",
                exe.display(),
                args.join(" "),
                code,
                stdout.trim(),
                if !stdout.trim().is_empty() && !stderr.trim().is_empty() {
                    " | "
                } else {
                    ""
                },
                stderr.trim()
            ))
        }
        None => Err(format!(
            "{} {} terminated without an exit code",
            exe.display(),
            args.join(" ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_opts() -> CliOptions {
        CliOptions {
            warmups: 1,
            runs: 1,
            profile: "debug".into(),
            filter: None,
            json: true,
            save_baseline: false,
            compare: false,
            baseline_path: None,
        }
    }

    fn sample_results() -> Vec<BenchRecord> {
        vec![BenchRecord {
            name: "runtime_x",
            kind: "lib",
            outcome: BenchOutcome::Measured(BenchStats {
                min: Duration::from_millis(1),
                median: Duration::from_millis(2),
                max: Duration::from_millis(3),
            }),
        }]
    }

    #[test]
    fn json_compare_renders_single_document() {
        let opts = sample_opts();
        let results = sample_results();
        let rows = vec![CompareRow {
            case: "runtime_x".into(),
            current_ms: 2.0,
            baseline_ms: 1.0,
            delta_ms: 1.0,
            delta_pct: 100.0,
        }];
        let rendered =
            render_full_json_report(&opts, &results, Some((Path::new("base.tsv"), &rows)));
        assert!(rendered.contains("\"results\": ["));
        assert!(rendered.contains("\"compare\": {"));
        assert_eq!(rendered.matches("\"baseline_path\"").count(), 1);
    }

    #[test]
    fn runtime_command_validation_rejects_nonzero_exit() {
        let output = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit", "7"])
                .output()
                .expect("cmd output")
        } else {
            Command::new("sh")
                .args(["-c", "exit 7"])
                .output()
                .expect("sh output")
        };
        let err = validate_runtime_output(Path::new("tool"), &["run", "x.sk"], &output)
            .expect_err("nonzero exit should fail");
        assert!(err.contains("exited with 7"), "{err}");
    }
}
