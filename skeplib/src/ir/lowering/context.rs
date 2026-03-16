use std::collections::HashMap;

use crate::diagnostic::DiagnosticBag;
use crate::ir::{BlockId, IrBuilder, IrType};

pub(super) struct IrLowerer {
    pub(super) builder: IrBuilder,
    pub(super) diags: DiagnosticBag,
    pub(super) functions: HashMap<String, (crate::ir::FunctionId, IrType)>,
    pub(super) globals: HashMap<String, (crate::ir::GlobalId, IrType)>,
    pub(super) structs: HashMap<String, (crate::ir::StructId, Vec<crate::ir::StructField>)>,
    pub(super) module_id: Option<String>,
    pub(super) direct_import_calls: HashMap<String, String>,
    pub(super) imported_global_names: HashMap<String, String>,
    pub(super) imported_struct_runtime: HashMap<String, String>,
    pub(super) namespace_call_targets: HashMap<String, String>,
    pub(super) project_mode: bool,
    pub(super) lifted_functions: Vec<crate::ir::IrFunction>,
    pub(super) fn_lit_counter: usize,
}

pub(super) struct FunctionLowering {
    pub(super) current_block: BlockId,
    pub(super) locals: HashMap<String, crate::ir::LocalId>,
    pub(super) scratch_counter: usize,
    pub(super) loops: Vec<LoopLowering>,
}

pub(super) struct LoopLowering {
    pub(super) continue_block: BlockId,
    pub(super) break_block: BlockId,
}

impl IrLowerer {
    pub(super) fn new() -> Self {
        Self {
            builder: IrBuilder::new(),
            diags: DiagnosticBag::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            module_id: None,
            direct_import_calls: HashMap::new(),
            imported_global_names: HashMap::new(),
            imported_struct_runtime: HashMap::new(),
            namespace_call_targets: HashMap::new(),
            project_mode: false,
            lifted_functions: Vec::new(),
            fn_lit_counter: 0,
        }
    }

    pub(super) fn new_project() -> Self {
        let mut this = Self::new();
        this.project_mode = true;
        this
    }
}
