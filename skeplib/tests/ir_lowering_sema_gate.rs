mod common;

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
