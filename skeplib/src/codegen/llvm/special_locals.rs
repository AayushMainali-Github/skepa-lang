use crate::ir::{IrFunction, LocalId, NativeLocalKind, NativeabilityAnalysis, Operand, TempId};

#[derive(Clone)]
pub enum SpecialLocalKind {
    IntArray { size: usize, init: Operand },
    FloatArray { size: usize, init: Operand },
    StringArray { size: usize, items: Vec<Operand> },
    ScalarStruct { fields: Vec<Operand> },
    StructAlias { root: LocalId },
}

pub struct SpecialLocals {
    locals: std::collections::HashMap<LocalId, SpecialLocalKind>,
    temp_roots: std::collections::HashMap<TempId, LocalId>,
}

impl SpecialLocals {
    pub fn analyze(func: &IrFunction) -> Self {
        let analysis = NativeabilityAnalysis::analyze(func);
        let mut locals = std::collections::HashMap::new();
        for local in &func.locals {
            if let Some(kind) = analysis.local(local.id) {
                locals.insert(local.id, map_kind(kind.clone()));
            }
        }
        let mut temp_roots = std::collections::HashMap::new();
        for temp in &func.temps {
            if let Some(root) = analysis.temp_root(temp.id) {
                temp_roots.insert(temp.id, root);
            }
        }
        Self { locals, temp_roots }
    }

    pub fn local(&self, local: LocalId) -> Option<&SpecialLocalKind> {
        self.locals.get(&local)
    }

    pub fn temp_root(&self, temp: TempId) -> Option<LocalId> {
        self.temp_roots.get(&temp).copied()
    }

    pub fn root_struct_local(&self, local: LocalId) -> Option<LocalId> {
        match self.locals.get(&local) {
            Some(SpecialLocalKind::ScalarStruct { .. }) => Some(local),
            Some(SpecialLocalKind::StructAlias { root }) => Some(*root),
            _ => None,
        }
    }
}

fn map_kind(kind: NativeLocalKind) -> SpecialLocalKind {
    match kind {
        NativeLocalKind::IntArray { size, init } => SpecialLocalKind::IntArray { size, init },
        NativeLocalKind::FloatArray { size, init } => SpecialLocalKind::FloatArray { size, init },
        NativeLocalKind::StringArray { size, items } => {
            SpecialLocalKind::StringArray { size, items }
        }
        NativeLocalKind::ScalarStruct { fields } => SpecialLocalKind::ScalarStruct { fields },
        NativeLocalKind::StructAlias { root } => SpecialLocalKind::StructAlias { root },
    }
}
