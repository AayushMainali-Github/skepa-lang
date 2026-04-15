mod common;

use skepart::RtValue;
use skeplib::ir::{IrInterpreter, lowering};

#[test]
fn project_lowers_direct_imported_custom_infix_operator() {
    let project = common::TempProject::new("project_imported_custom_infix_operator");
    project.file(
        "ops.sk",
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
from ops import xoxo;
fn main() -> Int {
  return 4 `xoxo` 2;
}
"#,
    );

    let program = lowering::compile_project_entry(&entry).expect("project lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(42));
}
