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
fn codegen_rejects_break_outside_loop_with_consistent_message() {
    let src = r#"
fn main() -> Int {
  break;
  return 0;
}
"#;
    let err = compile_source(src).expect_err("compile should fail");
    assert!(
        err.as_slice()
            .iter()
            .any(|d| d.message.contains("`break` used outside a loop"))
    );
}

#[test]
fn codegen_rejects_continue_outside_loop_with_consistent_message() {
    let src = r#"
fn main() -> Int {
  continue;
  return 0;
}
"#;
    let err = compile_source(src).expect_err("compile should fail");
    assert!(
        err.as_slice()
            .iter()
            .any(|d| d.message.contains("`continue` used outside a loop"))
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
fn runs_for_loop_with_break_and_continue() {
    let src = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 8; i = i + 1) {
    if (i == 2) {
      continue;
    }
    if (i == 6) {
      break;
    }
    acc = acc + (i % 3);
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(4));
}

#[test]
fn runs_infinite_for_loop_with_break() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  for (;;) {
    if (i == 5) {
      break;
    }
    i = i + 1;
  }
  return i;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(5));
}

#[test]
fn runs_nested_for_loops_with_inner_continue() {
    let src = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 3; i = i + 1) {
    for (let j = 0; j < 4; j = j + 1) {
      if (j == 1) {
        continue;
      }
      acc = acc + 1;
    }
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(9));
}

#[test]
fn runs_for_continue_inside_nested_if_branch() {
    let src = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 6; i = i + 1) {
    if (i < 4) {
      if ((i % 2) == 0) {
        continue;
      }
    }
    acc = acc + i;
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(13));
}

#[test]
fn runs_static_array_literal_index_and_assignment() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 3] = [1, 2, 3];
  let x = a[1];
  a[2] = x + 4;
  return a[0] + a[1] + a[2];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(9));
}

#[test]
fn runs_static_array_repeat_literal() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 4] = [3; 4];
  return a[0] + a[1] + a[2] + a[3];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(12));
}

#[test]
fn runs_str_builtins() {
    let src = r#"
import str;
fn main() -> Int {
  let s = "  hello  ";
  let t = str.trim(s);
  if (str.contains(t, "ell") && str.startsWith(t, "he") && str.endsWith(t, "lo")) {
    return str.len(t);
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(5));
}

#[test]
fn runs_str_case_conversion_builtins() {
    let src = r#"
import str;
fn main() -> Int {
  let a = str.toLower("SkEpA");
  let b = str.toUpper("laNg");
  if (a == "skepa" && b == "LANG") {
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
fn runs_str_indexof_slice_and_isempty() {
    let src = r#"
import str;
fn main() -> Int {
  let s = "skepa";
  let idx = str.indexOf(s, "ep");
  let miss = str.indexOf(s, "zz");
  let cut = str.slice(s, 1, 4);
  if (idx == 2 && miss == -1 && cut == "kep" && !str.isEmpty(cut) && str.isEmpty("")) {
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
fn vm_reports_str_slice_out_of_bounds() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.slice("abc", 1, 9);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let err = Vm::run_module_main(&module).expect_err("slice bounds");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
    assert!(err.message.contains("str.slice bounds out of range"));
}

#[test]
fn runs_nested_static_array_3d_read_write() {
    let src = r#"
fn main() -> Int {
  let t: [[[Int; 2]; 2]; 2] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
  t[1][0][1] = 42;
  return t[1][0][1];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_nested_static_array_4d_read_write() {
    let src = r#"
fn main() -> Int {
  let q: [[[[Int; 2]; 1]; 1]; 1] = [[[[1, 2]]]];
  q[0][0][0][1] = 9;
  return q[0][0][0][1];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(9));
}

#[test]
fn vm_reports_array_index_out_of_bounds() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  return a[5];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let err = Vm::run_module_main(&module).expect_err("oob");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
}

#[test]
fn vm_reports_nested_array_index_out_of_bounds() {
    let src = r#"
fn main() -> Int {
  let t: [[[Int; 2]; 2]; 2] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
  return t[1][3][0];
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let err = Vm::run_module_main(&module).expect_err("oob");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
}

#[test]
fn vm_reports_type_mismatch_for_len_on_non_collection_value() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::ArrayLen,
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
}

#[test]
fn for_bytecode_has_expected_jump_shape() {
    let src = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 3; i = i + 1) {
    acc = acc + i;
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile");
    let main = module.functions.get("main").expect("main");
    let code = &main.code;

    let jf_idx = code
        .iter()
        .position(|i| matches!(i, Instr::JumpIfFalse(_)))
        .expect("for should emit JumpIfFalse");
    let body_jump_idx = jf_idx + 1;
    assert!(matches!(code[body_jump_idx], Instr::Jump(_)));

    let backward_jumps: Vec<_> = code
        .iter()
        .enumerate()
        .filter_map(|(idx, instr)| match instr {
            Instr::Jump(target) if *target < idx => Some((*target, idx)),
            _ => None,
        })
        .collect();
    assert!(
        backward_jumps.len() >= 2,
        "expected two backward jumps (to cond and step), got {backward_jumps:?}"
    );
}

#[test]
fn for_bytecode_patches_break_jumps() {
    let src = r#"
fn main() -> Int {
  for (;;) {
    break;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let main = module.functions.get("main").expect("main");
    assert!(!main.code.iter().any(|i| {
        matches!(i, Instr::Jump(t) if *t == usize::MAX)
            || matches!(i, Instr::JumpIfFalse(t) if *t == usize::MAX)
    }));
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
fn short_circuit_and_skips_rhs_evaluation() {
    let src = r#"
fn main() -> Int {
  if (false && ((1 / 0) == 0)) {
    return 1;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn short_circuit_or_skips_rhs_evaluation() {
    let src = r#"
fn main() -> Int {
  if (true || ((1 / 0) == 0)) {
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
fn vm_reports_division_by_zero_inside_for_loop() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  for (; i < 1; i = i + 1) {
    let x = 1 / 0;
    return x;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("division by zero");
    assert_eq!(err.kind, VmErrorKind::DivisionByZero);
}

#[test]
fn vm_reports_type_mismatch_for_loop_like_bad_condition() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::JumpIfFalse(4),
                    Instr::Jump(0),
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
fn runs_unary_plus_numeric_values() {
    let src = r#"
fn main() -> Int {
  let a = +5;
  let b: Float = +2.5;
  if (b == 2.5) {
    return a;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(5));
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
fn disassemble_includes_short_circuit_and_modulo_instruction_flow() {
    let src = r#"
fn main() -> Int {
  if (true || false) {
    return 8 % 3;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let txt = module.disassemble();
    assert!(txt.contains("JumpIfTrue"));
    assert!(txt.contains("ModInt"));
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
fn modulo_handles_negative_operands_with_rust_semantics() {
    let src = r#"
fn main() -> Int {
  let a = -7 % 3;
  let b = 7 % -3;
  if (a == -1 && b == 1) {
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
fn float_negative_zero_compares_equal_and_not_less_than_zero() {
    let src = r#"
fn main() -> Int {
  let z: Float = -0.0;
  if (z == 0.0 && !(z < 0.0) && !(z > 0.0)) {
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

#[test]
fn bytecode_roundtrip_preserves_array_constants_and_ops() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 3] = [2, 4, 6];
  return a[0] + a[2];
}
"#;
    let module = compile_source(src).expect("compile");
    let bytes = module.to_bytes();
    let decoded = BytecodeModule::from_bytes(&bytes).expect("decode");
    let out = Vm::run_module_main(&decoded).expect("run");
    assert_eq!(out, Value::Int(8));
}

#[test]
fn runs_io_format_and_printf_with_escapes_and_percent() {
    let src = r#"
import io;
fn main() -> Int {
  let msg = io.format("v=%d f=%f ok=%b s=%s %%\n", 5, 1.5, true, "yo");
  io.printf("%s\t%s\\", msg, "done");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let mut host = TestHost::default();
    let out = Vm::run_module_main_with_host(&module, &mut host).expect("run");
    assert_eq!(out, Value::Int(0));
    assert_eq!(host.output, "v=5 f=1.5 ok=true s=yo %\n\tdone\\");
}

#[test]
fn vm_reports_io_format_runtime_type_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  let _x = io.format("n=%d", "bad");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("must be Int for `%d`"));
}

#[test]
fn vm_reports_io_printf_runtime_arity_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  io.printf("%d %d", 1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("expects 2 value argument(s), got 1"));
}

#[test]
fn runs_typed_io_print_builtins_with_newlines() {
    let src = r#"
import io;
fn main() -> Int {
  io.printInt(7);
  io.printFloat(2.5);
  io.printBool(false);
  io.printString("ok");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let mut host = TestHost::default();
    let out = Vm::run_module_main_with_host(&module, &mut host).expect("run");
    assert_eq!(out, Value::Int(0));
    assert_eq!(host.output, "7\n2.5\nfalse\nok\n");
}

#[test]
fn vm_reports_typed_io_print_runtime_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  let fmt = "x";
  io.printInt(fmt);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("io.printInt expects Int argument"));
}

#[test]
fn vm_io_print_and_println_have_expected_newline_behavior() {
    let src = r#"
import io;
fn main() -> Int {
  io.print("a");
  io.print("b");
  io.println("c");
  io.print("d");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let mut host = TestHost::default();
    let out = Vm::run_module_main_with_host(&module, &mut host).expect("run");
    assert_eq!(out, Value::Int(0));
    assert_eq!(host.output, "abc\nd");
}

#[test]
fn vm_reports_io_print_runtime_arity_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  io.print();
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("io.print expects 1 argument"));
}

#[test]
fn vm_reports_io_printf_runtime_invalid_specifier_for_dynamic_format() {
    let src = r#"
import io;
fn main() -> Int {
  let fmt = "bad=%q";
  io.printf(fmt, 1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("invalid specifier");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("unsupported format specifier `%q`"));
}

#[test]
fn vm_reports_io_format_runtime_trailing_percent_for_dynamic_format() {
    let src = r#"
import io;
fn main() -> Int {
  let fmt = "oops %";
  let _s = io.format(fmt, 1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("trailing percent");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("format string ends with `%`"));
}

#[test]
fn runs_arr_package_generic_ops_and_array_add() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 4] = [1, 2, 3, 2];
  let b: [Int; 2] = [9, 8];
  let c = a + b;
  if (arr.len(c) == 6 && !arr.isEmpty(c) && arr.contains(c, 8) && arr.indexOf(c, 2) == 1 && arr.sum(c) == 25) {
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
fn runs_arr_sum_on_nested_arrays_to_concatenate_rows() {
    let src = r#"
import arr;
fn main() -> Int {
  let rows: [[Int; 2]; 3] = [[1, 2], [3, 4], [5, 6]];
  let flat = arr.sum(rows);
  if (arr.len(flat) == 6 && arr.indexOf(flat, 4) == 3 && arr.contains(flat, 6)) {
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
fn vm_reports_arr_sum_empty_array_runtime_error() {
    let src = r#"
import arr;
fn main() -> Int {
  let xs: [Int; 0] = [];
  let _v = arr.sum(xs);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("empty sum");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("arr.sum expects non-empty array"));
}
