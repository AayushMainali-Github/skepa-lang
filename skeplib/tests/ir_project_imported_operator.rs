mod common;

use skepart::RtValue;
use skeplib::ir::{IrInterpreter, PrettyIr, lowering};

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

#[test]
fn project_imported_custom_infix_operator_calls_imported_function_id() {
    let project = common::TempProject::new("project_imported_custom_infix_operator_target");
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
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("ops::xoxo"));
    assert!(
        !printed.contains("main::xoxo"),
        "lowering should not invent a local operator target for the imported name"
    );
}
