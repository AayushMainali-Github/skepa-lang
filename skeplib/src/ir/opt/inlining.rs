use std::collections::HashMap;

use crate::ir::{
    BuiltinCall, ConstValue, FieldRef, FunctionId, Instr, IrFunction, IrLocal, IrProgram, IrTemp,
    IrType, LocalId, Operand, TempId, Terminator,
};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;
    let candidates = collect_candidates(program);
    if candidates.is_empty() {
        return false;
    }

    for func_idx in 0..program.functions.len() {
        let mut next_local = next_local_id(&program.functions[func_idx]);
        let mut next_temp = next_temp_id(&program.functions[func_idx]);
        let mut func_changed = false;

        for block_idx in 0..program.functions[func_idx].blocks.len() {
            let original_instrs = program.functions[func_idx].blocks[block_idx].instrs.clone();
            let mut rewritten = Vec::with_capacity(original_instrs.len());

            for instr in original_instrs {
                let Some(expanded) = try_inline_call(
                    &instr,
                    &candidates,
                    &mut next_local,
                    &mut next_temp,
                    &mut program.functions[func_idx],
                ) else {
                    rewritten.push(instr);
                    continue;
                };
                func_changed = true;
                rewritten.extend(expanded);
            }

            program.functions[func_idx].blocks[block_idx].instrs = rewritten;
        }

        changed |= func_changed;
    }

    changed
}

#[derive(Clone)]
struct InlineCandidate {
    params: Vec<(LocalId, IrType)>,
    locals: Vec<IrLocal>,
    temps: Vec<IrTemp>,
    instrs: Vec<Instr>,
    ret: Option<Operand>,
    ret_ty: IrType,
}

fn collect_candidates(program: &IrProgram) -> HashMap<FunctionId, InlineCandidate> {
    let mut out = HashMap::new();
    for func in &program.functions {
        let Some(candidate) = build_candidate(func) else {
            continue;
        };
        out.insert(func.id, candidate);
    }
    out
}

fn build_candidate(func: &IrFunction) -> Option<InlineCandidate> {
    if func.blocks.len() != 1 {
        return None;
    }
    let block = &func.blocks[0];
    let Terminator::Return(ret) = &block.terminator else {
        return None;
    };
    if block.instrs.iter().any(|instr| {
        matches!(
            instr,
            Instr::CallDirect { .. } | Instr::CallIndirect { .. } | Instr::CallBuiltin { .. }
        )
    }) {
        return None;
    }

    Some(InlineCandidate {
        params: func
            .params
            .iter()
            .map(|param| {
                let local = func.locals.iter().find(|local| local.name == param.name)?;
                Some((local.id, param.ty.clone()))
            })
            .collect::<Option<Vec<_>>>()?,
        locals: func.locals.clone(),
        temps: func.temps.clone(),
        instrs: block.instrs.clone(),
        ret: ret.clone(),
        ret_ty: func.ret_ty.clone(),
    })
}

fn try_inline_call(
    instr: &Instr,
    candidates: &HashMap<FunctionId, InlineCandidate>,
    next_local: &mut usize,
    next_temp: &mut usize,
    caller: &mut IrFunction,
) -> Option<Vec<Instr>> {
    let Instr::CallDirect {
        dst,
        ret_ty,
        function,
        args,
    } = instr
    else {
        return None;
    };
    let candidate = candidates.get(function)?;
    if args.len() != candidate.params.len() || !inline_types_compatible(ret_ty, &candidate.ret_ty) {
        return None;
    }

    let mut local_map = HashMap::new();
    let mut temp_map = HashMap::new();
    let mut expanded = Vec::new();

    for local in &candidate.locals {
        let id = LocalId(*next_local);
        *next_local += 1;
        caller.locals.push(IrLocal {
            id,
            name: format!("inline_{}_{}", function.0, local.name),
            ty: local.ty.clone(),
        });
        local_map.insert(local.id, id);
    }

    for temp in &candidate.temps {
        let id = TempId(*next_temp);
        *next_temp += 1;
        caller.temps.push(IrTemp {
            id,
            ty: temp.ty.clone(),
        });
        temp_map.insert(temp.id, id);
    }

    for ((param_local, param_ty), arg) in candidate.params.iter().zip(args.iter()) {
        expanded.push(Instr::StoreLocal {
            local: *local_map.get(param_local)?,
            ty: param_ty.clone(),
            value: arg.clone(),
        });
    }

    for inner in &candidate.instrs {
        expanded.push(remap_instr(inner, &local_map, &temp_map));
    }

    if let (Some(dst), Some(ret)) = (dst, candidate.ret.as_ref()) {
        expanded.push(Instr::Copy {
            dst: *dst,
            ty: ret_ty.clone(),
            src: remap_operand(ret, &local_map, &temp_map),
        });
    }

    let _ = &candidate.ret_ty;
    Some(expanded)
}

fn inline_types_compatible(actual: &IrType, expected: &IrType) -> bool {
    if actual == expected
        || matches!(actual, IrType::Unknown)
        || matches!(expected, IrType::Unknown)
    {
        return true;
    }

    match (actual, expected) {
        (IrType::Array { elem: a, size: asz }, IrType::Array { elem: b, size: bsz }) => {
            asz == bsz && inline_types_compatible(a, b)
        }
        (IrType::Vec { elem: a }, IrType::Vec { elem: b }) => inline_types_compatible(a, b),
        (IrType::Map { value: a }, IrType::Map { value: b }) => inline_types_compatible(a, b),
        (
            IrType::Fn {
                params: a_params,
                ret: a_ret,
            },
            IrType::Fn {
                params: b_params,
                ret: b_ret,
            },
        ) => {
            a_params.len() == b_params.len()
                && a_params
                    .iter()
                    .zip(b_params.iter())
                    .all(|(a, b)| inline_types_compatible(a, b))
                && inline_types_compatible(a_ret, b_ret)
        }
        _ => false,
    }
}

fn remap_instr(
    instr: &Instr,
    local_map: &HashMap<LocalId, LocalId>,
    temp_map: &HashMap<TempId, TempId>,
) -> Instr {
    match instr {
        Instr::Const { dst, ty, value } => Instr::Const {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            value: value.clone(),
        },
        Instr::Copy { dst, ty, src } => Instr::Copy {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            src: remap_operand(src, local_map, temp_map),
        },
        Instr::Unary {
            dst,
            ty,
            op,
            operand,
        } => Instr::Unary {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            op: *op,
            operand: remap_operand(operand, local_map, temp_map),
        },
        Instr::Binary {
            dst,
            ty,
            op,
            left,
            right,
        } => Instr::Binary {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            op: *op,
            left: remap_operand(left, local_map, temp_map),
            right: remap_operand(right, local_map, temp_map),
        },
        Instr::Compare {
            dst,
            op,
            left,
            right,
        } => Instr::Compare {
            dst: remap_temp(*dst, temp_map),
            op: *op,
            left: remap_operand(left, local_map, temp_map),
            right: remap_operand(right, local_map, temp_map),
        },
        Instr::Logic {
            dst,
            op,
            left,
            right,
        } => Instr::Logic {
            dst: remap_temp(*dst, temp_map),
            op: *op,
            left: remap_operand(left, local_map, temp_map),
            right: remap_operand(right, local_map, temp_map),
        },
        Instr::LoadGlobal { dst, ty, global } => Instr::LoadGlobal {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            global: *global,
        },
        Instr::StoreGlobal { global, ty, value } => Instr::StoreGlobal {
            global: *global,
            ty: ty.clone(),
            value: remap_operand(value, local_map, temp_map),
        },
        Instr::LoadLocal { dst, ty, local } => Instr::LoadLocal {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            local: remap_local(*local, local_map),
        },
        Instr::StoreLocal { local, ty, value } => Instr::StoreLocal {
            local: remap_local(*local, local_map),
            ty: ty.clone(),
            value: remap_operand(value, local_map, temp_map),
        },
        Instr::MakeArray {
            dst,
            elem_ty,
            items,
        } => Instr::MakeArray {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
            items: items
                .iter()
                .map(|item| remap_operand(item, local_map, temp_map))
                .collect(),
        },
        Instr::MakeArrayRepeat {
            dst,
            elem_ty,
            value,
            size,
        } => Instr::MakeArrayRepeat {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
            value: remap_operand(value, local_map, temp_map),
            size: *size,
        },
        Instr::VecNew { dst, elem_ty } => Instr::VecNew {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
        },
        Instr::VecLen { dst, vec } => Instr::VecLen {
            dst: remap_temp(*dst, temp_map),
            vec: remap_operand(vec, local_map, temp_map),
        },
        Instr::ArrayGet {
            dst,
            elem_ty,
            array,
            index,
        } => Instr::ArrayGet {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
            array: remap_operand(array, local_map, temp_map),
            index: remap_operand(index, local_map, temp_map),
        },
        Instr::ArraySet {
            elem_ty,
            array,
            index,
            value,
        } => Instr::ArraySet {
            elem_ty: elem_ty.clone(),
            array: remap_operand(array, local_map, temp_map),
            index: remap_operand(index, local_map, temp_map),
            value: remap_operand(value, local_map, temp_map),
        },
        Instr::VecPush { vec, value } => Instr::VecPush {
            vec: remap_operand(vec, local_map, temp_map),
            value: remap_operand(value, local_map, temp_map),
        },
        Instr::VecGet {
            dst,
            elem_ty,
            vec,
            index,
        } => Instr::VecGet {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
            vec: remap_operand(vec, local_map, temp_map),
            index: remap_operand(index, local_map, temp_map),
        },
        Instr::VecSet {
            elem_ty,
            vec,
            index,
            value,
        } => Instr::VecSet {
            elem_ty: elem_ty.clone(),
            vec: remap_operand(vec, local_map, temp_map),
            index: remap_operand(index, local_map, temp_map),
            value: remap_operand(value, local_map, temp_map),
        },
        Instr::VecDelete {
            dst,
            elem_ty,
            vec,
            index,
        } => Instr::VecDelete {
            dst: remap_temp(*dst, temp_map),
            elem_ty: elem_ty.clone(),
            vec: remap_operand(vec, local_map, temp_map),
            index: remap_operand(index, local_map, temp_map),
        },
        Instr::MakeStruct {
            dst,
            struct_id,
            fields,
        } => Instr::MakeStruct {
            dst: remap_temp(*dst, temp_map),
            struct_id: *struct_id,
            fields: fields
                .iter()
                .map(|field| remap_operand(field, local_map, temp_map))
                .collect(),
        },
        Instr::StructGet {
            dst,
            ty,
            base,
            field,
        } => Instr::StructGet {
            dst: remap_temp(*dst, temp_map),
            ty: ty.clone(),
            base: remap_operand(base, local_map, temp_map),
            field: remap_field(field),
        },
        Instr::StructSet {
            base,
            field,
            value,
            ty,
        } => Instr::StructSet {
            base: remap_operand(base, local_map, temp_map),
            field: remap_field(field),
            value: remap_operand(value, local_map, temp_map),
            ty: ty.clone(),
        },
        Instr::MakeClosure { dst, function } => Instr::MakeClosure {
            dst: remap_temp(*dst, temp_map),
            function: *function,
        },
        Instr::CallDirect {
            dst,
            ret_ty,
            function,
            args,
        } => Instr::CallDirect {
            dst: dst.map(|id| remap_temp(id, temp_map)),
            ret_ty: ret_ty.clone(),
            function: *function,
            args: args
                .iter()
                .map(|arg| remap_operand(arg, local_map, temp_map))
                .collect(),
        },
        Instr::CallIndirect {
            dst,
            ret_ty,
            callee,
            args,
        } => Instr::CallIndirect {
            dst: dst.map(|id| remap_temp(id, temp_map)),
            ret_ty: ret_ty.clone(),
            callee: remap_operand(callee, local_map, temp_map),
            args: args
                .iter()
                .map(|arg| remap_operand(arg, local_map, temp_map))
                .collect(),
        },
        Instr::CallBuiltin {
            dst,
            ret_ty,
            builtin,
            args,
        } => Instr::CallBuiltin {
            dst: dst.map(|id| remap_temp(id, temp_map)),
            ret_ty: ret_ty.clone(),
            builtin: remap_builtin(builtin),
            args: args
                .iter()
                .map(|arg| remap_operand(arg, local_map, temp_map))
                .collect(),
        },
    }
}

fn remap_operand(
    operand: &Operand,
    local_map: &HashMap<LocalId, LocalId>,
    temp_map: &HashMap<TempId, TempId>,
) -> Operand {
    match operand {
        Operand::Const(ConstValue::Int(v)) => Operand::Const(ConstValue::Int(*v)),
        Operand::Const(ConstValue::Float(v)) => Operand::Const(ConstValue::Float(*v)),
        Operand::Const(ConstValue::Bool(v)) => Operand::Const(ConstValue::Bool(*v)),
        Operand::Const(ConstValue::String(v)) => Operand::Const(ConstValue::String(v.clone())),
        Operand::Const(ConstValue::Unit) => Operand::Const(ConstValue::Unit),
        Operand::Temp(id) => Operand::Temp(remap_temp(*id, temp_map)),
        Operand::Local(id) => Operand::Local(remap_local(*id, local_map)),
        Operand::Global(id) => Operand::Global(*id),
    }
}

fn remap_temp(id: TempId, temp_map: &HashMap<TempId, TempId>) -> TempId {
    temp_map.get(&id).copied().unwrap_or(id)
}

fn remap_local(id: LocalId, local_map: &HashMap<LocalId, LocalId>) -> LocalId {
    local_map.get(&id).copied().unwrap_or(id)
}

fn remap_field(field: &FieldRef) -> FieldRef {
    FieldRef {
        index: field.index,
        name: field.name.clone(),
    }
}

fn remap_builtin(builtin: &BuiltinCall) -> BuiltinCall {
    BuiltinCall {
        package: builtin.package.clone(),
        name: builtin.name.clone(),
    }
}

fn next_local_id(func: &IrFunction) -> usize {
    func.locals
        .iter()
        .map(|local| local.id.0)
        .max()
        .map(|id| id + 1)
        .unwrap_or(0)
}

fn next_temp_id(func: &IrFunction) -> usize {
    func.temps
        .iter()
        .map(|temp| temp.id.0)
        .max()
        .map(|id| id + 1)
        .unwrap_or(0)
}
