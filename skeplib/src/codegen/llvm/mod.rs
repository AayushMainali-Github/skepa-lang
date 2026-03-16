mod block;
mod calls;
mod context;
mod runtime;
mod types;
mod value;

use crate::codegen::CodegenError;
use crate::ir::IrProgram;

pub fn compile_program(program: &IrProgram) -> Result<String, CodegenError> {
    context::LlvmEmitter::new(program).emit_program()
}
