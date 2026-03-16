use crate::codegen::CodegenError;
use crate::ir::{BasicBlock, BlockId, BranchTerminator, Terminator};

pub fn label(block: &BasicBlock) -> String {
    format!("bb{}_{}", block.id.0, block.name)
}

pub fn branch_targets(
    branch: &BranchTerminator,
    labels: impl Fn(BlockId) -> Result<String, CodegenError>,
) -> Result<(String, String), CodegenError> {
    Ok((labels(branch.then_block)?, labels(branch.else_block)?))
}

pub fn ensure_terminator(term: &Terminator) -> Result<(), CodegenError> {
    match term {
        Terminator::Unreachable => Err(CodegenError::InvalidIr(
            "LLVM backend does not support unreachable terminators yet".into(),
        )),
        _ => Ok(()),
    }
}
