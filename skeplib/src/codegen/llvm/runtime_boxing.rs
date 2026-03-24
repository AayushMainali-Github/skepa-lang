use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{IrFunction, IrType, TempId};
use std::collections::HashMap;

pub struct BoxedArgArray {
    pub array: String,
    pub values: Vec<String>,
}

pub fn emit_boxed_arg_array(
    func: &IrFunction,
    names: &ValueNames,
    args: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<BoxedArgArray, CodegenError> {
    let array = format!("%v{counter}");
    *counter += 1;
    let len = args.len().max(1);
    lines.push(format!("  {array} = alloca ptr, i64 {len}, align 8"));
    let mut values = Vec::with_capacity(args.len());
    for (index, arg) in args.iter().enumerate() {
        let arg_ty = infer_operand_type(func, arg);
        let boxed = emit_boxed_operand(func, names, arg, &arg_ty, lines, counter, string_literals)?;
        values.push(boxed.clone());
        let slot = format!("%v{counter}");
        *counter += 1;
        lines.push(format!(
            "  {slot} = getelementptr inbounds ptr, ptr {array}, i64 {index}"
        ));
        lines.push(format!("  store ptr {boxed}, ptr {slot}, align 8"));
    }
    Ok(BoxedArgArray { array, values })
}

pub fn emit_boxed_operand(
    func: &IrFunction,
    names: &ValueNames,
    operand: &crate::ir::Operand,
    ty: &IrType,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<String, CodegenError> {
    let value = operand_load(names, operand, func, lines, counter, ty, string_literals)?;
    let boxed = format!("%v{counter}");
    *counter += 1;
    let helper = match ty {
        IrType::Int => "skp_rt_value_from_int",
        IrType::Float => "skp_rt_value_from_float",
        IrType::Bool => "skp_rt_value_from_bool",
        IrType::String => "skp_rt_value_from_string",
        IrType::Array { .. } => "skp_rt_value_from_array",
        IrType::Vec { .. } => "skp_rt_value_from_vec",
        IrType::Named(_) => "skp_rt_value_from_struct",
        IrType::Fn { .. } => "skp_rt_value_from_function",
        _ => {
            return Err(CodegenError::Unsupported(
                "boxing is only implemented for Int/Float/Bool/String/Array/Vec/Struct/Function",
            ));
        }
    };
    lines.push(format!(
        "  {boxed} = call ptr @{helper}({} {value})",
        llvm_ty(ty)?
    ));
    Ok(boxed)
}

pub fn emit_unbox_value(
    names: &ValueNames,
    dst: TempId,
    ty: &IrType,
    raw: &str,
    lines: &mut Vec<String>,
) -> Result<(), CodegenError> {
    let dest = names.temp(dst)?;
    match ty {
        IrType::Int => lines.push(format!(
            "  {dest} = call i64 @skp_rt_value_to_int(ptr {raw})"
        )),
        IrType::Float => lines.push(format!(
            "  {dest} = call double @skp_rt_value_to_float(ptr {raw})"
        )),
        IrType::Bool => lines.push(format!(
            "  {dest} = call i1 @skp_rt_value_to_bool(ptr {raw})"
        )),
        IrType::String => lines.push(format!(
            "  {dest} = call ptr @skp_rt_value_to_string(ptr {raw})"
        )),
        IrType::Array { .. } => lines.push(format!(
            "  {dest} = call ptr @skp_rt_value_to_array(ptr {raw})"
        )),
        IrType::Vec { .. } => lines.push(format!(
            "  {dest} = call ptr @skp_rt_value_to_vec(ptr {raw})"
        )),
        IrType::Named(_) => lines.push(format!(
            "  {dest} = call ptr @skp_rt_value_to_struct(ptr {raw})"
        )),
        IrType::Fn { .. } => lines.push(format!(
            "  {dest} = call ptr @skp_rt_value_to_function(ptr {raw})"
        )),
        _ => {
            return Err(CodegenError::Unsupported(
                "unboxing is only implemented for Int/Float/Bool/String/Array/Vec/Struct/Function",
            ));
        }
    }
    emit_abort_if_error(lines);
    Ok(())
}

pub fn emit_abort_if_error(lines: &mut Vec<String>) {
    lines.push("  call void @skp_rt_abort_if_error()".into());
}

pub fn emit_free_boxed_value(value: &str, lines: &mut Vec<String>) {
    lines.push(format!("  call void @skp_rt_value_free(ptr {value})"));
}

pub fn emit_free_boxed_values(values: &[String], lines: &mut Vec<String>) {
    for value in values {
        emit_free_boxed_value(value, lines);
    }
}

pub fn infer_operand_type(func: &IrFunction, operand: &crate::ir::Operand) -> IrType {
    match operand {
        crate::ir::Operand::Const(crate::ir::ConstValue::Int(_)) => IrType::Int,
        crate::ir::Operand::Const(crate::ir::ConstValue::Float(_)) => IrType::Float,
        crate::ir::Operand::Const(crate::ir::ConstValue::Bool(_)) => IrType::Bool,
        crate::ir::Operand::Const(crate::ir::ConstValue::String(_)) => IrType::String,
        crate::ir::Operand::Const(crate::ir::ConstValue::Unit) => IrType::Void,
        crate::ir::Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .map(|temp| temp.ty.clone())
            .unwrap_or(IrType::Unknown),
        crate::ir::Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .map(|local| local.ty.clone())
            .unwrap_or(IrType::Unknown),
        _ => IrType::Unknown,
    }
}
