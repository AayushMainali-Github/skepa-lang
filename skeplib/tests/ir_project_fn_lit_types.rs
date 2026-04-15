mod common;

use skepart::RtValue;
use skeplib::ir::{IrInterpreter, lowering};

#[test]
fn project_lowers_function_literal_imported_struct_types() {
    let project = common::TempProject::new("project_fn_lit_imported_struct_types");
    project.file(
        "models.sk",
        r#"
struct User { id: Int }
export { User };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from models import User;
fn main() -> Int {
  let get_id: Fn(User) -> Int = fn(u: User) -> Int {
    return u.id;
  };
  let user = User { id: 42 };
  return get_id(user);
}
"#,
    );

    let program = lowering::compile_project_entry(&entry).expect("project lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(42));
}
