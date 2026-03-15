use std::collections::HashMap;
use std::rc::Rc;

use super::{BytecodeModule, FunctionChunk, Instr, IntLocalConstOp, Value};

pub(super) fn rewrite_direct_calls_to_indexes(module: &mut BytecodeModule) {
    let mut names = module.functions.keys().cloned().collect::<Vec<_>>();
    names.sort();
    let by_name = names
        .into_iter()
        .enumerate()
        .map(|(idx, n)| (n, idx))
        .collect::<HashMap<_, _>>();

    for chunk in module.functions.values_mut() {
        for instr in &mut chunk.code {
            let new_instr = match instr {
                Instr::Call { name, argc } => by_name
                    .get(name)
                    .copied()
                    .map(|idx| Instr::CallIdx { idx, argc: *argc }),
                _ => None,
            };
            if let Some(new_instr) = new_instr {
                *instr = new_instr;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TrivialDirectCall {
    AddConst(i64),
    StructFieldAdd(usize),
}

pub(super) fn rewrite_trivial_direct_calls(module: &mut BytecodeModule) {
    let mut names = module.functions.keys().cloned().collect::<Vec<_>>();
    names.sort();
    let trivial = names
        .iter()
        .map(|name| {
            module
                .functions
                .get(name)
                .and_then(trivial_direct_call_pattern)
        })
        .collect::<Vec<_>>();

    for chunk in module.functions.values_mut() {
        for instr in &mut chunk.code {
            if let Instr::CallIdx { idx, argc } = instr {
                match (*argc, trivial.get(*idx)) {
                    (1, Some(Some(TrivialDirectCall::AddConst(rhs)))) => {
                        *instr = Instr::CallIdxAddConst(*rhs);
                    }
                    (2, Some(Some(TrivialDirectCall::StructFieldAdd(slot)))) => {
                        *instr = Instr::CallIdxStructFieldAdd(*slot);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn trivial_direct_call_pattern(chunk: &FunctionChunk) -> Option<TrivialDirectCall> {
    match chunk.code.as_slice() {
        [
            Instr::LoadLocal(0),
            Instr::LoadConst(Value::Int(rhs)),
            Instr::Add,
            Instr::Return,
        ] if chunk.param_count == 1 && chunk.locals_count == 1 => {
            Some(TrivialDirectCall::AddConst(*rhs))
        }
        [
            Instr::LoadConst(Value::Int(rhs)),
            Instr::LoadLocal(0),
            Instr::Add,
            Instr::Return,
        ] if chunk.param_count == 1 && chunk.locals_count == 1 => {
            Some(TrivialDirectCall::AddConst(*rhs))
        }
        [
            Instr::LoadLocal(0),
            Instr::StructGetSlot(field_slot),
            Instr::LoadLocal(1),
            Instr::Add,
            Instr::Return,
        ] if chunk.param_count == 2 && chunk.locals_count == 2 => {
            Some(TrivialDirectCall::StructFieldAdd(*field_slot))
        }
        [
            Instr::StructGetLocalSlot {
                slot: 0,
                field_slot,
            },
            Instr::LoadLocal(1),
            Instr::Add,
            Instr::Return,
        ] if chunk.param_count == 2 && chunk.locals_count == 2 => {
            Some(TrivialDirectCall::StructFieldAdd(*field_slot))
        }
        _ => None,
    }
}

pub(super) fn rewrite_function_values_to_indexes(module: &mut BytecodeModule) {
    let mut names = module.functions.keys().cloned().collect::<Vec<_>>();
    names.sort();
    let by_name = names
        .into_iter()
        .enumerate()
        .map(|(idx, n)| (n, idx))
        .collect::<HashMap<_, _>>();

    for chunk in module.functions.values_mut() {
        for instr in &mut chunk.code {
            rewrite_instr_function_values(instr, &by_name);
        }
    }
}

fn rewrite_instr_function_values(instr: &mut Instr, by_name: &HashMap<String, usize>) {
    if let Instr::LoadConst(value) = instr {
        rewrite_value_function_indexes(value, by_name);
    }
}

fn rewrite_value_function_indexes(value: &mut Value, by_name: &HashMap<String, usize>) {
    match value {
        Value::Array(items) => {
            let mut rewritten = items.as_ref().to_vec();
            let mut changed = false;
            for item in &mut rewritten {
                let before = item.clone();
                rewrite_value_function_indexes(item, by_name);
                changed |= *item != before;
            }
            if changed {
                *value = Value::Array(Rc::<[Value]>::from(rewritten));
            }
        }
        Value::Struct { shape, fields } => {
            let mut rewritten = fields.as_ref().to_vec();
            let mut changed = false;
            for field_value in &mut rewritten {
                let before = field_value.clone();
                rewrite_value_function_indexes(field_value, by_name);
                changed |= *field_value != before;
            }
            if changed {
                *value = Value::Struct {
                    shape: shape.clone(),
                    fields: Rc::<[Value]>::from(rewritten),
                };
            }
        }
        Value::Function(fn_name) => {
            if let Some(idx) = by_name.get(fn_name.as_ref()).copied() {
                *value = Value::FunctionIdx(idx);
            }
        }
        _ => {}
    }
}

pub(super) fn peephole_optimize_module(module: &mut BytecodeModule) {
    for chunk in module.functions.values_mut() {
        peephole_optimize_chunk(chunk);
    }
}

fn peephole_optimize_chunk(chunk: &mut FunctionChunk) {
    if chunk.code.is_empty() {
        return;
    }
    rewrite_struct_local_field_add_to_local(chunk);
    rewrite_int_stack_const_op_to_local(chunk);
    rewrite_struct_method_complex_to_local(chunk);
    let len = chunk.code.len();
    let mut remove = vec![false; len];
    for (i, instr) in chunk.code.iter().enumerate() {
        if let Instr::Jump(target) = instr
            && *target == i + 1
        {
            remove[i] = true;
        }
    }
    if !remove.iter().any(|r| *r) {
        return;
    }

    let kept = remove.iter().filter(|r| !**r).count();
    let mut next_kept_at_or_after = vec![kept; len + 1];
    let mut next_new_idx = kept;
    for i in (0..len).rev() {
        if !remove[i] {
            next_new_idx -= 1;
        }
        next_kept_at_or_after[i] = next_new_idx;
    }

    let mut remapped = Vec::with_capacity(kept);
    for (i, instr) in chunk.code.iter().enumerate() {
        if remove[i] {
            continue;
        }
        let mapped = match instr {
            Instr::Jump(t) => Instr::Jump(next_kept_at_or_after[*t]),
            Instr::JumpIfFalse(t) => Instr::JumpIfFalse(next_kept_at_or_after[*t]),
            Instr::JumpIfTrue(t) => Instr::JumpIfTrue(next_kept_at_or_after[*t]),
            _ => instr.clone(),
        };
        remapped.push(mapped);
    }
    chunk.code = remapped;
}

fn rewrite_struct_local_field_add_to_local(chunk: &mut FunctionChunk) {
    let mut rewritten = Vec::with_capacity(chunk.code.len());
    let mut i = 0;
    while i < chunk.code.len() {
        if let (
            Instr::StructGetLocalSlot { slot, field_slot },
            Instr::IntStackOpToLocal {
                slot: dst,
                op: IntLocalConstOp::Add,
            },
        ) = (&chunk.code[i], chunk.code.get(i + 1).unwrap_or(&Instr::Pop))
        {
            rewritten.push(Instr::StructGetLocalSlotAddToLocal {
                struct_slot: *slot,
                field_slot: *field_slot,
                dst: *dst,
            });
            i += 2;
            continue;
        }
        rewritten.push(chunk.code[i].clone());
        i += 1;
    }
    chunk.code = rewritten;
}

fn rewrite_int_stack_const_op_to_local(chunk: &mut FunctionChunk) {
    let mut rewritten = Vec::with_capacity(chunk.code.len());
    let mut i = 0;
    while i < chunk.code.len() {
        if let (
            Instr::IntStackConstOp { op: stack_op, rhs },
            Some(Instr::IntStackOpToLocal { slot, op: local_op }),
        ) = (&chunk.code[i], chunk.code.get(i + 1))
        {
            rewritten.push(Instr::IntStackConstOpToLocal {
                slot: *slot,
                stack_op: *stack_op,
                local_op: *local_op,
                rhs: *rhs,
            });
            i += 2;
            continue;
        }
        rewritten.push(chunk.code[i].clone());
        i += 1;
    }
    chunk.code = rewritten;
}

fn rewrite_struct_method_complex_to_local(chunk: &mut FunctionChunk) {
    let mut rewritten = Vec::with_capacity(chunk.code.len());
    let mut i = 0;
    while i < chunk.code.len() {
        match chunk.code.get(i..i + 10) {
            Some(
                [
                    Instr::StructGetLocalSlot {
                        slot: struct_slot_a,
                        field_slot: lhs_field_slot,
                    },
                    Instr::IntLocalConstOp {
                        slot: arg_slot,
                        op: arg_op,
                        rhs: arg_rhs,
                    },
                    Instr::Add,
                    Instr::LoadConst(Value::Int(mul)),
                    Instr::MulInt,
                    Instr::StructGetLocalSlot {
                        slot: struct_slot_b,
                        field_slot: rhs_field_slot,
                    },
                    Instr::Add,
                    Instr::LoadConst(Value::Int(modulo)),
                    Instr::ModInt,
                    Instr::IntStackOpToLocal {
                        slot: dst,
                        op: IntLocalConstOp::Add,
                    },
                ],
            ) if struct_slot_a == struct_slot_b => {
                rewritten.push(Instr::StructFieldAddMulFieldModLocalToLocal {
                    struct_slot: *struct_slot_a,
                    arg_slot: *arg_slot,
                    arg_op: *arg_op,
                    arg_rhs: *arg_rhs,
                    lhs_field_slot: *lhs_field_slot,
                    rhs_field_slot: *rhs_field_slot,
                    mul: *mul,
                    modulo: *modulo,
                    dst: *dst,
                });
                i += 10;
                continue;
            }
            _ => {}
        }
        rewritten.push(chunk.code[i].clone());
        i += 1;
    }
    chunk.code = rewritten;
}
