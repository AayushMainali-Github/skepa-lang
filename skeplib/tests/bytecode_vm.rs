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
    let main = module.functions.get("main").expect("main chunk exists");
    let out = Vm::run_main(main).expect("vm run");
    assert_eq!(out, Value::Int(30));
}

#[test]
fn compile_reports_unsupported_constructs() {
    let src = r#"
fn main() -> Int {
  if (true) {
    return 1;
  }
  return 0;
}
"#;
    let err = compile_source(src).expect_err("compile should fail for if/while in this slice");
    assert!(err
        .as_slice()
        .iter()
        .any(|d| d.message.contains("not supported in bytecode v0 compiler slice")));
}
