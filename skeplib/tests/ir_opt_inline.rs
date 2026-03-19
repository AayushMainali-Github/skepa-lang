use skeplib::ir::{self, IrValue, PrettyIr};

#[path = "common.rs"]
mod common;

#[test]
fn inlining_removes_trivial_direct_calls_and_methods() {
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
    return self.a + x + self.b;
  }
}

fn main() -> Int {
  let p = Pair { a: 10, b: 5 };
  return step(p.mix(7));
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(23));

    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("CallDirect"));
}

#[test]
fn inlining_handles_direct_method_stress_without_changing_result() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn main() -> Int {
  let p = Pair { a: 2, b: 3 };
  return p.mix(1) + p.mix(2) + p.mix(3);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(21));
    assert_eq!(common::native_run_exit_code_ok(source), 21);
}

#[test]
fn inlining_preserves_function_value_signature_binding_without_name_lookup_tricks() {
    let source = r#"
fn apply_twice(f: Fn(Int) -> Int, x: Int) -> Int {
  return f(f(x));
}

fn inc(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  return apply_twice(inc, 5);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(7));
    assert_eq!(common::native_run_exit_code_ok(source), 7);
}
