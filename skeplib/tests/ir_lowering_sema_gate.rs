mod common;

use std::fs;

use skeplib::ir::lowering;

#[test]
fn compile_source_rejects_sema_invalid_programs_before_ir_lowering() {
    let source = r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 1);
  return vec.get(xs, 0);
}
"#;

    let diags = lowering::compile_source_unoptimized(source)
        .expect_err("IR lowering entrypoint should reject sema-invalid source");
    common::assert_has_diag(&diags, "Return type mismatch");
}

#[test]
fn compile_project_rejects_sema_invalid_programs_before_ir_lowering() {
    let root = common::make_temp_dir("ir_project_sema_gate");
    fs::create_dir_all(root.join("ops")).expect("create ops folder");
    fs::write(
        root.join("ops").join("math.sk"),
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}
export { xoxo };
"#,
    )
    .expect("write operator module");
    fs::write(
        root.join("main.sk"),
        r#"
from ops.math import xoxo;
fn main() -> Int {
  return xoxo("bad", 2);
}
"#,
    )
    .expect("write main");

    let err = lowering::compile_project_entry_unoptimized(&root.join("main.sk"))
        .expect_err("project IR lowering entrypoint should reject sema-invalid project");
    assert!(
        err.iter().any(|e| e.message.contains("Project semantic analysis failed before IR lowering")
            && e.message.contains("Argument 1")),
        "expected sema-gate diagnostic, got: {err:#?}"
    );

    let _ = fs::remove_dir_all(root);
}
