use std::env;
use std::fs;
use std::process::ExitCode;

use skeplib::bytecode::{compile_source, BytecodeModule};
use skeplib::parser::Parser;
use skeplib::sema::analyze_source;

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
        return Err(
            "Usage: skepac check <file.sk> | skepac build <in.sk> <out.skbc> | skepac disasm <file.sk|file.skbc>"
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
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read `{path}`: {e}"))?;

    let (_program, diagnostics) = Parser::parse_source(&source);
    if diagnostics.is_empty() {
        println!("ok: {path}");
        return Ok(ExitCode::from(0));
    }

    for d in diagnostics.as_slice() {
        eprintln!("[parse] {d}");
    }
    Ok(ExitCode::from(1))
}

fn build_file(input: &str, output: &str) -> Result<ExitCode, String> {
    let source = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read `{input}`: {e}"))?;

    let (_sema, sema_diags) = analyze_source(&source);
    if !sema_diags.is_empty() {
        for d in sema_diags.as_slice() {
            eprintln!("[sema] {d}");
        }
        return Ok(ExitCode::from(1));
    }

    let module = match compile_source(&source) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags.as_slice() {
                eprintln!("[codegen] {d}");
            }
            return Ok(ExitCode::from(1));
        }
    };

    let bytes = module.to_bytes();
    fs::write(output, bytes).map_err(|e| format!("Failed to write `{output}`: {e}"))?;
    println!("built: {output}");
    Ok(ExitCode::from(0))
}

fn disasm_file(path: &str) -> Result<ExitCode, String> {
    if path.ends_with(".skbc") {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read `{path}`: {e}"))?;
        let module = BytecodeModule::from_bytes(&bytes)
            .map_err(|e| format!("Failed to decode `{path}`: {e}"))?;
        print!("{}", module.disassemble());
        return Ok(ExitCode::from(0));
    }

    if path.ends_with(".sk") {
        let source = fs::read_to_string(path).map_err(|e| format!("Failed to read `{path}`: {e}"))?;
        let (_sema, sema_diags) = analyze_source(&source);
        if !sema_diags.is_empty() {
            for d in sema_diags.as_slice() {
                eprintln!("[sema] {d}");
            }
            return Ok(ExitCode::from(1));
        }
        let module = match compile_source(&source) {
            Ok(m) => m,
            Err(diags) => {
                for d in diags.as_slice() {
                    eprintln!("[codegen] {d}");
                }
                return Ok(ExitCode::from(1));
            }
        };
        print!("{}", module.disassemble());
        return Ok(ExitCode::from(0));
    }

    Err("disasm supports only .sk and .skbc files".to_string())
}
