use skeplib::bytecode::{BytecodeModule, FunctionChunk, Instr, Value, compile_source};
use skeplib::vm::{BuiltinHost, BuiltinRegistry, TestHost, Vm, VmConfig, VmErrorKind};
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
    assert!(
        err.as_slice()
            .iter()
            .any(|d| d.message.contains("Path assignment not supported"))
    );
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
fn runs_while_with_break() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  while (true) {
    if (i == 4) {
      break;
    }
    i = i + 1;
  }
  return i;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(4));
}

#[test]
fn runs_while_with_continue() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 5) {
    i = i + 1;
    if (i == 3) {
      continue;
    }
    acc = acc + i;
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(12));
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
    assert!(
        err.as_slice()
            .iter()
            .any(|d| d.message.contains("package.function"))
    );
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

#[test]
fn vm_stack_overflow_respects_configured_limit() {
    let src = r#"
fn f(x: Int) -> Int {
  return f(x + 1);
}
fn main() -> Int { return f(0); }
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main_with_config(
        &module,
        VmConfig {
            max_call_depth: 8,
            trace: false,
        },
    )
    .expect_err("overflow");
    assert_eq!(err.kind, VmErrorKind::StackOverflow);
    assert!(err.message.contains("8"));
}

#[test]
fn vm_reports_division_by_zero_kind() {
    let src = r#"
fn main() -> Int {
  return 10 / 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("should fail");
    assert_eq!(err.kind, VmErrorKind::DivisionByZero);
}

#[test]
fn runs_int_modulo() {
    let src = r#"
fn main() -> Int {
  return 17 % 5;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(2));
}

#[test]
fn vm_reports_modulo_by_zero_kind() {
    let src = r#"
fn main() -> Int {
  return 10 % 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("should fail");
    assert_eq!(err.kind, VmErrorKind::DivisionByZero);
}

#[test]
fn vm_reports_unknown_builtin_kind() {
    let src = r#"
fn main() -> Int {
  return pkg.work(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("unknown builtin");
    assert_eq!(err.kind, VmErrorKind::UnknownBuiltin);
}

#[test]
fn vm_reports_function_arity_mismatch_kind() {
    let src = r#"
fn f(x: Int) -> Int {
  return x;
}

fn main() -> Int {
  return f();
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
}

#[test]
fn vm_supports_string_concat_and_equality() {
    let src = r#"
fn main() -> Int {
  if ("ab" + "cd" == "abcd") {
    return 1;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(1));
}

#[test]
fn bytecode_decode_rejects_truncated_payload() {
    let src = r#"
fn main() -> Int { return 1; }
"#;
    let module = compile_source(src).expect("compile");
    let mut bytes = module.to_bytes();
    bytes.truncate(bytes.len().saturating_sub(3));
    let err = BytecodeModule::from_bytes(&bytes).expect_err("truncate should fail");
    assert!(err.contains("Unexpected EOF"));
}

#[test]
fn vm_reports_stack_underflow_for_invalid_program() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![Instr::Pop, Instr::Return],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("stack underflow");
    assert_eq!(err.kind, VmErrorKind::StackUnderflow);
}

#[test]
fn vm_reports_type_mismatch_for_bad_jump_condition() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::JumpIfFalse(4),
                    Instr::LoadConst(Value::Int(1)),
                    Instr::Return,
                    Instr::LoadConst(Value::Int(0)),
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("main@"));
}

fn custom_math_inc(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, skeplib::vm::VmError> {
    if args.len() != 1 {
        return Err(skeplib::vm::VmError {
            kind: VmErrorKind::ArityMismatch,
            message: "math.inc expects 1 arg".to_string(),
        });
    }
    match args[0] {
        Value::Int(v) => Ok(Value::Int(v + 1)),
        _ => Err(skeplib::vm::VmError {
            kind: VmErrorKind::TypeMismatch,
            message: "math.inc expects Int".to_string(),
        }),
    }
}

#[test]
fn vm_runs_with_custom_builtin_registry_extension() {
    let src = r#"
fn main() -> Int {
  return math.inc(41);
}
"#;
    let module = compile_source(src).expect("compile");
    let mut reg = BuiltinRegistry::with_defaults();
    reg.register("math", "inc", custom_math_inc);
    let mut host = TestHost::default();
    let out = Vm::run_module_main_with_registry(&module, &mut host, &reg).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn disassemble_outputs_named_instructions() {
    let src = r#"
fn main() -> Int {
  let x = 1;
  return x + 2;
}
"#;
    let module = compile_source(src).expect("compile");
    let txt = module.disassemble();
    assert!(txt.contains("fn main"));
    assert!(txt.contains("LoadConst Int(1)"));
    assert!(txt.contains("Add"));
    assert!(txt.contains("Return"));
}

#[test]
fn runs_float_arithmetic() {
    let src = r#"
fn main() -> Float {
  let x = 8.0;
  x = x / 2.0;
  return x + 0.25;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Float(4.25));
}

#[test]
fn supports_float_comparison_in_conditionals() {
    let src = r#"
fn main() -> Int {
  if (2.5 > 2.0) {
    return 1;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(1));
}

#[test]
fn bytecode_roundtrip_preserves_float_constant() {
    let src = r#"
fn main() -> Float { return 3.5; }
"#;
    let module = compile_source(src).expect("compile");
    let bytes = module.to_bytes();
    let decoded = BytecodeModule::from_bytes(&bytes).expect("decode");
    let out = Vm::run_module_main(&decoded).expect("run");
    assert_eq!(out, Value::Float(3.5));
}
