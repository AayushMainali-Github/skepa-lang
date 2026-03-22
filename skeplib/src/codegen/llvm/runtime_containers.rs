use crate::codegen::CodegenError;
use crate::codegen::llvm::runtime_boxing::{
    emit_abort_if_error, emit_boxed_operand, emit_unbox_value, infer_operand_type,
};
use crate::codegen::llvm::special_locals::{SpecialLocalKind, SpecialLocals};
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{IrFunction, IrProgram, IrType, TempId};
use std::collections::HashMap;

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
        emit_abort_if_error(lines);
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
    emit_abort_if_error(lines);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_get(
    func: &IrFunction,
    names: &ValueNames,
    special: &SpecialLocals,
    dst: TempId,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    if *elem_ty == IrType::Int
        && let crate::ir::Operand::Local(local) = array
        && let Some(SpecialLocalKind::IntArray { size, .. }) = special.local(*local)
    {
        let dest = names.temp(dst)?;
        match index {
            crate::ir::Operand::Const(crate::ir::ConstValue::Int(idx))
                if *idx >= 0 && (*idx as usize) < *size =>
            {
                lines.push(format!(
                    "  {dest} = load i64, ptr %local{}_elem{}, align 8",
                    local.0, idx
                ));
                return Ok(());
            }
            _ => {
                let idx = operand_load(
                    names,
                    index,
                    func,
                    lines,
                    counter,
                    &IrType::Int,
                    string_literals,
                )?;
                let dispatch = format!("array_get_dispatch_{counter}");
                *counter += 1;
                let oob = format!("array_get_oob_{counter}");
                *counter += 1;
                let join = format!("array_get_join_{counter}");
                *counter += 1;
                let mut incoming = Vec::new();
                lines.push(format!("  br label %{dispatch}"));
                lines.push(format!("{dispatch}:"));
                lines.push(format!("  switch i64 {idx}, label %{oob} ["));
                for slot in 0..*size {
                    lines.push(format!(
                        "    i64 {slot}, label %array_get_case_{counter}_{slot}"
                    ));
                }
                lines.push("  ]".into());
                let case_tag = *counter;
                *counter += 1;
                for slot in 0..*size {
                    let case_label = format!("array_get_case_{case_tag}_{slot}");
                    let value = format!("%v{counter}");
                    *counter += 1;
                    lines.push(format!("{case_label}:"));
                    lines.push(format!(
                        "  {value} = load i64, ptr %local{}_elem{}, align 8",
                        local.0, slot
                    ));
                    lines.push(format!("  br label %{join}"));
                    incoming.push((value, case_label));
                }
                let array_ptr = format!("%v{counter}");
                *counter += 1;
                let raw = format!("%v{counter}");
                *counter += 1;
                let fallback = format!("%v{counter}");
                *counter += 1;
                lines.push(format!("{oob}:"));
                lines.push(format!(
                    "  {array_ptr} = load ptr, ptr %local{}, align 8",
                    local.0
                ));
                lines.push(format!(
                    "  {raw} = call ptr @skp_rt_array_get(ptr {array_ptr}, i64 {idx})"
                ));
                emit_abort_if_error(lines);
                lines.push(format!(
                    "  {fallback} = call i64 @skp_rt_value_to_int(ptr {raw})"
                ));
                emit_abort_if_error(lines);
                lines.push(format!("  br label %{join}"));
                lines.push(format!("{join}:"));
                let phi = incoming
                    .into_iter()
                    .map(|(value, label)| format!("[ {value}, %{label} ]"))
                    .chain(std::iter::once(format!("[ {fallback}, %{oob} ]")))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("  {dest} = phi i64 {phi}"));
                return Ok(());
            }
        }
    }
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
    emit_abort_if_error(lines);
    emit_unbox_value(names, dst, elem_ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_set(
    func: &IrFunction,
    names: &ValueNames,
    special: &SpecialLocals,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    if *elem_ty == IrType::Int
        && let crate::ir::Operand::Local(local) = array
        && let Some(SpecialLocalKind::IntArray { size, .. }) = special.local(*local)
    {
        let stored = operand_load(
            names,
            value,
            func,
            lines,
            counter,
            &IrType::Int,
            string_literals,
        )?;
        match index {
            crate::ir::Operand::Const(crate::ir::ConstValue::Int(idx))
                if *idx >= 0 && (*idx as usize) < *size =>
            {
                lines.push(format!(
                    "  store i64 {stored}, ptr %local{}_elem{}, align 8",
                    local.0, idx
                ));
                return Ok(());
            }
            _ => {
                let idx = operand_load(
                    names,
                    index,
                    func,
                    lines,
                    counter,
                    &IrType::Int,
                    string_literals,
                )?;
                let dispatch = format!("array_set_dispatch_{counter}");
                *counter += 1;
                let oob = format!("array_set_oob_{counter}");
                *counter += 1;
                let join = format!("array_set_join_{counter}");
                *counter += 1;
                lines.push(format!("  br label %{dispatch}"));
                lines.push(format!("{dispatch}:"));
                lines.push(format!("  switch i64 {idx}, label %{oob} ["));
                for slot in 0..*size {
                    lines.push(format!(
                        "    i64 {slot}, label %array_set_case_{counter}_{slot}"
                    ));
                }
                lines.push("  ]".into());
                let case_tag = *counter;
                *counter += 1;
                for slot in 0..*size {
                    let case_label = format!("array_set_case_{case_tag}_{slot}");
                    lines.push(format!("{case_label}:"));
                    lines.push(format!(
                        "  store i64 {stored}, ptr %local{}_elem{}, align 8",
                        local.0, slot
                    ));
                    lines.push(format!("  br label %{join}"));
                }
                let boxed = format!("%v{counter}");
                *counter += 1;
                let array_ptr = format!("%v{counter}");
                *counter += 1;
                lines.push(format!("{oob}:"));
                lines.push(format!(
                    "  {boxed} = call ptr @skp_rt_value_from_int(i64 {stored})"
                ));
                lines.push(format!(
                    "  {array_ptr} = load ptr, ptr %local{}, align 8",
                    local.0
                ));
                lines.push(format!(
                    "  call void @skp_rt_array_set(ptr {array_ptr}, i64 {idx}, ptr {boxed})"
                ));
                emit_abort_if_error(lines);
                lines.push(format!("  br label %{join}"));
                lines.push(format!("{join}:"));
                return Ok(());
            }
        }
    }
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
    emit_abort_if_error(lines);
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
    emit_abort_if_error(lines);
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
    emit_abort_if_error(lines);
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
    emit_abort_if_error(lines);
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
    emit_abort_if_error(lines);
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
        emit_abort_if_error(lines);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_get(
    func: &IrFunction,
    names: &ValueNames,
    special: &SpecialLocals,
    dst: TempId,
    ty: &IrType,
    base: &crate::ir::Operand,
    field: &crate::ir::FieldRef,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    if *ty == IrType::Int
        && let crate::ir::Operand::Local(local) = base
        && let Some(root) = special.root_struct_local(*local)
    {
        let slot = format!("%v{counter}");
        *counter += 1;
        let dest = names.temp(dst)?;
        lines.push(format!(
            "  {slot} = load i64, ptr %local{}_field{}, align 8",
            root.0, field.index
        ));
        lines.push(format!("  {dest} = add i64 0, {slot}"));
        return Ok(());
    }
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
    emit_abort_if_error(lines);
    emit_unbox_value(names, dst, ty, &raw, lines)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_set(
    func: &IrFunction,
    names: &ValueNames,
    _special: &SpecialLocals,
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
    emit_abort_if_error(lines);
    Ok(())
}
