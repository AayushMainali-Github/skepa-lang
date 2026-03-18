use crate::codegen::CodegenError;
use crate::codegen::llvm::block::{branch_targets, ensure_terminator, label};
use crate::codegen::llvm::calls::{self, DirectCall};
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::types::llvm_ty;
use crate::codegen::llvm::value::{ValueNames, llvm_symbol, operand_load};
use crate::ir::{
    BinaryOp, CmpOp, ConstValue, Instr, IrFunction, IrProgram, Operand, Terminator, UnaryOp,
};
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
                    crate::ir::IrType::String
                    | crate::ir::IrType::Named(_)
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
        if !self.program.globals.is_empty() {
            out.push(String::new());
        }

        if !self.string_literals.is_empty() {
            for (value, name) in &self.string_literals {
                let bytes = encode_c_string(value);
                out.push(format!(
                    "{name} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
                    value.len() + 1,
                    bytes
                ));
            }
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

        runtime::emit_runtime_decls(self.program, &mut out)?;
        out.push(String::new());

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
                            "only Int/Bool/String constants are supported",
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
                let compare_ty = infer_compare_operand_type(self.program, func, left, right);
                let left = operand_load(
                    names,
                    left,
                    func,
                    lines,
                    counter,
                    &compare_ty,
                    &self.string_literals,
                )?;
                let right = operand_load(
                    names,
                    right,
                    func,
                    lines,
                    counter,
                    &compare_ty,
                    &self.string_literals,
                )?;
                let pred = match op {
                    CmpOp::Eq => "eq",
                    CmpOp::Ne => "ne",
                    CmpOp::Lt => "slt",
                    CmpOp::Le => "sle",
                    CmpOp::Gt => "sgt",
                    CmpOp::Ge => "sge",
                };
                lines.push(format!(
                    "  {dest} = icmp {pred} {} {left}, {right}",
                    llvm_ty(&compare_ty)?
                ));
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
                    &self.string_literals,
                )?;
                let (then_label, else_label) =
                    branch_targets(branch, |block| self.block_label(func, block))?;
                lines.push(format!(
                    "  br i1 {cond}, label %{then_label}, label %{else_label}"
                ));
            }
            Terminator::Return(Some(value)) => {
                let value = operand_load(
                    names,
                    value,
                    func,
                    lines,
                    counter,
                    &func.ret_ty,
                    &self.string_literals,
                )?;
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

fn collect_string_literals(program: &IrProgram) -> HashMap<String, String> {
    let mut literals = HashMap::new();
    let mut index = 0usize;
    for func in &program.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                collect_instr_string_literals(instr, &mut literals, &mut index);
            }
            if let Terminator::Return(Some(Operand::Const(ConstValue::String(value)))) =
                &block.terminator
            {
                literals.entry(value.clone()).or_insert_with(|| {
                    let name = format!("@.str.{index}");
                    index += 1;
                    name
                });
            }
        }
    }
    literals
}

fn collect_instr_string_literals(
    instr: &Instr,
    literals: &mut HashMap<String, String>,
    index: &mut usize,
) {
    let mut add_operand = |operand: &Operand| {
        if let Operand::Const(ConstValue::String(value)) = operand {
            literals.entry(value.clone()).or_insert_with(|| {
                let name = format!("@.str.{index}");
                *index += 1;
                name
            });
        }
    };
    match instr {
        Instr::Const {
            value: ConstValue::String(value),
            ..
        } => {
            literals.entry(value.clone()).or_insert_with(|| {
                let name = format!("@.str.{index}");
                *index += 1;
                name
            });
        }
        Instr::Copy { src, .. } => add_operand(src),
        Instr::Unary { operand, .. } => add_operand(operand),
        Instr::Binary { left, right, .. } | Instr::Compare { left, right, .. } => {
            add_operand(left);
            add_operand(right);
        }
        Instr::StoreGlobal { value, .. } | Instr::StoreLocal { value, .. } => add_operand(value),
        Instr::CallDirect { args, .. } => {
            for arg in args {
                add_operand(arg);
            }
        }
        Instr::CallBuiltin { builtin, args, .. } => {
            for arg in args {
                add_operand(arg);
            }
            literals.entry(builtin.package.clone()).or_insert_with(|| {
                let name = format!("@.str.{index}");
                *index += 1;
                name
            });
            literals.entry(builtin.name.clone()).or_insert_with(|| {
                let name = format!("@.str.{index}");
                *index += 1;
                name
            });
        }
        Instr::CallIndirect { callee, args, .. } => {
            add_operand(callee);
            for arg in args {
                add_operand(arg);
            }
        }
        Instr::MakeArray { items, .. } => {
            for item in items {
                add_operand(item);
            }
        }
        Instr::MakeArrayRepeat { value, .. } => add_operand(value),
        Instr::ArrayGet { array, index, .. }
        | Instr::VecGet {
            vec: array, index, ..
        } => {
            add_operand(array);
            add_operand(index);
        }
        Instr::StructGet { base, .. } => add_operand(base),
        Instr::ArraySet {
            array,
            index,
            value,
            ..
        }
        | Instr::VecSet {
            vec: array,
            index,
            value,
            ..
        } => {
            add_operand(array);
            add_operand(index);
            add_operand(value);
        }
        Instr::VecPush { vec, value, .. } => {
            add_operand(vec);
            add_operand(value);
        }
        Instr::VecDelete { vec, index, .. } => {
            add_operand(vec);
            add_operand(index);
        }
        Instr::VecLen { vec, .. } => add_operand(vec),
        Instr::MakeStruct { fields, .. } => {
            for field in fields {
                add_operand(field);
            }
        }
        Instr::StructSet { base, value, .. } => {
            add_operand(base);
            add_operand(value);
        }
        Instr::MakeClosure { .. } => {}
        _ => {}
    }
}

fn encode_c_string(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out.push_str("\\00");
    out
}

fn infer_compare_operand_type(
    program: &IrProgram,
    func: &IrFunction,
    left: &Operand,
    right: &Operand,
) -> crate::ir::IrType {
    match infer_operand_type(program, func, left)
        .or_else(|| infer_operand_type(program, func, right))
    {
        Some(crate::ir::IrType::Bool) => crate::ir::IrType::Bool,
        _ => crate::ir::IrType::Int,
    }
}

fn infer_operand_type(
    program: &IrProgram,
    func: &IrFunction,
    operand: &Operand,
) -> Option<crate::ir::IrType> {
    match operand {
        Operand::Const(ConstValue::Bool(_)) => Some(crate::ir::IrType::Bool),
        Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .map(|temp| temp.ty.clone()),
        Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .map(|local| local.ty.clone()),
        Operand::Global(id) => program
            .globals
            .iter()
            .find(|global| global.id == *id)
            .map(|global| global.ty.clone()),
        _ => None,
    }
}
