use crate::ir::{IrFunction, LocalId, NativeLocalKind, NativeabilityAnalysis, Operand, TempId};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeArrayPlan {
    IntRepeat { size: usize, init: Operand },
    FloatRepeat { size: usize, init: Operand },
    StringItems { size: usize, items: Vec<Operand> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NativeStructPlan {
    ScalarFields { fields: Vec<Operand> },
    Alias { root: LocalId },
}

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

    pub fn array_local(&self, local: LocalId) -> Option<NativeArrayPlan> {
        match self.nativeability.local(local)? {
            NativeLocalKind::IntArray { size, init } => Some(NativeArrayPlan::IntRepeat {
                size: *size,
                init: init.clone(),
            }),
            NativeLocalKind::FloatArray { size, init } => Some(NativeArrayPlan::FloatRepeat {
                size: *size,
                init: init.clone(),
            }),
            NativeLocalKind::StringArray { size, items } => Some(NativeArrayPlan::StringItems {
                size: *size,
                items: items.clone(),
            }),
            NativeLocalKind::ScalarStruct { .. } | NativeLocalKind::StructAlias { .. } => None,
        }
    }

    pub fn struct_local(&self, local: LocalId) -> Option<NativeStructPlan> {
        match self.nativeability.local(local)? {
            NativeLocalKind::ScalarStruct { fields } => Some(NativeStructPlan::ScalarFields {
                fields: fields.clone(),
            }),
            NativeLocalKind::StructAlias { root } => Some(NativeStructPlan::Alias { root: *root }),
            NativeLocalKind::IntArray { .. }
            | NativeLocalKind::FloatArray { .. }
            | NativeLocalKind::StringArray { .. } => None,
        }
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
    use super::{NativeAggregatePlan, NativeArrayPlan, NativeStructPlan};
    use crate::ir;

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
            plan.array_local(ints.id),
            Some(NativeArrayPlan::IntRepeat { size: 4, .. })
        ));
        assert!(matches!(
            plan.array_local(floats.id),
            Some(NativeArrayPlan::FloatRepeat { size: 3, .. })
        ));
        assert!(matches!(
            plan.array_local(words.id),
            Some(NativeArrayPlan::StringItems { size: 2, .. })
        ));
        assert!(matches!(
            plan.struct_local(pair.id),
            Some(NativeStructPlan::ScalarFields { .. })
        ));
    }
}
