use crate::codegen::CodegenError;
use crate::codegen::llvm::LlvmEmitSection;
use crate::codegen::llvm::calls;
use crate::codegen::llvm::function;
use crate::codegen::llvm::instr_core;
use crate::codegen::llvm::instr_runtime;
use crate::codegen::llvm::instr_scalar;
use crate::codegen::llvm::module;
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::strings::{
    collect_string_literals, collect_string_literals_for_functions,
};
use crate::codegen::llvm::terminator;
use crate::codegen::llvm::value::{ValueNames, llvm_function_symbol, llvm_symbol};
use crate::ir::{FunctionId, GlobalId, Instr, IrFunction, IrProgram, LoweredIrFunction};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct OwnershipPlan {
    owned_functions: Option<HashSet<FunctionId>>,
    owned_globals: Option<HashSet<GlobalId>>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
}

impl OwnershipPlan {
    pub fn whole_program(program: &IrProgram) -> Self {
        Self {
            owned_functions: Some(program.functions.iter().map(|func| func.id).collect()),
            owned_globals: Some(program.globals.iter().map(|global| global.id).collect()),
            ctor_priority: 65_535,
            module_init_function: program.module_init.as_ref().map(|init| init.function),
        }
    }

    pub fn partitioned(
        owned_functions: HashSet<FunctionId>,
        owned_globals: HashSet<GlobalId>,
        ctor_priority: u32,
        module_init_function: Option<FunctionId>,
    ) -> Self {
        Self {
            owned_functions: Some(owned_functions),
            owned_globals: Some(owned_globals),
            ctor_priority,
            module_init_function,
        }
    }

    pub fn owns_function(&self, id: FunctionId) -> bool {
        self.owned_functions
            .as_ref()
            .is_none_or(|owned| owned.contains(&id))
    }

    pub fn owns_global(&self, id: GlobalId) -> bool {
        self.owned_globals
            .as_ref()
            .is_none_or(|owned| owned.contains(&id))
    }
}

pub struct LlvmEmitter<'a> {
    program: &'a IrProgram,
    string_literals: HashMap<String, String>,
    ownership: OwnershipPlan,
}

impl<'a> LlvmEmitter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        let ownership = OwnershipPlan::whole_program(program);
        let string_literals = collect_string_literals(program);
        Self {
            program,
            string_literals,
            ownership,
        }
    }

    pub fn new_with_ownership(program: &'a IrProgram, ownership: OwnershipPlan) -> Self {
        let string_literals = ownership
            .owned_functions
            .as_ref()
            .map(|owned| collect_string_literals_for_functions(program, owned))
            .unwrap_or_else(|| collect_string_literals(program));
        Self {
            program,
            string_literals,
            ownership,
        }
    }

    pub fn emit_program(&self) -> Result<String, CodegenError> {
        let sections = [
            self.emit_section_lines(LlvmEmitSection::Module)?,
            self.emit_section_lines(LlvmEmitSection::Runtime)?,
            self.emit_section_lines(LlvmEmitSection::Functions)?,
        ];
        let total_lines = sections.iter().map(Vec::len).sum();
        let mut out = Vec::with_capacity(total_lines);
        for mut section in sections {
            out.append(&mut section);
        }
        Ok(out.join("\n"))
    }

    pub fn emit_section(&self, section: LlvmEmitSection) -> Result<String, CodegenError> {
        Ok(self.emit_section_lines(section)?.join("\n"))
    }

    fn emit_section_lines(&self, section: LlvmEmitSection) -> Result<Vec<String>, CodegenError> {
        module::ensure_reserved_symbol_space(self.program)?;
        match section {
            LlvmEmitSection::Module => self.emit_module_section_lines(),
            LlvmEmitSection::Runtime => self.emit_runtime_section_lines(),
            LlvmEmitSection::Functions => self.emit_functions_section_lines(),
        }
    }

    fn emit_module_section_lines(&self) -> Result<Vec<String>, CodegenError> {
        let mut out = vec![
            "; ModuleID = 'skepa'".to_string(),
            "source_filename = \"skepa\"".to_string(),
            String::new(),
        ];
        module::emit_globals(self.program, &self.ownership, &mut out)?;
        module::emit_string_literal_storage(&self.string_literals, &mut out);
        if !self.string_literals.is_empty() || self.ownership.module_init_function.is_some() {
            if !self.string_literals.is_empty() {
                out.extend(module::emit_runtime_string_init(&self.string_literals)?);
                out.push(String::new());
            }
            let init_name = module::emit_module_initializer(
                self.program,
                &self.string_literals,
                self.ownership.module_init_function,
                &mut out,
            )?;
            out.push(format!(
                "@llvm.global_ctors = appending global [1 x {{ i32, ptr, ptr }}] [{{ i32, ptr, ptr }} {{ i32 {}, ptr {}, ptr null }}]",
                self.ownership.ctor_priority,
                llvm_symbol(&init_name),
            ));
            out.push(String::new());
        }
        Ok(out)
    }

    fn emit_runtime_section_lines(&self) -> Result<Vec<String>, CodegenError> {
        let mut out = Vec::new();
        runtime::emit_runtime_decls(self.program, &mut out)?;
        out.push(String::new());
        Ok(out)
    }

    fn emit_functions_section_lines(&self) -> Result<Vec<String>, CodegenError> {
        let mut out = Vec::with_capacity(self.program.functions.len() * 8);
        for func in &self.program.functions {
            if self.ownership.owns_function(func.id) {
                out.extend(self.emit_function(func)?);
            } else {
                out.push(function::emit_function_declaration(func)?);
            }
            out.push(String::new());
        }
        if let Some(void_main) = self.program.functions.iter().find(|func| {
            func.name == "main" && func.ret_ty.is_void() && self.ownership.owns_function(func.id)
        }) {
            out.push("define i32 @\"main\"() {".into());
            out.push("entry:".into());
            out.push(format!(
                "  call void {}()",
                llvm_function_symbol(&void_main.name, &void_main.ret_ty)
            ));
            out.push("  ret i32 0".into());
            out.push("}".into());
            out.push(String::new());
        }
        Ok(out)
    }

    fn emit_function(&self, func: &IrFunction) -> Result<Vec<String>, CodegenError> {
        function::validate_function_layout(func)?;
        let names = function::value_names(func);
        let lowered = LoweredIrFunction::analyze(func);
        let mut lines = function::emit_function_header(func)?;
        lines.reserve(function::estimated_function_line_capacity(func, &lowered));

        let mut counter = 0usize;
        for (idx, block) in func.blocks.iter().enumerate() {
            function::begin_block(func, block, idx, &lowered, &mut lines)?;
            for instr in &block.instrs {
                calls::ensure_supported(instr)?;
                runtime::ensure_supported(instr)?;
                self.emit_instr(func, &names, &lowered, instr, &mut lines, &mut counter)?;
            }
            terminator::emit_terminator(
                func,
                &names,
                &block.terminator,
                &mut lines,
                &mut counter,
                &self.string_literals,
            )?;
        }

        function::finish_function(&mut lines);
        Ok(lines)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_instr(
        &self,
        func: &IrFunction,
        names: &ValueNames,
        lowered: &LoweredIrFunction,
        instr: &Instr,
        lines: &mut Vec<String>,
        counter: &mut usize,
    ) -> Result<(), CodegenError> {
        if instr_scalar::emit_scalar_instr(
            self.program,
            func,
            names,
            instr,
            lines,
            counter,
            &self.string_literals,
        )? {
            return Ok(());
        }
        if instr_core::emit_core_instr(
            self.program,
            func,
            names,
            lowered,
            instr,
            lines,
            counter,
            &self.string_literals,
        )? {
            return Ok(());
        }
        if instr_runtime::emit_runtime_instr(
            self.program,
            func,
            names,
            lowered,
            instr,
            lines,
            counter,
            &self.string_literals,
        )? {
            return Ok(());
        }
        match instr {
            Instr::Const { .. }
            | Instr::Copy { .. }
            | Instr::Unary { .. }
            | Instr::Binary { .. }
            | Instr::Compare { .. }
            | Instr::LoadGlobal { .. }
            | Instr::StoreGlobal { .. }
            | Instr::LoadLocal { .. }
            | Instr::StoreLocal { .. }
            | Instr::CallDirect { .. }
            | Instr::CallBuiltin { .. }
            | Instr::MakeClosure { .. }
            | Instr::CallIndirect { .. }
            | Instr::MakeArray { .. }
            | Instr::MakeArrayRepeat { .. }
            | Instr::ArrayGet { .. }
            | Instr::ArraySet { .. }
            | Instr::VecNew { .. }
            | Instr::VecLen { .. }
            | Instr::VecPush { .. }
            | Instr::VecGet { .. }
            | Instr::VecSet { .. }
            | Instr::VecDelete { .. }
            | Instr::MakeStruct { .. }
            | Instr::StructGet { .. }
            | Instr::StructSet { .. } => {
                unreachable!("scalar/core/runtime instructions handled earlier")
            }
            Instr::Logic { .. } => Err(CodegenError::Unsupported(
                "Logic instructions should be lowered to control flow before LLVM emission",
            )),
        }
    }
}
