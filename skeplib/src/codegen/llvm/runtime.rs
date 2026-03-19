use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_symbol, operand_load, raw_string_ptr};
use crate::ir::Instr;
use crate::ir::{BuiltinCall, IrFunction, IrProgram, IrType, TempId};
use std::collections::HashMap;

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    let _ = instr;
    Ok(())
}

pub fn emit_runtime_decls(program: &IrProgram, out: &mut Vec<String>) -> Result<(), CodegenError> {
    out.push("declare ptr @skp_rt_string_from_utf8(ptr, i64)".into());
    out.push("declare i1 @skp_rt_string_eq(ptr, ptr)".into());
    out.push("declare i64 @skp_rt_builtin_str_len(ptr)".into());
    out.push("declare i1 @skp_rt_builtin_str_contains(ptr, ptr)".into());
    out.push("declare i64 @skp_rt_builtin_str_index_of(ptr, ptr)".into());
    out.push("declare ptr @skp_rt_builtin_str_slice(ptr, i64, i64)".into());
    out.push("declare ptr @skp_rt_call_builtin(ptr, ptr, i64, ptr)".into());
    out.push("declare ptr @skp_rt_value_from_int(i64)".into());
    out.push("declare ptr @skp_rt_value_from_bool(i1)".into());
    out.push("declare ptr @skp_rt_value_from_float(double)".into());
    out.push("declare ptr @skp_rt_value_from_unit()".into());
    out.push("declare ptr @skp_rt_value_from_string(ptr)".into());
    out.push("declare ptr @skp_rt_value_from_array(ptr)".into());
    out.push("declare ptr @skp_rt_value_from_vec(ptr)".into());
    out.push("declare ptr @skp_rt_value_from_struct(ptr)".into());
    out.push("declare ptr @skp_rt_value_from_function(i32)".into());
    out.push("declare i64 @skp_rt_value_to_int(ptr)".into());
    out.push("declare i1 @skp_rt_value_to_bool(ptr)".into());
    out.push("declare double @skp_rt_value_to_float(ptr)".into());
    out.push("declare ptr @skp_rt_value_to_string(ptr)".into());
    out.push("declare ptr @skp_rt_value_to_array(ptr)".into());
    out.push("declare ptr @skp_rt_value_to_vec(ptr)".into());
    out.push("declare ptr @skp_rt_value_to_struct(ptr)".into());
    out.push("declare i32 @skp_rt_value_to_function(ptr)".into());
    out.push("declare ptr @skp_rt_array_new(i64)".into());
    out.push("declare ptr @skp_rt_array_repeat(ptr, i64)".into());
    out.push("declare ptr @skp_rt_array_get(ptr, i64)".into());
    out.push("declare void @skp_rt_array_set(ptr, i64, ptr)".into());
    out.push("declare ptr @skp_rt_vec_new()".into());
    out.push("declare i64 @skp_rt_vec_len(ptr)".into());
    out.push("declare void @skp_rt_vec_push(ptr, ptr)".into());
    out.push("declare ptr @skp_rt_vec_get(ptr, i64)".into());
    out.push("declare void @skp_rt_vec_set(ptr, i64, ptr)".into());
    out.push("declare ptr @skp_rt_vec_delete(ptr, i64)".into());
    out.push("declare ptr @skp_rt_struct_new(i64, i64)".into());
    out.push("declare ptr @skp_rt_struct_get(ptr, i64)".into());
    out.push("declare void @skp_rt_struct_set(ptr, i64, ptr)".into());
    emit_indirect_call_dispatch(program, out)?;
    Ok(())
}

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
    let argv = emit_boxed_arg_array(func, names, call.args, lines, counter, string_literals)?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_call_builtin(ptr {package_ptr}, ptr {name_ptr}, i64 {}, ptr {argv})",
        call.args.len()
    ));
    if call.ret_ty.is_void() {
        return Ok(());
    }
    let Some(dst) = call.dst else {
        return Err(CodegenError::InvalidIr(
            "non-void builtin call must write to a destination temp".into(),
        ));
    };
    emit_unbox_value(names, dst, call.ret_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_indirect_call(
    func: &IrFunction,
    names: &ValueNames,
    dst: Option<TempId>,
    ret_ty: &IrType,
    callee: &crate::ir::Operand,
    args: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let callee = operand_load(
        names,
        callee,
        func,
        lines,
        counter,
        &IrType::Fn {
            params: Vec::new(),
            ret: Box::new(ret_ty.clone()),
        },
        string_literals,
    )?;
    let argv = emit_boxed_arg_array(func, names, args, lines, counter, string_literals)?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @__skp_rt_call_function_dispatch(i32 {callee}, i64 {}, ptr {argv})",
        args.len()
    ));
    if ret_ty.is_void() {
        return Ok(());
    }
    let Some(dst) = dst else {
        return Err(CodegenError::InvalidIr(
            "non-void indirect call must write to a destination temp".into(),
        ));
    };
    emit_unbox_value(names, dst, ret_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_make_array(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    elem_ty: &IrType,
    items: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let dest = names.temp(dst)?;
    lines.push(format!(
        "  {dest} = call ptr @skp_rt_array_new(i64 {})",
        items.len()
    ));
    for (index, item) in items.iter().enumerate() {
        let boxed =
            emit_boxed_operand(func, names, item, elem_ty, lines, counter, string_literals)?;
        lines.push(format!(
            "  call void @skp_rt_array_set(ptr {dest}, i64 {index}, ptr {boxed})"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_make_array_repeat(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    elem_ty: &IrType,
    value: &crate::ir::Operand,
    size: usize,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let dest = names.temp(dst)?;
    let boxed = emit_boxed_operand(func, names, value, elem_ty, lines, counter, string_literals)?;
    lines.push(format!(
        "  {dest} = call ptr @skp_rt_array_repeat(ptr {boxed}, i64 {size})"
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_get(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let array = operand_load(
        names,
        array,
        func,
        lines,
        counter,
        &IrType::Array {
            elem: Box::new(elem_ty.clone()),
            size: 0,
        },
        string_literals,
    )?;
    let index = operand_load(
        names,
        index,
        func,
        lines,
        counter,
        &IrType::Int,
        string_literals,
    )?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_array_get(ptr {array}, i64 {index})"
    ));
    emit_unbox_value(names, dst, elem_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_set(
    func: &IrFunction,
    names: &ValueNames,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let array = operand_load(
        names,
        array,
        func,
        lines,
        counter,
        &IrType::Array {
            elem: Box::new(elem_ty.clone()),
            size: 0,
        },
        string_literals,
    )?;
    let index = operand_load(
        names,
        index,
        func,
        lines,
        counter,
        &IrType::Int,
        string_literals,
    )?;
    let boxed = emit_boxed_operand(func, names, value, elem_ty, lines, counter, string_literals)?;
    lines.push(format!(
        "  call void @skp_rt_array_set(ptr {array}, i64 {index}, ptr {boxed})"
    ));
    Ok(())
}

pub fn emit_vec_new(
    names: &ValueNames,
    dst: TempId,
    lines: &mut Vec<String>,
) -> Result<(), CodegenError> {
    let dest = names.temp(dst)?;
    lines.push(format!("  {dest} = call ptr @skp_rt_vec_new()"));
    Ok(())
}

pub fn emit_vec_len(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    vec: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let vec = operand_load(
        names,
        vec,
        func,
        lines,
        counter,
        &IrType::Vec {
            elem: Box::new(IrType::Unknown),
        },
        string_literals,
    )?;
    let dest = names.temp(dst)?;
    lines.push(format!("  {dest} = call i64 @skp_rt_vec_len(ptr {vec})"));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_vec_push(
    func: &IrFunction,
    names: &ValueNames,
    elem_ty: &IrType,
    vec: &crate::ir::Operand,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let elem_ty = if matches!(elem_ty, IrType::Unknown) {
        infer_operand_type(func, value)
    } else {
        elem_ty.clone()
    };
    let vec = operand_load(
        names,
        vec,
        func,
        lines,
        counter,
        &IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        },
        string_literals,
    )?;
    let boxed = emit_boxed_operand(
        func,
        names,
        value,
        &elem_ty,
        lines,
        counter,
        string_literals,
    )?;
    lines.push(format!(
        "  call void @skp_rt_vec_push(ptr {vec}, ptr {boxed})"
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_vec_get(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    elem_ty: &IrType,
    vec: &crate::ir::Operand,
    index: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let vec = operand_load(
        names,
        vec,
        func,
        lines,
        counter,
        &IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        },
        string_literals,
    )?;
    let index = operand_load(
        names,
        index,
        func,
        lines,
        counter,
        &IrType::Int,
        string_literals,
    )?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_vec_get(ptr {vec}, i64 {index})"
    ));
    emit_unbox_value(names, dst, elem_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_vec_set(
    func: &IrFunction,
    names: &ValueNames,
    elem_ty: &IrType,
    vec: &crate::ir::Operand,
    index: &crate::ir::Operand,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let vec = operand_load(
        names,
        vec,
        func,
        lines,
        counter,
        &IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        },
        string_literals,
    )?;
    let index = operand_load(
        names,
        index,
        func,
        lines,
        counter,
        &IrType::Int,
        string_literals,
    )?;
    let boxed = emit_boxed_operand(func, names, value, elem_ty, lines, counter, string_literals)?;
    lines.push(format!(
        "  call void @skp_rt_vec_set(ptr {vec}, i64 {index}, ptr {boxed})"
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_vec_delete(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    elem_ty: &IrType,
    vec: &crate::ir::Operand,
    index: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let vec = operand_load(
        names,
        vec,
        func,
        lines,
        counter,
        &IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        },
        string_literals,
    )?;
    let index = operand_load(
        names,
        index,
        func,
        lines,
        counter,
        &IrType::Int,
        string_literals,
    )?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_vec_delete(ptr {vec}, i64 {index})"
    ));
    emit_unbox_value(names, dst, elem_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_make_struct(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    struct_id: crate::ir::StructId,
    fields: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let struct_info = program
        .structs
        .iter()
        .find(|candidate| candidate.id == struct_id)
        .ok_or_else(|| CodegenError::InvalidIr(format!("unknown struct {:?}", struct_id)))?;
    let dest = names.temp(dst)?;
    lines.push(format!(
        "  {dest} = call ptr @skp_rt_struct_new(i64 {}, i64 {})",
        struct_id.0,
        fields.len()
    ));
    for (index, (field, field_info)) in fields.iter().zip(&struct_info.fields).enumerate() {
        let boxed = emit_boxed_operand(
            func,
            names,
            field,
            &field_info.ty,
            lines,
            counter,
            string_literals,
        )?;
        lines.push(format!(
            "  call void @skp_rt_struct_set(ptr {dest}, i64 {index}, ptr {boxed})"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_get(
    func: &IrFunction,
    names: &ValueNames,
    dst: TempId,
    ty: &IrType,
    base: &crate::ir::Operand,
    field: &crate::ir::FieldRef,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let base = operand_load(
        names,
        base,
        func,
        lines,
        counter,
        &IrType::Named(String::new()),
        string_literals,
    )?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @skp_rt_struct_get(ptr {base}, i64 {})",
        field.index
    ));
    emit_unbox_value(names, dst, ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_set(
    func: &IrFunction,
    names: &ValueNames,
    ty: &IrType,
    base: &crate::ir::Operand,
    field: &crate::ir::FieldRef,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let base = operand_load(
        names,
        base,
        func,
        lines,
        counter,
        &IrType::Named(String::new()),
        string_literals,
    )?;
    let boxed = emit_boxed_operand(func, names, value, ty, lines, counter, string_literals)?;
    lines.push(format!(
        "  call void @skp_rt_struct_set(ptr {base}, i64 {}, ptr {boxed})",
        field.index
    ));
    Ok(())
}

fn emit_boxed_arg_array(
    func: &IrFunction,
    names: &ValueNames,
    args: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<String, CodegenError> {
    let array = format!("%v{counter}");
    *counter += 1;
    let len = args.len().max(1);
    lines.push(format!("  {array} = alloca ptr, i64 {len}, align 8"));
    for (index, arg) in args.iter().enumerate() {
        let arg_ty = infer_operand_type(func, arg);
        let boxed = emit_boxed_operand(func, names, arg, &arg_ty, lines, counter, string_literals)?;
        let slot = format!("%v{counter}");
        *counter += 1;
        lines.push(format!(
            "  {slot} = getelementptr inbounds ptr, ptr {array}, i64 {index}"
        ));
        lines.push(format!("  store ptr {boxed}, ptr {slot}, align 8"));
    }
    Ok(array)
}

fn emit_boxed_operand(
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

fn emit_unbox_value(
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
            "  {dest} = call i32 @skp_rt_value_to_function(ptr {raw})"
        )),
        _ => {
            return Err(CodegenError::Unsupported(
                "unboxing is only implemented for Int/Float/Bool/String/Array/Vec/Struct/Function",
            ));
        }
    }
    Ok(())
}

fn infer_operand_type(func: &IrFunction, operand: &crate::ir::Operand) -> IrType {
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

fn emit_indirect_call_dispatch(
    program: &IrProgram,
    out: &mut Vec<String>,
) -> Result<(), CodegenError> {
    for func in &program.functions {
        out.extend(emit_indirect_wrapper(func)?);
        out.push(String::new());
    }

    out.push("define internal ptr @__skp_rt_call_function_dispatch(i32 %function, i64 %argc, ptr %argv) {".into());
    out.push("entry:".into());
    if program.functions.is_empty() {
        out.push("  %unit = call ptr @skp_rt_value_from_unit()".into());
        out.push("  ret ptr %unit".into());
        out.push("}".into());
        return Ok(());
    }
    let cases = program
        .functions
        .iter()
        .map(|func| format!("    i32 {}, label %case{}", func.id.0, func.id.0))
        .collect::<Vec<_>>()
        .join("\n");
    out.push(format!(
        "  switch i32 %function, label %default [\n{cases}\n  ]"
    ));
    for func in &program.functions {
        out.push(format!("case{}:", func.id.0));
        out.push(format!(
            "  %call{} = call ptr @__skp_rt_fnwrap_{}(i64 %argc, ptr %argv)",
            func.id.0, func.id.0
        ));
        out.push(format!("  ret ptr %call{}", func.id.0));
    }
    out.push("default:".into());
    out.push("  %unit = call ptr @skp_rt_value_from_unit()".into());
    out.push("  ret ptr %unit".into());
    out.push("}".into());
    Ok(())
}

fn emit_indirect_wrapper(func: &IrFunction) -> Result<Vec<String>, CodegenError> {
    let mut lines = vec![format!(
        "define internal ptr @__skp_rt_fnwrap_{}(i64 %argc, ptr %argv) {{",
        func.id.0
    )];
    lines.push("entry:".into());
    for (index, param) in func.params.iter().enumerate() {
        lines.push(format!(
            "  %argslot{index} = getelementptr inbounds ptr, ptr %argv, i64 {index}"
        ));
        lines.push(format!(
            "  %argraw{index} = load ptr, ptr %argslot{index}, align 8"
        ));
        match &param.ty {
            IrType::Int => lines.push(format!(
                "  %arg{index} = call i64 @skp_rt_value_to_int(ptr %argraw{index})"
            )),
            IrType::Float => lines.push(format!(
                "  %arg{index} = call double @skp_rt_value_to_float(ptr %argraw{index})"
            )),
            IrType::Bool => lines.push(format!(
                "  %arg{index} = call i1 @skp_rt_value_to_bool(ptr %argraw{index})"
            )),
            IrType::String => lines.push(format!(
                "  %arg{index} = call ptr @skp_rt_value_to_string(ptr %argraw{index})"
            )),
            IrType::Array { .. } => lines.push(format!(
                "  %arg{index} = call ptr @skp_rt_value_to_array(ptr %argraw{index})"
            )),
            IrType::Vec { .. } => lines.push(format!(
                "  %arg{index} = call ptr @skp_rt_value_to_vec(ptr %argraw{index})"
            )),
            IrType::Named(_) => lines.push(format!(
                "  %arg{index} = call ptr @skp_rt_value_to_struct(ptr %argraw{index})"
            )),
            IrType::Fn { .. } => lines.push(format!(
                "  %arg{index} = call i32 @skp_rt_value_to_function(ptr %argraw{index})"
            )),
            _ => {
                return Err(CodegenError::Unsupported(
                    "indirect-call trampoline only supports Int/Float/Bool/String/Named/Array/Vec/Fn/Void signatures",
                ));
            }
        }
    }
    let joined_args = func
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| Ok(format!("{} %arg{index}", llvm_ty(&param.ty)?)))
        .collect::<Result<Vec<_>, CodegenError>>()?
        .join(", ");
    if func.ret_ty.is_void() {
        lines.push(format!(
            "  call void {}({joined_args})",
            llvm_symbol(&func.name)
        ));
        lines.push("  %unit = call ptr @skp_rt_value_from_unit()".into());
        lines.push("  ret ptr %unit".into());
    } else {
        lines.push(format!(
            "  %ret = call {} {}({joined_args})",
            llvm_ty(&func.ret_ty)?,
            llvm_symbol(&func.name)
        ));
        let boxer = match &func.ret_ty {
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
                    "indirect-call trampoline only supports Int/Float/Bool/String/Named/Array/Vec/Fn/Void signatures",
                ));
            }
        };
        lines.push(format!(
            "  %boxed = call ptr @{boxer}({} %ret)",
            llvm_ty(&func.ret_ty)?
        ));
        lines.push("  ret ptr %boxed".into());
    }
    lines.push("}".into());
    Ok(lines)
}
