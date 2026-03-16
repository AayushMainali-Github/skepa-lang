use crate::codegen::CodegenError;
use crate::codegen::llvm::block::{branch_targets, ensure_terminator, label};
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_symbol, operand_load};
use crate::ir::{
    BinaryOp, CmpOp, ConstValue, Instr, IrFunction, IrProgram, Operand, Terminator, UnaryOp,
};

pub struct LlvmEmitter<'a> {
    program: &'a IrProgram,
}

impl<'a> LlvmEmitter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    pub fn emit_program(&self) -> Result<String, CodegenError> {
        let mut out = vec![
            "; ModuleID = 'skepa'".to_string(),
            "source_filename = \"skepa\"".to_string(),
            String::new(),
        ];

        for global in &self.program.globals {
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
                Some(_) | None => match global.ty {
                    // Non-constant initializers are materialized through __globals_init.
                    crate::ir::IrType::Int | crate::ir::IrType::Bool => "0".into(),
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "only Int/Bool globals are supported in initial LLVM lowering",
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
        if !self.program.globals.is_empty() {
            out.push(String::new());
        }

        if let Some(module_init) = &self.program.module_init {
            let init = self
                .program
                .functions
                .iter()
                .find(|func| func.id == module_init.function)
                .ok_or_else(|| {
                    CodegenError::InvalidIr(format!(
                        "module_init points at missing function {:?}",
                        module_init.function
                    ))
                })?;
            out.push(format!(
                "@llvm.global_ctors = appending global [1 x {{ i32, ptr, ptr }}] [{{ i32, ptr, ptr }} {{ i32 65535, ptr {}, ptr null }}]",
                llvm_symbol(&init.name)
            ));
            out.push(String::new());
        }

        for func in &self.program.functions {
            out.extend(self.emit_function(func)?);
            out.push(String::new());
        }

        Ok(out.join("\n"))
    }

    fn emit_function(&self, func: &IrFunction) -> Result<Vec<String>, CodegenError> {
        let names = ValueNames::new(func);
        let ret_ty = llvm_ty(&func.ret_ty)?;
        let params = func
            .params
            .iter()
            .map(|param| Ok(format!("{} %arg{}", llvm_ty(&param.ty)?, param.id.0)))
            .collect::<Result<Vec<_>, CodegenError>>()?
            .join(", ");

        let mut lines = vec![format!(
            "define {ret_ty} {}({params}) {{",
            llvm_symbol(&func.name)
        )];

        let mut counter = 0usize;
        for (idx, block) in func.blocks.iter().enumerate() {
            ensure_terminator(&block.terminator)?;
            lines.push(format!("{}:", label(block)));
            if idx == 0 {
                for local in &func.locals {
                    lines.push(format!(
                        "  %local{} = alloca {}, align 8",
                        local.id.0,
                        llvm_ty(&local.ty)?
                    ));
                }
                for param in &func.params {
                    if let Some(local) = func.locals.iter().find(|local| local.name == param.name) {
                        lines.push(format!(
                            "  store {} %arg{}, ptr %local{}, align 8",
                            llvm_ty(&param.ty)?,
                            param.id.0,
                            local.id.0
                        ));
                    }
                }
            }
            for instr in &block.instrs {
                calls::ensure_supported(instr)?;
                runtime::ensure_supported(instr)?;
                self.emit_instr(func, &names, instr, &mut lines, &mut counter)?;
            }
            self.emit_terminator(func, &names, &block.terminator, &mut lines, &mut counter)?;
        }

        lines.push("}".into());
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
                    ConstValue::Bool(v) => {
                        let int = if *v { 1 } else { 0 };
                        lines.push(format!("  {dest} = add {} 0, {int}", llvm_ty(ty)?));
                    }
                    _ => {
                        return Err(CodegenError::Unsupported(
                            "only Int/Bool constants are supported",
                        ));
                    }
                }
            }
            Instr::Copy { dst, ty, src } => {
                let dest = names.temp(*dst)?;
                let value = operand_load(names, src, func, lines, counter, ty)?;
                lines.push(format!("  {dest} = add {} 0, {value}", llvm_ty(ty)?));
            }
            Instr::Unary {
                dst,
                ty,
                op,
                operand,
            } => {
                let dest = names.temp(*dst)?;
                let value = operand_load(names, operand, func, lines, counter, ty)?;
                match (op, ty) {
                    (UnaryOp::Neg, crate::ir::IrType::Int) => {
                        lines.push(format!("  {dest} = sub i64 0, {value}"));
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
                let left = operand_load(names, left, func, lines, counter, ty)?;
                let right = operand_load(names, right, func, lines, counter, ty)?;
                let opname = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "sdiv",
                    BinaryOp::Mod => "srem",
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
                let left =
                    operand_load(names, left, func, lines, counter, &crate::ir::IrType::Int)?;
                let right =
                    operand_load(names, right, func, lines, counter, &crate::ir::IrType::Int)?;
                let pred = match op {
                    CmpOp::Eq => "eq",
                    CmpOp::Ne => "ne",
                    CmpOp::Lt => "slt",
                    CmpOp::Le => "sle",
                    CmpOp::Gt => "sgt",
                    CmpOp::Ge => "sge",
                };
                lines.push(format!("  {dest} = icmp {pred} i64 {left}, {right}"));
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
                let value = operand_load(names, value, func, lines, counter, ty)?;
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
                let value = operand_load(names, value, func, lines, counter, ty)?;
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
                )?;
            }
            _ => {
                return Err(CodegenError::Unsupported(
                    "instruction not yet supported in LLVM lowering",
                ));
            }
        }
        Ok(())
    }

    fn emit_terminator(
        &self,
        func: &IrFunction,
        names: &ValueNames,
        term: &Terminator,
        lines: &mut Vec<String>,
        counter: &mut usize,
    ) -> Result<(), CodegenError> {
        match term {
            Terminator::Jump(target) => {
                let target = self.block_label(func, *target)?;
                lines.push(format!("  br label %{target}"));
            }
            Terminator::Branch(branch) => {
                let cond = operand_load(
                    names,
                    &branch.cond,
                    func,
                    lines,
                    counter,
                    &crate::ir::IrType::Bool,
                )?;
                let (then_label, else_label) =
                    branch_targets(branch, |block| self.block_label(func, block))?;
                lines.push(format!(
                    "  br i1 {cond}, label %{then_label}, label %{else_label}"
                ));
            }
            Terminator::Return(Some(value)) => {
                let value = operand_load(names, value, func, lines, counter, &func.ret_ty)?;
                lines.push(format!("  ret {} {value}", llvm_ty(&func.ret_ty)?));
            }
            Terminator::Return(None) => lines.push("  ret void".into()),
            Terminator::Panic { .. } => {
                return Err(CodegenError::Unsupported(
                    "panic terminators are not lowered yet",
                ));
            }
            Terminator::Unreachable => unreachable!(),
        }
        Ok(())
    }

    fn block_label(
        &self,
        func: &IrFunction,
        id: crate::ir::BlockId,
    ) -> Result<String, CodegenError> {
        let block = func
            .blocks
            .iter()
            .find(|block| block.id == id)
            .ok_or_else(|| CodegenError::MissingBlock(format!("{:?}", id)))?;
        Ok(label(block))
    }
}
