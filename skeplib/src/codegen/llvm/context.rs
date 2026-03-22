use crate::codegen::CodegenError;
use crate::codegen::llvm::calls;
use crate::codegen::llvm::function;
use crate::codegen::llvm::instr_core;
use crate::codegen::llvm::instr_runtime;
use crate::codegen::llvm::instr_scalar;
use crate::codegen::llvm::module;
use crate::codegen::llvm::runtime;
use crate::codegen::llvm::strings::collect_string_literals;
use crate::codegen::llvm::terminator;
use crate::codegen::llvm::value::{ValueNames, llvm_function_symbol, llvm_symbol};
use crate::ir::{
    Instr, IrFunction, IrProgram, NativeAggregatePlan, NativeCallPlan, NativeStringPlan,
};
use std::collections::HashMap;

pub struct LlvmEmitter<'a> {
    program: &'a IrProgram,
    string_literals: HashMap<String, String>,
}

impl<'a> LlvmEmitter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self {
            program,
            string_literals: collect_string_literals(program),
        }
    }

    pub fn emit_program(&self) -> Result<String, CodegenError> {
        module::ensure_reserved_symbol_space(self.program)?;
        let mut out = vec![
            "; ModuleID = 'skepa'".to_string(),
            "source_filename = \"skepa\"".to_string(),
            String::new(),
        ];

        module::emit_globals(self.program, &mut out)?;

        module::emit_string_literal_storage(&self.string_literals, &mut out);

        if !self.string_literals.is_empty() || self.program.module_init.is_some() {
            if !self.string_literals.is_empty() {
                out.extend(module::emit_runtime_string_init(&self.string_literals)?);
                out.push(String::new());
            }
            let init_name =
                module::emit_module_initializer(self.program, &self.string_literals, &mut out)?;
            out.push(format!(
                "@llvm.global_ctors = appending global [1 x {{ i32, ptr, ptr }}] [{{ i32, ptr, ptr }} {{ i32 65535, ptr {}, ptr null }}]",
                llvm_symbol(&init_name)
            ));
            out.push(String::new());
        }

        runtime::emit_runtime_decls(self.program, &mut out)?;
        out.push(String::new());

        for func in &self.program.functions {
            out.extend(self.emit_function(func)?);
            out.push(String::new());
        }

        if let Some(void_main) = self
            .program
            .functions
            .iter()
            .find(|func| func.name == "main" && func.ret_ty.is_void())
        {
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

        Ok(out.join("\n"))
    }

    fn emit_function(&self, func: &IrFunction) -> Result<Vec<String>, CodegenError> {
        function::validate_function_layout(func)?;
        let names = function::value_names(func);
        let native = NativeAggregatePlan::analyze(func);
        let calls = NativeCallPlan::analyze(func);
        let strings = NativeStringPlan::analyze(func);
        let mut lines = function::emit_function_header(func)?;

        let mut counter = 0usize;
        for (idx, block) in func.blocks.iter().enumerate() {
            function::begin_block(func, block, idx, &native, &mut lines)?;
            for instr in &block.instrs {
                calls::ensure_supported(instr)?;
                runtime::ensure_supported(instr)?;
                self.emit_instr(
                    func,
                    &names,
                    &native,
                    &calls,
                    &strings,
                    instr,
                    &mut lines,
                    &mut counter,
                )?;
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
        native: &NativeAggregatePlan,
        calls: &NativeCallPlan,
        strings: &NativeStringPlan,
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
            native,
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
            native,
            calls,
            strings,
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
