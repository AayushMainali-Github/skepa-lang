use crate::ir::{
    BuiltinCall, ConstValue, Instr, IrFunction, IrProgram, LocalId, Operand, TempId, Terminator,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct FunctionConstValues {
    pub temps: HashMap<TempId, ConstValue>,
    pub locals: HashMap<LocalId, ConstValue>,
}

pub fn collect_string_literals(program: &IrProgram) -> HashMap<String, String> {
    let mut literals = HashMap::new();
    let mut index = 0usize;
    for func in &program.functions {
        let consts = analyze_const_values(func);
        for value in consts.temps.values().chain(consts.locals.values()) {
            if let ConstValue::String(value) = value {
                literals.entry(value.clone()).or_insert_with(|| {
                    let name = format!("@.str.{index}");
                    index += 1;
                    name
                });
            }
        }
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

pub fn analyze_const_values(func: &IrFunction) -> FunctionConstValues {
    let store_values = collect_local_store_values(func);
    let temp_defs = collect_temp_defs(func);
    let mut values = FunctionConstValues::default();

    loop {
        let next_locals = store_values
            .iter()
            .filter_map(|(local, stores)| {
                let mut resolved = stores
                    .iter()
                    .map(|operand| resolve_operand_const(operand, &values))
                    .collect::<Option<Vec<_>>>()?;
                let first = resolved.pop()?;
                if resolved.iter().all(|value| value == &first) {
                    Some((*local, first))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let next_temps = temp_defs
            .iter()
            .filter_map(|(temp, instr)| eval_temp_instr(instr, &values).map(|value| (*temp, value)))
            .collect::<HashMap<_, _>>();

        if next_locals == values.locals && next_temps == values.temps {
            values.locals = next_locals;
            values.temps = next_temps;
            break;
        }
        values.locals = next_locals;
        values.temps = next_temps;
    }

    values
}

pub fn encode_c_string(value: &str) -> String {
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

pub fn runtime_string_symbol(raw_symbol: &str) -> String {
    raw_symbol.replacen("@.str.", "@.rtstr.", 1)
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

fn collect_local_store_values(func: &IrFunction) -> HashMap<LocalId, Vec<Operand>> {
    let mut out: HashMap<LocalId, Vec<Operand>> = HashMap::new();
    for block in &func.blocks {
        for instr in &block.instrs {
            if let Instr::StoreLocal { local, value, .. } = instr {
                out.entry(*local).or_default().push(value.clone());
            }
        }
    }
    out
}

fn collect_temp_defs(func: &IrFunction) -> HashMap<TempId, Instr> {
    let mut out = HashMap::new();
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Const { dst, .. }
                | Instr::Copy { dst, .. }
                | Instr::LoadLocal { dst, .. }
                | Instr::CallBuiltin { dst: Some(dst), .. } => {
                    out.insert(*dst, instr.clone());
                }
                _ => {}
            }
        }
    }
    out
}

fn resolve_operand_const(operand: &Operand, values: &FunctionConstValues) -> Option<ConstValue> {
    match operand {
        Operand::Const(value) => Some(value.clone()),
        Operand::Temp(id) => values.temps.get(id).cloned(),
        Operand::Local(id) => values.locals.get(id).cloned(),
        Operand::Global(_) => None,
    }
}

fn eval_temp_instr(instr: &Instr, values: &FunctionConstValues) -> Option<ConstValue> {
    match instr {
        Instr::Const { value, .. } => Some(value.clone()),
        Instr::Copy { src, .. } => resolve_operand_const(src, values),
        Instr::LoadLocal { local, .. } => resolve_operand_const(&Operand::Local(*local), values),
        Instr::CallBuiltin { builtin, args, .. } => eval_const_builtin(builtin, args, values),
        _ => None,
    }
}

pub fn eval_const_builtin(
    builtin: &BuiltinCall,
    args: &[Operand],
    values: &FunctionConstValues,
) -> Option<ConstValue> {
    let resolved = args
        .iter()
        .map(|arg| resolve_operand_const(arg, values))
        .collect::<Option<Vec<_>>>()?;
    match (
        builtin.package.as_str(),
        builtin.name.as_str(),
        resolved.as_slice(),
    ) {
        ("str", "len", [ConstValue::String(value)]) => {
            Some(ConstValue::Int(value.chars().count() as i64))
        }
        ("str", "contains", [ConstValue::String(haystack), ConstValue::String(needle)]) => {
            Some(ConstValue::Bool(haystack.contains(needle)))
        }
        ("str", "indexOf", [ConstValue::String(haystack), ConstValue::String(needle)]) => {
            let index = if haystack.is_ascii() && needle.is_ascii() {
                haystack.find(needle).map(|idx| idx as i64).unwrap_or(-1)
            } else {
                haystack
                    .find(needle)
                    .map(|idx| haystack[..idx].chars().count() as i64)
                    .unwrap_or(-1)
            };
            Some(ConstValue::Int(index))
        }
        (
            "str",
            "slice",
            [
                ConstValue::String(value),
                ConstValue::Int(start),
                ConstValue::Int(end),
            ],
        ) if *start >= 0 && *end >= 0 => {
            let start = *start as usize;
            let end = *end as usize;
            let sliced = const_string_slice(value, start, end)?;
            Some(ConstValue::String(sliced))
        }
        _ => None,
    }
}

fn const_string_slice(value: &str, start: usize, end: usize) -> Option<String> {
    if value.is_ascii() {
        if start > end || end > value.len() {
            return None;
        }
        return Some(value[start..end].to_string());
    }
    let start_byte = nth_char_boundary(value, start)?;
    let end_byte = nth_char_boundary(value, end)?;
    if start_byte > end_byte {
        return None;
    }
    Some(value[start_byte..end_byte].to_string())
}

fn nth_char_boundary(value: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return Some(0);
    }
    let char_len = value.chars().count();
    if index > char_len {
        return None;
    }
    if index == char_len {
        return Some(value.len());
    }
    value.char_indices().nth(index).map(|(offset, _)| offset)
}
