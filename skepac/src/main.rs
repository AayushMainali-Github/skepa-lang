use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use skeplib::bytecode::{BytecodeModule, compile_project_entry};
use skeplib::diagnostic::Diagnostic;
use skeplib::resolver::ResolveError;
use skeplib::sema::analyze_project_entry;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_IO: u8 = 3;
const EXIT_PARSE: u8 = 10;
const EXIT_SEMA: u8 = 11;
const EXIT_CODEGEN: u8 = 12;
const EXIT_DECODE: u8 = 13;
const EXIT_RESOLVE: u8 = 15;

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
        return Err(
            "Usage: skepac check <entry.sk> | skepac build <entry.sk> <out.skbc> | skepac disasm <entry.sk|file.skbc>"
                .to_string(),
        );
    };

    match cmd.as_str() {
        "check" => {
            let Some(path) = args.next() else {
                return Err("Usage: skepac check <file.sk>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac check <file.sk>".to_string());
            }
            check_file(&path)
        }
        "build" => {
            let Some(input) = args.next() else {
                return Err("Usage: skepac build <in.sk> <out.skbc>".to_string());
            };
            let Some(output) = args.next() else {
                return Err("Usage: skepac build <in.sk> <out.skbc>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac build <in.sk> <out.skbc>".to_string());
            }
            build_file(&input, &output)
        }
        "disasm" => {
            let Some(path) = args.next() else {
                return Err("Usage: skepac disasm <file.sk|file.skbc>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac disasm <file.sk|file.skbc>".to_string());
            }
            disasm_file(&path)
        }
        _ => Err("Unknown command. Supported: check, build, disasm".to_string()),
    }
}

fn check_file(path: &str) -> Result<ExitCode, String> {
    if let Err(e) = fs::read_to_string(path) {
        eprintln!("Failed to read `{path}`: {e}");
        return Ok(ExitCode::from(EXIT_IO));
    }
    match analyze_project_entry(Path::new(path)) {
        Ok((_sema, diagnostics)) => {
            if diagnostics.is_empty() {
                println!("ok: {path}");
                return Ok(ExitCode::from(EXIT_OK));
            }
            for d in diagnostics.as_slice() {
                print_diag("parse", d);
            }
            Ok(ExitCode::from(EXIT_PARSE))
        }
        Err(errs) => {
            print_resolve_errors(&errs);
            Ok(ExitCode::from(EXIT_RESOLVE))
        }
    }
}

fn build_file(input: &str, output: &str) -> Result<ExitCode, String> {
    match analyze_project_entry(Path::new(input)) {
        Ok((_sema, sema_diags)) => {
            if !sema_diags.is_empty() {
                for d in sema_diags.as_slice() {
                    print_diag("sema", d);
                }
                return Ok(ExitCode::from(EXIT_SEMA));
            }
        }
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_RESOLVE));
        }
    }

    let module = match compile_project_entry(Path::new(input)) {
        Ok(m) => m,
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };

    let bytes = module.to_bytes();
    if let Err(e) = fs::write(output, bytes) {
        eprintln!("Failed to write `{output}`: {e}");
        return Ok(ExitCode::from(EXIT_IO));
    }
    println!("built: {output}");
    Ok(ExitCode::from(EXIT_OK))
}

fn disasm_file(path: &str) -> Result<ExitCode, String> {
    if path.ends_with(".skbc") {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to read `{path}`: {e}");
                return Ok(ExitCode::from(EXIT_IO));
            }
        };
        let module = match BytecodeModule::from_bytes(&bytes) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to decode `{path}`: {e}");
                return Ok(ExitCode::from(EXIT_DECODE));
            }
        };
        print!("{}", module.disassemble());
        return Ok(ExitCode::from(EXIT_OK));
    }

    if path.ends_with(".sk") {
        match analyze_project_entry(Path::new(path)) {
            Ok((_sema, sema_diags)) => {
                if !sema_diags.is_empty() {
                    for d in sema_diags.as_slice() {
                        print_diag("sema", d);
                    }
                    return Ok(ExitCode::from(EXIT_SEMA));
                }
            }
            Err(errs) => {
                print_resolve_errors(&errs);
                return Ok(ExitCode::from(EXIT_RESOLVE));
            }
        }
        let module = match compile_project_entry(Path::new(path)) {
            Ok(m) => m,
            Err(errs) => {
                print_resolve_errors(&errs);
                return Ok(ExitCode::from(EXIT_CODEGEN));
            }
        };
        print!("{}", module.disassemble());
        return Ok(ExitCode::from(EXIT_OK));
    }

    Err("disasm supports only .sk and .skbc files".to_string())
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
                "[E-RESOLVE][resolve] {}:{}:{}: {}",
                path.display(),
                line,
                col,
                e.message
            );
        } else {
            eprintln!("[E-RESOLVE][resolve] {}", e.message);
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
