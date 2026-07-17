use skeplib::ir::{self, PrettyIr};

#[test]
fn dce_keeps_unused_vec_delete() {
    let source = r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 1);
  vec.push(xs, 2);
  let _removed = vec.delete(xs, 0);
  return vec.len(xs);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(
        printed.contains("VecDelete"),
        "unused VecDelete must survive DCE, got:\n{printed}"
    );
}

#[test]
fn dce_keeps_unused_trapping_division() {
    let source = r#"
fn main() -> Int {
  let _dead = 1 / 0;
  return 1;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(
        printed.contains("Binary") && printed.contains("Div"),
        "unused trapping 1/0 must survive DCE, got:\n{printed}"
    );
}
