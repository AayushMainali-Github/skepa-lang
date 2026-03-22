use crate::codegen::CodegenError;
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::runtime_boxing::{
    emit_abort_if_error, emit_boxed_arg_array, emit_free_boxed_value, emit_free_boxed_values,
    emit_unbox_value, infer_operand_type,
};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_function_symbol, operand_load};
use crate::ir::{IrFunction, IrProgram, IrType, LoweredIrFunction, NativeCallLowering, TempId};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_indirect_call(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    dst: Option<TempId>,
    ret_ty: &IrType,
    callee: &crate::ir::Operand,
    args: &[crate::ir::Operand],
    lines: &mut Vec<String>,
    counter: &mut usize,
    lowered: &LoweredIrFunction,
    string_literals: &HashMap<String, String>,
) -> Result<(), CodegenError> {
    match lowered.operand_call_lowering(callee) {
        NativeCallLowering::KnownFunction(function) => {
            return calls::emit_direct_call(
                program,
                func,
                names,
                DirectCall {
                    dst,
                    ret_ty,
                    function,
                    args,
                },
                lines,
                counter,
                string_literals,
            );
        }
        NativeCallLowering::Dynamic => {}
    }
    let callee_ty = infer_operand_type(func, callee);
    let (callee_params, callee_ret) = match &callee_ty {
        IrType::Fn { params, ret } => (params.clone(), ret.as_ref().clone()),
        other => {
            return Err(CodegenError::InvalidIr(format!(
                "indirect callee must have function type, got {:?}",
                other
            )));
        }
    };
    if callee_ret != *ret_ty {
        return Err(CodegenError::InvalidIr(format!(
            "indirect call return type mismatch: callee returns {:?}, call expects {:?}",
            callee_ret, ret_ty
        )));
    }
    if callee_params.len() != args.len() {
        return Err(CodegenError::InvalidIr(format!(
            "indirect call arity mismatch: callee expects {}, got {}",
            callee_params.len(),
            args.len()
        )));
    }
    let callee = operand_load(
        names,
        callee,
        func,
        lines,
        counter,
        &callee_ty,
        string_literals,
    )?;
    let boxed_args = emit_boxed_arg_array(func, names, args, lines, counter, string_literals)?;
    let raw = format!("%v{counter}");
    *counter += 1;
    lines.push(format!(
        "  {raw} = call ptr @__skp_rt_call_function_dispatch(i32 {callee}, i64 {}, ptr {})",
        args.len(),
        boxed_args.array
    ));
    emit_abort_if_error(lines);
    emit_free_boxed_values(&boxed_args.values, lines);
    if ret_ty.is_void() {
        emit_free_boxed_value(&raw, lines);
        return Ok(());
    }
    let Some(dst) = dst else {
        return Err(CodegenError::InvalidIr(
            "non-void indirect call must write to a destination temp".into(),
        ));
    };
    emit_unbox_value(names, dst, ret_ty, &raw, lines)?;
    emit_free_boxed_value(&raw, lines);
    Ok(())
}

pub fn emit_indirect_call_dispatch(
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
    out.push("  %invalid = call ptr @skp_rt_call_function(i32 -1, i64 %argc, ptr %argv)".into());
    out.push("  ret ptr %invalid".into());
    out.push("}".into());
    Ok(())
}

fn emit_indirect_wrapper(func: &IrFunction) -> Result<Vec<String>, CodegenError> {
    let mut lines = vec![format!(
        "define internal ptr @__skp_rt_fnwrap_{}(i64 %argc, ptr %argv) {{",
        func.id.0
    )];
    lines.push("entry:".into());
    let argc_ok = format!("%argc_ok_{}", func.id.0);
    lines.push(format!(
        "  {argc_ok} = icmp eq i64 %argc, {}",
        func.params.len()
    ));
    lines.push(format!(
        "  br i1 {argc_ok}, label %argc_ok, label %argc_bad"
    ));
    lines.push("argc_bad:".into());
    lines.push(format!(
        "  %argc_err = call ptr @skp_rt_call_function(i32 {}, i64 %argc, ptr %argv)",
        func.id.0
    ));
    lines.push("  ret ptr %argc_err".into());
    lines.push("argc_ok:".into());
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
        lines.push("  call void @skp_rt_abort_if_error()".into());
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
            llvm_function_symbol(&func.name, &func.ret_ty)
        ));
        lines.push("  %unit = call ptr @skp_rt_value_from_unit()".into());
        lines.push("  ret ptr %unit".into());
    } else {
        lines.push(format!(
            "  %ret = call {} {}({joined_args})",
            llvm_ty(&func.ret_ty)?,
            llvm_function_symbol(&func.name, &func.ret_ty)
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
