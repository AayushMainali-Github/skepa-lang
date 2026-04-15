mod common;

use skepart::RtValue;
use skeplib::ir::{IrInterpreter, lowering};

#[test]
fn lowering_stops_after_return_in_statement_list() {
    let source = r#"
fn main() -> Int {
  return 1;
  return 2;
}
"#;

    let program = lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(1));
}

#[test]
fn lowering_stops_after_break_in_loop_body() {
    let source = r#"
fn main() -> Int {
  while (true) {
    break;
    return 9;
  }
  return 3;
}
"#;

    let program = lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(3));
}

#[test]
fn lowering_stops_after_continue_in_loop_body() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  while (i < 2) {
    i = i + 1;
    continue;
    return 9;
  }
  return i;
}
"#;

    let program = lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(2));
}
