mod common;

use skepart::RtValue;
use skeplib::ir::IrInterpreter;
use skeplib::ir::{ConstValue, Instr, lowering};

fn int_consts(program: &skeplib::ir::IrProgram) -> Vec<i64> {
    let mut out = Vec::new();
    for func in &program.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                if let Instr::Const {
                    value: ConstValue::Int(n),
                    ..
                } = instr
                {
                    out.push(*n);
                }
            }
        }
    }
    out
}

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

#[test]
fn lowering_does_not_emit_unreachable_statement_consts_after_terminators() {
    let source = r#"
fn after_return() -> Int {
  return 1;
  let dead = 101;
  return dead;
}

fn after_break() -> Int {
  while (true) {
    break;
    let dead = 202;
    return dead;
  }
  return 2;
}

fn after_continue() -> Int {
  let i = 0;
  while (i < 1) {
    i = i + 1;
    continue;
    let dead = 303;
    return dead;
  }
  return i;
}
"#;

    let program = lowering::compile_source(source).expect("IR lowering should succeed");
    let consts = int_consts(&program);
    assert!(
        !consts.contains(&101),
        "dead post-return const leaked into IR"
    );
    assert!(
        !consts.contains(&202),
        "dead post-break const leaked into IR"
    );
    assert!(
        !consts.contains(&303),
        "dead post-continue const leaked into IR"
    );
}
