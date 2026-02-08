use skeplib::bytecode::{compile_source, Instr, Value};
use skeplib::vm::Vm;

#[test]
fn compiles_main_to_bytecode_with_locals_and_return() {
    let src = r#"
fn main() -> Int {
  let x = 2;
  let y = x + 3;
  return y;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let main = module.functions.get("main").expect("main chunk exists");
    assert!(main.locals_count >= 2);
    assert!(main.code.iter().any(|i| matches!(i, Instr::AddInt)));
    assert!(matches!(main.code.last(), Some(Instr::Return)));
}

#[test]
fn runs_compiled_main_and_returns_int() {
    let src = r#"
fn main() -> Int {
  let x = 10;
  x = x + 5;
  return x * 2;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(30));
}

#[test]
fn compile_reports_unsupported_constructs() {
    let src = r#"
fn main() -> Int {
  io.println("x");
  return 0;
}
"#;
    let err = compile_source(src).expect_err("compile should fail for unsupported path/string calls");
    assert!(err
        .as_slice()
        .iter()
        .any(|d| d.message.contains("not supported") || d.message.contains("Only direct function calls are supported")));
}

#[test]
fn runs_if_else_branching() {
    let src = r#"
fn main() -> Int {
  let x = 2;
  if (x > 1) {
    return 10;
  } else {
    return 20;
  }
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(10));
}

#[test]
fn runs_while_loop_with_assignment_updates() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 5) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(10));
}

#[test]
fn runs_bool_logic_and_not_for_conditions() {
    let src = r#"
fn main() -> Int {
  let t = true;
  if (!false && t) {
    return 1;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(1));
}

#[test]
fn runs_user_defined_function_calls_with_args() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn twice(x: Int) -> Int {
  return add(x, x);
}

fn main() -> Int {
  return twice(7);
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(14));
}
