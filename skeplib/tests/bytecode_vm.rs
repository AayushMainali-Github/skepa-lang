use skeplib::bytecode::{compile_source, Instr, Value};
use skeplib::vm::{TestHost, Vm, VmErrorKind};
use std::collections::VecDeque;

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
    assert!(main.code.iter().any(|i| matches!(i, Instr::Add)));
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
  user.name = "x";
  return 0;
}
"#;
    let err = compile_source(src).expect_err("compile should fail for unsupported path assignment");
    assert!(err
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Path assignment not supported")));
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

#[test]
fn runs_io_println_and_readline_through_builtin_registry() {
    let src = r#"
import io;

fn main() -> Int {
  let name = io.readLine();
  io.println("hi " + name);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let mut host = TestHost {
        output: String::new(),
        input: VecDeque::from([String::from("sam")]),
    };
    let out = Vm::run_module_main_with_host(&module, &mut host).expect("vm run");
    assert_eq!(out, Value::Int(0));
    assert_eq!(host.output, "hi sam\n");
}

#[test]
fn compile_rejects_non_direct_builtin_path_depth() {
    let src = r#"
fn main() -> Int {
  a.b.c();
  return 0;
}
"#;
    let err = compile_source(src).expect_err("should reject deep path call");
    assert!(err
        .as_slice()
        .iter()
        .any(|d| d.message.contains("package.function")));
}

#[test]
fn bytecode_module_roundtrip_bytes() {
    let src = r#"
fn main() -> Int {
  return 42;
}
"#;
    let module = compile_source(src).expect("compile");
    let bytes = module.to_bytes();
    let decoded = skeplib::bytecode::BytecodeModule::from_bytes(&bytes).expect("decode");
    let out = Vm::run_module_main(&decoded).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn bytecode_decode_rejects_bad_magic() {
    let bad = vec![0, 1, 2, 3, 1, 0, 0, 0];
    let err = skeplib::bytecode::BytecodeModule::from_bytes(&bad).expect_err("bad header");
    assert!(err.contains("magic"));
}

#[test]
fn bytecode_decode_rejects_unknown_version() {
    let mut bytes = b"SKBC".to_vec();
    bytes.extend_from_slice(&99u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes()); // zero functions
    let err = skeplib::bytecode::BytecodeModule::from_bytes(&bytes).expect_err("bad version");
    assert!(err.contains("Unsupported bytecode version"));
}

#[test]
fn vm_reports_stack_overflow_on_unbounded_recursion() {
    let src = r#"
fn f(x: Int) -> Int {
  return f(x + 1);
}

fn main() -> Int {
  return f(0);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("should overflow");
    assert_eq!(err.kind, VmErrorKind::StackOverflow);
}
