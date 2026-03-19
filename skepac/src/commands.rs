use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use skeplib::codegen;
use skeplib::ir;
use skeplib::resolver::ResolveError;
use skeplib::sema::analyze_project_entry_phased;

use crate::cli::{EXIT_CODEGEN, EXIT_IO, EXIT_OK, EXIT_PARSE, EXIT_RESOLVE, EXIT_SEMA};
use crate::output::{print_diag, print_resolve_errors};

pub fn check_file(path: &str) -> Result<i32, String> {
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

pub fn build_object_file(input: &str, output: &str) -> Result<i32, String> {
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

pub fn build_native_file(input: &str, output: &str) -> Result<i32, String> {
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

pub fn build_llvm_ir_file(input: &str, output: &str) -> Result<i32, String> {
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

pub fn run_native_file(input: &str) -> Result<i32, String> {
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

struct TempPathGuard(PathBuf);

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn temp_native_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    let ext = if cfg!(windows) { "exe" } else { "out" };
    std::env::temp_dir().join(format!("skepac_run_{nanos}.{ext}"))
}
