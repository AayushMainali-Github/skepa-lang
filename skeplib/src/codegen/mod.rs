pub mod llvm;

use std::fmt;

use crate::ir::IrProgram;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Unsupported(&'static str),
    MissingBlock(String),
    InvalidIr(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported codegen shape: {msg}"),
            Self::MissingBlock(name) => write!(f, "missing basic block `{name}`"),
            Self::InvalidIr(msg) => write!(f, "invalid IR for codegen: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {}

pub fn compile_program_to_llvm_ir(program: &IrProgram) -> Result<String, CodegenError> {
    llvm::compile_program(program)
}
