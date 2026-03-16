use crate::codegen::CodegenError;
use crate::ir::IrType;

pub fn llvm_ty(ty: &IrType) -> Result<&'static str, CodegenError> {
    match ty {
        IrType::Int => Ok("i64"),
        IrType::Bool => Ok("i1"),
        IrType::Void => Ok("void"),
        _ => Err(CodegenError::Unsupported(
            "only Int/Bool/Void lowering is implemented",
        )),
    }
}
