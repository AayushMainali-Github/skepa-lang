use crate::codegen::CodegenError;
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{
    Instr, IrFunction, IrProgram, LoweredIrFunction, NativeArrayPlan, NativeStructPlan,
};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_core_instr(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    lowered: &LoweredIrFunction,
    instr: &Instr,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<bool, CodegenError> {
    match instr {
        Instr::LoadGlobal { dst, ty, global } => {
            let dest = names.temp(*dst)?;
            lines.push(format!(
                "  {dest} = load {}, ptr @g{}, align 8",
                llvm_ty(ty)?,
                global.0
            ));
            Ok(true)
        }
        Instr::StoreGlobal { global, ty, value } => {
            let value = operand_load(names, value, func, lines, counter, ty, string_literals)?;
            lines.push(format!(
                "  store {} {value}, ptr @g{}, align 8",
                llvm_ty(ty)?,
                global.0
            ));
            Ok(true)
        }
        Instr::LoadLocal { dst, ty, local } => {
            let dest = names.temp(*dst)?;
            lines.push(format!(
                "  {dest} = load {}, ptr %local{}, align 8",
                llvm_ty(ty)?,
                local.0
            ));
            Ok(true)
        }
        Instr::StoreLocal { local, ty, value } => {
            if let Some(kind) = lowered.array_local(*local) {
                match kind {
                    NativeArrayPlan::IntRepeat { size, init } => {
                        if matches!(value, crate::ir::Operand::Temp(temp) if lowered.temp_root(*temp) == Some(*local))
                        {
                            let init = operand_load(
                                names,
                                &init,
                                func,
                                lines,
                                counter,
                                &crate::ir::IrType::Int,
                                string_literals,
                            )?;
                            for index in 0..size {
                                lines.push(format!(
                                    "  store i64 {init}, ptr %local{}_elem{}, align 8",
                                    local.0, index
                                ));
                            }
                            return Ok(true);
                        }
                    }
                    NativeArrayPlan::FloatRepeat { size, init } => {
                        if matches!(value, crate::ir::Operand::Temp(temp) if lowered.temp_root(*temp) == Some(*local))
                        {
                            let init = operand_load(
                                names,
                                &init,
                                func,
                                lines,
                                counter,
                                &crate::ir::IrType::Float,
                                string_literals,
                            )?;
                            for index in 0..size {
                                lines.push(format!(
                                    "  %v{counter} = getelementptr inbounds [{} x double], ptr %local{}_data, i64 0, i64 {}",
                                    size,
                                    local.0,
                                    index
                                ));
                                lines.push(format!(
                                    "  store double {init}, ptr %v{counter}, align 8"
                                ));
                                *counter += 1;
                            }
                            return Ok(true);
                        }
                    }
                    NativeArrayPlan::StringItems { size, items } => {
                        if matches!(value, crate::ir::Operand::Temp(temp) if lowered.temp_root(*temp) == Some(*local))
                        {
                            for (index, item) in items.iter().enumerate().take(size) {
                                let loaded = operand_load(
                                    names,
                                    item,
                                    func,
                                    lines,
                                    counter,
                                    &crate::ir::IrType::String,
                                    string_literals,
                                )?;
                                lines.push(format!(
                                    "  store ptr {loaded}, ptr %local{}_elem{}, align 8",
                                    local.0, index
                                ));
                            }
                            return Ok(true);
                        }
                    }
                }
            }
            if let Some(kind) = lowered.struct_local(*local) {
                match kind {
                    NativeStructPlan::ScalarFields { fields } => {
                        if matches!(value, crate::ir::Operand::Temp(temp) if lowered.temp_root(*temp) == Some(*local))
                        {
                            for (index, field) in fields.iter().enumerate() {
                                let loaded = operand_load(
                                    names,
                                    field,
                                    func,
                                    lines,
                                    counter,
                                    &crate::ir::IrType::Int,
                                    string_literals,
                                )?;
                                lines.push(format!(
                                    "  store i64 {loaded}, ptr %local{}_field{}, align 8",
                                    local.0, index
                                ));
                            }
                            return Ok(true);
                        }
                    }
                    NativeStructPlan::Alias { root } => {
                        if matches!(value, crate::ir::Operand::Local(src) if lowered.root_struct_local(*src) == Some(root))
                        {
                            return Ok(true);
                        }
                    }
                }
            }
            let value = operand_load(names, value, func, lines, counter, ty, string_literals)?;
            lines.push(format!(
                "  store {} {value}, ptr %local{}, align 8",
                llvm_ty(ty)?,
                local.0
            ));
            Ok(true)
        }
        Instr::CallDirect {
            dst,
            ret_ty,
            function,
            args,
        } => {
            calls::emit_direct_call(
                program,
                func,
                names,
                DirectCall {
                    dst: *dst,
                    ret_ty,
                    function: *function,
                    args,
                },
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
