use crate::codegen::CodegenError;
use crate::codegen::llvm::strings::{encode_c_string, runtime_string_symbol};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{llvm_float_literal, llvm_symbol};
use crate::ir::{ConstValue, IrProgram, Operand};
use std::collections::HashMap;

pub const RESERVED_LLVM_HELPER_PREFIXES: &[&str] = &["__skp_codegen_", "__skp_rt_", "__skp_init_"];

pub fn ensure_reserved_symbol_space(program: &IrProgram) -> Result<(), CodegenError> {
    for func in &program.functions {
        if RESERVED_LLVM_HELPER_PREFIXES
            .iter()
            .any(|prefix| func.name.starts_with(prefix))
        {
            return Err(CodegenError::InvalidIr(format!(
                "function {} uses reserved LLVM helper prefix",
                func.name
            )));
        }
    }
    Ok(())
}

pub fn emit_globals(program: &IrProgram, out: &mut Vec<String>) -> Result<(), CodegenError> {
    for global in &program.globals {
        let init = match &global.init {
            Some(Operand::Const(ConstValue::Int(v)))
                if matches!(global.ty, crate::ir::IrType::Int) =>
            {
                v.to_string()
            }
            Some(Operand::Const(ConstValue::Bool(v)))
                if matches!(global.ty, crate::ir::IrType::Bool) =>
            {
                if *v {
                    "1".into()
                } else {
                    "0".into()
                }
            }
            Some(Operand::Const(ConstValue::Float(v)))
                if matches!(global.ty, crate::ir::IrType::Float) =>
            {
                llvm_float_literal(*v)
            }
            Some(_) | None => match global.ty {
                crate::ir::IrType::Int | crate::ir::IrType::Bool => "0".into(),
                crate::ir::IrType::Float => "0.0".into(),
                crate::ir::IrType::String
                | crate::ir::IrType::Named(_)
                | crate::ir::IrType::Opaque(_)
                | crate::ir::IrType::Array { .. }
                | crate::ir::IrType::Vec { .. } => "null".into(),
                _ => {
                    return Err(CodegenError::Unsupported(
                        "only scalar and runtime-backed pointer globals are supported in current LLVM lowering",
                    ));
                }
            },
        };
        out.push(format!(
            "@g{} = global {} {}, align 8",
            global.id.0,
            llvm_ty(&global.ty)?,
            init
        ));
    }
    if !program.globals.is_empty() {
        out.push(String::new());
    }
    Ok(())
}

pub fn emit_string_literal_storage(
    string_literals: &HashMap<String, String>,
    out: &mut Vec<String>,
) {
    if string_literals.is_empty() {
        return;
    }
    for (value, name) in string_literals {
        let bytes = encode_c_string(value);
        out.push(format!(
            "{name} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
            value.len() + 1,
            bytes
        ));
        out.push(format!(
            "{} = internal global ptr null, align 8",
            runtime_string_symbol(name)
        ));
    }
    out.push(String::new());
}

pub fn emit_runtime_string_init(
    string_literals: &HashMap<String, String>,
) -> Result<Vec<String>, CodegenError> {
    let mut lines = vec!["define internal void @\"__skp_init_runtime_strings\"() {".into()];
    lines.push("entry:".into());
    let mut counter = 0usize;
    for (value, name) in string_literals {
        let gep = format!("%v{counter}");
        counter += 1;
        let bytes = value.len() + 1;
        lines.push(format!(
            "  {gep} = getelementptr inbounds [{bytes} x i8], ptr {name}, i64 0, i64 0"
        ));
        let string = format!("%v{counter}");
        counter += 1;
        lines.push(format!(
            "  {string} = call ptr @skp_rt_string_from_utf8(ptr {gep}, i64 {})",
            value.len()
        ));
        lines.push(format!(
            "  store ptr {string}, ptr {}, align 8",
            runtime_string_symbol(name)
        ));
        lines.push("  call void @skp_rt_abort_if_error()".into());
    }
    lines.push("  ret void".into());
    lines.push("}".into());
    Ok(lines)
}

pub fn emit_module_initializer(
    program: &IrProgram,
    string_literals: &HashMap<String, String>,
    out: &mut Vec<String>,
) -> Result<String, CodegenError> {
    let init_name = "__skp_codegen_init".to_string();
    out.push(format!(
        "define internal void {}() {{",
        llvm_symbol(&init_name)
    ));
    out.push("entry:".into());
    if !string_literals.is_empty() {
        out.push(format!(
            "  call void {}()",
            llvm_symbol("__skp_init_runtime_strings")
        ));
    }
    if let Some(module_init) = &program.module_init {
        let init = program
            .functions
            .iter()
            .find(|func| func.id == module_init.function)
            .ok_or_else(|| {
                CodegenError::InvalidIr(format!(
                    "module_init points at missing function {:?}",
                    module_init.function
                ))
            })?;
        out.push(format!("  call void {}()", llvm_symbol(&init.name)));
    }
    out.push("  ret void".into());
    out.push("}".into());
    Ok(init_name)
}
