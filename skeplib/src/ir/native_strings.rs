use crate::ir::{
    BuiltinCall, ConstValue, Instr, IrFunction, IrProgram, LocalId, Operand, TempId, Terminator,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NativeStringPlan {
    temps: HashMap<TempId, ConstValue>,
    locals: HashMap<LocalId, ConstValue>,
}

impl NativeStringPlan {
    pub fn analyze(func: &IrFunction) -> Self {
        let store_values = collect_local_store_values(func);
        let temp_defs = collect_temp_defs(func);
        let mut values = Self::default();

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
                .filter_map(|(temp, instr)| {
                    eval_temp_instr(instr, &values).map(|value| (*temp, value))
                })
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

    pub fn const_value(&self, operand: &Operand) -> Option<ConstValue> {
        resolve_operand_const(operand, self)
    }

    pub fn eval_const_builtin(
        &self,
        builtin: &BuiltinCall,
        args: &[Operand],
    ) -> Option<ConstValue> {
        eval_const_builtin(builtin, args, self)
    }
}

pub fn collect_program_string_constants(program: &IrProgram) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();

    let mut add = |value: &str| {
        if seen.insert(value.to_string()) {
            ordered.push(value.to_string());
        }
    };

    for func in &program.functions {
        let plan = NativeStringPlan::analyze(func);
        for value in plan.temps.values().chain(plan.locals.values()) {
            if let ConstValue::String(value) = value {
                add(value);
            }
        }
        for block in &func.blocks {
            for instr in &block.instrs {
                collect_instr_string_literals(instr, &mut add);
            }
            if let Terminator::Return(Some(Operand::Const(ConstValue::String(value)))) =
                &block.terminator
            {
                add(value);
            }
        }
    }

    ordered
}

fn collect_instr_string_literals(instr: &Instr, add: &mut impl FnMut(&str)) {
    let mut add_operand = |operand: &Operand| {
        if let Operand::Const(ConstValue::String(value)) = operand {
            add(value);
        }
    };
    match instr {
        Instr::Const {
            value: ConstValue::String(value),
            ..
        } => add(value),
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
            add(&builtin.package);
            add(&builtin.name);
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

fn resolve_operand_const(operand: &Operand, values: &NativeStringPlan) -> Option<ConstValue> {
    match operand {
        Operand::Const(value) => Some(value.clone()),
        Operand::Temp(id) => values.temps.get(id).cloned(),
        Operand::Local(id) => values.locals.get(id).cloned(),
        Operand::Global(_) => None,
    }
}

fn eval_temp_instr(instr: &Instr, values: &NativeStringPlan) -> Option<ConstValue> {
    match instr {
        Instr::Const { value, .. } => Some(value.clone()),
        Instr::Copy { src, .. } => resolve_operand_const(src, values),
        Instr::LoadLocal { local, .. } => resolve_operand_const(&Operand::Local(*local), values),
        Instr::CallBuiltin { builtin, args, .. } => eval_const_builtin(builtin, args, values),
        _ => None,
    }
}

fn eval_const_builtin(
    builtin: &BuiltinCall,
    args: &[Operand],
    values: &NativeStringPlan,
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

#[cfg(test)]
mod tests {
    use super::{NativeStringPlan, collect_program_string_constants};
    use crate::ir;
    use crate::ir::{BuiltinCall, ConstValue, Operand};

    #[test]
    fn native_string_plan_tracks_foldable_string_builtins() {
        let source = r#"
fn main() -> Int {
  let s = "skepa";
  let cut = str.slice(s, 1, 4);
  return str.indexOf(cut, "e");
}
"#;
        let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let plan = NativeStringPlan::analyze(main);
        let folded = plan
            .eval_const_builtin(
                &BuiltinCall {
                    package: "str".into(),
                    name: "slice".into(),
                },
                &[
                    Operand::Const(ConstValue::String("skepa".into())),
                    Operand::Const(ConstValue::Int(1)),
                    Operand::Const(ConstValue::Int(4)),
                ],
            )
            .expect("slice should fold");
        assert_eq!(folded, ConstValue::String("kep".into()));

        let literals = collect_program_string_constants(&program);
        assert!(literals.contains(&"skepa".to_string()));
        assert!(literals.contains(&"kep".to_string()));
    }
}
