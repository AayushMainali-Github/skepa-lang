use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

use skeplib::codegen;
use skeplib::diagnostic::Diagnostic;
use skeplib::ir;
use skeplib::resolver::ResolveError;
use skeplib::sema::analyze_project_entry_phased;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_IO: u8 = 3;
const EXIT_PARSE: u8 = 10;
const EXIT_SEMA: u8 = 11;
const EXIT_CODEGEN: u8 = 12;
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
            "Usage: skepac check <entry.sk> | skepac run <entry.sk> | skepac build-native <entry.sk> <out.exe> | skepac build-obj <entry.sk> <out.obj> | skepac build-llvm-ir <entry.sk> <out.ll>"
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
        "run" => {
            let Some(input) = args.next() else {
                return Err("Usage: skepac run <in.sk>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac run <in.sk>".to_string());
            }
            run_native_file(&input)
        }
        "build-native" => {
            let Some(input) = args.next() else {
                return Err("Usage: skepac build-native <in.sk> <out.exe>".to_string());
            };
            let Some(output) = args.next() else {
                return Err("Usage: skepac build-native <in.sk> <out.exe>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac build-native <in.sk> <out.exe>".to_string());
            }
            build_native_file(&input, &output)
        }
        "build-llvm-ir" => {
            let Some(input) = args.next() else {
                return Err("Usage: skepac build-llvm-ir <in.sk> <out.ll>".to_string());
            };
            let Some(output) = args.next() else {
                return Err("Usage: skepac build-llvm-ir <in.sk> <out.ll>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac build-llvm-ir <in.sk> <out.ll>".to_string());
            }
            build_llvm_ir_file(&input, &output)
        }
        "build-obj" => {
            let Some(input) = args.next() else {
                return Err("Usage: skepac build-obj <in.sk> <out.obj>".to_string());
            };
            let Some(output) = args.next() else {
                return Err("Usage: skepac build-obj <in.sk> <out.obj>".to_string());
            };
            if args.next().is_some() {
                return Err("Usage: skepac build-obj <in.sk> <out.obj>".to_string());
            }
            build_object_file(&input, &output)
        }
        _ => Err(
            "Unknown command. Supported: check, run, build-native, build-obj, build-llvm-ir"
                .to_string(),
        ),
    }
}

fn check_file(path: &str) -> Result<ExitCode, String> {
    if let Err(e) = fs::read_to_string(path) {
        eprintln!("Failed to read `{path}`: {e}");
        return Ok(ExitCode::from(EXIT_IO));
    }
    match analyze_project_entry_phased(Path::new(path)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if parse_diags.is_empty() && sema_diags.is_empty() {
                println!("ok: {path}");
                return Ok(ExitCode::from(EXIT_OK));
            }
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(ExitCode::from(EXIT_PARSE));
            }
            for d in sema_diags.as_slice() {
                print_diag("sema", d);
            }
            Ok(ExitCode::from(EXIT_SEMA))
        }
        Err(errs) => {
            print_resolve_errors(&errs);
            Ok(ExitCode::from(EXIT_RESOLVE))
        }
    }
}

fn build_object_file(input: &str, output: &str) -> Result<ExitCode, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match ir::lowering::compile_project_entry(Path::new(input)) {
        Ok(program) => program,
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    if let Err(err) = codegen::compile_program_to_object_file(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(ExitCode::from(EXIT_CODEGEN));
    }
    println!("built object: {output}");
    Ok(ExitCode::from(EXIT_OK))
}

fn build_native_file(input: &str, output: &str) -> Result<ExitCode, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match ir::lowering::compile_project_entry(Path::new(input)) {
        Ok(program) => program,
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    if let Err(err) = codegen::compile_program_to_executable(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(ExitCode::from(EXIT_CODEGEN));
    }
    println!("built native: {output}");
    Ok(ExitCode::from(EXIT_OK))
}

fn build_llvm_ir_file(input: &str, output: &str) -> Result<ExitCode, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match ir::lowering::compile_project_entry(Path::new(input)) {
        Ok(program) => program,
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    if let Err(err) = codegen::write_program_llvm_ir(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(ExitCode::from(EXIT_CODEGEN));
    }
    println!("built llvm ir: {output}");
    Ok(ExitCode::from(EXIT_OK))
}

fn run_native_file(input: &str) -> Result<ExitCode, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match ir::lowering::compile_project_entry(Path::new(input)) {
        Ok(program) => program,
        Err(errs) => {
            print_resolve_errors(&errs);
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    let exe_path = temp_native_path();
    if let Err(err) = codegen::compile_program_to_executable(&program, &exe_path) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(ExitCode::from(EXIT_CODEGEN));
    }
    let output = Command::new(&exe_path).output();
    let _ = fs::remove_file(&exe_path);
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            eprintln!("[E-CODEGEN][codegen] failed to run native executable: {err}");
            return Ok(ExitCode::from(EXIT_CODEGEN));
        }
    };
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(ExitCode::from(
        output.status.code().unwrap_or(EXIT_CODEGEN as i32) as u8,
    ))
}

fn validate_frontend(input: &str) -> Result<Option<ExitCode>, String> {
    match analyze_project_entry_phased(Path::new(input)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(Some(ExitCode::from(EXIT_PARSE)));
            }
            if !sema_diags.is_empty() {
                for d in sema_diags.as_slice() {
                    print_diag("sema", d);
                }
                return Ok(Some(ExitCode::from(EXIT_SEMA)));
            }
            Ok(None)
        }
        Err(errs) => {
            print_resolve_errors(&errs);
            Ok(Some(ExitCode::from(EXIT_RESOLVE)))
        }
    }
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

fn temp_native_path() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    let ext = if cfg!(windows) { "exe" } else { "out" };
    std::env::temp_dir().join(format!("skepac_run_{nanos}.{ext}"))
}
