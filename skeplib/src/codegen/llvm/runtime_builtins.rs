use crate::codegen::CodegenError;
use crate::codegen::llvm::runtime_boxing::{
    emit_abort_if_error, emit_boxed_arg_array, emit_free_boxed_value, emit_free_boxed_values,
    emit_unbox_value,
};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load, raw_string_ptr};
use crate::ir::{BuiltinCall, IrFunction, IrType, TempId};
use std::collections::HashMap;

pub struct BuiltinCallInstr<'a> {
    pub dst: Option<TempId>,
    pub ret_ty: &'a IrType,
    pub builtin: &'a BuiltinCall,
    pub args: &'a [crate::ir::Operand],
}

pub fn emit_builtin_call(
    func: &IrFunction,
    names: &ValueNames,
    call: BuiltinCallInstr<'_>,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let helper = match (call.builtin.package.as_str(), call.builtin.name.as_str()) {
        ("str", "len") => "skp_rt_builtin_str_len",
        ("str", "contains") => "skp_rt_builtin_str_contains",
        ("str", "indexOf") => "skp_rt_builtin_str_index_of",
        ("str", "slice") => "skp_rt_builtin_str_slice",
        _ => return emit_builtin_call_generic(func, names, call, lines, counter, string_literals),
    };

    let expected = match call.builtin.name.as_str() {
        "len" => vec![IrType::String],
        "contains" => vec![IrType::String, IrType::String],
        "indexOf" => vec![IrType::String, IrType::String],
        "slice" => vec![IrType::String, IrType::Int, IrType::Int],
        _ => unreachable!(),
    };
    if call.args.len() != expected.len() {
        return Err(CodegenError::InvalidIr(format!(
            "builtin arity mismatch for {}.{}",
            call.builtin.package, call.builtin.name
        )));
    }

    let mut lowered_args = Vec::with_capacity(call.args.len());
    for (arg, ty) in call.args.iter().zip(expected.iter()) {
        let value = operand_load(names, arg, func, lines, counter, ty, string_literals)?;
        lowered_args.push(format!("{} {value}", llvm_ty(ty)?));
    }
    let joined_args = lowered_args.join(", ");
    let ret_llvm_ty = llvm_ty(call.ret_ty)?;

    if call.ret_ty.is_void() {
        lines.push(format!("  call {ret_llvm_ty} @{helper}({joined_args})"));
        emit_abort_if_error(lines);
        return Ok(());
    }

    let Some(dst) = call.dst else {
        return Err(CodegenError::InvalidIr(
            "non-void builtin call must write to a destination temp".into(),
        ));
    };
    let dest = names.temp(dst)?;
    lines.push(format!(
        "  {dest} = call {ret_llvm_ty} @{helper}({joined_args})"
    ));
    emit_abort_if_error(lines);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_builtin_call_generic(
    func: &IrFunction,
    names: &ValueNames,
    call: BuiltinCallInstr<'_>,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let package_ptr = raw_string_ptr(&call.builtin.package, lines, counter, string_literals)?;
    let name_ptr = raw_string_ptr(&call.builtin.name, lines, counter, string_literals)?;
    let boxed_args = emit_boxed_arg_array(func, names, call.args, lines, counter, string_literals)?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_call_builtin(ptr {package_ptr}, ptr {name_ptr}, i64 {}, ptr {})",
        call.args.len(),
        boxed_args.array
    ));
    emit_abort_if_error(lines);
    emit_free_boxed_values(&boxed_args.values, lines);
    if call.ret_ty.is_void() {
        emit_free_boxed_value(&raw, lines);
        return Ok(());
    }
    let Some(dst) = call.dst else {
        return Err(CodegenError::InvalidIr(
            "non-void builtin call must write to a destination temp".into(),
        ));
    };
    emit_unbox_value(names, dst, call.ret_ty, &raw, lines)?;
    emit_free_boxed_value(&raw, lines);
    Ok(())
}
