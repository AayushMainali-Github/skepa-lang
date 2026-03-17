use skeplib::ir::{self, IrInterpError, IrValue, PrettyIr};

#[path = "common.rs"]
mod common;

#[test]
fn const_fold_simplifies_constants_and_constant_branches() {
    let source = r#"
fn main() -> Int {
  let x = 1 + 2;
  if (true) {
    return x;
  }
  return 99;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(3));

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("Jump(BlockId("));
    assert!(!printed.contains("Branch(BranchTerminator"));
}

#[test]
fn const_fold_does_not_fold_division_by_zero() {
    let source = r#"
fn main() -> Int {
  return 8 / 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let err = ir::IrInterpreter::new(&program)
        .run_main()
        .expect_err("optimized IR should still trap at runtime");
    assert!(matches!(err, IrInterpError::DivisionByZero));

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("Binary"));
    assert!(!printed.contains("Const { dst"));
}

#[test]
fn const_fold_preserves_string_concat_semantics() {
    let source = r#"
fn main() -> String {
  return "alpha" + "-beta";
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::String("alpha-beta".into()));
    assert_eq!(
        String::from_utf8_lossy(&common::native_run_ok(source).stdout),
        ""
    );
}

#[test]
fn const_fold_preserves_native_semantics_for_int_programs() {
    let source = r#"
fn main() -> Int {
  let x = (3 + 4) * 2;
  if (2 < 3) {
    return x;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let ir_value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(ir_value, IrValue::Int(14));
    assert_eq!(common::native_run_exit_code_ok(source), 14);
}
