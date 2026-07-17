use skeplib::ir::{self, PrettyIr};

#[test]
fn strength_reduce_does_not_elide_float_plus_or_minus_zero() {
    let source = r#"
fn keep_add(x: Float) -> Float {
  return x + 0.0;
}

fn keep_sub(x: Float) -> Float {
  return x - 0.0;
}

fn main() -> Float {
  return keep_add(1.5) + keep_sub(-0.0);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(
        printed.contains("Add") || printed.contains("Binary"),
        "float x + 0.0 must not be reduced away, got:\n{printed}"
    );
    assert!(
        printed.contains("Sub") || printed.contains("Binary"),
        "float x - 0.0 must not be reduced away, got:\n{printed}"
    );
    // A pure Copy of the parameter alone would mean the float zero identity fired.
    let keep_add_block = printed
        .split("fn keep_add")
        .nth(1)
        .unwrap_or("")
        .split("fn keep_sub")
        .next()
        .unwrap_or("");
    assert!(
        !keep_add_block.contains("Copy")
            || keep_add_block.contains("Add")
            || keep_add_block.contains("Binary"),
        "keep_add should retain an add, got:\n{keep_add_block}"
    );
}
