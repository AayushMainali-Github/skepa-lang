use std::env;
use std::fs;
use std::process::ExitCode;

use skeplib::bytecode::{compile_source, BytecodeModule};
use skeplib::diagnostic::Diagnostic;
use skeplib::sema::analyze_source;
use skeplib::vm::{Vm, VmConfig};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
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
            let (opts, path) = parse_run_args(args, "Usage: skeparun run-bc [--trace] <file.skbc>")?;
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
        Ok(v) => v.parse::<usize>().map_err(|_| "SKEPA_MAX_CALL_DEPTH must be a positive integer".to_string())?,
        Err(_) => VmConfig::default().max_call_depth,
    };
    Ok((VmConfig { trace, max_call_depth }, path))
}

fn run_file(path: &str, config: VmConfig) -> Result<ExitCode, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read `{path}`: {e}"))?;

    let (_sema, sema_diags) = analyze_source(&source);
    if !sema_diags.is_empty() {
        for d in sema_diags.as_slice() {
            print_diag("sema", d);
        }
        return Ok(ExitCode::from(1));
    }

    let module = match compile_source(&source) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags.as_slice() {
                print_diag("codegen", d);
            }
            return Ok(ExitCode::from(1));
        }
    };

    match Vm::run_module_main_with_config(&module, config) {
        Ok(value) => match value {
            skeplib::bytecode::Value::Int(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            _ => Ok(ExitCode::from(0)),
        },
        Err(e) => {
            eprintln!("[{}][runtime] {e}", e.kind.code());
            Ok(ExitCode::from(1))
        }
    }
}

fn run_bytecode_file(path: &str, config: VmConfig) -> Result<ExitCode, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read `{path}`: {e}"))?;
    let module = BytecodeModule::from_bytes(&bytes)
        .map_err(|e| format!("Failed to decode `{path}`: {e}"))?;

    match Vm::run_module_main_with_config(&module, config) {
        Ok(value) => match value {
            skeplib::bytecode::Value::Int(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            _ => Ok(ExitCode::from(0)),
        },
        Err(e) => {
            eprintln!("[{}][runtime] {e}", e.kind.code());
            Ok(ExitCode::from(1))
        }
    }
}

fn print_diag(phase: &str, d: &Diagnostic) {
    eprintln!("[{}][{}] {}", phase_code(phase), phase, d);
}

fn phase_code(phase: &str) -> &'static str {
    match phase {
        "parse" => "E-PARSE",
        "sema" => "E-SEMA",
        "codegen" => "E-CODEGEN",
        _ => "E-UNKNOWN",
    }
}
