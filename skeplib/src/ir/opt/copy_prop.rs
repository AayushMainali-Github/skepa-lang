use std::collections::HashMap;

use crate::ir::{ConstValue, Instr, IrProgram, IrType, Operand, Terminator};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        for block in &mut func.blocks {
            let mut copies = HashMap::new();
            for instr in &mut block.instrs {
                changed |= rewrite_instr(instr, &copies);
                update_copies(instr, &mut copies);
            }
            changed |= rewrite_terminator(&mut block.terminator, &copies);
        }
    }

    changed
}

fn rewrite_instr(instr: &mut Instr, copies: &HashMap<crate::ir::TempId, Operand>) -> bool {
    let mut changed = false;
    match instr {
        Instr::Copy { src, .. } | Instr::Unary { operand: src, .. } => {
            changed |= rewrite_operand(src, copies);
        }
        Instr::Binary { left, right, .. }
        | Instr::Compare { left, right, .. }
        | Instr::Logic { left, right, .. } => {
            changed |= rewrite_operand(left, copies);
            changed |= rewrite_operand(right, copies);
        }
        Instr::StoreGlobal { value, .. }
        | Instr::StoreLocal { value, .. }
        | Instr::MakeArrayRepeat { value, .. }
        | Instr::VecPush { value, .. } => {
            changed |= rewrite_operand(value, copies);
        }
        Instr::MakeArray { items, .. } => {
            for item in items {
                changed |= rewrite_operand(item, copies);
            }
        }
        Instr::VecLen { vec, .. } => {
            changed |= rewrite_operand(vec, copies);
        }
        Instr::ArrayGet { array, index, .. }
        | Instr::VecGet {
            vec: array, index, ..
        } => {
            changed |= rewrite_operand(array, copies);
            changed |= rewrite_operand(index, copies);
        }
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
            changed |= rewrite_operand(array, copies);
            changed |= rewrite_operand(index, copies);
            changed |= rewrite_operand(value, copies);
        }
        Instr::VecDelete { vec, index, .. } => {
            changed |= rewrite_operand(vec, copies);
            changed |= rewrite_operand(index, copies);
        }
        Instr::MakeStruct { fields, .. } => {
            for field in fields {
                changed |= rewrite_operand(field, copies);
            }
        }
        Instr::StructGet { base, .. } => {
            changed |= rewrite_operand(base, copies);
        }
        Instr::StructSet { base, value, .. } => {
            changed |= rewrite_operand(base, copies);
            changed |= rewrite_operand(value, copies);
        }
        Instr::CallDirect { args, .. } | Instr::CallBuiltin { args, .. } => {
            for arg in args {
                changed |= rewrite_operand(arg, copies);
            }
        }
        Instr::CallIndirect { callee, args, .. } => {
            changed |= rewrite_operand(callee, copies);
            for arg in args {
                changed |= rewrite_operand(arg, copies);
            }
        }
        Instr::Const { .. }
        | Instr::LoadGlobal { .. }
        | Instr::LoadLocal { .. }
        | Instr::VecNew { .. }
        | Instr::MakeClosure { .. } => {}
    }
    changed
}

fn update_copies(instr: &Instr, copies: &mut HashMap<crate::ir::TempId, Operand>) {
    match instr {
        Instr::Copy { dst, src, ty } => {
            if is_copy_propagation_safe_type(ty) {
                copies.insert(*dst, src.clone());
            } else {
                copies.remove(dst);
            }
        }
        Instr::Const { dst, value, .. } => {
            if is_copy_propagation_safe_const(value) {
                copies.insert(*dst, Operand::Const(value.clone()));
            } else {
                copies.remove(dst);
            }
        }
        Instr::Unary { dst, .. }
        | Instr::Binary { dst, .. }
        | Instr::Compare { dst, .. }
        | Instr::Logic { dst, .. }
        | Instr::LoadGlobal { dst, .. }
        | Instr::LoadLocal { dst, .. }
        | Instr::MakeArray { dst, .. }
        | Instr::MakeArrayRepeat { dst, .. }
        | Instr::VecNew { dst, .. }
        | Instr::VecLen { dst, .. }
        | Instr::ArrayGet { dst, .. }
        | Instr::VecGet { dst, .. }
        | Instr::VecDelete { dst, .. }
        | Instr::MakeStruct { dst, .. }
        | Instr::StructGet { dst, .. }
        | Instr::MakeClosure { dst, .. } => {
            copies.remove(dst);
        }
        Instr::StoreGlobal { .. }
        | Instr::StoreLocal { .. }
        | Instr::ArraySet { .. }
        | Instr::VecPush { .. }
        | Instr::VecSet { .. }
        | Instr::StructSet { .. }
        | Instr::CallDirect { dst: None, .. }
        | Instr::CallIndirect { dst: None, .. }
        | Instr::CallBuiltin { dst: None, .. } => {
            copies.clear();
        }
        Instr::CallDirect { dst: Some(dst), .. }
        | Instr::CallIndirect { dst: Some(dst), .. }
        | Instr::CallBuiltin { dst: Some(dst), .. } => {
            copies.remove(dst);
            copies.clear();
        }
    }
}

fn rewrite_terminator(
    terminator: &mut Terminator,
    copies: &HashMap<crate::ir::TempId, Operand>,
) -> bool {
    match terminator {
        Terminator::Branch(branch) => rewrite_operand(&mut branch.cond, copies),
        Terminator::Return(Some(value)) => rewrite_operand(value, copies),
        Terminator::Jump(_)
        | Terminator::Return(None)
        | Terminator::Panic { .. }
        | Terminator::Unreachable => false,
    }
}

fn rewrite_operand(operand: &mut Operand, copies: &HashMap<crate::ir::TempId, Operand>) -> bool {
    let mut changed = false;
    while let Operand::Temp(id) = operand {
        let Some(replacement) = copies.get(id).cloned() else {
            break;
        };
        if replacement == *operand {
            break;
        }
        *operand = replacement;
        changed = true;
    }
    changed
}

fn is_copy_propagation_safe_const(value: &ConstValue) -> bool {
    matches!(
        value,
        ConstValue::Int(_) | ConstValue::Float(_) | ConstValue::Bool(_)
    )
}

fn is_copy_propagation_safe_type(ty: &IrType) -> bool {
    matches!(ty, IrType::Int | IrType::Float | IrType::Bool)
}
