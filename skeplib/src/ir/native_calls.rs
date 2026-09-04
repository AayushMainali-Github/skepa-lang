use crate::ir::{FunctionId, Instr, IrFunction, LocalId, Operand, TempId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCallLowering {
    KnownFunction(FunctionId),
    Dynamic,
}

#[derive(Debug, Clone, Default)]
pub struct NativeCallPlan {
    temps: HashMap<TempId, FunctionId>,
    locals: HashMap<LocalId, FunctionId>,
}

impl NativeCallPlan {
    pub fn analyze(func: &IrFunction) -> Self {
        // Analyze all assignments as a small, flow-insensitive dataflow problem. A
        // value is promoted only when every assignment that can reach its use has
        // the same known function. Conflicting or unknown assignments remain
        // dynamic, which is safer than choosing a callee from one branch.
        let mut temp_states: HashMap<TempId, ValueState> = HashMap::new();
        let mut local_states: HashMap<LocalId, ValueState> = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instrs {
                if let Instr::MakeClosure { dst, function } = instr {
                    temp_states.insert(*dst, ValueState::Known(*function));
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            let previous_temps = temp_states.clone();
            let previous_locals = local_states.clone();
            let mut next_temps = HashMap::new();
            let mut next_locals = HashMap::new();

            for block in &func.blocks {
                for instr in &block.instrs {
                    match instr {
                        Instr::MakeClosure { dst, function } => {
                            merge_state(&mut next_temps, *dst, ValueState::Known(*function));
                        }
                        Instr::Copy { dst, src, .. } => {
                            merge_state(
                                &mut next_temps,
                                *dst,
                                resolve_operand_state(src, &previous_temps, &previous_locals),
                            );
                        }
                        Instr::StoreLocal { local, value, .. } => {
                            merge_state(
                                &mut next_locals,
                                *local,
                                resolve_operand_state(value, &previous_temps, &previous_locals),
                            );
                        }
                        Instr::LoadLocal { dst, local, .. } => {
                            merge_state(
                                &mut next_temps,
                                *dst,
                                previous_locals.get(local).copied().unwrap_or_default(),
                            );
                        }
                        _ => {}
                    }
                }
            }

            if next_temps != previous_temps || next_locals != previous_locals {
                changed = true;
            }
            temp_states = next_temps;
            local_states = next_locals;
        }

        let temps = temp_states
            .into_iter()
            .filter_map(|(temp, state)| state.known().map(|function| (temp, function)))
            .collect();
        let locals = local_states
            .into_iter()
            .filter_map(|(local, state)| state.known().map(|function| (local, function)))
            .collect();

        Self { temps, locals }
    }

    pub fn known_function(&self, operand: &Operand) -> Option<FunctionId> {
        resolve_operand_function(operand, &self.temps, &self.locals)
    }

    pub fn operand_lowering(&self, operand: &Operand) -> NativeCallLowering {
        match self.known_function(operand) {
            Some(function) => NativeCallLowering::KnownFunction(function),
            None => NativeCallLowering::Dynamic,
        }
    }

    pub fn temp_lowering(&self, temp: TempId) -> NativeCallLowering {
        match self.temps.get(&temp).copied() {
            Some(function) => NativeCallLowering::KnownFunction(function),
            None => NativeCallLowering::Dynamic,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ValueState {
    #[default]
    Unknown,
    Known(FunctionId),
    Dynamic,
}

impl ValueState {
    fn known(self) -> Option<FunctionId> {
        match self {
            Self::Known(function) => Some(function),
            Self::Unknown | Self::Dynamic => None,
        }
    }
}

fn merge_state<K>(states: &mut HashMap<K, ValueState>, key: K, incoming: ValueState)
where
    K: Eq + std::hash::Hash,
{
    let merged = match (states.get(&key).copied().unwrap_or_default(), incoming) {
        (ValueState::Dynamic, _) | (_, ValueState::Dynamic) => ValueState::Dynamic,
        (ValueState::Unknown, state) | (state, ValueState::Unknown) => state,
        (ValueState::Known(left), ValueState::Known(right)) if left == right => {
            ValueState::Known(left)
        }
        (ValueState::Known(_), ValueState::Known(_)) => ValueState::Dynamic,
    };
    states.insert(key, merged);
}

fn resolve_operand_state(
    operand: &Operand,
    temps: &HashMap<TempId, ValueState>,
    locals: &HashMap<LocalId, ValueState>,
) -> ValueState {
    match operand {
        Operand::Temp(id) => temps.get(id).copied().unwrap_or_default(),
        Operand::Local(id) => locals.get(id).copied().unwrap_or_default(),
        Operand::Const(_) | Operand::Global(_) => ValueState::Dynamic,
    }
}

fn resolve_operand_function(
    operand: &Operand,
    temps: &HashMap<TempId, FunctionId>,
    locals: &HashMap<LocalId, FunctionId>,
) -> Option<FunctionId> {
    match operand {
        Operand::Temp(id) => temps.get(id).copied(),
        Operand::Local(id) => locals.get(id).copied(),
        Operand::Const(_) | Operand::Global(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeCallLowering, NativeCallPlan};
    use crate::ir;

    #[test]
    fn native_call_plan_tracks_known_function_values_through_locals() {
        let source = r#"
fn inc(x: Int) -> Int { return x + 1; }

fn main() -> Int {
  let f: Fn(Int) -> Int = inc;
  return f(41);
}
"#;

        let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
        let main = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .unwrap();
        let plan = NativeCallPlan::analyze(main);
        let local = main.locals.iter().find(|local| local.name == "f").unwrap();
        assert_eq!(
            plan.operand_lowering(&ir::Operand::Local(local.id)),
            NativeCallLowering::KnownFunction(ir::FunctionId(0))
        );
    }

    #[test]
    fn native_call_plan_does_not_choose_a_callee_from_one_branch() {
        let fn_ty = ir::IrType::Fn {
            params: vec![ir::IrType::Int],
            ret: Box::new(ir::IrType::Int),
        };
        let main = ir::IrFunction {
            id: ir::FunctionId(2),
            name: "main".into(),
            params: vec![],
            locals: vec![ir::IrLocal {
                id: ir::LocalId(0),
                name: "f".into(),
                ty: fn_ty.clone(),
            }],
            temps: vec![
                ir::IrTemp {
                    id: ir::TempId(0),
                    ty: fn_ty.clone(),
                },
                ir::IrTemp {
                    id: ir::TempId(1),
                    ty: fn_ty.clone(),
                },
                ir::IrTemp {
                    id: ir::TempId(2),
                    ty: ir::IrType::Bool,
                },
            ],
            ret_ty: ir::IrType::Int,
            entry: ir::BlockId(0),
            blocks: vec![
                ir::BasicBlock {
                    id: ir::BlockId(0),
                    name: "entry".into(),
                    instrs: vec![
                        ir::Instr::MakeClosure {
                            dst: ir::TempId(0),
                            function: ir::FunctionId(0),
                        },
                        ir::Instr::StoreLocal {
                            local: ir::LocalId(0),
                            ty: fn_ty.clone(),
                            value: ir::Operand::Temp(ir::TempId(0)),
                        },
                        ir::Instr::Const {
                            dst: ir::TempId(2),
                            ty: ir::IrType::Bool,
                            value: ir::ConstValue::Bool(false),
                        },
                    ],
                    terminator: ir::Terminator::Branch(ir::BranchTerminator {
                        cond: ir::Operand::Temp(ir::TempId(2)),
                        then_block: ir::BlockId(1),
                        else_block: ir::BlockId(2),
                    }),
                },
                ir::BasicBlock {
                    id: ir::BlockId(1),
                    name: "then".into(),
                    instrs: vec![
                        ir::Instr::MakeClosure {
                            dst: ir::TempId(1),
                            function: ir::FunctionId(1),
                        },
                        ir::Instr::StoreLocal {
                            local: ir::LocalId(0),
                            ty: fn_ty.clone(),
                            value: ir::Operand::Temp(ir::TempId(1)),
                        },
                    ],
                    terminator: ir::Terminator::Jump(ir::BlockId(2)),
                },
                ir::BasicBlock {
                    id: ir::BlockId(2),
                    name: "join".into(),
                    instrs: vec![],
                    terminator: ir::Terminator::Return(None),
                },
            ],
        };

        let plan = NativeCallPlan::analyze(&main);
        assert_eq!(
            plan.operand_lowering(&ir::Operand::Local(ir::LocalId(0))),
            NativeCallLowering::Dynamic
        );
    }
}
