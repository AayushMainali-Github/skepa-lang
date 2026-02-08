use std::env;
use std::fs;
use std::process::ExitCode;

use skeplib::parser::Parser;

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
        return Err("Usage: skepac check <file.sk>".to_string());
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
        _ => Err("Unknown command. Supported: check".to_string()),
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
        eprintln!("{d}");
    }
    Ok(ExitCode::from(1))
}
