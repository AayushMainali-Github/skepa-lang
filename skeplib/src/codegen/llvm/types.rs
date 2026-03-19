use crate::codegen::CodegenError;
use crate::ir::IrType;

pub fn llvm_ty(ty: &IrType) -> Result<&'static str, CodegenError> {
    match ty {
        IrType::Int => Ok("i64"),
        IrType::Float => Ok("double"),
        IrType::Bool => Ok("i1"),
        IrType::String => Ok("ptr"),
        IrType::Named(_) => Ok("ptr"),
        IrType::Array { .. } => Ok("ptr"),
        IrType::Vec { .. } => Ok("ptr"),
        IrType::Fn { .. } => Ok("i32"),
        IrType::Void => Ok("void"),
        _ => Err(CodegenError::Unsupported(
            "only Int/Float/Bool/String/Named/Array/Vec/Fn/Void lowering is implemented",
        )),
    }
}
