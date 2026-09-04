mod common;

use skeplib::ir::{self, IrInterpreter, IrValue};

#[test]
fn licm_keeps_constants_in_their_loop_when_names_repeat() {
    let source = r#"
fn main() -> Int {
  let first = 0;
  let second = 0;
  while (first < 1) {
    let marker = 10;
    first = first + marker;
  }
  while (second < 1) {
    let marker = 20;
    second = second + marker;
  }
  return first + second;
}
"#;

    let program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let mut optimized = program.clone();
    ir::opt::optimize_program(&mut optimized);
    assert_eq!(
        IrInterpreter::new(&optimized).run_main(),
        Ok(IrValue::Int(30))
    );
    assert_eq!(common::native_run_exit_code_ok(source), 30);
}
