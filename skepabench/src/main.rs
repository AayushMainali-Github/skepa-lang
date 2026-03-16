mod baseline;
mod cli;
mod workloads;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use skeplib::bytecode::{BytecodeModule, compile_project_graph, compile_source};
use skeplib::diagnostic::DiagnosticBag;
use skeplib::parser::Parser;
use skeplib::resolver::resolve_project;
use skeplib::sema::analyze_project_graph_phased;
use skeplib::vm::Vm;

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

    if opts.json {
        println!("{}", render_json_report(&opts, &results));
    } else {
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
            println!("{}", render_compare_json(&baseline_path, &rows));
        } else {
            print_compare_report(&baseline_path, &rows);
        }
    }

    Ok(())
}

fn benchmark_cases(
    workspace: &BenchWorkspace,
    opts: &CliOptions,
) -> Result<Vec<BenchCase>, String> {
    let workloads = workload_config(opts);
    let small_src = fs::read_to_string(&workspace.small_file).map_err(|err| err.to_string())?;
    let medium_graph = resolve_project(&workspace.medium_entry).map_err(format_resolve_errors)?;
    let small_graph = resolve_project(&workspace.small_file).map_err(format_resolve_errors)?;
    let small_graph_for_sema = small_graph.clone();
    let small_graph_for_codegen = small_graph.clone();
    let medium_graph_for_sema = medium_graph.clone();
    let medium_graph_for_codegen = medium_graph.clone();

    let loop_module =
        compile_source(&src_loop_accumulate(workloads.loop_iterations)).map_err(format_diags)?;
    let arith_module =
        compile_source(&src_arith_workload(workloads.arith_iterations)).map_err(format_diags)?;
    let arith_local_const_module = compile_source(&src_arith_local_const_workload(
        workloads.arith_local_const_iterations,
    ))
    .map_err(format_diags)?;
    let arith_local_local_module = compile_source(&src_arith_local_local_workload(
        workloads.arith_local_local_iterations,
    ))
    .map_err(format_diags)?;
    let arith_chain_module =
        compile_source(&src_arith_chain_workload(workloads.arith_chain_iterations))
            .map_err(format_diags)?;
    let call_module = compile_source(&src_function_call_chain(workloads.call_iterations))
        .map_err(format_diags)?;
    let array_module =
        compile_source(&src_array_workload(workloads.array_iterations)).map_err(format_diags)?;
    let struct_module = compile_source(&src_struct_method_workload(workloads.struct_iterations))
        .map_err(format_diags)?;
    let struct_field_module = compile_source(&src_struct_field_workload(
        workloads.struct_field_iterations,
    ))
    .map_err(format_diags)?;
    let struct_complex_method_module = compile_source(&src_struct_complex_method_workload(
        workloads.struct_complex_method_iterations,
    ))
    .map_err(format_diags)?;
    let string_module =
        compile_source(&src_string_workload(workloads.string_iterations)).map_err(format_diags)?;

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
                    analyze_project_graph_phased(&small_graph_for_sema);
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_small_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_codegen",
            kind: CaseKind::Library,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || {
                    let _ = compile_project_graph(&small_graph_for_codegen, &small_path)
                        .map_err(|err| err.to_string())?;
                    Ok(())
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
                    analyze_project_graph_phased(&medium_graph_for_sema);
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_medium_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_medium_codegen",
            kind: CaseKind::Library,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || {
                    let _ = compile_project_graph(&medium_graph_for_codegen, &medium_path)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "runtime_loop_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&loop_module)),
        },
        BenchCase {
            name: "runtime_arith_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&arith_module)),
        },
        BenchCase {
            name: "runtime_arith_local_const",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&arith_local_const_module)),
        },
        BenchCase {
            name: "runtime_arith_local_local",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&arith_local_local_module)),
        },
        BenchCase {
            name: "runtime_arith_chain",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&arith_chain_module)),
        },
        BenchCase {
            name: "runtime_call_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&call_module)),
        },
        BenchCase {
            name: "runtime_array_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&array_module)),
        },
        BenchCase {
            name: "runtime_struct_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&struct_module)),
        },
        BenchCase {
            name: "runtime_struct_field_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&struct_field_module)),
        },
        BenchCase {
            name: "runtime_struct_method_complex",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&struct_complex_method_module)),
        },
        BenchCase {
            name: "runtime_string_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&string_module)),
        },
    ];

    let cli_tools = cli_tools(&opts.profile)?;
    if let Some((skepac, skeparun)) = cli_tools {
        let skepac_small = skepac.clone();
        cases.push(BenchCase {
            name: "cli_small_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["check", path_str(&small_path)?])
            }),
        });
        let skeparun_small = skeparun.clone();
        cases.push(BenchCase {
            name: "cli_small_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || run_command(&skeparun_small, &["run", path_str(&small_path)?])
            }),
        });
        let skepac_medium = skepac.clone();
        cases.push(BenchCase {
            name: "cli_medium_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["check", path_str(&medium_path)?])
            }),
        });
        let skeparun_medium = skeparun.clone();
        cases.push(BenchCase {
            name: "cli_medium_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skeparun_medium, &["run", path_str(&medium_path)?])
            }),
        });
    } else {
        cases.push(skipped_case(
            "cli_small_check",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_small_run",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_check",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_run",
            "missing skepac/skeparun binary in selected profile",
        ));
    }

    Ok(cases)
}

fn skipped_case(name: &'static str, reason: &'static str) -> BenchCase {
    BenchCase {
        name,
        kind: CaseKind::Cli,
        runner: Box::new(move || Err(format!("SKIP:{reason}"))),
    }
}

fn cli_tools(profile: &str) -> Result<Option<(PathBuf, PathBuf)>, String> {
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
    let skeparun = exe_dir.join(exe_name("skeparun"));
    if skepac.exists() && skeparun.exists() {
        Ok(Some((skepac, skeparun)))
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

fn run_module(module: &BytecodeModule) -> Result<(), String> {
    match Vm::run_module_main(module) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
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
