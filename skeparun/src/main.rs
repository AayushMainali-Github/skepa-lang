use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use skeplib::bytecode::{BytecodeModule, compile_project_graph};
use skeplib::diagnostic::Diagnostic;
use skeplib::resolver::{ResolveError, resolve_project};
use skeplib::sema::analyze_project_graph_phased;
use skeplib::vm::{Vm, VmConfig};

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_IO: u8 = 3;
const EXIT_SEMA: u8 = 11;
const EXIT_CODEGEN: u8 = 12;
const EXIT_DECODE: u8 = 13;
const EXIT_RUNTIME: u8 = 14;
const EXIT_RESOLVE: u8 = 15;

#[derive(Default)]
struct PhaseProfiler {
    label: &'static str,
    started_at: Option<Instant>,
    phases: Vec<(&'static str, Duration)>,
}

impl PhaseProfiler {
    fn new(label: &'static str) -> Self {
        let started_at = if profiling_enabled() {
            Some(Instant::now())
        } else {
            None
        };
        Self {
            label,
            started_at,
            phases: Vec::new(),
        }
    }

    fn enabled(&self) -> bool {
        self.started_at.is_some()
    }

    fn record(&mut self, phase: &'static str, elapsed: Duration) {
        if self.enabled() {
            self.phases.push((phase, elapsed));
        }
    }

    fn print(&self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        eprintln!(
            "[run-profile] session={} total_ms={:.3}",
            self.label,
            started_at.elapsed().as_secs_f64() * 1_000.0
        );
        for (phase, elapsed) in &self.phases {
            eprintln!(
                "[run-profile] phase={} total_ms={:.3}",
                phase,
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
}

fn profiling_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SKEPA_PROFILE_RUN").is_some())
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Err("Usage: skeparun run <file.sk> | skeparun run-bc <file.skbc>".to_string());
    };

    match cmd.as_str() {
        "run" => {
            let (opts, path) = parse_run_args(args, "Usage: skeparun run [--trace] <file.sk>")?;
            run_file(&path, opts)
        }
        "run-bc" => {
            let (opts, path) =
                parse_run_args(args, "Usage: skeparun run-bc [--trace] <file.skbc>")?;
            run_bytecode_file(&path, opts)
        }
        _ => Err("Unknown command. Supported: run, run-bc".to_string()),
    }
}

fn parse_run_args(
    mut args: impl Iterator<Item = String>,
    usage: &str,
) -> Result<(VmConfig, String), String> {
    let mut trace = false;
    let mut path: Option<String> = None;
    for arg in args.by_ref() {
        if arg == "--trace" {
            trace = true;
            continue;
        }
        if path.is_none() {
            path = Some(arg);
        } else {
            return Err(usage.to_string());
        }
    }
    let Some(path) = path else {
        return Err(usage.to_string());
    };
    let max_call_depth = match env::var("SKEPA_MAX_CALL_DEPTH") {
        Ok(v) => {
            let parsed = v
                .parse::<usize>()
                .map_err(|_| "SKEPA_MAX_CALL_DEPTH must be an integer >= 1".to_string())?;
            if parsed == 0 {
                return Err("SKEPA_MAX_CALL_DEPTH must be an integer >= 1".to_string());
            }
            parsed
        }
        Err(_) => VmConfig::default().max_call_depth,
    };
    Ok((
        VmConfig {
            trace,
            max_call_depth,
        },
        path,
    ))
}

fn run_file(path: &str, config: VmConfig) -> Result<ExitCode, String> {
    let mut profiler = PhaseProfiler::new("run");

    let started = Instant::now();
    if let Err(e) = fs::read_to_string(path) {
        eprintln!("Failed to read `{path}`: {e}");
        return Ok(ExitCode::from(EXIT_IO));
    }
    profiler.record("read_source", started.elapsed());

    let started = Instant::now();
    let graph = match resolve_project(Path::new(path)) {
        Ok(graph) => {
            profiler.record("resolve_only", started.elapsed());
            if profiler.enabled() {
                eprintln!("[run-profile] modules_resolved={}", graph.modules.len());
            }
            graph
        }
        Err(errs) => {
            profiler.record("resolve_only", started.elapsed());
            print_resolve_errors(&errs);
            profiler.print();
            return Ok(ExitCode::from(EXIT_RESOLVE));
        }
    };

    let started = Instant::now();
    let (_sema, parse_diags, sema_diags) = analyze_project_graph_phased(&graph);
    profiler.record("sema_total", started.elapsed());
    if profiler.enabled() {
        eprintln!(
            "[run-profile] sema_parse_diags={} sema_diags={}",
            parse_diags.as_slice().len(),
            sema_diags.as_slice().len()
        );
    }
    if !parse_diags.is_empty() || !sema_diags.is_empty() {
        for d in parse_diags.as_slice() {
            print_diag("parse", d);
        }
        for d in sema_diags.as_slice() {
            print_diag("sema", d);
        }
        profiler.print();
        return Ok(ExitCode::from(EXIT_SEMA));
    }

    let started = Instant::now();
    let module = match compile_project_graph(&graph, Path::new(path)) {
        Ok(m) => m,
        Err(err) => {
            profiler.record("codegen_total", started.elapsed());
            eprintln!("[{}][codegen] {}", phase_code("codegen"), err);
            profiler.print();
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    profiler.record("codegen_total", started.elapsed());

    let started = Instant::now();
    let result = match Vm::run_module_main_with_config(&module, config) {
        Ok(value) => match value {
            skeplib::bytecode::Value::Int(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            _ => Ok(ExitCode::from(EXIT_OK)),
        },
        Err(e) => {
            eprintln!("[{}][runtime] {e}", e.kind.code());
            Ok(ExitCode::from(EXIT_RUNTIME))
        }
    };
    profiler.record("vm_execute", started.elapsed());
    profiler.print();
    result
}

fn run_bytecode_file(path: &str, config: VmConfig) -> Result<ExitCode, String> {
    let mut profiler = PhaseProfiler::new("run-bc");

    let started = Instant::now();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read `{path}`: {e}");
            return Ok(ExitCode::from(EXIT_IO));
        }
    };
    profiler.record("read_bytecode", started.elapsed());

    let started = Instant::now();
    let module = match BytecodeModule::from_bytes(&bytes) {
        Ok(m) => m,
        Err(e) => {
            profiler.record("decode_bytecode", started.elapsed());
            eprintln!("Failed to decode `{path}`: {e}");
            profiler.print();
            return Ok(ExitCode::from(EXIT_DECODE));
        }
    };
    profiler.record("decode_bytecode", started.elapsed());

    let started = Instant::now();
    let result = match Vm::run_module_main_with_config(&module, config) {
        Ok(value) => match value {
            skeplib::bytecode::Value::Int(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            _ => Ok(ExitCode::from(EXIT_OK)),
        },
        Err(e) => {
            eprintln!("[{}][runtime] {e}", e.kind.code());
            Ok(ExitCode::from(EXIT_RUNTIME))
        }
    };
    profiler.record("vm_execute", started.elapsed());
    profiler.print();
    result
}

fn print_diag(phase: &str, d: &Diagnostic) {
    eprintln!("[{}][{}] {}", phase_code(phase), phase, d);
}

fn print_resolve_errors(errs: &[ResolveError]) {
    for e in errs {
        if let Some(path) = &e.path {
            let line = e.line.unwrap_or(0);
            let col = e.col.unwrap_or(0);
            eprintln!(
                "[{}][resolve] {}:{}:{}: {}",
                e.code,
                path.display(),
                line,
                col,
                e.message
            );
        } else {
            eprintln!("[{}][resolve] {}", e.code, e.message);
        }
    }
}

fn phase_code(phase: &str) -> &'static str {
    match phase {
        "parse" => "E-PARSE",
        "sema" => "E-SEMA",
        "codegen" => "E-CODEGEN",
        _ => "E-UNKNOWN",
    }
}
