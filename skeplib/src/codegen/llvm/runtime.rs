use crate::codegen::CodegenError;
use crate::ir::Instr;

pub fn ensure_supported(instr: &Instr) -> Result<(), CodegenError> {
    match instr {
        Instr::MakeArray { .. }
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
        | Instr::StructSet { .. }
        | Instr::MakeClosure { .. } => Err(CodegenError::Unsupported(
            "runtime-backed values are not lowered until later LLVM milestones",
        )),
        _ => Ok(()),
    }
}
