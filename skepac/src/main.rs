use std::env;
use std::fs;
use std::process::ExitCode;

use skeplib::bytecode::{compile_source, BytecodeModule};
use skeplib::diagnostic::Diagnostic;
use skeplib::parser::Parser;
use skeplib::sema::analyze_source;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_IO: u8 = 3;
const EXIT_PARSE: u8 = 10;
const EXIT_SEMA: u8 = 11;
const EXIT_CODEGEN: u8 = 12;
const EXIT_DECODE: u8 = 13;

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
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read `{path}`: {e}");
            return Ok(ExitCode::from(EXIT_IO));
        }
    };

    let (_program, diagnostics) = Parser::parse_source(&source);
    if diagnostics.is_empty() {
        println!("ok: {path}");
        return Ok(ExitCode::from(EXIT_OK));
    }

    for d in diagnostics.as_slice() {
        print_diag("parse", d);
    }
    Ok(ExitCode::from(EXIT_PARSE))
}

fn build_file(input: &str, output: &str) -> Result<ExitCode, String> {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read `{input}`: {e}");
            return Ok(ExitCode::from(EXIT_IO));
        }
    };

    let (_sema, sema_diags) = analyze_source(&source);
    if !sema_diags.is_empty() {
        for d in sema_diags.as_slice() {
            print_diag("sema", d);
        }
        return Ok(ExitCode::from(EXIT_SEMA));
    }

    let module = match compile_source(&source) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags.as_slice() {
                print_diag("codegen", d);
            }
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
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read `{path}`: {e}");
                return Ok(ExitCode::from(EXIT_IO));
            }
        };
        let (_sema, sema_diags) = analyze_source(&source);
        if !sema_diags.is_empty() {
            for d in sema_diags.as_slice() {
                print_diag("sema", d);
            }
            return Ok(ExitCode::from(EXIT_SEMA));
        }
        let module = match compile_source(&source) {
            Ok(m) => m,
            Err(diags) => {
                for d in diags.as_slice() {
                    print_diag("codegen", d);
                }
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

fn phase_code(phase: &str) -> &'static str {
    match phase {
        "parse" => "E-PARSE",
        "sema" => "E-SEMA",
        "codegen" => "E-CODEGEN",
        _ => "E-UNKNOWN",
    }
}
