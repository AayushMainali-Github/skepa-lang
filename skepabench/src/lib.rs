pub mod baseline;
mod cases;
pub mod cli;
mod process;
mod report;
pub mod workloads;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::baseline::{
    compare_results, default_baseline_path, load_baseline, print_compare_report,
    print_table_report, write_baseline,
};
use crate::cases::benchmark_cases;
use crate::cli::parse_args;
use crate::workloads::{BenchWorkspace, workload_config};

pub const DEFAULT_WARMUPS: usize = 4;
pub const DEFAULT_RUNS: usize = 15;

pub const LOOP_ITERATIONS: usize = 16_000_000;
pub const ARITH_ITERATIONS: usize = 10_000_000;
pub const ARITH_LOCAL_CONST_ITERATIONS: usize = 14_000_000;
pub const ARITH_LOCAL_LOCAL_ITERATIONS: usize = 12_000_000;
pub const ARITH_CHAIN_ITERATIONS: usize = 8_000_000;
pub const CALL_ITERATIONS: usize = 35_000_000;
pub const ARRAY_ITERATIONS: usize = 10_000_000;
pub const STRUCT_ITERATIONS: usize = 10_000_000;
pub const STRUCT_FIELD_ITERATIONS: usize = 14_000_000;
pub const STRUCT_COMPLEX_METHOD_ITERATIONS: usize = 16_000_000;
pub const STRING_ITERATIONS: usize = 2_000_000;
pub const MEDIUM_ACCUMULATE_LIMIT: usize = 160_000;

pub struct CliOptions {
    pub warmups: usize,
    pub runs: usize,
    pub profile: String,
    pub filter: Option<String>,
    pub json: bool,
    pub save_baseline: bool,
    pub compare: bool,
    pub baseline_path: Option<PathBuf>,
}

pub struct WorkloadConfig {
    pub loop_iterations: usize,
    pub arith_iterations: usize,
    pub arith_local_const_iterations: usize,
    pub arith_local_local_iterations: usize,
    pub arith_chain_iterations: usize,
    pub call_iterations: usize,
    pub array_iterations: usize,
    pub struct_iterations: usize,
    pub struct_field_iterations: usize,
    pub struct_complex_method_iterations: usize,
    pub string_iterations: usize,
    pub medium_accumulate_limit: usize,
}

pub enum CaseKind {
    Library,
    Cli,
}

pub struct BenchCase {
    pub name: &'static str,
    pub kind: CaseKind,
    pub runner: Box<dyn FnMut() -> Result<(), String>>,
}

pub struct BenchStats {
    pub min: Duration,
    pub median: Duration,
    pub max: Duration,
}

pub enum BenchOutcome {
    Measured(BenchStats),
    Skipped(String),
}

pub struct BenchRecord {
    pub name: &'static str,
    pub kind: &'static str,
    pub outcome: BenchOutcome,
}

pub struct BaselineReport {
    pub warmups: usize,
    pub runs: usize,
    pub profile: String,
    pub results: Vec<BaselineRecord>,
}

pub struct BaselineRecord {
    pub case: String,
    pub kind: String,
    pub status: String,
    pub median_ms: Option<f64>,
    pub min_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub reason: Option<String>,
}

pub struct CompareRow {
    pub case: String,
    pub current_ms: f64,
    pub baseline_ms: f64,
    pub delta_ms: f64,
    pub delta_pct: f64,
}

pub fn cli_main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

pub fn run() -> Result<(), String> {
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

pub use process::{run_runtime_command, validate_runtime_output};
pub use report::render_full_json_report;
