use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_symbol, operand_load};
use crate::ir::{Instr, IrFunction, IrProgram, IrType, Operand, TempId};

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    match instr {
        Instr::CallIndirect { .. } | Instr::CallBuiltin { .. } => Err(CodegenError::Unsupported(
            "indirect and builtin calls are not lowered yet",
        )),
        _ => Ok(()),
    }
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

    let mut lowered_args = Vec::with_capacity(call.args.len());
    for (arg, param) in call.args.iter().zip(&callee.params) {
        let value = operand_load(names, arg, func, lines, counter, &param.ty)?;
        lowered_args.push(format!("{} {value}", llvm_ty(&param.ty)?));
    }
    let joined_args = lowered_args.join(", ");
    let ret_llvm_ty = llvm_ty(call.ret_ty)?;

    if call.ret_ty.is_void() {
        lines.push(format!(
            "  call {ret_llvm_ty} {}({joined_args})",
            llvm_symbol(&callee.name)
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
        llvm_symbol(&callee.name)
    ));
    Ok(())
}
