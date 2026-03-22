use crate::codegen::CodegenError;
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::special_locals::{SpecialLocalKind, SpecialLocals};
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, operand_load};
use crate::ir::{Instr, IrFunction, IrProgram};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_core_instr(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    special: &SpecialLocals,
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
            if let Some(kind) = special.local(*local) {
                match kind {
                    SpecialLocalKind::ScalarStruct { fields } => {
                        if matches!(value, crate::ir::Operand::Temp(temp) if special.temp_root(*temp) == Some(*local))
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
                    SpecialLocalKind::StructAlias { root } => {
                        if matches!(value, crate::ir::Operand::Local(src) if special.root_struct_local(*src) == Some(*root))
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
