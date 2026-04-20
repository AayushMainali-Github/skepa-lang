use crate::ir::{Instr, IrFunction, IrType, LocalId, Operand, TempId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum NativeLocalKind {
    IntArray { size: usize, init: Operand },
    FloatArray { size: usize, init: Operand },
    StringArray { size: usize, items: Vec<Operand> },
    ScalarStruct { fields: Vec<Operand> },
    StructAlias { root: LocalId },
}

#[derive(Debug, Clone, Default)]
pub struct NativeabilityAnalysis {
    locals: HashMap<LocalId, NativeLocalKind>,
    temp_roots: HashMap<TempId, LocalId>,
}

impl NativeabilityAnalysis {
    pub fn analyze(func: &IrFunction) -> Self {
        let mut temp_structs = HashMap::new();
        let mut temp_int_arrays = HashMap::new();
        let mut temp_float_arrays = HashMap::new();
        let mut temp_string_arrays = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    Instr::MakeStruct { dst, fields, .. }
                        if fields.iter().all(|field| operand_is_int(func, field)) =>
                    {
                        temp_structs.insert(*dst, fields.clone());
                    }
                    Instr::MakeArrayRepeat {
                        dst,
                        elem_ty: IrType::Int,
                        value,
                        size,
                    } if operand_is_int(func, value) => {
                        temp_int_arrays.insert(*dst, (value.clone(), *size));
                    }
                    Instr::MakeArrayRepeat {
                        dst,
                        elem_ty: IrType::Float,
                        value,
                        size,
                    } if operand_is_float(func, value) => {
                        temp_float_arrays.insert(*dst, (value.clone(), *size));
                    }
                    Instr::MakeArray {
                        dst,
                        elem_ty: IrType::String,
                        items,
                    } if !items.is_empty()
                        && items.iter().all(|item| operand_is_string(func, item)) =>
                    {
                        temp_string_arrays.insert(*dst, items.clone());
                    }
                    _ => {}
                }
            }
        }

        let mut stores: HashMap<LocalId, Vec<Operand>> = HashMap::new();
        for block in &func.blocks {
            for instr in &block.instrs {
                if let Instr::StoreLocal { local, value, .. } = instr {
                    stores.entry(*local).or_default().push(value.clone());
                }
            }
        }

        let mut locals = HashMap::new();
        let mut temp_roots = HashMap::new();

        for local in &func.locals {
            let Some(values) = stores.get(&local.id) else {
                continue;
            };
            if values.len() != 1 {
                continue;
            }
            match (&local.ty, &values[0]) {
                (IrType::Named(_), Operand::Temp(temp)) => {
                    let Some(fields) = temp_structs.get(temp).cloned() else {
                        continue;
                    };
                    if !struct_root_safe(func, local.id) {
                        continue;
                    }
                    locals.insert(local.id, NativeLocalKind::ScalarStruct { fields });
                    temp_roots.insert(*temp, local.id);
                }
                (IrType::Array { elem, size }, Operand::Temp(temp)) if **elem == IrType::Int => {
                    let Some((init, init_size)) = temp_int_arrays.get(temp).cloned() else {
                        continue;
                    };
                    if *size != init_size || !array_root_safe(func, local.id, &IrType::Int) {
                        continue;
                    }
                    locals.insert(local.id, NativeLocalKind::IntArray { size: *size, init });
                    temp_roots.insert(*temp, local.id);
                }
                (IrType::Array { elem, size }, Operand::Temp(temp)) if **elem == IrType::Float => {
                    let Some((init, init_size)) = temp_float_arrays.get(temp).cloned() else {
                        continue;
                    };
                    if *size != init_size || !array_root_safe(func, local.id, &IrType::Float) {
                        continue;
                    }
                    locals.insert(local.id, NativeLocalKind::FloatArray { size: *size, init });
                    temp_roots.insert(*temp, local.id);
                }
                (IrType::Array { elem, size }, Operand::Temp(temp)) if **elem == IrType::String => {
                    let Some(items) = temp_string_arrays.get(temp).cloned() else {
                        continue;
                    };
                    if *size != items.len() || !string_array_root_safe(func, local.id) {
                        continue;
                    }
                    locals.insert(
                        local.id,
                        NativeLocalKind::StringArray { size: *size, items },
                    );
                    temp_roots.insert(*temp, local.id);
                }
                _ => {}
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for local in &func.locals {
                if locals.contains_key(&local.id) {
                    continue;
                }
                let Some(values) = stores.get(&local.id) else {
                    continue;
                };
                if values.len() != 1 {
                    continue;
                }
                let Operand::Local(src) = values[0] else {
                    continue;
                };
                let Some(root) = root_struct_local(&locals, src) else {
                    continue;
                };
                if !struct_alias_safe(func, local.id) {
                    continue;
                }
                locals.insert(local.id, NativeLocalKind::StructAlias { root });
                changed = true;
            }
        }

        Self { locals, temp_roots }
    }

    pub fn local(&self, local: LocalId) -> Option<&NativeLocalKind> {
        self.locals.get(&local)
    }

    pub fn temp_root(&self, temp: TempId) -> Option<LocalId> {
        self.temp_roots.get(&temp).copied()
    }

    pub fn root_struct_local(&self, local: LocalId) -> Option<LocalId> {
        root_struct_local(&self.locals, local)
    }
}

fn operand_is_int(func: &IrFunction, operand: &Operand) -> bool {
    match operand {
        Operand::Const(crate::ir::ConstValue::Int(_)) => true,
        Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .is_some_and(|temp| temp.ty == IrType::Int),
        Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .is_some_and(|local| local.ty == IrType::Int),
        Operand::Global(_) | Operand::Const(_) => false,
    }
}

fn operand_is_float(func: &IrFunction, operand: &Operand) -> bool {
    match operand {
        Operand::Const(crate::ir::ConstValue::Float(_)) => true,
        Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .is_some_and(|temp| temp.ty == IrType::Float),
        Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .is_some_and(|local| local.ty == IrType::Float),
        Operand::Global(_) | Operand::Const(_) => false,
    }
}

fn operand_is_string(func: &IrFunction, operand: &Operand) -> bool {
    match operand {
        Operand::Const(crate::ir::ConstValue::String(_)) => true,
        Operand::Temp(id) => func
            .temps
            .iter()
            .find(|temp| temp.id == *id)
            .is_some_and(|temp| temp.ty == IrType::String),
        Operand::Local(id) => func
            .locals
            .iter()
            .find(|local| local.id == *id)
            .is_some_and(|local| local.ty == IrType::String),
        Operand::Global(_) | Operand::Const(_) => false,
    }
}

fn root_struct_local(
    locals: &HashMap<LocalId, NativeLocalKind>,
    local: LocalId,
) -> Option<LocalId> {
    match locals.get(&local) {
        Some(NativeLocalKind::ScalarStruct { .. }) => Some(local),
        Some(NativeLocalKind::StructAlias { root }) => Some(*root),
        Some(NativeLocalKind::IntArray { .. })
        | Some(NativeLocalKind::FloatArray { .. })
        | Some(NativeLocalKind::StringArray { .. })
        | None => None,
    }
}

fn struct_root_safe(func: &IrFunction, target: LocalId) -> bool {
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::StoreLocal { local, .. } if *local == target => {}
                Instr::StoreLocal {
                    value: Operand::Local(local),
                    ..
                } if *local == target => {}
                Instr::StructGet {
                    base: Operand::Local(local),
                    ty: IrType::Int,
                    ..
                } if *local == target => {}
                _ if instr_mentions_local(instr, target) => return false,
                _ => {}
            }
        }
        if terminator_mentions_local(&block.terminator, target) {
            return false;
        }
    }
    true
}

fn array_root_safe(func: &IrFunction, target: LocalId, elem_ty: &IrType) -> bool {
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::StoreLocal { local, .. } if *local == target => {}
                Instr::ArrayGet {
                    array: Operand::Local(local),
                    elem_ty: instr_elem_ty,
                    ..
                }
                | Instr::ArraySet {
                    array: Operand::Local(local),
                    elem_ty: instr_elem_ty,
                    ..
                } if *local == target && instr_elem_ty == elem_ty => {}
                _ if instr_mentions_local(instr, target) => return false,
                _ => {}
            }
        }
        if terminator_mentions_local(&block.terminator, target) {
            return false;
        }
    }
    true
}

fn struct_alias_safe(func: &IrFunction, target: LocalId) -> bool {
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::StoreLocal { local, .. } if *local == target => {}
                Instr::StructGet {
                    base: Operand::Local(local),
                    ty: IrType::Int,
                    ..
                } if *local == target => {}
                _ if instr_mentions_local(instr, target) => return false,
                _ => {}
            }
        }
        if terminator_mentions_local(&block.terminator, target) {
            return false;
        }
    }
    true
}

fn string_array_root_safe(func: &IrFunction, target: LocalId) -> bool {
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::StoreLocal { local, .. } if *local == target => {}
                Instr::ArrayGet {
                    array: Operand::Local(local),
                    elem_ty: IrType::String,
                    ..
                } if *local == target => {}
                _ if instr_mentions_local(instr, target) => return false,
                _ => {}
            }
        }
        if terminator_mentions_local(&block.terminator, target) {
            return false;
        }
    }
    true
}

fn instr_mentions_local(instr: &Instr, target: LocalId) -> bool {
    let mut found = false;
    visit_operands(instr, &mut |operand| {
        if let Operand::Local(local) = operand
            && *local == target
        {
            found = true;
        }
    });
    found
}

fn terminator_mentions_local(terminator: &crate::ir::Terminator, target: LocalId) -> bool {
    let mut found = false;
    match terminator {
        crate::ir::Terminator::Branch(branch) => {
            if let Operand::Local(local) = &branch.cond
                && *local == target
            {
                found = true;
            }
        }
        crate::ir::Terminator::Return(Some(Operand::Local(local))) if *local == target => {
            found = true;
        }
        _ => {}
    }
    found
}

fn visit_operands(instr: &Instr, f: &mut impl FnMut(&Operand)) {
    match instr {
        Instr::Copy { src, .. }
        | Instr::StoreGlobal { value: src, .. }
        | Instr::StoreLocal { value: src, .. }
        | Instr::VecLen { vec: src, .. }
        | Instr::VecPush { value: src, .. }
        | Instr::MakeArrayRepeat { value: src, .. } => f(src),
        Instr::Unary { operand, .. } => f(operand),
        Instr::Binary { left, right, .. }
        | Instr::Compare { left, right, .. }
        | Instr::Logic { left, right, .. } => {
            f(left);
            f(right);
        }
        Instr::MakeArray { items, .. }
        | Instr::MakeStruct { fields: items, .. }
        | Instr::CallDirect { args: items, .. }
        | Instr::CallIndirect { args: items, .. }
        | Instr::CallBuiltin { args: items, .. } => {
            for item in items {
                f(item);
            }
        }
        Instr::ArrayGet { array, index, .. }
        | Instr::VecGet {
            vec: array, index, ..
        }
        | Instr::VecDelete {
            vec: array, index, ..
        } => {
            f(array);
            f(index);
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
            f(array);
            f(index);
            f(value);
        }
        Instr::StructGet { base, .. } => f(base),
        Instr::StructSet { base, value, .. } => {
            f(base);
            f(value);
        }
        Instr::LoadGlobal { .. }
        | Instr::LoadLocal { .. }
        | Instr::Const { .. }
        | Instr::VecNew { .. }
        | Instr::MakeClosure { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeLocalKind, NativeabilityAnalysis};
    use crate::ir;

    #[test]
    fn nativeability_detects_hot_local_native_shapes() {
        let source = r#"
struct Pair {
  a: Int,
  b: Int
}

fn main() -> Int {
  let ints: [Int; 4] = [0; 4];
  let floats: [Float; 3] = [1.5; 3];
  let words: [String; 2] = ["a", "b"];
  let p = Pair { a: 1, b: 2 };
  return ints[0] + p.a + 1;
}
"#;

        let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
        let main = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .expect("main should exist");
        let analysis = NativeabilityAnalysis::analyze(main);

        assert!(matches!(
            analysis.local(ir::LocalId(0)),
            Some(NativeLocalKind::IntArray { size: 4, .. })
        ));
        assert!(matches!(
            analysis.local(ir::LocalId(1)),
            Some(NativeLocalKind::FloatArray { size: 3, .. })
        ));
        assert!(matches!(
            analysis.local(ir::LocalId(2)),
            Some(NativeLocalKind::StringArray { size: 2, .. })
        ));
        assert!(matches!(
            analysis.local(ir::LocalId(3)),
            Some(NativeLocalKind::ScalarStruct { .. })
        ));
    }
}
