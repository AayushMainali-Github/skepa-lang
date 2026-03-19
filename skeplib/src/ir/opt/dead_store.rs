use std::collections::HashSet;

use crate::ir::{Instr, IrProgram, IrType, Operand};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        for block in &mut func.blocks {
            let mut shadowed_locals = HashSet::new();
            let mut shadowed_globals = HashSet::new();
            let mut kept = Vec::with_capacity(block.instrs.len());

            for instr in block.instrs.iter().rev() {
                match instr {
                    Instr::StoreLocal { local, ty, .. } => {
                        if is_dead_store_safe_type(ty) && shadowed_locals.contains(local) {
                            changed = true;
                            continue;
                        }
                        if is_dead_store_safe_type(ty) {
                            shadowed_locals.insert(*local);
                        } else {
                            shadowed_locals.remove(local);
                        }
                    }
                    Instr::StoreGlobal { global, ty, .. } => {
                        if is_dead_store_safe_type(ty) && shadowed_globals.contains(global) {
                            changed = true;
                            continue;
                        }
                        if is_dead_store_safe_type(ty) {
                            shadowed_globals.insert(*global);
                        } else {
                            shadowed_globals.remove(global);
                        }
                    }
                    _ => {}
                }

                collect_reads_and_effects(instr, &mut shadowed_locals, &mut shadowed_globals);

                kept.push(instr.clone());
            }

            kept.reverse();
            block.instrs = kept;
        }
    }

    changed
}

fn collect_reads_and_effects(
    instr: &Instr,
    shadowed_locals: &mut HashSet<crate::ir::LocalId>,
    shadowed_globals: &mut HashSet<crate::ir::GlobalId>,
) {
    match instr {
        Instr::Copy { src, .. } | Instr::Unary { operand: src, .. } => {
            collect_operand_reads(src, shadowed_locals, shadowed_globals);
        }
        Instr::Binary { left, right, .. }
        | Instr::Compare { left, right, .. }
        | Instr::Logic { left, right, .. } => {
            collect_operand_reads(left, shadowed_locals, shadowed_globals);
            collect_operand_reads(right, shadowed_locals, shadowed_globals);
        }
        Instr::LoadGlobal { global, .. } => {
            shadowed_globals.remove(global);
        }
        Instr::LoadLocal { local, .. } => {
            shadowed_locals.remove(local);
        }
        Instr::StoreGlobal { value, .. } | Instr::StoreLocal { value, .. } => {
            collect_operand_reads(value, shadowed_locals, shadowed_globals);
        }
        Instr::MakeArray { items, .. } => {
            for item in items {
                collect_operand_reads(item, shadowed_locals, shadowed_globals);
            }
        }
        Instr::MakeArrayRepeat { value, .. } | Instr::VecPush { value, .. } => {
            collect_operand_reads(value, shadowed_locals, shadowed_globals);
        }
        Instr::VecLen { vec, .. } => collect_operand_reads(vec, shadowed_locals, shadowed_globals),
        Instr::ArrayGet { array, index, .. }
        | Instr::VecGet {
            vec: array, index, ..
        } => {
            collect_operand_reads(array, shadowed_locals, shadowed_globals);
            collect_operand_reads(index, shadowed_locals, shadowed_globals);
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
            collect_operand_reads(array, shadowed_locals, shadowed_globals);
            collect_operand_reads(index, shadowed_locals, shadowed_globals);
            collect_operand_reads(value, shadowed_locals, shadowed_globals);
        }
        Instr::VecDelete { vec, index, .. } => {
            collect_operand_reads(vec, shadowed_locals, shadowed_globals);
            collect_operand_reads(index, shadowed_locals, shadowed_globals);
        }
        Instr::MakeStruct { fields, .. } => {
            for field in fields {
                collect_operand_reads(field, shadowed_locals, shadowed_globals);
            }
        }
        Instr::StructGet { base, .. } => {
            collect_operand_reads(base, shadowed_locals, shadowed_globals)
        }
        Instr::StructSet { base, value, .. } => {
            collect_operand_reads(base, shadowed_locals, shadowed_globals);
            collect_operand_reads(value, shadowed_locals, shadowed_globals);
        }
        Instr::CallDirect { args, .. } | Instr::CallBuiltin { args, .. } => {
            for arg in args {
                collect_operand_reads(arg, shadowed_locals, shadowed_globals);
            }
        }
        Instr::CallIndirect { callee, args, .. } => {
            collect_operand_reads(callee, shadowed_locals, shadowed_globals);
            for arg in args {
                collect_operand_reads(arg, shadowed_locals, shadowed_globals);
            }
        }
        Instr::Const { .. } | Instr::VecNew { .. } | Instr::MakeClosure { .. } => {}
    }

    if is_effect_barrier(instr) {
        shadowed_locals.clear();
        shadowed_globals.clear();
    }
}

fn collect_operand_reads(
    operand: &Operand,
    shadowed_locals: &mut HashSet<crate::ir::LocalId>,
    shadowed_globals: &mut HashSet<crate::ir::GlobalId>,
) {
    match operand {
        Operand::Local(id) => {
            shadowed_locals.remove(id);
        }
        Operand::Global(id) => {
            shadowed_globals.remove(id);
        }
        Operand::Const(_) | Operand::Temp(_) => {}
    }
}

fn is_dead_store_safe_type(ty: &IrType) -> bool {
    matches!(ty, IrType::Int | IrType::Float | IrType::Bool)
}

fn is_effect_barrier(instr: &Instr) -> bool {
    matches!(
        instr,
        Instr::ArraySet { .. }
            | Instr::VecPush { .. }
            | Instr::VecSet { .. }
            | Instr::VecDelete { .. }
            | Instr::StructSet { .. }
            | Instr::CallDirect { .. }
            | Instr::CallIndirect { .. }
            | Instr::CallBuiltin { .. }
    )
}
