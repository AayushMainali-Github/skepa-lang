mod common;

use skeplib::{codegen, ir};

#[test]
fn llvm_codegen_uses_imported_operator_symbol_for_project_infix_calls() {
    let project = common::TempProject::new("codegen_imported_operator_symbol");
    project.file(
        "ops/math.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}
export { xoxo };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from ops.math import xoxo;
fn main() -> Int {
  return 4 `xoxo` 2;
}
"#,
    );

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @\"ops.math::xoxo\""));
    assert!(llvm_ir.contains("call i64 @\"ops.math::xoxo\""));
    assert!(!llvm_ir.contains("@\"main::xoxo\""));
}
