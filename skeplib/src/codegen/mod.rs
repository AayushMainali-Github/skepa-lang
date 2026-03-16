pub mod llvm;

use std::fmt;
use std::path::Path;
use std::process::Command;
use std::{fs, io};

use crate::ir::IrProgram;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Unsupported(&'static str),
    MissingBlock(String),
    InvalidIr(String),
    Io(String),
    Tool(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported codegen shape: {msg}"),
            Self::MissingBlock(name) => write!(f, "missing basic block `{name}`"),
            Self::InvalidIr(msg) => write!(f, "invalid IR for codegen: {msg}"),
            Self::Io(msg) => write!(f, "i/o failure during codegen: {msg}"),
            Self::Tool(msg) => write!(f, "native toolchain failure: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<io::Error> for CodegenError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn compile_program_to_llvm_ir(program: &IrProgram) -> Result<String, CodegenError> {
    llvm::compile_program(program)
}

pub fn write_program_llvm_ir(program: &IrProgram, path: &Path) -> Result<(), CodegenError> {
    let ir = compile_program_to_llvm_ir(program)?;
    fs::write(path, ir)?;
    Ok(())
}

pub fn compile_program_to_bitcode_file(
    program: &IrProgram,
    path: &Path,
) -> Result<(), CodegenError> {
    let ll_path = temp_codegen_path("module", "ll");
    write_program_llvm_ir(program, &ll_path)?;
    let result = run_tool(
        "llvm-as",
        &[
            ll_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&ll_path);
    result
}

pub fn compile_program_to_object_file(
    program: &IrProgram,
    path: &Path,
) -> Result<(), CodegenError> {
    let bc_path = temp_codegen_path("module", "bc");
    compile_program_to_bitcode_file(program, &bc_path)?;
    let result = run_tool(
        "llc",
        &[
            "-filetype=obj",
            bc_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&bc_path);
    result
}

pub fn compile_program_to_executable(program: &IrProgram, path: &Path) -> Result<(), CodegenError> {
    let obj_path = temp_codegen_path("module", object_extension());
    compile_program_to_object_file(program, &obj_path)?;
    let result = run_tool(
        "clang",
        &[
            obj_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&obj_path);
    result
}

fn run_tool(tool: &str, args: &[&str]) -> Result<(), CodegenError> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|err| CodegenError::Tool(format!("failed to run `{tool}`: {err}")))?;
    if output.status.success() {
        return Ok(());
    }
    Err(CodegenError::Tool(format!(
        "`{tool}` failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn temp_codegen_path(name: &str, ext: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    std::env::temp_dir().join(format!("skepa_codegen_{name}_{nanos}.{ext}"))
}

fn object_extension() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}
