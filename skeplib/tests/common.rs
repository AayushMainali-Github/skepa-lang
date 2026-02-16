#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use skeplib::ast::Program;
use skeplib::bytecode::{BytecodeModule, compile_source};
use skeplib::diagnostic::DiagnosticBag;
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
