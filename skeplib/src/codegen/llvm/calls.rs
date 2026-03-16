use crate::codegen::CodegenError;
use crate::ir::Instr;

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    match instr {
        Instr::CallDirect { .. } | Instr::CallIndirect { .. } | Instr::CallBuiltin { .. } => Err(
            CodegenError::Unsupported("calls are not lowered until the direct-call milestone"),
        ),
        _ => Ok(()),
    }
}
