use crate::codegen::CodegenError;
use crate::codegen::llvm::runtime_builtins;
use crate::codegen::llvm::runtime_containers;
use crate::codegen::llvm::runtime_decls::runtime_declarations;
use crate::codegen::llvm::runtime_indirect;
use crate::codegen::llvm::value::ValueNames;
use crate::ir::Instr;
use crate::ir::{IrFunction, IrProgram, IrType, NativeAggregatePlan, TempId};
use std::collections::HashMap;

pub use crate::codegen::llvm::runtime_builtins::BuiltinCallInstr;

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    let _ = instr;
    Ok(())
}

pub fn emit_runtime_decls(program: &IrProgram, out: &mut Vec<String>) -> Result<(), CodegenError> {
    for (_, decl) in runtime_declarations() {
        out.push((*decl).into());
    }
    runtime_indirect::emit_indirect_call_dispatch(program, out)?;
    Ok(())
}

pub fn emit_builtin_call(
    func: &IrFunction,
    names: &ValueNames,
    call: BuiltinCallInstr<'_>,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    runtime_builtins::emit_builtin_call(func, names, call, lines, counter, string_literals)
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
    runtime_indirect::emit_indirect_call(
        func,
        names,
        dst,
        ret_ty,
        callee,
        args,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_make_array(
        func,
        names,
        dst,
        elem_ty,
        items,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_make_array_repeat(
        func,
        names,
        dst,
        elem_ty,
        value,
        size,
        lines,
        counter,
        string_literals,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_get(
    func: &IrFunction,
    names: &ValueNames,
    native: &NativeAggregatePlan,
    dst: TempId,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    runtime_containers::emit_array_get(
        func,
        names,
        native,
        dst,
        elem_ty,
        array,
        index,
        lines,
        counter,
        string_literals,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn emit_array_set(
    func: &IrFunction,
    names: &ValueNames,
    native: &NativeAggregatePlan,
    elem_ty: &IrType,
    array: &crate::ir::Operand,
    index: &crate::ir::Operand,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    runtime_containers::emit_array_set(
        func,
        names,
        native,
        elem_ty,
        array,
        index,
        value,
        lines,
        counter,
        string_literals,
    )
}

pub fn emit_vec_new(
    names: &ValueNames,
    dst: TempId,
    lines: &mut Vec<String>,
) -> Result<(), CodegenError> {
    runtime_containers::emit_vec_new(names, dst, lines)
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
    runtime_containers::emit_vec_len(func, names, dst, vec, lines, counter, string_literals)
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
    runtime_containers::emit_vec_push(
        func,
        names,
        elem_ty,
        vec,
        value,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_vec_get(
        func,
        names,
        dst,
        elem_ty,
        vec,
        index,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_vec_set(
        func,
        names,
        elem_ty,
        vec,
        index,
        value,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_vec_delete(
        func,
        names,
        dst,
        elem_ty,
        vec,
        index,
        lines,
        counter,
        string_literals,
    )
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
    runtime_containers::emit_make_struct(
        program,
        func,
        names,
        dst,
        struct_id,
        fields,
        lines,
        counter,
        string_literals,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_get(
    func: &IrFunction,
    names: &ValueNames,
    native: &NativeAggregatePlan,
    dst: TempId,
    ty: &IrType,
    base: &crate::ir::Operand,
    field: &crate::ir::FieldRef,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    runtime_containers::emit_struct_get(
        func,
        names,
        native,
        dst,
        ty,
        base,
        field,
        lines,
        counter,
        string_literals,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn emit_struct_set(
    func: &IrFunction,
    names: &ValueNames,
    native: &NativeAggregatePlan,
    ty: &IrType,
    base: &crate::ir::Operand,
    field: &crate::ir::FieldRef,
    value: &crate::ir::Operand,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    runtime_containers::emit_struct_set(
        func,
        names,
        native,
        ty,
        base,
        field,
        value,
        lines,
        counter,
        string_literals,
    )
}
