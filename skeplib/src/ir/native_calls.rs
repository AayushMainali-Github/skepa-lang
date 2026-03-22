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
        let mut temps = HashMap::new();
        let mut locals = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instrs {
                if let Instr::MakeClosure { dst, function } = instr {
                    temps.insert(*dst, *function);
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block in &func.blocks {
                for instr in &block.instrs {
                    match instr {
                        Instr::Copy { dst, src, .. } => {
                            if let Some(function) = resolve_operand_function(src, &temps, &locals)
                                && temps.insert(*dst, function) != Some(function)
                            {
                                changed = true;
                            }
                        }
                        Instr::StoreLocal { local, value, .. } => {
                            if let Some(function) = resolve_operand_function(value, &temps, &locals)
                                && locals.insert(*local, function) != Some(function)
                            {
                                changed = true;
                            }
                        }
                        Instr::LoadLocal { dst, local, .. } => {
                            if let Some(function) = locals.get(local).copied()
                                && temps.insert(*dst, function) != Some(function)
                            {
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

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
}
