use skeplib::ir::{self, IrValue, PrettyIr};

#[path = "common.rs"]
mod common;

#[test]
fn optimizer_pipeline_interacts_correctly_on_mixed_program() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

fn step(x: Int) -> Int {
  return x + 1;
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn main() -> Int {
  let p = Pair { a: 2, b: 3 };
  let x = 1 + 2;
  let y = x;
  let z = y;
  if (true) {
    return step(p.mix(z));
  }
  return 99;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(9));
    assert_eq!(common::native_run_exit_code_ok(source), 9);

    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("CallDirect"));
    assert!(!printed.contains("Copy {"));
    assert!(!printed.contains("Branch(BranchTerminator"));
}

#[test]
fn optimizer_fixed_point_is_stable_across_multiple_runs() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 7) {
    let keep = ((1 + 2) * 2) / 1;
    acc = acc + keep;
    i = i + 1;
  }
  return acc;
}
"#;

    let mut program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    ir::opt::optimize_program(&mut program);
    let once = PrettyIr::new(&program).to_string();
    for _ in 0..4 {
        ir::opt::optimize_program(&mut program);
    }
    let many = PrettyIr::new(&program).to_string();
    assert_eq!(once, many);
}
