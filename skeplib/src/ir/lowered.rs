use crate::ir::{
    FunctionId, IrFunction, LocalId, NativeAggregatePlan, NativeArrayPlan, NativeCallLowering,
    NativeCallPlan, NativeStringPlan, NativeStringValue, NativeStructPlan, Operand, TempId,
};

#[derive(Debug, Clone)]
pub struct LoweredIrFunction {
    pub native_aggregates: NativeAggregatePlan,
    pub native_calls: NativeCallPlan,
    pub native_strings: NativeStringPlan,
}

impl LoweredIrFunction {
    pub fn analyze(func: &IrFunction) -> Self {
        Self {
            native_aggregates: NativeAggregatePlan::analyze(func),
            native_calls: NativeCallPlan::analyze(func),
            native_strings: NativeStringPlan::analyze(func),
        }
    }

    pub fn array_local(&self, local: LocalId) -> Option<NativeArrayPlan> {
        self.native_aggregates.array_local(local)
    }

    pub fn struct_local(&self, local: LocalId) -> Option<NativeStructPlan> {
        self.native_aggregates.struct_local(local)
    }

    pub fn root_struct_local(&self, local: LocalId) -> Option<LocalId> {
        self.native_aggregates.root_struct_local(local)
    }

    pub fn temp_root(&self, temp: TempId) -> Option<LocalId> {
        self.native_aggregates.temp_root(temp)
    }

    pub fn root_struct_plan(&self, local: LocalId) -> Option<(LocalId, NativeStructPlan)> {
        self.native_aggregates.root_struct_plan(local)
    }

    pub fn known_function(&self, operand: &Operand) -> Option<FunctionId> {
        self.native_calls.known_function(operand)
    }

    pub fn operand_call_lowering(&self, operand: &Operand) -> NativeCallLowering {
        self.native_calls.operand_lowering(operand)
    }

    pub fn temp_call_lowering(&self, temp: TempId) -> NativeCallLowering {
        self.native_calls.temp_lowering(temp)
    }

    pub fn string_value(&self, operand: &Operand) -> Option<NativeStringValue> {
        self.native_strings.string_value(operand)
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{self, ConstValue, LoweredIrFunction, NativeCallLowering, Operand};

    #[test]
    fn lowered_function_bundles_native_lowering_state() {
        let source = r#"
fn inc(x: Int) -> Int { return x + 1; }

fn main() -> Int {
  let f: Fn(Int) -> Int = inc;
  let xs: [Int; 4] = [0; 4];
  return f(xs[0]);
}
"#;

        let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
        let main = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .expect("main function");
        let lowered = LoweredIrFunction::analyze(main);

        let array_local = main.locals.iter().find(|local| local.name == "xs").unwrap();
        let fn_local = main.locals.iter().find(|local| local.name == "f").unwrap();

        assert!(
            lowered
                .native_aggregates
                .array_local(array_local.id)
                .is_some()
        );
        assert!(matches!(
            lowered
                .native_calls
                .operand_lowering(&Operand::Local(fn_local.id)),
            NativeCallLowering::KnownFunction(_)
        ));
        assert!(
            lowered
                .native_strings
                .string_value(&Operand::Const(ConstValue::String("x".into())))
                .is_some()
        );
    }
}
