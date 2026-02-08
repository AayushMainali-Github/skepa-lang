use std::env;
use std::fs;
use std::process::ExitCode;

use skeplib::bytecode::compile_source;
use skeplib::sema::analyze_source;
use skeplib::vm::Vm;

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
        return Err("Usage: skeparun run <file.sk>".to_string());
    };

    match cmd.as_str() {
        "run" => {
            let Some(path) = args.next() else {
                return Err("Usage: skeparun run <file.sk>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skeparun run <file.sk>".to_string());
            }
            run_file(&path)
        }
        _ => Err("Unknown command. Supported: run".to_string()),
    }
}

fn run_file(path: &str) -> Result<ExitCode, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read `{path}`: {e}"))?;

    let (_sema, sema_diags) = analyze_source(&source);
    if !sema_diags.is_empty() {
        for d in sema_diags.as_slice() {
            eprintln!("{d}");
        }
        return Ok(ExitCode::from(1));
    }

    let module = match compile_source(&source) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags.as_slice() {
                eprintln!("{d}");
            }
            return Ok(ExitCode::from(1));
        }
    };

    match Vm::run_module_main(&module) {
        Ok(value) => match value {
            skeplib::bytecode::Value::Int(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            _ => Ok(ExitCode::from(0)),
        },
        Err(e) => {
            eprintln!("Runtime error: {e}");
            Ok(ExitCode::from(1))
        }
    }
}
