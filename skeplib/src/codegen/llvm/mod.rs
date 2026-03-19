mod block;
mod calls;
mod compare;
mod context;
mod function;
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

pub fn compile_program(program: &IrProgram) -> Result<String, CodegenError> {
    context::LlvmEmitter::new(program).emit_program()
}
