#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use skepart::{RtErrorKind, RtValue};
use skeplib::ast::Program;
use skeplib::codegen;
use skeplib::diagnostic::DiagnosticBag;
use skeplib::ir;
use skeplib::ir::{IrInterpError, IrInterpreter, IrProgram};
use skeplib::parser::Parser;
use skeplib::sema::{SemaResult, analyze_source};

pub struct TempProject {
    root: PathBuf,
}

impl TempProject {
    pub fn new(prefix: &str) -> Self {
        let root = make_temp_dir(prefix);
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
        let path = self.root.join(relative.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create temp project parent dir");
        }
        fs::write(&path, contents).expect("write temp project file");
        path
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Debug)]
pub struct NativeRunResult {
    pub output: Output,
}

impl NativeRunResult {
    pub fn exit_code(&self) -> i32 {
        self.output
            .status
            .code()
            .expect("native executable should produce an exit code")
    }

    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).replace("\r\n", "\n")
    }

    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).replace("\r\n", "\n")
    }

    pub fn expect_success(&self) {
        assert!(
            self.output.status.success(),
            "native executable should succeed, stderr: {}",
            self.stderr_lossy()
        );
    }

    pub fn printed_int(&self) -> i64 {
        self.expect_success();
        let stdout = self.stdout_lossy();
        stdout
            .trim()
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("expected integer stdout, got `{stdout}`"))
    }
}

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

pub fn assert_has_diag(diags: &DiagnosticBag, needle: &str) {
    assert!(
        diags.as_slice().iter().any(|d| d.message.contains(needle)),
        "missing diagnostic containing `{needle}` in {:?}",
        diags.as_slice()
    );
}

pub fn assert_diag_has_message(diags: &DiagnosticBag, needle: &str) {
    assert_has_diag(diags, needle);
}

pub fn assert_diag_count_at_least(diags: &DiagnosticBag, min: usize) {
    assert!(
        diags.len() >= min,
        "expected at least {min} diagnostics, got {:?}",
        diags.as_slice()
    );
}

pub fn assert_no_diags(diags: &DiagnosticBag) {
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
}

pub fn assert_sema_success(result: &SemaResult, diags: &DiagnosticBag) {
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
    assert_no_diags(diags);
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
    native_run_program(&program).output
}

pub fn native_run_exit_code_ok(src: &str) -> i32 {
    native_run_ok(src)
        .status
        .code()
        .expect("native executable should produce an exit code")
}

pub fn native_run_printed_int_ok(src: &str) -> i64 {
    native_run_structured(src).printed_int()
}

pub fn native_run_project_ok(entry: &Path) -> Output {
    let program = compile_project_ir_ok(entry);
    native_run_program(&program).output
}

pub fn native_run_project_exit_code_ok(entry: &Path) -> i32 {
    native_run_project_ok(entry)
        .status
        .code()
        .expect("native executable should produce an exit code")
}

pub fn native_run_structured(src: &str) -> NativeRunResult {
    let program = compile_ir_ok(src);
    native_run_program(&program)
}

pub fn native_run_project_structured(entry: &Path) -> NativeRunResult {
    let program = compile_project_ir_ok(entry);
    native_run_program(&program)
}

pub fn llvm_tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

pub fn require_llvm_tool(tool: &str) {
    assert!(
        llvm_tool_available(tool),
        "expected `{tool}` to be available on PATH for this test"
    );
}

pub fn assert_runtime_error_kind(err: &IrInterpError, expected: RtErrorKind) {
    let actual = match err {
        IrInterpError::DivisionByZero => RtErrorKind::DivisionByZero,
        IrInterpError::IndexOutOfBounds => RtErrorKind::IndexOutOfBounds,
        IrInterpError::TypeMismatch(_) => RtErrorKind::TypeMismatch,
        IrInterpError::InvalidField(_) => RtErrorKind::MissingField,
        IrInterpError::InvalidOperand(_) => RtErrorKind::InvalidArgument,
        IrInterpError::UnsupportedBuiltin(_) => RtErrorKind::UnsupportedBuiltin,
        other => panic!("unsupported interpreter error for kind assertion: {other:?}"),
    };
    assert_eq!(actual, expected);
}

pub fn assert_native_matches_ir_value(src: &str, expected: RtValue) {
    let program = compile_ir_ok(src);
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(ir_value, expected);

    match expected {
        RtValue::Int(v) => assert_eq!(native_run_structured(src).exit_code(), v as i32),
        RtValue::Bool(v) => assert_eq!(
            native_run_structured(src).exit_code(),
            if v { 1 } else { 0 }
        ),
        RtValue::String(ref s) => {
            let result = native_run_structured(src);
            result.expect_success();
            assert_eq!(result.stdout_lossy().trim(), s.as_str());
        }
        _ => panic!("native/IR shared value assertion is only supported for int/bool/string"),
    }
}

pub fn assert_native_matches_ir_error_kind(src: &str, expected: RtErrorKind) {
    let program = compile_ir_ok(src);
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("IR interpreter should fail");
    assert_runtime_error_kind(&err, expected);

    let native = native_run_structured(src);
    assert!(
        !native.output.status.success(),
        "native executable should fail for source"
    );
}

fn native_run_program(program: &IrProgram) -> NativeRunResult {
    let exe_path = temp_artifact_path("native_test", exe_ext());
    codegen::compile_program_to_executable(program, &exe_path)
        .expect("native executable build should succeed");
    let output = run_output(&mut Command::new(&exe_path)).expect("native executable should run");
    let _ = fs::remove_file(&exe_path);
    NativeRunResult { output }
}

pub fn make_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}_{}", unique_suffix()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn temp_artifact_path(label: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("skepa_{label}_{}.{ext}", unique_suffix()))
}

fn unique_suffix() -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = std::process::id();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{pid}_{nanos}_{seq}")
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

fn run_output(command: &mut Command) -> std::io::Result<Output> {
    #[cfg(windows)]
    {
        let previous = set_error_mode(SEM_NOGPFAULTERRORBOX | SEM_FAILCRITICALERRORS);
        let result = command.output();
        let _ = set_error_mode(previous);
        result
    }
    #[cfg(not(windows))]
    {
        command.output()
    }
}

#[cfg(windows)]
const SEM_FAILCRITICALERRORS: u32 = 0x0001;
#[cfg(windows)]
const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;

#[cfg(windows)]
fn set_error_mode(mode: u32) -> u32 {
    unsafe extern "system" {
        fn SetErrorMode(u_mode: u32) -> u32;
    }
    unsafe { SetErrorMode(mode) }
}
