use skeplib::ir::{self, PrettyIr};

#[test]
fn const_fold_wraps_max_plus_one_without_debug_panic() {
    let source = r#"
fn main() -> Int {
  return 9223372036854775807 + 1;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(
        printed.contains(&format!("Int({})", i64::MIN)),
        "expected wrapped MIN constant, got:\n{printed}"
    );
}

#[test]
fn const_fold_does_not_fold_min_div_neg_one() {
    let source = r#"
fn main() -> Int {
  return -9223372036854775808 / -1;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(
        printed.contains("Binary") && printed.contains("Div"),
        "MIN / -1 must stay unfolded, got:\n{printed}"
    );
}
