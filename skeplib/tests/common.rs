#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::ast::Program;
use skeplib::bytecode::{BytecodeModule, compile_source};
use skeplib::codegen;
use skeplib::diagnostic::DiagnosticBag;
use skeplib::ir;
use skeplib::ir::{IrInterpError, IrInterpreter, IrProgram};
use skeplib::parser::Parser;
use skeplib::sema::{SemaResult, analyze_source};
use skeplib::vm::{Vm, VmError};

pub fn parse_ok(src: &str) -> Program {
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    program
}

pub fn parse_err(src: &str) -> DiagnosticBag {
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        !diags.is_empty(),
        "expected parser diagnostics but got none for:\n{src}"
    );
    diags
}

pub fn sema_ok(src: &str) -> (SemaResult, DiagnosticBag) {
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    (result, diags)
}

pub fn sema_err(src: &str) -> (SemaResult, DiagnosticBag) {
    let (result, diags) = analyze_source(src);
    assert!(
        result.has_errors || !diags.is_empty(),
        "expected sema diagnostics but got none for:\n{src}"
    );
    (result, diags)
}

pub fn compile_ok(src: &str) -> BytecodeModule {
    compile_source(src).expect("compile should succeed")
}

pub fn compile_err(src: &str) -> DiagnosticBag {
    compile_source(src).expect_err("compile should fail")
}

pub fn vm_run_ok(src: &str) -> skeplib::bytecode::Value {
    let module = compile_ok(src);
    Vm::run_module_main(&module).expect("vm run")
}

pub fn vm_run_err(src: &str) -> VmError {
    let module = compile_ok(src);
    Vm::run_module_main(&module).expect_err("vm run should fail")
}

pub fn assert_has_diag(diags: &DiagnosticBag, needle: &str) {
    assert!(
        diags.as_slice().iter().any(|d| d.message.contains(needle)),
        "missing diagnostic containing `{needle}` in {:?}",
        diags.as_slice()
    );
}

pub fn fixtures_dir(group: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(group)
}

pub fn sk_files_in(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).expect("fixture directory exists");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "sk") {
            out.push(path);
        }
    }
    out.sort();
    out
}

pub fn compile_ir_ok(src: &str) -> IrProgram {
    ir::lowering::compile_source(src).expect("IR lowering should succeed")
}

pub fn compile_project_ir_ok(entry: &Path) -> IrProgram {
    ir::lowering::compile_project_entry(entry).expect("project IR lowering should succeed")
}

pub fn ir_run_ok(src: &str) -> skepart::value::RtValue {
    let program = compile_ir_ok(src);
    IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source")
}

pub fn ir_run_err(src: &str) -> IrInterpError {
    let program = compile_ir_ok(src);
    IrInterpreter::new(&program)
        .run_main()
        .expect_err("IR interpreter should fail")
}

pub fn native_run_ok(src: &str) -> Output {
    let program = compile_ir_ok(src);
    native_run_program(&program)
}

pub fn native_run_project_ok(entry: &Path) -> Output {
    let program = compile_project_ir_ok(entry);
    native_run_program(&program)
}

fn native_run_program(program: &IrProgram) -> Output {
    let exe_path = temp_artifact_path("native_test", exe_ext());
    codegen::compile_program_to_executable(program, &exe_path)
        .expect("native executable build should succeed");
    let output = std::process::Command::new(&exe_path)
        .output()
        .expect("native executable should run");
    let _ = fs::remove_file(&exe_path);
    output
}

fn temp_artifact_path(label: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("skepa_{label}_{nanos}.{ext}"))
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}
