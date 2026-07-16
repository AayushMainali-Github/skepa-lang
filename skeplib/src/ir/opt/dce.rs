use std::collections::HashSet;

use crate::ir::{Instr, IrProgram, Operand, Terminator};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        for block in &mut func.blocks {
            let mut live = HashSet::new();
            collect_terminator_uses(&block.terminator, &mut live);

            let mut kept = Vec::with_capacity(block.instrs.len());
            for instr in block.instrs.iter().rev() {
                collect_instr_uses(instr, &mut live);
                if let Some(dst) = instr_dst(instr) {
                    if !live.contains(&dst) && is_pure(instr) {
                        changed = true;
                        continue;
                    }
                    live.remove(&dst);
                }
                kept.push(instr.clone());
            }
            kept.reverse();
            block.instrs = kept;
        }
    }

    changed
}

fn instr_dst(instr: &Instr) -> Option<crate::ir::TempId> {
    match instr {
        Instr::Const { dst, .. }
        | Instr::Copy { dst, .. }
        | Instr::Unary { dst, .. }
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
        | Instr::MakeClosure { dst, .. } => Some(*dst),
        Instr::CallDirect { dst, .. }
        | Instr::CallIndirect { dst, .. }
        | Instr::CallBuiltin { dst, .. } => *dst,
        Instr::StoreGlobal { .. }
        | Instr::StoreLocal { .. }
        | Instr::ArraySet { .. }
        | Instr::VecPush { .. }
        | Instr::VecSet { .. }
        | Instr::StructSet { .. } => None,
    }
}

fn is_pure(instr: &Instr) -> bool {
    match instr {
        Instr::Const { .. }
        | Instr::Copy { .. }
        | Instr::Compare { .. }
        | Instr::Logic { .. }
        | Instr::LoadGlobal { .. }
        | Instr::LoadLocal { .. }
        | Instr::MakeArray { .. }
        | Instr::MakeArrayRepeat { .. }
        | Instr::VecNew { .. }
        | Instr::VecLen { .. }
        | Instr::MakeStruct { .. }
        | Instr::StructGet { .. }
        | Instr::MakeClosure { .. } => true,
        // Negation can trap on i64::MIN in debug builds.
        Instr::Unary {
            op: crate::ir::UnaryOp::Neg,
            ..
        } => false,
        Instr::Unary { .. } => true,
        // Division/remainder and shifts can trap at runtime.
        Instr::Binary {
            op:
                crate::ir::BinaryOp::Div
                | crate::ir::BinaryOp::Mod
                | crate::ir::BinaryOp::Shl
                | crate::ir::BinaryOp::Shr,
            ..
        } => false,
        Instr::Binary { .. } => true,
        // Indexing may trap; VecDelete mutates the vector.
        Instr::ArrayGet { .. } | Instr::VecGet { .. } | Instr::VecDelete { .. } => false,
        _ => false,
    }
}

fn collect_terminator_uses(term: &Terminator, live: &mut HashSet<crate::ir::TempId>) {
    match term {
        Terminator::Branch(branch) => collect_operand_uses(&branch.cond, live),
        Terminator::Return(Some(value)) => collect_operand_uses(value, live),
        Terminator::Jump(_)
        | Terminator::Return(None)
        | Terminator::Panic { .. }
        | Terminator::Unreachable => {}
    }
}

fn collect_instr_uses(instr: &Instr, live: &mut HashSet<crate::ir::TempId>) {
    match instr {
        Instr::Copy { src, .. } | Instr::Unary { operand: src, .. } => {
            collect_operand_uses(src, live);
        }
        Instr::Binary { left, right, .. }
        | Instr::Compare { left, right, .. }
        | Instr::Logic { left, right, .. } => {
            collect_operand_uses(left, live);
            collect_operand_uses(right, live);
        }
        Instr::StoreGlobal { value, .. }
        | Instr::StoreLocal { value, .. }
        | Instr::MakeArrayRepeat { value, .. }
        | Instr::VecPush { value, .. } => {
            collect_operand_uses(value, live);
        }
        Instr::MakeArray { items, .. } => {
            for item in items {
                collect_operand_uses(item, live);
            }
        }
        Instr::VecLen { vec, .. } => collect_operand_uses(vec, live),
        Instr::ArrayGet { array, index, .. }
        | Instr::VecGet {
            vec: array, index, ..
        } => {
            collect_operand_uses(array, live);
            collect_operand_uses(index, live);
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
            collect_operand_uses(array, live);
            collect_operand_uses(index, live);
            collect_operand_uses(value, live);
        }
        Instr::VecDelete { vec, index, .. } => {
            collect_operand_uses(vec, live);
            collect_operand_uses(index, live);
        }
        Instr::MakeStruct { fields, .. } => {
            for field in fields {
                collect_operand_uses(field, live);
            }
        }
        Instr::StructGet { base, .. } => collect_operand_uses(base, live),
        Instr::StructSet { base, value, .. } => {
            collect_operand_uses(base, live);
            collect_operand_uses(value, live);
        }
        Instr::CallDirect { args, .. } | Instr::CallBuiltin { args, .. } => {
            for arg in args {
                collect_operand_uses(arg, live);
            }
        }
        Instr::CallIndirect { callee, args, .. } => {
            collect_operand_uses(callee, live);
            for arg in args {
                collect_operand_uses(arg, live);
            }
        }
        Instr::Const { .. }
        | Instr::LoadGlobal { .. }
        | Instr::LoadLocal { .. }
        | Instr::VecNew { .. }
        | Instr::MakeClosure { .. } => {}
    }
}

fn collect_operand_uses(operand: &Operand, live: &mut HashSet<crate::ir::TempId>) {
    if let Operand::Temp(id) = operand {
        live.insert(*id);
    }
}
