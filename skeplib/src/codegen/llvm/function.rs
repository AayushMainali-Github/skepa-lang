use crate::codegen::CodegenError;
use crate::codegen::llvm::block::{ensure_terminator, label};
use crate::codegen::llvm::special_locals::{SpecialLocalKind, SpecialLocals};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_symbol};
use crate::ir::{BasicBlock, IrFunction};

pub fn validate_function_layout(func: &IrFunction) -> Result<(), CodegenError> {
    if func.locals.len() < func.params.len() {
        return Err(CodegenError::InvalidIr(format!(
            "function {} is missing parameter-backed locals",
            func.name
        )));
    }
    for (param, local) in func.params.iter().zip(func.locals.iter()) {
        if param.ty != local.ty {
            return Err(CodegenError::InvalidIr(format!(
                "function {} has mismatched parameter/local types for param {}",
                func.name, param.name
            )));
        }
    }
    Ok(())
}

pub fn emit_function_header(func: &IrFunction) -> Result<Vec<String>, CodegenError> {
    let ret_ty = llvm_ty(&func.ret_ty)?;
    let params = func
        .params
        .iter()
        .map(|param| Ok(format!("{} %arg{}", llvm_ty(&param.ty)?, param.id.0)))
        .collect::<Result<Vec<_>, CodegenError>>()?
        .join(", ");
    Ok(vec![format!(
        "define {ret_ty} {}({params}) {{",
        llvm_symbol(&func.name)
    )])
}

pub fn begin_block(
    func: &IrFunction,
    block: &BasicBlock,
    idx: usize,
    special: &SpecialLocals,
    lines: &mut Vec<String>,
) -> Result<(), CodegenError> {
    ensure_terminator(&block.terminator)?;
    lines.push(format!("{}:", label(block)));
    if idx == 0 {
        for local in &func.locals {
            lines.push(format!(
                "  %local{} = alloca {}, align 8",
                local.id.0,
                llvm_ty(&local.ty)?
            ));
            if let Some(SpecialLocalKind::ScalarStruct { fields }) = special.local(local.id) {
                for (index, _) in fields.iter().enumerate() {
                    lines.push(format!(
                        "  %local{}_field{} = alloca i64, align 8",
                        local.id.0, index
                    ));
                }
            }
        }
        for (param, local) in func.params.iter().zip(func.locals.iter()) {
            lines.push(format!(
                "  store {} %arg{}, ptr %local{}, align 8",
                llvm_ty(&param.ty)?,
                param.id.0,
                local.id.0
            ));
        }
    }
    Ok(())
}

pub fn finish_function(lines: &mut Vec<String>) {
    lines.push("}".into());
}

pub fn value_names(func: &IrFunction) -> ValueNames {
    ValueNames::new(func)
}
