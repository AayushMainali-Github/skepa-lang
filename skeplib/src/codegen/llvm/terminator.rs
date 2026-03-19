use crate::codegen::CodegenError;
use crate::codegen::llvm::block::{branch_targets, label};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{IrFunction, Terminator};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_terminator(
    func: &IrFunction,
    names: &ValueNames,
    term: &Terminator,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    match term {
        Terminator::Jump(target) => {
            let target = block_label(func, *target)?;
            lines.push(format!("  br label %{target}"));
        }
        Terminator::Branch(branch) => {
            let cond = operand_load(
                names,
                &branch.cond,
                func,
                lines,
                counter,
                &crate::ir::IrType::Bool,
                string_literals,
            )?;
            let (then_label, else_label) =
                branch_targets(branch, |block| block_label(func, block))?;
            lines.push(format!(
                "  br i1 {cond}, label %{then_label}, label %{else_label}"
            ));
        }
        Terminator::Return(Some(value)) => {
            let value = operand_load(
                names,
                value,
                func,
                lines,
                counter,
                &func.ret_ty,
                string_literals,
            )?;
            lines.push(format!("  ret {} {value}", llvm_ty(&func.ret_ty)?));
        }
        Terminator::Return(None) => lines.push("  ret void".into()),
        Terminator::Panic { .. } => {
            return Err(CodegenError::InvalidIr(
                "LLVM backend does not lower panic terminators".into(),
            ));
        }
        Terminator::Unreachable => {
            return Err(CodegenError::InvalidIr(
                "LLVM backend does not lower unreachable terminators".into(),
            ));
        }
    }
    Ok(())
}

pub fn block_label(func: &IrFunction, id: crate::ir::BlockId) -> Result<String, CodegenError> {
    let block = func
        .blocks
        .iter()
        .find(|block| block.id == id)
        .ok_or_else(|| CodegenError::MissingBlock(format!("{:?}", id)))?;
    Ok(label(block))
}
