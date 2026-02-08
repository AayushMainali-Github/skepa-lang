use skeplib::ast::Program;
use skeplib::bytecode::BytecodeModule;
use skeplib::parser::Parser;
use skeplib::sema::SemaResult;
use skeplib::types::TypeInfo;
use skeplib::vm::Vm;

#[test]
fn can_build_empty_program_structs() {
    let _ = Program;
    let _ = Parser;
    let _ = TypeInfo;
    let _ = SemaResult;
    let _ = BytecodeModule;
    let _ = Vm;
}
