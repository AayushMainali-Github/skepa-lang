use crate::codegen::CodegenError;
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::compare::{emit_compare, infer_compare_operand_type};
use crate::codegen::llvm::function;
use crate::codegen::llvm::module;
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::strings::collect_string_literals;
use crate::codegen::llvm::terminator;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_float_literal, llvm_symbol, operand_load};
use crate::ir::{BinaryOp, ConstValue, Instr, IrFunction, IrProgram, Operand, UnaryOp};
use std::collections::HashMap;

pub struct LlvmEmitter<'a> {
    program: &'a IrProgram,
    string_literals: HashMap<String, String>,
}

impl<'a> LlvmEmitter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self {
            program,
            string_literals: collect_string_literals(program),
        }
    }

    pub fn emit_program(&self) -> Result<String, CodegenError> {
        module::ensure_reserved_symbol_space(self.program)?;
        let mut out = vec![
            "; ModuleID = 'skepa'".to_string(),
            "source_filename = \"skepa\"".to_string(),
            String::new(),
        ];

        module::emit_globals(self.program, &mut out)?;

        module::emit_string_literal_storage(&self.string_literals, &mut out);

        if !self.string_literals.is_empty() || self.program.module_init.is_some() {
            if !self.string_literals.is_empty() {
                out.extend(module::emit_runtime_string_init(&self.string_literals)?);
                out.push(String::new());
            }
            let init_name =
                module::emit_module_initializer(self.program, &self.string_literals, &mut out)?;
            out.push(format!(
                "@llvm.global_ctors = appending global [1 x {{ i32, ptr, ptr }}] [{{ i32, ptr, ptr }} {{ i32 65535, ptr {}, ptr null }}]",
                llvm_symbol(&init_name)
            ));
            out.push(String::new());
        }

        runtime::emit_runtime_decls(self.program, &mut out)?;
        out.push(String::new());

        for func in &self.program.functions {
            out.extend(self.emit_function(func)?);
            out.push(String::new());
        }

        Ok(out.join("\n"))
    }

    fn emit_function(&self, func: &IrFunction) -> Result<Vec<String>, CodegenError> {
        function::validate_function_layout(func)?;
        let names = function::value_names(func);
        let mut lines = function::emit_function_header(func)?;

        let mut counter = 0usize;
        for (idx, block) in func.blocks.iter().enumerate() {
            function::begin_block(func, block, idx, &mut lines)?;
            for instr in &block.instrs {
                calls::ensure_supported(instr)?;
                runtime::ensure_supported(instr)?;
                self.emit_instr(func, &names, instr, &mut lines, &mut counter)?;
            }
            terminator::emit_terminator(
                func,
                &names,
                &block.terminator,
                &mut lines,
                &mut counter,
                &self.string_literals,
            )?;
        }

        function::finish_function(&mut lines);
        Ok(lines)
    }

    fn emit_instr(
        &self,
        func: &IrFunction,
        names: &ValueNames,
        instr: &Instr,
        lines: &mut Vec<String>,
        counter: &mut usize,
    ) -> Result<(), CodegenError> {
        match instr {
            Instr::Const { dst, ty, value } => {
                let dest = names.temp(*dst)?;
                match value {
                    ConstValue::Int(v) => {
                        lines.push(format!("  {dest} = add {} 0, {v}", llvm_ty(ty)?))
                    }
                    ConstValue::Float(v) => lines.push(format!(
                        "  {dest} = fadd {} 0.0, {}",
                        llvm_ty(ty)?,
                        llvm_float_literal(*v)
                    )),
                    ConstValue::Bool(v) => {
                        let int = if *v { 1 } else { 0 };
                        lines.push(format!("  {dest} = add {} 0, {int}", llvm_ty(ty)?));
                    }
                    ConstValue::String(_) => {
                        let value = operand_load(
                            names,
                            &Operand::Const(value.clone()),
                            func,
                            lines,
                            counter,
                            ty,
                            &self.string_literals,
                        )?;
                        lines.push(format!("  {dest} = bitcast ptr {value} to ptr"));
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "only Int/Float/Bool/String constants are supported",
                        ));
                    }
                }
            }
            Instr::Copy { dst, ty, src } => {
                let dest = names.temp(*dst)?;
                let value =
                    operand_load(names, src, func, lines, counter, ty, &self.string_literals)?;
                if matches!(
                    ty,
                    crate::ir::IrType::String
                        | crate::ir::IrType::Named(_)
                        | crate::ir::IrType::Array { .. }
                        | crate::ir::IrType::Vec { .. }
                ) {
                    lines.push(format!("  {dest} = bitcast ptr {value} to ptr"));
                } else if matches!(ty, crate::ir::IrType::Fn { .. }) {
                    lines.push(format!("  {dest} = add i32 0, {value}"));
                } else if matches!(ty, crate::ir::IrType::Float) {
                    lines.push(format!("  {dest} = fadd {} 0.0, {value}", llvm_ty(ty)?));
                } else {
                    lines.push(format!("  {dest} = add {} 0, {value}", llvm_ty(ty)?));
                }
            }
            Instr::Unary {
                dst,
                ty,
                op,
                operand,
            } => {
                let dest = names.temp(*dst)?;
                let value = operand_load(
                    names,
                    operand,
                    func,
                    lines,
                    counter,
                    ty,
                    &self.string_literals,
                )?;
                match (op, ty) {
                    (UnaryOp::Neg, crate::ir::IrType::Int) => {
                        lines.push(format!("  {dest} = sub i64 0, {value}"));
                    }
                    (UnaryOp::Neg, crate::ir::IrType::Float) => {
                        lines.push(format!("  {dest} = fneg double {value}"));
                    }
                    (UnaryOp::Not, crate::ir::IrType::Bool) => {
                        lines.push(format!("  {dest} = xor i1 {value}, true"));
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "unsupported unary op/type in LLVM lowering",
                        ));
                    }
                }
            }
            Instr::Binary {
                dst,
                ty,
                op,
                left,
                right,
            } => {
                let dest = names.temp(*dst)?;
                let left =
                    operand_load(names, left, func, lines, counter, ty, &self.string_literals)?;
                let right = operand_load(
                    names,
                    right,
                    func,
                    lines,
                    counter,
                    ty,
                    &self.string_literals,
                )?;
                let opname = match (op, ty) {
                    (BinaryOp::Add, crate::ir::IrType::Float) => "fadd",
                    (BinaryOp::Sub, crate::ir::IrType::Float) => "fsub",
                    (BinaryOp::Mul, crate::ir::IrType::Float) => "fmul",
                    (BinaryOp::Div, crate::ir::IrType::Float) => "fdiv",
                    (BinaryOp::Mod, crate::ir::IrType::Float) => {
                        return Err(CodegenError::Unsupported(
                            "float modulo is not implemented in LLVM lowering",
                        ));
                    }
                    (BinaryOp::Add, _) => "add",
                    (BinaryOp::Sub, _) => "sub",
                    (BinaryOp::Mul, _) => "mul",
                    (BinaryOp::Div, _) => "sdiv",
                    (BinaryOp::Mod, _) => "srem",
                };
                lines.push(format!(
                    "  {dest} = {opname} {} {left}, {right}",
                    llvm_ty(ty)?
                ));
            }
            Instr::Compare {
                dst,
                op,
                left,
                right,
            } => {
                let dest = names.temp(*dst)?;
                let compare_ty = infer_compare_operand_type(self.program, func, left, right);
                emit_compare(
                    names,
                    func,
                    &self.string_literals,
                    dest,
                    *op,
                    left,
                    right,
                    &compare_ty,
                    lines,
                    counter,
                )?;
            }
            Instr::LoadGlobal { dst, ty, global } => {
                let dest = names.temp(*dst)?;
                lines.push(format!(
                    "  {dest} = load {}, ptr @g{}, align 8",
                    llvm_ty(ty)?,
                    global.0
                ));
            }
            Instr::StoreGlobal { global, ty, value } => {
                let value = operand_load(
                    names,
                    value,
                    func,
                    lines,
                    counter,
                    ty,
                    &self.string_literals,
                )?;
                lines.push(format!(
                    "  store {} {value}, ptr @g{}, align 8",
                    llvm_ty(ty)?,
                    global.0
                ));
            }
            Instr::LoadLocal { dst, ty, local } => {
                let dest = names.temp(*dst)?;
                lines.push(format!(
                    "  {dest} = load {}, ptr %local{}, align 8",
                    llvm_ty(ty)?,
                    local.0
                ));
            }
            Instr::StoreLocal { local, ty, value } => {
                let value = operand_load(
                    names,
                    value,
                    func,
                    lines,
                    counter,
                    ty,
                    &self.string_literals,
                )?;
                lines.push(format!(
                    "  store {} {value}, ptr %local{}, align 8",
                    llvm_ty(ty)?,
                    local.0
                ));
            }
            Instr::Logic { .. } => {
                return Err(CodegenError::Unsupported(
                    "Logic instructions should be lowered to control flow before LLVM emission",
                ));
            }
            Instr::CallDirect {
                dst,
                ret_ty,
                function,
                args,
            } => {
                calls::emit_direct_call(
                    self.program,
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
                    &self.string_literals,
                )?;
            }
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
                    &self.string_literals,
                )?;
            }
            Instr::MakeClosure { dst, function } => {
                let dest = names.temp(*dst)?;
                lines.push(format!("  {dest} = add i32 0, {}", function.0));
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
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
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
                    *dst,
                    elem_ty,
                    array,
                    index,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
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
                    elem_ty,
                    array,
                    index,
                    value,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
            }
            Instr::VecNew { dst, .. } => {
                runtime::emit_vec_new(names, *dst, lines)?;
            }
            Instr::VecLen { dst, vec } => {
                runtime::emit_vec_len(
                    func,
                    names,
                    *dst,
                    vec,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
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
                    &self.string_literals,
                )?;
            }
            Instr::MakeStruct {
                dst,
                struct_id,
                fields,
            } => {
                runtime::emit_make_struct(
                    self.program,
                    func,
                    names,
                    *dst,
                    *struct_id,
                    fields,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
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
                    *dst,
                    ty,
                    base,
                    field,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
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
                    ty,
                    base,
                    field,
                    value,
                    lines,
                    counter,
                    &self.string_literals,
                )?;
            }
        }
        Ok(())
    }
}
