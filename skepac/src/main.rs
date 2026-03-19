use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
const USAGE_TOP: &str = "Usage: skepac check <entry.sk> | skepac run <entry.sk> | skepac build-native <entry.sk> <out.exe> | skepac build-obj <entry.sk> <out.obj> | skepac build-llvm-ir <entry.sk> <out.ll>";
const USAGE_CHECK: &str = "Usage: skepac check <file.sk>";
const USAGE_RUN: &str = "Usage: skepac run <in.sk>";
const USAGE_BUILD_NATIVE: &str = "Usage: skepac build-native <in.sk> <out.exe>";
const USAGE_BUILD_OBJ: &str = "Usage: skepac build-obj <in.sk> <out.obj>";
const USAGE_BUILD_LLVM_IR: &str = "Usage: skepac build-llvm-ir <in.sk> <out.ll>";

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(EXIT_USAGE as i32)
        }
    }
}

fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Err(USAGE_TOP.to_string());
    };

    match cmd.as_str() {
        "check" => {
            let Some(path) = args.next() else {
                return Err(USAGE_CHECK.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_CHECK.to_string());
            }
            check_file(&path)
        }
        "run" => {
            let Some(input) = args.next() else {
                return Err(USAGE_RUN.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_RUN.to_string());
            }
            run_native_file(&input)
        }
        "build-native" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_NATIVE.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_NATIVE.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_NATIVE.to_string());
            }
            build_native_file(&input, &output)
        }
        "build-llvm-ir" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            }
            build_llvm_ir_file(&input, &output)
        }
        "build-obj" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_OBJ.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_OBJ.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_OBJ.to_string());
            }
            build_object_file(&input, &output)
        }
        _ => Err(
            "Unknown command. Supported: check, run, build-native, build-obj, build-llvm-ir"
                .to_string(),
        ),
    }
}

fn check_file(path: &str) -> Result<i32, String> {
    match analyze_project_entry_phased(Path::new(path)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if parse_diags.is_empty() && sema_diags.is_empty() {
                println!("ok: {path}");
                return Ok(EXIT_OK as i32);
            }
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(EXIT_PARSE as i32);
            }
            for d in sema_diags.as_slice() {
                print_diag("sema", d);
            }
            Ok(EXIT_SEMA as i32)
        }
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                return Ok(EXIT_IO as i32);
            }
            print_resolve_errors(&errs);
            Ok(EXIT_RESOLVE as i32)
        }
    }
}

fn build_object_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Err(err) = codegen::compile_program_to_object_file(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    println!("built object: {output}");
    Ok(EXIT_OK as i32)
}

fn build_native_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Err(err) = codegen::compile_program_to_executable(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    println!("built native: {output}");
    Ok(EXIT_OK as i32)
}

fn build_llvm_ir_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Err(err) = codegen::write_program_llvm_ir(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    println!("built llvm ir: {output}");
    Ok(EXIT_OK as i32)
}

fn run_native_file(input: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    let exe_path = temp_native_path();
    let _cleanup = TempPathGuard::new(exe_path.clone());
    if let Err(err) = codegen::compile_program_to_executable(&program, &exe_path) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    let output = Command::new(&exe_path).output();
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            eprintln!("[E-RUNTIME][runtime] failed to run native executable: {err}");
            return Ok(1);
        }
    };
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    let status = output.status;
    let Some(code) = status.code() else {
        eprintln!("[E-RUNTIME][runtime] native executable terminated without an exit code");
        return Ok(1);
    };
    Ok(code)
}

fn validate_frontend(input: &str) -> Result<Option<i32>, String> {
    match analyze_project_entry_phased(Path::new(input)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(Some(EXIT_PARSE as i32));
            }
            if !sema_diags.is_empty() {
                for d in sema_diags.as_slice() {
                    print_diag("sema", d);
                }
                return Ok(Some(EXIT_SEMA as i32));
            }
            Ok(None)
        }
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                return Ok(Some(EXIT_IO as i32));
            }
            print_resolve_errors(&errs);
            Ok(Some(EXIT_RESOLVE as i32))
        }
    }
}

fn has_io_resolve_error(errs: &[ResolveError]) -> bool {
    errs.iter().any(|err| err.code == "E-MOD-IO")
}

fn compile_project_or_report(input: &str) -> Result<ir::IrProgram, i32> {
    match ir::lowering::compile_project_entry(Path::new(input)) {
        Ok(program) => Ok(program),
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                Err(EXIT_IO as i32)
            } else {
                print_resolve_errors(&errs);
                Err(EXIT_RESOLVE as i32)
            }
        }
    }
}

struct TempPathGuard(std::path::PathBuf);

impl TempPathGuard {
    fn new(path: std::path::PathBuf) -> Self {
        Self(path)
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
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
