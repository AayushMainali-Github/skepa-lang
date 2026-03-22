use crate::ir::{IrFunction, LocalId, NativeLocalKind, NativeabilityAnalysis, TempId};

#[derive(Debug, Clone, Default)]
pub struct NativeAggregatePlan {
    nativeability: NativeabilityAnalysis,
}

impl NativeAggregatePlan {
    pub fn analyze(func: &IrFunction) -> Self {
        Self {
            nativeability: NativeabilityAnalysis::analyze(func),
        }
    }

    pub fn local(&self, local: LocalId) -> Option<&NativeLocalKind> {
        self.nativeability.local(local)
    }

    pub fn temp_root(&self, temp: TempId) -> Option<LocalId> {
        self.nativeability.temp_root(temp)
    }

    pub fn root_struct_local(&self, local: LocalId) -> Option<LocalId> {
        self.nativeability.root_struct_local(local)
    }
}

#[cfg(test)]
mod tests {
    use super::NativeAggregatePlan;
    use crate::ir::{self, NativeLocalKind};

    #[test]
    fn native_aggregate_plan_exposes_scalar_and_array_shapes() {
        let src = r#"
fn main() -> Void {
  let ints: [Int; 4] = [7; 4];
  let floats: [Float; 3] = [1.5; 3];
  let words: [String; 2] = ["a", "b"];
  let pair = Pair { left: 1, right: 2 };
}

struct Pair {
  left: Int,
  right: Int,
}
"#;

        let program = ir::lowering::compile_source(src).expect("IR lowering should succeed");
        let main = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .expect("main");
        let plan = NativeAggregatePlan::analyze(main);

        let ints = main
            .locals
            .iter()
            .find(|local| local.name == "ints")
            .unwrap();
        let floats = main
            .locals
            .iter()
            .find(|local| local.name == "floats")
            .unwrap();
        let words = main
            .locals
            .iter()
            .find(|local| local.name == "words")
            .unwrap();
        let pair = main
            .locals
            .iter()
            .find(|local| local.name == "pair")
            .unwrap();

        assert!(matches!(
            plan.local(ints.id),
            Some(NativeLocalKind::IntArray { size: 4, .. })
        ));
        assert!(matches!(
            plan.local(floats.id),
            Some(NativeLocalKind::FloatArray { size: 3, .. })
        ));
        assert!(matches!(
            plan.local(words.id),
            Some(NativeLocalKind::StringArray { size: 2, .. })
        ));
        assert!(matches!(
            plan.local(pair.id),
            Some(NativeLocalKind::ScalarStruct { .. })
        ));
    }
}
