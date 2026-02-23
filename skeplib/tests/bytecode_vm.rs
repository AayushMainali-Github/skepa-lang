mod common;

use common::{assert_has_diag, compile_err, compile_ok, vm_run_ok};
use skeplib::bytecode::{BytecodeModule, FunctionChunk, Instr, Value, compile_source};
use skeplib::vm::{BuiltinHost, BuiltinRegistry, TestHost, Vm, VmConfig, VmErrorKind};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skepa_vm_{label}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sk_string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}

#[test]
fn compiles_main_to_bytecode_with_locals_and_return() {
    let src = r#"
fn main() -> Int {
  let x = 2;
  let y = x + 3;
  return y;
}
"#;
    let module = compile_ok(src);
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
    let out = vm_run_ok(src);
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
    let err = compile_err(src);
    assert_has_diag(&err, "Path assignment not supported");
}

#[test]
fn codegen_rejects_break_outside_loop_with_consistent_message() {
    let src = r#"
fn main() -> Int {
  break;
  return 0;
}
"#;
    let err = compile_err(src);
    assert_has_diag(&err, "`break` used outside a loop");
}

#[test]
fn codegen_rejects_continue_outside_loop_with_consistent_message() {
    let src = r#"
fn main() -> Int {
  continue;
  return 0;
}
"#;
    let err = compile_err(src);
    assert_has_diag(&err, "`continue` used outside a loop");
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
    let out = vm_run_ok(src);
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
    let out = vm_run_ok(src);
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
fn runs_match_statement_with_int_and_wildcard_dispatch() {
    let src = r#"
fn main() -> Int {
  let x = 2;
  match (x) {
    0 => { return 10; }
    2 => { return 20; }
    _ => { return 30; }
  }
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(20));
}

#[test]
fn runs_match_statement_with_string_or_pattern() {
    let src = r#"
fn main() -> Int {
  let s = "Y";
  match (s) {
    "y" | "Y" => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(1));
}

#[test]
fn match_target_expression_is_evaluated_once() {
    let src = r#"
let n: Int = 0;

fn next() -> Int {
  n = n + 1;
  return n;
}

fn main() -> Int {
  match (next()) {
    1 => { return n; }
    _ => { return 99; }
  }
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(1));
}

#[test]
fn runs_match_statement_with_float_literal_patterns() {
    let src = r#"
fn main() -> Int {
  let x: Float = 2.5;
  match (x) {
    1.0 => { return 10; }
    2.5 | 3.5 => { return 20; }
    _ => { return 30; }
  }
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(20));
}

#[test]
fn runs_nested_match_stress_case() {
    let src = r#"
fn bucket(n: Int) -> Int {
  match (n % 3) {
    0 => {
      match (n % 2) {
        0 => { return 10; }
        _ => { return 11; }
      }
    }
    1 => {
      match (n) {
        1 | 4 | 7 => { return 20; }
        _ => { return 21; }
      }
    }
    _ => {
      match (n > 5) {
        true => { return 30; }
        _ => { return 31; }
      }
    }
  }
}

fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 9) {
    acc = acc + bucket(i);
    i = i + 1;
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(183));
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
fn vm_reports_struct_get_type_mismatch_on_non_struct() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::StructGet("id".to_string()),
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
fn vm_reports_unknown_method_on_struct_receiver() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let u = User { id: 1 };
  return u.nope(2);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("unknown method");
    assert_eq!(err.kind, VmErrorKind::UnknownFunction);
    assert!(
        err.message
            .contains("Unknown method `nope` on struct `User`")
    );
}

#[test]
fn vm_reports_unknown_struct_field_with_clear_message() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Struct {
                        name: "User".to_string(),
                        fields: vec![("id".to_string(), Value::Int(1))],
                    }),
                    Instr::StructGet("name".to_string()),
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("unknown field");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(
        err.message
            .contains("Unknown struct field `name` on `User`")
    );
}

#[test]
fn vm_reports_struct_set_path_with_non_struct_intermediate() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Struct {
                        name: "User".to_string(),
                        fields: vec![("id".to_string(), Value::Int(1))],
                    }),
                    Instr::LoadConst(Value::Int(42)),
                    Instr::StructSetPath(vec!["id".to_string(), "x".to_string()]),
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("invalid nested set path");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("StructSetPath failed"));
}

#[test]
fn vm_reports_struct_set_path_with_empty_path() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Struct {
                        name: "User".to_string(),
                        fields: vec![("id".to_string(), Value::Int(1))],
                    }),
                    Instr::LoadConst(Value::Int(42)),
                    Instr::StructSetPath(vec![]),
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("empty set path");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("requires non-empty field path"));
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
fn runs_struct_literal_field_access_assignment_and_method_call() {
    let src = r#"
struct User { id: Int, name: String }
impl User {
  fn bump(self, delta: Int) -> Int {
    return self.id + delta;
  }
}
fn main() -> Int {
  let u = User { id: 7, name: "sam" };
  let before = u.id;
  u.id = before + 5;
  return u.bump(3);
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(15));
}

#[test]
fn runs_nested_struct_field_assignment() {
    let src = r#"
struct Profile { age: Int }
struct User { profile: Profile, name: String }

fn main() -> Int {
  let u = User { profile: Profile { age: 20 }, name: "a" };
  u.profile.age = 42;
  return u.profile.age;
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_method_call_on_call_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn makeUser(x: Int) -> User {
  return User { id: x };
}
fn main() -> Int {
  return makeUser(9).bump(4);
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(13));
}

#[test]
fn runs_method_call_on_index_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn main() -> Int {
  let users: [User; 2] = [User { id: 2 }, User { id: 5 }];
  return users[1].bump(7);
}
"#;
    let module = compile_source(src).expect("compile should succeed");
    let out = Vm::run_module_main(&module).expect("vm run");
    assert_eq!(out, Value::Int(12));
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
        rng_state: 0,
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
fn runs_function_value_call_from_local_binding() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = add;
  return f(20, 22);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_function_value_call_through_parameter() {
    let src = r#"
fn apply(f: Fn(Int, Int) -> Int, x: Int, y: Int) -> Int {
  return f(x, y);
}

fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  return apply(add, 3, 7);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(10));
}

#[test]
fn vm_reports_callvalue_type_mismatch_for_non_function_callee() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(7)),
                    Instr::CallValue { argc: 0 },
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
fn vm_reports_type_mismatch_for_function_value_equality() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Function("f".to_string())),
                    Instr::LoadConst(Value::Function("f".to_string())),
                    Instr::Eq,
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
fn vm_reports_type_mismatch_for_function_value_inequality() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Function("f".to_string())),
                    Instr::LoadConst(Value::Function("g".to_string())),
                    Instr::Neq,
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
fn runs_function_value_call_via_grouped_callee_expr() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  return (add)(8, 9);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(17));
}

#[test]
fn runs_function_value_call_via_array_index_callee_expr() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
fn mul(a: Int, b: Int) -> Int { return a * b; }

fn main() -> Int {
  let ops: [Fn(Int, Int) -> Int; 2] = [add, mul];
  return ops[1](6, 7);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_non_capturing_function_literal() {
    let src = r#"
fn main() -> Int {
  let f: Fn(Int) -> Int = fn(x: Int) -> Int {
    return x + 2;
  };
  return f(40);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn compile_rejects_capturing_function_literal() {
    let src = r#"
fn main() -> Int {
  let y = 5;
  let f: Fn(Int) -> Int = fn(x: Int) -> Int {
    return x + y;
  };
  return f(1);
}
"#;
    let err = compile_err(src);
    assert_has_diag(&err, "Unknown local `y`");
}

#[test]
fn runs_immediate_function_literal_call() {
    let src = r#"
fn main() -> Int {
  return (fn(x: Int) -> Int { return x + 1; })(41);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_function_literal_passed_as_argument() {
    let src = r#"
fn apply(f: Fn(Int) -> Int, x: Int) -> Int {
  return f(x);
}

fn main() -> Int {
  return apply(fn(x: Int) -> Int { return x + 2; }, 40);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_function_returning_function_literal_and_chained_call() {
    let src = r#"
fn makeInc() -> Fn(Int) -> Int {
  return fn(x: Int) -> Int { return x + 1; };
}

fn main() -> Int {
  return makeInc()(41);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_function_type_in_struct_field_and_call_via_grouping() {
    let src = r#"
struct Op {
  apply: Fn(Int, Int) -> Int
}

fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let op: Op = Op { apply: add };
  return (op.apply)(20, 22);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(42));
}

#[test]
fn runs_array_of_functions_returned_from_function() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
fn mul(a: Int, b: Int) -> Int { return a * b; }

fn makeOps() -> [Fn(Int, Int) -> Int; 2] {
  return [add, mul];
}

fn main() -> Int {
  let ops = makeOps();
  return ops[0](2, 3) + ops[1](2, 3);
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(11));
}

#[test]
fn vm_reports_unknown_method_for_method_style_call_on_function_field() {
    let src = r#"
struct Op {
  apply: Fn(Int, Int) -> Int
}

fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let op: Op = Op { apply: add };
  return op.apply(1, 2);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("unknown method");
    assert_eq!(err.kind, VmErrorKind::UnknownFunction);
    assert!(
        err.message
            .contains("Unknown method `apply` on struct `Op`")
    );
}

#[test]
fn bytecode_roundtrip_preserves_function_value_and_callvalue_instr() {
    let module = BytecodeModule {
        functions: vec![
            (
                "inc".to_string(),
                FunctionChunk {
                    name: "inc".to_string(),
                    code: vec![
                        Instr::LoadLocal(0),
                        Instr::LoadConst(Value::Int(1)),
                        Instr::Add,
                        Instr::Return,
                    ],
                    locals_count: 1,
                    param_count: 1,
                },
            ),
            (
                "main".to_string(),
                FunctionChunk {
                    name: "main".to_string(),
                    code: vec![
                        Instr::LoadConst(Value::Function("inc".to_string())),
                        Instr::LoadConst(Value::Int(41)),
                        Instr::CallValue { argc: 1 },
                        Instr::Return,
                    ],
                    locals_count: 0,
                    param_count: 0,
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    let bytes = module.to_bytes();
    let decoded = BytecodeModule::from_bytes(&bytes).expect("decode");
    let out = Vm::run_module_main(&decoded).expect("run");
    assert_eq!(out, Value::Int(42));
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
fn vm_runs_os_cwd_builtin() {
    let src = r#"
import os;
fn main() -> String {
  return os.cwd();
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    match out {
        Value::String(s) => assert!(!s.is_empty()),
        other => panic!("expected String from os.cwd, got {:?}", other),
    }
}

#[test]
fn vm_runs_os_platform_builtin() {
    let src = r#"
import os;
fn main() -> String {
  return os.platform();
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    match out {
        Value::String(s) => assert!(matches!(s.as_str(), "windows" | "linux" | "macos")),
        other => panic!("expected String from os.platform, got {:?}", other),
    }
}

#[test]
fn vm_os_cwd_rejects_wrong_arity() {
    let src = r#"
import os;
fn main() -> String {
  return os.cwd(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("os.cwd expects 0 arguments"));
}

#[test]
fn vm_os_platform_rejects_wrong_arity() {
    let src = r#"
import os;
fn main() -> String {
  return os.platform(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("os.platform expects 0 arguments"));
}

#[test]
fn vm_runs_os_sleep_builtin() {
    let src = r#"
import os;
fn main() -> Int {
  os.sleep(1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn vm_os_sleep_rejects_negative_ms() {
    let src = r#"
import os;
fn main() -> Int {
  os.sleep(-1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected os.sleep error");
    assert_eq!(err.kind, VmErrorKind::HostError);
    assert!(
        err.message
            .contains("os.sleep expects non-negative milliseconds")
    );
}

#[test]
fn vm_os_sleep_rejects_wrong_arity() {
    let src = r#"
import os;
fn main() -> Int {
  os.sleep();
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("os.sleep expects 1 argument"));
}

#[test]
fn vm_os_sleep_rejects_wrong_type() {
    let src = r#"
import os;
fn main() -> Int {
  os.sleep(true);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("os.sleep expects Int argument"));
}

#[test]
fn vm_runs_os_exec_shell_and_returns_exit_code() {
    let cmd = if cfg!(target_os = "windows") {
        "exit /b 0"
    } else {
        "exit 0"
    };
    let src = format!(
        r#"
import os;
fn main() -> Int {{
  return os.execShell("{cmd}");
}}
"#
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn vm_os_exec_shell_returns_non_zero_exit_code() {
    let cmd = if cfg!(target_os = "windows") {
        "exit /b 7"
    } else {
        "exit 7"
    };
    let src = format!(
        r#"
import os;
fn main() -> Int {{
  return os.execShell("{cmd}");
}}
"#
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(7));
}

#[test]
fn vm_runs_os_exec_shell_out_and_captures_stdout() {
    let cmd = if cfg!(target_os = "windows") {
        "echo hello"
    } else {
        "printf hello"
    };
    let src = format!(
        r#"
import os;
fn main() -> String {{
  return os.execShellOut("{cmd}");
}}
"#
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    match out {
        Value::String(s) => assert!(s.contains("hello")),
        other => panic!("expected String from os.execShellOut, got {:?}", other),
    }
}

#[test]
fn vm_os_exec_shell_out_rejects_wrong_arity() {
    let src = r#"
import os;
fn main() -> String {
  return os.execShellOut();
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("os.execShellOut expects 1 argument"));
}

#[test]
fn vm_os_exec_shell_out_rejects_wrong_type() {
    let src = r#"
import os;
fn main() -> String {
  return os.execShellOut(false);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(
        err.message
            .contains("os.execShellOut expects String argument")
    );
}

#[test]
fn vm_os_exec_shell_rejects_wrong_type() {
    let src = r#"
import os;
fn main() -> Int {
  return os.execShell(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("os.execShell expects String argument"));
}

#[test]
fn vm_runs_fs_exists_for_missing_path() {
    let src = r#"
import fs;
fn main() -> Bool {
  return fs.exists("nope");
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Bool(false));
}

#[test]
fn vm_runs_fs_join_builtin() {
    let src = r#"
import fs;
import str;
fn main() -> Int {
  let p = fs.join("alpha", "beta");
  if (str.contains(p, "alpha") && str.contains(p, "beta")) {
    return 0;
  }
  return 1;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn vm_fs_exists_rejects_wrong_type() {
    let src = r#"
import fs;
fn main() -> Bool {
  return fs.exists(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("fs.exists expects String argument"));
}

#[test]
fn vm_fs_join_rejects_wrong_arity() {
    let src = r#"
import fs;
fn main() -> String {
  return fs.join("a");
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("fs.join expects 2 arguments"));
}

#[test]
fn vm_runs_fs_mkdir_all_and_remove_dir_all() {
    let root = make_temp_dir("fs_mkdir_remove_dir");
    let nested = root.join("a").join("b").join("c");
    let nested_s = sk_string_escape(&nested.display().to_string());
    let root_s = sk_string_escape(&root.display().to_string());
    let src = format!(
        r#"
import fs;
fn main() -> Int {{
  fs.mkdirAll("{0}");
  if (!fs.exists("{0}")) {{
    return 1;
  }}
  fs.removeDirAll("{1}");
  if (fs.exists("{1}")) {{
    return 2;
  }}
  return 0;
}}
"#,
        nested_s, root_s
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn vm_runs_fs_remove_file() {
    let root = make_temp_dir("fs_remove_file");
    let file = root.join("x.txt");
    let file_s = sk_string_escape(&file.display().to_string());
    fs::write(&file, "x").expect("seed file");
    let src = format!(
        r#"
import fs;
fn main() -> Int {{
  fs.removeFile("{0}");
  if (fs.exists("{0}")) {{
    return 1;
  }}
  return 0;
}}
"#,
        file_s
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_remove_file_missing_path_errors() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.removeFile("definitely_missing_file_123456.tmp");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected removeFile error");
    assert_eq!(err.kind, VmErrorKind::HostError);
    assert!(err.message.contains("fs.removeFile failed"));
}

#[test]
fn vm_fs_remove_dir_all_missing_path_errors() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.removeDirAll("definitely_missing_dir_123456");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected removeDirAll error");
    assert_eq!(err.kind, VmErrorKind::HostError);
    assert!(err.message.contains("fs.removeDirAll failed"));
}

#[test]
fn vm_fs_mkdir_all_rejects_wrong_type() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.mkdirAll(1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("fs.mkdirAll expects String argument"));
}

#[test]
fn vm_fs_remove_dir_all_rejects_wrong_arity() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.removeDirAll();
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("fs.removeDirAll expects 1 argument"));
}

#[test]
fn vm_runs_fs_write_and_read_text() {
    let root = make_temp_dir("fs_write_read");
    let file = root.join("x.txt");
    let file_s = sk_string_escape(&file.display().to_string());
    let src = format!(
        r#"
import fs;
fn main() -> String {{
  fs.writeText("{0}", "hello");
  return fs.readText("{0}");
}}
"#,
        file_s
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::String("hello".to_string()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_write_text_overwrites_existing_file() {
    let root = make_temp_dir("fs_write_overwrite");
    let file = root.join("x.txt");
    let file_s = sk_string_escape(&file.display().to_string());
    fs::write(&file, "old").expect("seed file");
    let src = format!(
        r#"
import fs;
fn main() -> String {{
  fs.writeText("{0}", "new");
  return fs.readText("{0}");
}}
"#,
        file_s
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::String("new".to_string()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_append_text_appends_and_can_create_file() {
    let root = make_temp_dir("fs_append");
    let file = root.join("x.txt");
    let file_s = sk_string_escape(&file.display().to_string());
    let src = format!(
        r#"
import fs;
fn main() -> String {{
  fs.appendText("{0}", "a");
  fs.appendText("{0}", "b");
  return fs.readText("{0}");
}}
"#,
        file_s
    );
    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::String("ab".to_string()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_read_text_missing_file_errors() {
    let root = make_temp_dir("fs_read_missing");
    let file = root.join("missing.txt");
    let file_s = sk_string_escape(&file.display().to_string());
    let src = format!(
        r#"
import fs;
fn main() -> String {{
  return fs.readText("{0}");
}}
"#,
        file_s
    );
    let module = compile_source(&src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected read error");
    assert_eq!(err.kind, VmErrorKind::HostError);
    assert!(err.message.contains("fs.readText failed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_write_and_append_error_when_parent_missing() {
    let root = make_temp_dir("fs_parent_missing");
    let missing_parent = root.join("missing_dir");
    let write_path = missing_parent.join("a.txt");
    let write_path_s = sk_string_escape(&write_path.display().to_string());
    let write_src = format!(
        r#"
import fs;
fn main() -> Int {{
  fs.writeText("{0}", "x");
  return 0;
}}
"#,
        write_path_s
    );
    let write_mod = compile_source(&write_src).expect("compile");
    let write_err = Vm::run_module_main(&write_mod).expect_err("expected write error");
    assert_eq!(write_err.kind, VmErrorKind::HostError);
    assert!(write_err.message.contains("fs.writeText failed"));

    let append_src = format!(
        r#"
import fs;
fn main() -> Int {{
  fs.appendText("{0}", "x");
  return 0;
}}
"#,
        write_path_s
    );
    let append_mod = compile_source(&append_src).expect("compile");
    let append_err = Vm::run_module_main(&append_mod).expect_err("expected append error");
    assert_eq!(append_err.kind, VmErrorKind::HostError);
    assert!(append_err.message.contains("fs.appendText failed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn vm_fs_read_text_rejects_wrong_type() {
    let src = r#"
import fs;
fn main() -> String {
  return fs.readText(1);
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected type error");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("fs.readText expects String argument"));
}

#[test]
fn vm_fs_write_text_rejects_wrong_arity() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.writeText("a");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("expected arity error");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("fs.writeText expects 2 arguments"));
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
fn bytecode_roundtrip_preserves_struct_values_and_method_dispatch() {
    let src = r#"
struct User { id: Int, name: String }
impl User {
  fn bump(self, d: Int) -> Int {
    return self.id + d;
  }
}
fn main() -> Int {
  let u = User { id: 10, name: "sam" };
  return u.bump(5);
}
"#;
    let module = compile_source(src).expect("compile");
    let bytes = module.to_bytes();
    let decoded = BytecodeModule::from_bytes(&bytes).expect("decode");
    let out = Vm::run_module_main(&decoded).expect("run");
    assert_eq!(out, Value::Int(15));
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
  if (arr.len(c) == 6 && !arr.isEmpty(c) && arr.contains(c, 8) && arr.indexOf(c, 2) == 1) {
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
fn runs_arr_contains_and_indexof_for_nested_arrays() {
    let src = r#"
import arr;
fn main() -> Int {
  let rows: [[Int; 2]; 3] = [[1, 2], [3, 4], [5, 6]];
  if (arr.contains(rows, [3, 4]) && arr.indexOf(rows, [5, 6]) == 2 && arr.indexOf(rows, [9, 9]) == -1) {
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
fn arr_is_empty_handles_zero_sized_arrays() {
    let src = r#"
import arr;
fn main() -> Int {
  let z: [Int; 0] = [1; 0];
  if (arr.isEmpty(z) && arr.len(z) == 0) {
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
fn vm_reports_arr_builtin_runtime_arity_mismatch_from_manual_bytecode() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Array(vec![Value::Int(1)])),
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "len".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("arr.len expects 1 argument"));
}

#[test]
fn vm_reports_arr_count_runtime_errors_from_manual_bytecode() {
    let arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Array(vec![Value::Int(1)])),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "count".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("arr.count expects 2 arguments"));

    let type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "count".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(
        err.message
            .contains("arr.count expects Array as first argument")
    );
}

#[test]
fn vm_reports_arr_first_last_runtime_errors_from_manual_bytecode() {
    let first_arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Array(vec![Value::Int(1)])),
                    Instr::LoadConst(Value::Int(2)),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "first".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&first_arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("arr.first expects 1 argument"));

    let first_type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("x".to_string())),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "first".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&first_type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("arr.first expects Array argument"));

    let last_arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Array(vec![Value::Int(1)])),
                    Instr::LoadConst(Value::Int(2)),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "last".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&last_arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("arr.last expects 1 argument"));

    let last_type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("x".to_string())),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "last".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&last_type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("arr.last expects Array argument"));
}

#[test]
fn regression_arr_concat_large_arrays() {
    let n = 1500usize;
    let mut src = String::from("import arr;\nfn main() -> Int {\n  let a: [Int; ");
    src.push_str(&n.to_string());
    src.push_str("] = [1; ");
    src.push_str(&n.to_string());
    src.push_str("];\n  let b: [Int; ");
    src.push_str(&n.to_string());
    src.push_str("] = [2; ");
    src.push_str(&n.to_string());
    src.push_str("];\n  let c = a + b;\n");
    src.push_str("  if (arr.len(c) != ");
    src.push_str(&(2 * n).to_string());
    src.push_str(") { return 1; }\n  return arr.first(c) + arr.last(c);\n}\n");

    let module = compile_source(&src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(3));
}

#[test]
fn runs_arr_count_first_last() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 5] = [2, 9, 2, 3, 2];
  if (arr.count(a, 2) == 3 && arr.first(a) == 2 && arr.last(a) == 2) {
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
fn vm_reports_arr_first_last_on_empty_array() {
    let src = r#"
import arr;
fn main() -> Int {
  let z: [Int; 0] = [1; 0];
  let _a = arr.first(z);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("empty");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
    assert!(err.message.contains("arr.first on empty array"));
}

#[test]
fn runs_str_lastindexof_and_replace() {
    let src = r#"
import str;
fn main() -> Int {
  let s = "a-b-a-b";
  let i = str.lastIndexOf(s, "a");
  let r = str.replace(s, "-", "_");
  if (i == 4 && r == "a_b_a_b" && str.lastIndexOf(s, "z") == -1) {
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
fn runs_str_repeat() {
    let src = r#"
import str;
fn main() -> Int {
  let s = str.repeat("ab", 3);
  if (s == "ababab") {
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
fn vm_reports_str_repeat_negative_count() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.repeat("x", -1);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("negative repeat");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
    assert!(err.message.contains("str.repeat count must be >= 0"));
}

#[test]
fn vm_reports_str_repeat_output_too_large() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.repeat("x", 1000001);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("repeat too large");
    assert_eq!(err.kind, VmErrorKind::IndexOutOfBounds);
    assert!(err.message.contains("str.repeat output too large"));
}

#[test]
fn runs_arr_join_and_unicode_last_indexof() {
    let src = r#"
import arr;
import str;
fn main() -> Int {
  let a: [String; 3] = ["hi", "sk", "lang"];
  let j = arr.join(a, "::");
  let s = "naïve-naïve";
  let idx = str.lastIndexOf(s, "ï");
  if (j == "hi::sk::lang" && idx == 8) {
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
fn vm_reports_arr_join_runtime_type_mismatch_for_non_string_elements() {
    let module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Array(vec![Value::Int(1), Value::Int(2)])),
                    Instr::LoadConst(Value::String(",".to_string())),
                    Instr::CallBuiltin {
                        package: "arr".to_string(),
                        name: "join".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&module).expect_err("join type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("arr.join expects Array[String]"));
}

#[test]
fn runs_datetime_now_builtins() {
    let src = r#"
import datetime;
fn main() -> Int {
  let s = datetime.nowUnix();
  let ms = datetime.nowMillis();
  if (s < 0) {
    return 1;
  }
  if (ms < s * 1000) {
    return 2;
  }
  if (ms > (s + 2) * 1000) {
    return 3;
  }
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(0));
}

#[test]
fn vm_reports_datetime_runtime_arity_mismatch_from_manual_bytecode() {
    let unix_arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "nowUnix".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&unix_arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("datetime.nowUnix expects 0 arguments"));

    let millis_arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "nowMillis".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&millis_arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(
        err.message
            .contains("datetime.nowMillis expects 0 arguments")
    );
}

#[test]
fn runs_datetime_from_unix_and_millis() {
    let src = r#"
import datetime;
fn main() -> Int {
  let a = datetime.fromUnix(0);
  let b = datetime.fromMillis(1234);
  let c = datetime.fromUnix(-1);
  let d = datetime.fromMillis(-1);
  if (a == "1970-01-01T00:00:00Z"
      && b == "1970-01-01T00:00:01.234Z"
      && c == "1969-12-31T23:59:59Z"
      && d == "1969-12-31T23:59:59.999Z") {
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
fn vm_reports_datetime_from_runtime_errors_from_manual_bytecode() {
    let from_unix_arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "fromUnix".to_string(),
                        argc: 0,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&from_unix_arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("datetime.fromUnix expects 1 argument"));

    let from_millis_type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("bad".to_string())),
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "fromMillis".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&from_millis_type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(
        err.message
            .contains("datetime.fromMillis expects Int argument")
    );
}

#[test]
fn runs_datetime_parse_unix() {
    let src = r#"
import datetime;
fn main() -> Int {
  let z = datetime.parseUnix("1970-01-01T00:00:00Z");
  let p = datetime.parseUnix("1970-01-01T00:00:01Z");
  let n = datetime.parseUnix("1969-12-31T23:59:59Z");
  if (z == 0 && p == 1 && n == -1) {
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
fn vm_reports_datetime_parse_unix_invalid_format() {
    let src = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.parseUnix("2026-02-17 12:34:56");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("invalid datetime format");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("datetime.parseUnix expects format"));
}

#[test]
fn vm_reports_datetime_parse_unix_runtime_errors_from_manual_bytecode() {
    let arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "parseUnix".to_string(),
                        argc: 0,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(
        err.message
            .contains("datetime.parseUnix expects 1 argument")
    );

    let type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "parseUnix".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(
        err.message
            .contains("datetime.parseUnix expects String argument")
    );
}

#[test]
fn runs_datetime_component_extractors() {
    let src = r#"
import datetime;
fn main() -> Int {
  let ts = 1704112496; // 2024-01-01T12:34:56Z
  if (datetime.year(ts) == 2024
      && datetime.month(ts) == 1
      && datetime.day(ts) == 1
      && datetime.hour(ts) == 12
      && datetime.minute(ts) == 34
      && datetime.second(ts) == 56) {
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
fn vm_reports_datetime_component_runtime_errors_from_manual_bytecode() {
    let arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "year".to_string(),
                        argc: 0,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("datetime.year expects 1 argument"));

    let type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("bad".to_string())),
                    Instr::CallBuiltin {
                        package: "datetime".to_string(),
                        name: "second".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("datetime.second expects Int argument"));
}

#[test]
fn runs_datetime_roundtrip_from_unix_and_parse_unix() {
    let src = r#"
import datetime;
fn main() -> Int {
  let a = 0;
  let b = -1;
  let c = 1704112496;
  if (datetime.parseUnix(datetime.fromUnix(a)) == a
      && datetime.parseUnix(datetime.fromUnix(b)) == b
      && datetime.parseUnix(datetime.fromUnix(c)) == c) {
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
fn runs_datetime_parse_unix_leap_year_and_rejects_invalid_date() {
    let ok_src = r#"
import datetime;
fn main() -> Int {
  let ts = datetime.parseUnix("2024-02-29T00:00:00Z");
  if (datetime.month(ts) == 2 && datetime.day(ts) == 29) {
    return 1;
  }
  return 0;
}
"#;
    let ok_module = compile_source(ok_src).expect("compile");
    let out = Vm::run_module_main(&ok_module).expect("run");
    assert_eq!(out, Value::Int(1));

    let bad_src = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.parseUnix("2023-02-29T00:00:00Z");
  return 0;
}
"#;
    let bad_module = compile_source(bad_src).expect("compile");
    let err = Vm::run_module_main(&bad_module).expect_err("invalid date");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("datetime.parseUnix day out of range"));
}

#[test]
fn vm_reports_datetime_parse_unix_invalid_time_ranges() {
    let bad_hour = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.parseUnix("2026-01-01T24:00:00Z");
  return 0;
}
"#;
    let m1 = compile_source(bad_hour).expect("compile");
    let e1 = Vm::run_module_main(&m1).expect_err("hour out of range");
    assert_eq!(e1.kind, VmErrorKind::TypeMismatch);
    assert!(e1.message.contains("datetime.parseUnix time out of range"));

    let bad_month = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.parseUnix("2026-13-01T00:00:00Z");
  return 0;
}
"#;
    let m2 = compile_source(bad_month).expect("compile");
    let e2 = Vm::run_module_main(&m2).expect_err("month out of range");
    assert_eq!(e2.kind, VmErrorKind::TypeMismatch);
    assert!(e2.message.contains("datetime.parseUnix month out of range"));
}

#[test]
fn runs_random_seed_and_updates_host_state() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed(12345);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let mut host = TestHost::default();
    let out = Vm::run_module_main_with_host(&module, &mut host).expect("run");
    assert_eq!(out, Value::Int(0));
    assert_eq!(host.rng_state, 12345u64);
}

#[test]
fn vm_reports_random_seed_runtime_errors_from_manual_bytecode() {
    let arity_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::CallBuiltin {
                        package: "random".to_string(),
                        name: "seed".to_string(),
                        argc: 0,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&arity_module).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("random.seed expects 1 argument"));

    let type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("bad".to_string())),
                    Instr::CallBuiltin {
                        package: "random".to_string(),
                        name: "seed".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("random.seed expects Int argument"));
}

#[test]
fn runs_random_int_and_float_with_seed_determinism_and_ranges() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed(42);
  let i1 = random.int(10, 20);
  let f1 = random.float();
  random.seed(42);
  let i2 = random.int(10, 20);
  let f2 = random.float();
  if (i1 == i2 && f1 == f2 && i1 >= 10 && i1 <= 20 && f1 >= 0.0 && f1 < 1.0) {
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
fn vm_reports_random_int_float_runtime_arity_errors_from_manual_bytecode() {
    let int_arity = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "random".to_string(),
                        name: "int".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&int_arity).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("random.int expects 2 arguments"));

    let float_arity = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::Int(1)),
                    Instr::CallBuiltin {
                        package: "random".to_string(),
                        name: "float".to_string(),
                        argc: 1,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&float_arity).expect_err("arity mismatch");
    assert_eq!(err.kind, VmErrorKind::ArityMismatch);
    assert!(err.message.contains("random.float expects 0 arguments"));
}

#[test]
fn vm_reports_random_int_runtime_type_and_bounds_errors() {
    let src = r#"
import random;
fn main() -> Int {
  let _x = random.int(10, 5);
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("invalid bounds");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("random.int expects min <= max"));

    let type_module = BytecodeModule {
        functions: vec![(
            "main".to_string(),
            FunctionChunk {
                name: "main".to_string(),
                code: vec![
                    Instr::LoadConst(Value::String("x".to_string())),
                    Instr::LoadConst(Value::Int(2)),
                    Instr::CallBuiltin {
                        package: "random".to_string(),
                        name: "int".to_string(),
                        argc: 2,
                    },
                    Instr::Return,
                ],
                locals_count: 0,
                param_count: 0,
            },
        )]
        .into_iter()
        .collect(),
    };
    let err = Vm::run_module_main(&type_module).expect_err("type mismatch");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("random.int argument 1 expects Int"));
}

#[test]
fn runs_random_int_single_point_range_is_constant() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed(999);
  let a = random.int(5, 5);
  let b = random.int(5, 5);
  if (a == 5 && b == 5) {
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
fn runs_datetime_component_extractors_for_negative_timestamp() {
    let src = r#"
import datetime;
fn main() -> Int {
  let ts = -1;
  if (datetime.year(ts) == 1969
      && datetime.month(ts) == 12
      && datetime.day(ts) == 31
      && datetime.hour(ts) == 23
      && datetime.minute(ts) == 59
      && datetime.second(ts) == 59) {
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
fn vm_reports_datetime_parse_unix_invalid_non_digit_fields() {
    let src = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.parseUnix("202A-01-01T00:00:00Z");
  return 0;
}
"#;
    let module = compile_source(src).expect("compile");
    let err = Vm::run_module_main(&module).expect_err("invalid year");
    assert_eq!(err.kind, VmErrorKind::TypeMismatch);
    assert!(err.message.contains("datetime.parseUnix invalid year"));
}

#[test]
fn runs_global_variables_across_functions() {
    let src = r#"
let counter: Int = 0;

fn inc() -> Int {
  counter = counter + 1;
  return counter;
}

fn main() -> Int {
  let a = inc();
  let b = inc();
  return a * 10 + b;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(12));
}

#[test]
fn runs_global_initializer_before_main() {
    let src = r#"
let g: Int = seed();

fn seed() -> Int {
  return 7;
}

fn main() -> Int {
  return g;
}
"#;
    let module = compile_source(src).expect("compile");
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(7));
}

#[test]
fn lowering_uses_fully_qualified_name_for_from_import_call() {
    let src = r#"
from utils.math import add as plus;
fn main() -> Int {
  return plus(1, 2);
}
"#;
    let module = compile_source(src).expect("compile");
    let main = module.functions.get("main").expect("main fn");
    assert!(main.code.iter().any(|i| {
        matches!(
            i,
            Instr::Call { name, argc } if name == "utils.math.add" && *argc == 2
        )
    }));
}

#[test]
fn lowering_uses_fully_qualified_name_for_namespace_call() {
    let src = r#"
import utils.math;
fn main() -> Int {
  return utils.math.add(1, 2);
}
"#;
    let module = compile_source(src).expect("compile");
    let main = module.functions.get("main").expect("main fn");
    assert!(main.code.iter().any(|i| {
        matches!(
            i,
            Instr::Call { name, argc } if name == "utils.math.add" && *argc == 2
        )
    }));
}
