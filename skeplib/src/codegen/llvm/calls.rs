use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_function_symbol, operand_load};
use crate::ir::{Instr, IrFunction, IrProgram, IrType, Operand, TempId};
use std::collections::HashMap;

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    let _ = instr;
    Ok(())
}

pub struct DirectCall<'a> {
    pub dst: Option<TempId>,
    pub ret_ty: &'a IrType,
    pub function: crate::ir::FunctionId,
    pub args: &'a [Operand],
}

pub fn emit_direct_call(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    call: DirectCall<'_>,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    let callee = program
        .functions
        .iter()
        .find(|candidate| candidate.id == call.function)
        .ok_or_else(|| CodegenError::InvalidIr(format!("unknown callee {:?}", call.function)))?;

    if callee.params.len() != call.args.len() {
        return Err(CodegenError::InvalidIr(format!(
            "call arity mismatch for {}",
            callee.name
        )));
    }
    if callee.ret_ty != *call.ret_ty {
        return Err(CodegenError::InvalidIr(format!(
            "call return type mismatch for {}: callee returns {:?}, call expects {:?}",
            callee.name, callee.ret_ty, call.ret_ty
        )));
    }

    let mut lowered_args = Vec::with_capacity(call.args.len());
    for (arg, param) in call.args.iter().zip(&callee.params) {
        let value = operand_load(names, arg, func, lines, counter, &param.ty, string_literals)?;
        lowered_args.push(format!("{} {value}", llvm_ty(&param.ty)?));
    }
    let joined_args = lowered_args.join(", ");
    let ret_llvm_ty = llvm_ty(&callee.ret_ty)?;

    if callee.ret_ty.is_void() {
        lines.push(format!(
            "  call {ret_llvm_ty} {}({joined_args})",
            llvm_function_symbol(&callee.name, &callee.ret_ty)
        ));
        return Ok(());
    }

    let Some(dst) = call.dst else {
        return Err(CodegenError::InvalidIr(
            "non-void direct call must write to a destination temp".into(),
        ));
    };
    let dest = names.temp(dst)?;
    lines.push(format!(
        "  {dest} = call {ret_llvm_ty} {}({joined_args})",
        llvm_function_symbol(&callee.name, &callee.ret_ty)
    ));
    Ok(())
}
