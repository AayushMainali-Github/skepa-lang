use crate::codegen::CodegenError;
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::value::ValueNames;
use crate::ir::{Instr, IrFunction, IrProgram, NativeAggregatePlan};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn emit_runtime_instr(
    program: &IrProgram,
    func: &IrFunction,
    names: &ValueNames,
    native: &NativeAggregatePlan,
    instr: &Instr,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<bool, CodegenError> {
    match instr {
        Instr::CallBuiltin {
            dst,
            ret_ty,
            builtin,
            args,
        } => {
            runtime::emit_builtin_call(
                func,
                names,
                runtime::BuiltinCallInstr {
                    dst: *dst,
                    ret_ty,
                    builtin,
                    args,
                },
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::MakeClosure { dst, function } => {
            let dest = names.temp(*dst)?;
            lines.push(format!("  {dest} = add i32 0, {}", function.0));
            Ok(true)
        }
        Instr::CallIndirect {
            dst,
            ret_ty,
            callee,
            args,
        } => {
            runtime::emit_indirect_call(
                func,
                names,
                *dst,
                ret_ty,
                callee,
                args,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::MakeArray {
            dst,
            elem_ty,
            items,
        } => {
            runtime::emit_make_array(
                func,
                names,
                *dst,
                elem_ty,
                items,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::MakeArrayRepeat {
            dst,
            elem_ty,
            value,
            size,
        } => {
            runtime::emit_make_array_repeat(
                func,
                names,
                *dst,
                elem_ty,
                value,
                *size,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::ArrayGet {
            dst,
            elem_ty,
            array,
            index,
        } => {
            runtime::emit_array_get(
                func,
                names,
                native,
                *dst,
                elem_ty,
                array,
                index,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::ArraySet {
            elem_ty,
            array,
            index,
            value,
        } => {
            runtime::emit_array_set(
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
            )?;
            Ok(true)
        }
        Instr::VecNew { dst, .. } => {
            runtime::emit_vec_new(names, *dst, lines)?;
            Ok(true)
        }
        Instr::VecLen { dst, vec } => {
            runtime::emit_vec_len(func, names, *dst, vec, lines, counter, string_literals)?;
            Ok(true)
        }
        Instr::VecPush { vec, value } => {
            runtime::emit_vec_push(
                func,
                names,
                &crate::ir::IrType::Unknown,
                vec,
                value,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::VecGet {
            dst,
            elem_ty,
            vec,
            index,
        } => {
            runtime::emit_vec_get(
                func,
                names,
                *dst,
                elem_ty,
                vec,
                index,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::VecSet {
            elem_ty,
            vec,
            index,
            value,
        } => {
            runtime::emit_vec_set(
                func,
                names,
                elem_ty,
                vec,
                index,
                value,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::VecDelete {
            dst,
            elem_ty,
            vec,
            index,
        } => {
            runtime::emit_vec_delete(
                func,
                names,
                *dst,
                elem_ty,
                vec,
                index,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::MakeStruct {
            dst,
            struct_id,
            fields,
        } => {
            runtime::emit_make_struct(
                program,
                func,
                names,
                *dst,
                *struct_id,
                fields,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::StructGet {
            dst,
            ty,
            base,
            field,
        } => {
            runtime::emit_struct_get(
                func,
                names,
                native,
                *dst,
                ty,
                base,
                field,
                lines,
                counter,
                string_literals,
            )?;
            Ok(true)
        }
        Instr::StructSet {
            base,
            field,
            value,
            ty,
        } => {
            runtime::emit_struct_set(
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
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
