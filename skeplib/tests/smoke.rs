use skepart::value::RtValue;
use skeplib::ast::Program;
use skeplib::codegen::CodegenError;
use skeplib::ir::IrProgram;
use skeplib::parser::Parser;
use skeplib::sema::SemaResult;
use skeplib::types::TypeInfo;

#[test]
fn can_build_empty_program_structs() {
    let _ = Program::default();
    let _ = Parser::default();
    let _ = TypeInfo::Unknown;
    let _ = SemaResult::default();
    let _ = IrProgram::default();
    let _ = RtValue::Unit;
    let _ = CodegenError::Unsupported("smoke");
}
