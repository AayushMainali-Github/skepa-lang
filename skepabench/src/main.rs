use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use skeplib::bytecode::{BytecodeModule, compile_project_graph, compile_source};
use skeplib::diagnostic::DiagnosticBag;
use skeplib::parser::Parser;
use skeplib::resolver::resolve_project;
use skeplib::sema::analyze_project_graph_phased;
use skeplib::vm::Vm;

const DEFAULT_WARMUPS: usize = 4;
const DEFAULT_RUNS: usize = 15;

const LOOP_ITERATIONS: usize = 4_000_000;
const ARITH_ITERATIONS: usize = 4_000_000;
const ARITH_LOCAL_CONST_ITERATIONS: usize = 6_000_000;
const ARITH_LOCAL_LOCAL_ITERATIONS: usize = 5_000_000;
const ARITH_CHAIN_ITERATIONS: usize = 3_000_000;
const CALL_ITERATIONS: usize = 2_000_000;
const ARRAY_ITERATIONS: usize = 1_600_000;
const STRUCT_ITERATIONS: usize = 1_000_000;
const STRUCT_FIELD_ITERATIONS: usize = 2_500_000;
const STRUCT_COMPLEX_METHOD_ITERATIONS: usize = 1_500_000;
const STRING_ITERATIONS: usize = 400_000;
const MEDIUM_ACCUMULATE_LIMIT: usize = 80_000;

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

struct BenchWorkspace {
    root: PathBuf,
    small_file: PathBuf,
    medium_entry: PathBuf,
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

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<CliOptions, String> {
    let mut warmups = DEFAULT_WARMUPS;
    let mut runs = DEFAULT_RUNS;
    let mut profile = String::from("debug");
    let mut filter = None;
    let mut json = false;
    let mut save_baseline = false;
    let mut compare = false;
    let mut baseline_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmups" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --warmups".to_string());
                };
                warmups = value
                    .parse::<usize>()
                    .map_err(|_| "--warmups must be a positive integer".to_string())?;
            }
            "--runs" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --runs".to_string());
                };
                runs = value
                    .parse::<usize>()
                    .map_err(|_| "--runs must be a positive integer".to_string())?;
            }
            "--profile" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --profile".to_string());
                };
                if value != "debug" && value != "release" {
                    return Err("--profile must be `debug` or `release`".to_string());
                }
                profile = value;
            }
            "--filter" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --filter".to_string());
                };
                filter = Some(value);
            }
            "--json" => {
                json = true;
            }
            "--save-baseline" => {
                save_baseline = true;
            }
            "--compare" => {
                compare = true;
            }
            "--baseline-path" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --baseline-path".to_string());
                };
                baseline_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: cargo run -p skepabench -- [--warmups N] [--runs N] [--profile debug|release] [--filter SUBSTR] [--json] [--save-baseline] [--compare] [--baseline-path PATH]"
                        .to_string(),
                );
            }
            _ => return Err(format!("Unknown argument `{arg}`")),
        }
    }

    if warmups == 0 || runs == 0 {
        return Err("--warmups and --runs must be >= 1".to_string());
    }

    Ok(CliOptions {
        warmups,
        runs,
        profile,
        filter,
        json,
        save_baseline,
        compare,
        baseline_path,
    })
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

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn print_table_report(opts: &CliOptions, results: &[BenchRecord]) {
    println!(
        "skepabench warmups={} runs={} profile={}",
        opts.warmups, opts.runs, opts.profile
    );
    println!(
        "{:<28} {:<8} {:>10} {:>10} {:>10}",
        "case", "kind", "median_ms", "min_ms", "max_ms"
    );

    for result in results {
        match &result.outcome {
            BenchOutcome::Measured(stats) => {
                println!(
                    "{:<28} {:<8} {:>10.3} {:>10.3} {:>10.3}",
                    result.name,
                    result.kind,
                    duration_ms(stats.median),
                    duration_ms(stats.min),
                    duration_ms(stats.max),
                );
            }
            BenchOutcome::Skipped(reason) => {
                println!(
                    "{:<28} {:<8} skipped    {}",
                    result.name, result.kind, reason
                );
            }
        }
    }
}

fn render_json_report(opts: &CliOptions, results: &[BenchRecord]) -> String {
    let report = baseline_report_from_results(opts, results);
    render_report_json(&report)
}

fn write_baseline(path: &Path, opts: &CliOptions, results: &[BenchRecord]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let report = baseline_report_from_results(opts, results);
    fs::write(path, render_baseline_tsv(&report)).map_err(|err| err.to_string())
}

fn load_baseline(path: &Path) -> Result<BaselineReport, String> {
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    parse_baseline_tsv(&text)
}

fn baseline_report_from_results(opts: &CliOptions, results: &[BenchRecord]) -> BaselineReport {
    BaselineReport {
        warmups: opts.warmups,
        runs: opts.runs,
        profile: opts.profile.clone(),
        results: results
            .iter()
            .map(|result| match &result.outcome {
                BenchOutcome::Measured(stats) => BaselineRecord {
                    case: result.name.to_string(),
                    kind: result.kind.to_string(),
                    status: "measured".to_string(),
                    median_ms: Some(duration_ms(stats.median)),
                    min_ms: Some(duration_ms(stats.min)),
                    max_ms: Some(duration_ms(stats.max)),
                    reason: None,
                },
                BenchOutcome::Skipped(reason) => BaselineRecord {
                    case: result.name.to_string(),
                    kind: result.kind.to_string(),
                    status: "skipped".to_string(),
                    median_ms: None,
                    min_ms: None,
                    max_ms: None,
                    reason: Some(reason.clone()),
                },
            })
            .collect(),
    }
}

fn default_baseline_path(profile: &str) -> PathBuf {
    PathBuf::from("skepabench")
        .join("baselines")
        .join(format!("{profile}.tsv"))
}

fn workload_config(opts: &CliOptions) -> WorkloadConfig {
    let _ = opts;
    WorkloadConfig {
        loop_iterations: LOOP_ITERATIONS,
        arith_iterations: ARITH_ITERATIONS,
        arith_local_const_iterations: ARITH_LOCAL_CONST_ITERATIONS,
        arith_local_local_iterations: ARITH_LOCAL_LOCAL_ITERATIONS,
        arith_chain_iterations: ARITH_CHAIN_ITERATIONS,
        call_iterations: CALL_ITERATIONS,
        array_iterations: ARRAY_ITERATIONS,
        struct_iterations: STRUCT_ITERATIONS,
        struct_field_iterations: STRUCT_FIELD_ITERATIONS,
        struct_complex_method_iterations: STRUCT_COMPLEX_METHOD_ITERATIONS,
        string_iterations: STRING_ITERATIONS,
        medium_accumulate_limit: MEDIUM_ACCUMULATE_LIMIT,
    }
}

fn compare_results(baseline: &BaselineReport, results: &[BenchRecord]) -> Vec<CompareRow> {
    let mut rows = Vec::new();
    for result in results {
        let BenchOutcome::Measured(stats) = &result.outcome else {
            continue;
        };
        let Some(baseline_record) = baseline
            .results
            .iter()
            .find(|record| record.case == result.name && record.status == "measured")
        else {
            continue;
        };
        let Some(baseline_ms) = baseline_record.median_ms else {
            continue;
        };
        let current_ms = duration_ms(stats.median);
        let delta_ms = current_ms - baseline_ms;
        let delta_pct = if baseline_ms == 0.0 {
            0.0
        } else {
            (delta_ms / baseline_ms) * 100.0
        };
        rows.push(CompareRow {
            case: result.name.to_string(),
            current_ms,
            baseline_ms,
            delta_ms,
            delta_pct,
        });
    }
    rows.sort_by(|a, b| {
        b.delta_pct
            .abs()
            .partial_cmp(&a.delta_pct.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn print_compare_report(path: &Path, rows: &[CompareRow]) {
    println!("baseline {}", path.display());
    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>10}",
        "case", "current_ms", "base_ms", "delta_ms", "delta_pct"
    );
    for row in rows {
        println!(
            "{:<28} {:>12.3} {:>12.3} {:>12.3} {:>9.1}%",
            row.case, row.current_ms, row.baseline_ms, row.delta_ms, row.delta_pct
        );
    }
}

fn render_compare_json(path: &Path, rows: &[CompareRow]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"baseline_path\": \"{}\",\n",
        json_escape(&path.display().to_string())
    ));
    out.push_str("  \"rows\": [\n");
    for (idx, row) in rows.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"case\": \"{}\",\n",
            json_escape(&row.case)
        ));
        out.push_str(&format!("      \"current_ms\": {:.3},\n", row.current_ms));
        out.push_str(&format!("      \"baseline_ms\": {:.3},\n", row.baseline_ms));
        out.push_str(&format!("      \"delta_ms\": {:.3},\n", row.delta_ms));
        out.push_str(&format!("      \"delta_pct\": {:.3}\n", row.delta_pct));
        out.push_str("    }");
        if idx + 1 != rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push('}');
    out
}

fn render_report_json(report: &BaselineReport) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"warmups\": {},\n", report.warmups));
    out.push_str(&format!("  \"runs\": {},\n", report.runs));
    out.push_str(&format!(
        "  \"profile\": \"{}\",\n",
        json_escape(&report.profile)
    ));
    out.push_str("  \"results\": [\n");
    for (idx, record) in report.results.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"case\": \"{}\",\n",
            json_escape(&record.case)
        ));
        out.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        out.push_str(&format!(
            "      \"status\": \"{}\"",
            json_escape(&record.status)
        ));
        if let Some(median_ms) = record.median_ms {
            out.push_str(",\n");
            out.push_str(&format!("      \"median_ms\": {:.3},\n", median_ms));
            out.push_str(&format!(
                "      \"min_ms\": {:.3},\n",
                record.min_ms.unwrap_or_default()
            ));
            out.push_str(&format!(
                "      \"max_ms\": {:.3}\n",
                record.max_ms.unwrap_or_default()
            ));
        } else if let Some(reason) = &record.reason {
            out.push_str(",\n");
            out.push_str(&format!("      \"reason\": \"{}\"\n", json_escape(reason)));
        } else {
            out.push('\n');
        }
        out.push_str("    }");
        if idx + 1 != report.results.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push('}');
    out
}

fn render_baseline_tsv(report: &BaselineReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("warmups\t{}\n", report.warmups));
    out.push_str(&format!("runs\t{}\n", report.runs));
    out.push_str(&format!("profile\t{}\n", escape_tsv(&report.profile)));
    out.push_str("case\tkind\tstatus\tmedian_ms\tmin_ms\tmax_ms\treason\n");
    for record in &report.results {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            escape_tsv(&record.case),
            escape_tsv(&record.kind),
            escape_tsv(&record.status),
            record
                .median_ms
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default(),
            record.min_ms.map(|v| format!("{v:.3}")).unwrap_or_default(),
            record.max_ms.map(|v| format!("{v:.3}")).unwrap_or_default(),
            escape_tsv(record.reason.as_deref().unwrap_or_default()),
        ));
    }
    out
}

fn parse_baseline_tsv(text: &str) -> Result<BaselineReport, String> {
    let mut lines = text.lines();
    let warmups = parse_header_usize(lines.next(), "warmups")?;
    let runs = parse_header_usize(lines.next(), "runs")?;
    let profile = parse_header_string(lines.next(), "profile")?;
    let Some(header) = lines.next() else {
        return Err("baseline file missing results header".to_string());
    };
    if header != "case\tkind\tstatus\tmedian_ms\tmin_ms\tmax_ms\treason" {
        return Err("baseline file has invalid results header".to_string());
    }

    let mut results = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let cols: Vec<_> = line.split('\t').collect();
        if cols.len() != 7 {
            return Err(format!("baseline row has invalid column count: {line}"));
        }
        results.push(BaselineRecord {
            case: unescape_tsv(cols[0])?,
            kind: unescape_tsv(cols[1])?,
            status: unescape_tsv(cols[2])?,
            median_ms: parse_optional_f64(cols[3])?,
            min_ms: parse_optional_f64(cols[4])?,
            max_ms: parse_optional_f64(cols[5])?,
            reason: parse_optional_string(cols[6])?,
        });
    }

    Ok(BaselineReport {
        warmups,
        runs,
        profile,
        results,
    })
}

fn parse_header_usize(line: Option<&str>, key: &str) -> Result<usize, String> {
    let value = parse_header_string(line, key)?;
    value
        .parse::<usize>()
        .map_err(|_| format!("baseline {key} value is not a usize"))
}

fn parse_header_string(line: Option<&str>, key: &str) -> Result<String, String> {
    let Some(line) = line else {
        return Err(format!("baseline file missing `{key}` header"));
    };
    let Some((found, value)) = line.split_once('\t') else {
        return Err(format!("baseline `{key}` header is malformed"));
    };
    if found != key {
        return Err(format!("baseline header order mismatch: expected `{key}`"));
    }
    unescape_tsv(value)
}

fn parse_optional_f64(raw: &str) -> Result<Option<f64>, String> {
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse::<f64>()
            .map(Some)
            .map_err(|_| format!("invalid float value `{raw}` in baseline"))
    }
}

fn parse_optional_string(raw: &str) -> Result<Option<String>, String> {
    if raw.is_empty() {
        Ok(None)
    } else {
        Ok(Some(unescape_tsv(raw)?))
    }
}

fn escape_tsv(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_tsv(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(next) = chars.next() else {
            return Err("invalid trailing escape in baseline".to_string());
        };
        match next {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            other => return Err(format!("invalid escape `\\{other}` in baseline")),
        }
    }
    Ok(out)
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
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

impl BenchWorkspace {
    fn create(medium_accumulate_limit: usize) -> io::Result<Self> {
        let root = unique_temp_dir("skepabench")?;
        fs::create_dir_all(&root)?;

        let small_file = root.join("small.sk");
        fs::write(&small_file, src_small_single_file())?;

        let medium_entry = root.join("main.sk");
        let math_dir = root.join("utils");
        let model_dir = root.join("models");
        fs::create_dir_all(&math_dir)?;
        fs::create_dir_all(&model_dir)?;
        fs::write(&medium_entry, src_medium_main(medium_accumulate_limit))?;
        fs::write(math_dir.join("math.sk"), src_medium_math())?;
        fs::write(model_dir.join("user.sk"), src_medium_user())?;

        Ok(Self {
            root,
            small_file,
            medium_entry,
        })
    }
}

impl Drop for BenchWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_temp_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

fn src_small_single_file() -> String {
    r#"
fn addOne(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let value = addOne(41);
  if (value == 42) {
    return 0;
  }
  return 1;
}
"#
    .trim()
    .to_string()
}

fn src_medium_main(medium_accumulate_limit: usize) -> String {
    format!(
        r#"
from utils.math import accumulate;
from models.user import makeUser;

fn main() -> Int {{
  let total = accumulate({medium_accumulate_limit});
  let u = makeUser(3, "skepa");
  if (u.bump(4) == 7 && total > 0) {{
    return 0;
  }}
  return 1;
}}
"#
    )
    .trim()
    .to_string()
}

fn src_medium_math() -> String {
    r#"
fn accumulate(limit: Int) -> Int {
  let i = 0;
  let acc = 0;
  while (i < limit) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}

export { accumulate };
"#
    .trim()
    .to_string()
}

fn src_medium_user() -> String {
    r#"
struct User { id: Int, name: String }

impl User {
  fn bump(self, delta: Int) -> Int {
    return self.id + delta;
  }
}

fn makeUser(id: Int, name: String) -> User {
  return User { id: id, name: name };
}

export { User, makeUser };
"#
    .trim()
    .to_string()
}

fn src_loop_accumulate(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + i;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_function_call_chain(iterations: usize) -> String {
    format!(
        r#"
fn step(x: Int) -> Int {{
  return x + 1;
}}

fn main() -> Int {{
  let i = 0;
  while (i < {iterations}) {{
    i = step(i);
  }}
  return i;
}}
"#
    )
}

fn src_arith_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 1;
  let acc = 17;
  while (i < {iterations}) {{
    acc = acc + ((i * 3) % 97);
    acc = acc - (i % 11);
    acc = acc + ((acc / 3) % 29);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_arith_local_const_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 1;
  let acc = 17;
  while (i < {iterations}) {{
    let a = acc - 11;
    let b = a * 3;
    let c = b / 2;
    acc = c % 97;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_arith_local_local_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 1;
  let a = 17;
  let b = 31;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + ((a * b) % 97);
    acc = acc - (a / b);
    a = a + 3;
    b = b + 5;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_arith_chain_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 1;
  let x = 19;
  let y = 23;
  let z = 29;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + (((x * 3) + (y * 5) - (z % 7)) / 3);
    x = x + 1;
    y = y + 2;
    z = z + 3;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_array_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let arr: [Int; 8] = [0; 8];
  let i = 0;
  while (i < {iterations}) {{
    let idx = i % 8;
    arr[idx] = arr[idx] + 1;
    i = i + 1;
  }}
  return arr[0] + arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7];
}}
"#
    )
}

fn src_struct_method_workload(iterations: usize) -> String {
    format!(
        r#"
struct User {{ id: Int }}

impl User {{
  fn bump(self, delta: Int) -> Int {{
    return self.id + delta;
  }}
}}

fn main() -> Int {{
  let u = User {{ id: 1 }};
  let i = 0;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + u.bump(2);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_struct_field_workload(iterations: usize) -> String {
    format!(
        r#"
struct Pair {{ a: Int, b: Int }}

fn main() -> Int {{
  let p = Pair {{ a: 11, b: 7 }};
  let i = 0;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + p.a;
    acc = acc + p.b;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_struct_complex_method_workload(iterations: usize) -> String {
    format!(
        r#"
struct Pair {{ a: Int, b: Int }}

impl Pair {{
  fn mix(self, x: Int) -> Int {{
    return ((self.a + x) * 3 + self.b) % 1000000007;
  }}
}}

fn main() -> Int {{
  let p = Pair {{ a: 11, b: 7 }};
  let i = 0;
  let total = 0;
  while (i < {iterations}) {{
    total = total + p.mix(i % 13);
    i = i + 1;
  }}
  return total;
}}
"#
    )
}

fn src_string_workload(iterations: usize) -> String {
    format!(
        r#"
import str;

fn main() -> Int {{
  let i = 0;
  let total = 0;
  while (i < {iterations}) {{
    let s = "skepa-language";
    total = total + str.len(s);
    total = total + str.indexOf(s, "lang");
    let cut = str.slice(s, 0, 5);
    if (str.contains(cut, "ske")) {{
      total = total + 1;
    }}
    i = i + 1;
  }}
  return total;
}}
"#
    )
}
