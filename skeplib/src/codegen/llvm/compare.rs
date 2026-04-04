use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{CmpOp, ConstValue, IrFunction, IrProgram, Operand};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_compare(
    names: &ValueNames,
    func: &IrFunction,
    string_literals: &HashMap<String, String>,
    dest: &str,
    op: CmpOp,
    left: &Operand,
    right: &Operand,
    compare_ty: &crate::ir::IrType,
    lines: &mut Vec<String>,
    counter: &mut usize,
) -> Result<(), CodegenError> {
    let left = operand_load(
        names,
        left,
        func,
        lines,
        counter,
        compare_ty,
        string_literals,
    )?;
    let right = operand_load(
        names,
        right,
        func,
        lines,
        counter,
        compare_ty,
        string_literals,
    )?;

    match compare_ty {
        crate::ir::IrType::String => {
            let eq = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {eq} = call i1 @skp_rt_string_eq(ptr {left}, ptr {right})"
            ));
            match op {
                CmpOp::Eq => lines.push(format!("  {dest} = xor i1 {eq}, false")),
                CmpOp::Ne => lines.push(format!("  {dest} = xor i1 {eq}, true")),
                _ => {
                    return Err(CodegenError::Unsupported(
                        "string ordering comparisons are not implemented in LLVM lowering",
                    ));
                }
            }
        }
        crate::ir::IrType::Bytes => {
            let eq = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {eq} = call i1 @skp_rt_bytes_eq(ptr {left}, ptr {right})"
            ));
            match op {
                CmpOp::Eq => lines.push(format!("  {dest} = xor i1 {eq}, false")),
                CmpOp::Ne => lines.push(format!("  {dest} = xor i1 {eq}, true")),
                _ => {
                    return Err(CodegenError::Unsupported(
                        "bytes ordering comparisons are not implemented in LLVM lowering",
                    ));
                }
            }
        }
        crate::ir::IrType::Option { .. } => {
            let eq = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {eq} = call i1 @skp_rt_option_eq(ptr {left}, ptr {right})"
            ));
            match op {
                CmpOp::Eq => lines.push(format!("  {dest} = xor i1 {eq}, false")),
                CmpOp::Ne => lines.push(format!("  {dest} = xor i1 {eq}, true")),
                _ => {
                    return Err(CodegenError::Unsupported(
                        "option ordering comparisons are not implemented in LLVM lowering",
                    ));
                }
            }
        }
        crate::ir::IrType::Float => {
            let pred = match op {
                CmpOp::Eq => "oeq",
                CmpOp::Ne => "one",
                CmpOp::Lt => "olt",
                CmpOp::Le => "ole",
                CmpOp::Gt => "ogt",
                CmpOp::Ge => "oge",
            };
            lines.push(format!("  {dest} = fcmp {pred} double {left}, {right}"));
        }
        _ => {
            let pred = match op {
                CmpOp::Eq => "eq",
                CmpOp::Ne => "ne",
                CmpOp::Lt => "slt",
                CmpOp::Le => "sle",
                CmpOp::Gt => "sgt",
                CmpOp::Ge => "sge",
            };
            lines.push(format!(
                "  {dest} = icmp {pred} {} {left}, {right}",
                llvm_ty(compare_ty)?
            ));
        }
    }

    Ok(())
}

pub fn infer_compare_operand_type(
    program: &IrProgram,
    func: &IrFunction,
    left: &Operand,
    right: &Operand,
) -> crate::ir::IrType {
    match infer_operand_type(program, func, left)
        .or_else(|| infer_operand_type(program, func, right))
    {
        Some(crate::ir::IrType::Bool) => crate::ir::IrType::Bool,
        Some(crate::ir::IrType::Float) => crate::ir::IrType::Float,
        Some(crate::ir::IrType::String) => crate::ir::IrType::String,
        Some(crate::ir::IrType::Bytes) => crate::ir::IrType::Bytes,
        Some(crate::ir::IrType::Int) => crate::ir::IrType::Int,
        Some(other) => other,
        None => crate::ir::IrType::Int,
    }
}

fn infer_operand_type(
    program: &IrProgram,
    func: &IrFunction,
    operand: &Operand,
) -> Option<crate::ir::IrType> {
    match operand {
        Operand::Const(ConstValue::Int(_)) => Some(crate::ir::IrType::Int),
        Operand::Const(ConstValue::Float(_)) => Some(crate::ir::IrType::Float),
        Operand::Const(ConstValue::Bool(_)) => Some(crate::ir::IrType::Bool),
        Operand::Const(ConstValue::String(_)) => Some(crate::ir::IrType::String),
        Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .map(|temp| temp.ty.clone()),
        Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .map(|local| local.ty.clone()),
        Operand::Global(id) => program
            .globals
            .iter()
            .find(|global| global.id == *id)
            .map(|global| global.ty.clone()),
        _ => None,
    }
}
