mod block;
mod calls;
mod compare;
mod context;
mod function;
mod instr_core;
mod instr_runtime;
mod instr_scalar;
mod module;
mod runtime;
mod runtime_boxing;
mod runtime_builtins;
mod runtime_containers;
mod runtime_decls;
mod runtime_indirect;
mod strings;
mod terminator;
mod types;
mod value;

use crate::codegen::CodegenError;
use crate::ir::IrProgram;

pub(crate) use context::OwnershipPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlvmEmitSection {
    Module,
    Runtime,
    Functions,
}

pub fn compile_program(program: &IrProgram) -> Result<String, CodegenError> {
    context::LlvmEmitter::new(program).emit_program()
}

pub fn compile_program_section(
    program: &IrProgram,
    section: LlvmEmitSection,
) -> Result<String, CodegenError> {
    context::LlvmEmitter::new(program).emit_section(section)
}

pub(crate) fn compile_program_with_ownership(
    program: &IrProgram,
    ownership: OwnershipPlan,
) -> Result<String, CodegenError> {
    context::LlvmEmitter::new_with_ownership(program, ownership).emit_program()
}
