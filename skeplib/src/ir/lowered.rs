use crate::ir::{IrFunction, NativeAggregatePlan, NativeCallPlan, NativeStringPlan};

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
