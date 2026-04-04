use crate::codegen::CodegenError;
use crate::codegen::llvm::compare::{emit_compare, infer_compare_operand_type};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_float_literal, operand_load};
use crate::ir::{BinaryOp, ConstValue, Instr, IrFunction, IrProgram, Operand, UnaryOp};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_scalar_instr(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    instr: &Instr,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<bool, CodegenError> {
    match instr {
        Instr::Const { dst, ty, value } => {
            let dest = names.temp(*dst)?;
            match value {
                ConstValue::Int(v) => lines.push(format!("  {dest} = add {} 0, {v}", llvm_ty(ty)?)),
                ConstValue::Float(v) => lines.push(format!(
                    "  {dest} = fadd {} 0.0, {}",
                    llvm_ty(ty)?,
                    llvm_float_literal(*v)
                )),
                ConstValue::Bool(v) => {
                    let int = if *v { 1 } else { 0 };
                    lines.push(format!("  {dest} = add {} 0, {int}", llvm_ty(ty)?));
                }
                ConstValue::String(_) => {
                    let value = operand_load(
                        names,
                        &Operand::Const(value.clone()),
                        func,
                        lines,
                        counter,
                        ty,
                        string_literals,
                    )?;
                    lines.push(format!("  {dest} = bitcast ptr {value} to ptr"));
                }
                _ => {
                    return Err(CodegenError::Unsupported(
                        "only Int/Float/Bool/String constants are supported",
                    ));
                }
            }
            Ok(true)
        }
        Instr::Copy { dst, ty, src } => {
            let dest = names.temp(*dst)?;
            let value = operand_load(names, src, func, lines, counter, ty, string_literals)?;
            if matches!(
                ty,
                crate::ir::IrType::String
                    | crate::ir::IrType::Option { .. }
                    | crate::ir::IrType::Result { .. }
                    | crate::ir::IrType::Named(_)
                    | crate::ir::IrType::Array { .. }
                    | crate::ir::IrType::Vec { .. }
                    | crate::ir::IrType::Map { .. }
                    | crate::ir::IrType::Fn { .. }
            ) {
                lines.push(format!("  {dest} = bitcast ptr {value} to ptr"));
            } else if matches!(ty, crate::ir::IrType::Float) {
                lines.push(format!("  {dest} = fadd {} 0.0, {value}", llvm_ty(ty)?));
            } else {
                lines.push(format!("  {dest} = add {} 0, {value}", llvm_ty(ty)?));
            }
            Ok(true)
        }
        Instr::Unary {
            dst,
            ty,
            op,
            operand,
        } => {
            let dest = names.temp(*dst)?;
            let value = operand_load(names, operand, func, lines, counter, ty, string_literals)?;
            match (op, ty) {
                (UnaryOp::Neg, crate::ir::IrType::Int) => {
                    lines.push(format!("  {dest} = sub i64 0, {value}"));
                }
                (UnaryOp::Neg, crate::ir::IrType::Float) => {
                    lines.push(format!("  {dest} = fneg double {value}"));
                }
                (UnaryOp::Not, crate::ir::IrType::Bool) => {
                    lines.push(format!("  {dest} = xor i1 {value}, true"));
                }
                (UnaryOp::BitNot, crate::ir::IrType::Int) => {
                    lines.push(format!("  {dest} = xor i64 {value}, -1"));
                }
                _ => {
                    return Err(CodegenError::Unsupported(
                        "unsupported unary op/type in LLVM lowering",
                    ));
                }
            }
            Ok(true)
        }
        Instr::Binary {
            dst,
            ty,
            op,
            left,
            right,
        } => {
            let dest = names.temp(*dst)?;
            let left = operand_load(names, left, func, lines, counter, ty, string_literals)?;
            let right = operand_load(names, right, func, lines, counter, ty, string_literals)?;
            if matches!(ty, crate::ir::IrType::Int) && matches!(op, BinaryOp::Div | BinaryOp::Mod) {
                let zero_check = format!("%v{counter}");
                *counter += 1;
                let trap_label = format!("div_zero_{counter}");
                *counter += 1;
                let cont_label = format!("div_cont_{counter}");
                *counter += 1;
                let opname = match op {
                    BinaryOp::Div => "sdiv",
                    BinaryOp::Mod => "srem",
                    _ => unreachable!(),
                };
                lines.push(format!("  {zero_check} = icmp eq i64 {right}, 0"));
                lines.push(format!(
                    "  br i1 {zero_check}, label %{trap_label}, label %{cont_label}"
                ));
                lines.push(format!("{trap_label}:"));
                lines.push("  call void @skp_rt_raise_division_by_zero()".into());
                lines.push("  unreachable".into());
                lines.push(format!("{cont_label}:"));
                lines.push(format!("  {dest} = {opname} i64 {left}, {right}"));
                return Ok(true);
            }
            if matches!(ty, crate::ir::IrType::Int) && matches!(op, BinaryOp::Shl | BinaryOp::Shr) {
                let negative_check = format!("%v{counter}");
                *counter += 1;
                let trap_label = format!("shift_neg_{counter}");
                *counter += 1;
                let cont_label = format!("shift_cont_{counter}");
                *counter += 1;
                let masked = format!("%v{counter}");
                *counter += 1;
                let opname = match op {
                    BinaryOp::Shl => "shl",
                    BinaryOp::Shr => "ashr",
                    _ => unreachable!(),
                };
                lines.push(format!("  {negative_check} = icmp slt i64 {right}, 0"));
                lines.push(format!(
                    "  br i1 {negative_check}, label %{trap_label}, label %{cont_label}"
                ));
                lines.push(format!("{trap_label}:"));
                lines.push("  call void @skp_rt_raise_negative_shift_count()".into());
                lines.push("  unreachable".into());
                lines.push(format!("{cont_label}:"));
                lines.push(format!("  {masked} = and i64 {right}, 63"));
                lines.push(format!("  {dest} = {opname} i64 {left}, {masked}"));
                return Ok(true);
            }
            let opname = match (op, ty) {
                (BinaryOp::Add, crate::ir::IrType::Float) => "fadd",
                (BinaryOp::Sub, crate::ir::IrType::Float) => "fsub",
                (BinaryOp::Mul, crate::ir::IrType::Float) => "fmul",
                (BinaryOp::Div, crate::ir::IrType::Float) => "fdiv",
                (BinaryOp::Mod, crate::ir::IrType::Float) => {
                    return Err(CodegenError::Unsupported(
                        "float modulo is not implemented in LLVM lowering",
                    ));
                }
                (BinaryOp::Add, _) => "add",
                (BinaryOp::Sub, _) => "sub",
                (BinaryOp::Mul, _) => "mul",
                (BinaryOp::Div, _) => "sdiv",
                (BinaryOp::Mod, _) => "srem",
                (BinaryOp::BitAnd, _) => "and",
                (BinaryOp::BitOr, _) => "or",
                (BinaryOp::BitXor, _) => "xor",
                (BinaryOp::Shl, _) => "shl",
                (BinaryOp::Shr, _) => "ashr",
            };
            lines.push(format!(
                "  {dest} = {opname} {} {left}, {right}",
                llvm_ty(ty)?
            ));
            Ok(true)
        }
        Instr::Compare {
            dst,
            op,
            left,
            right,
        } => {
            let dest = names.temp(*dst)?;
            let compare_ty = infer_compare_operand_type(program, func, left, right);
            emit_compare(
                names,
                func,
                string_literals,
                dest,
                *op,
                left,
                right,
                &compare_ty,
                lines,
                counter,
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
