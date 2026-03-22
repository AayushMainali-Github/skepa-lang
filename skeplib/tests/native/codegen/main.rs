use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::codegen;
use skeplib::ir;

#[path = "../../common.rs"]
mod common;

fn temp_file(name: &str, ext: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    std::env::temp_dir().join(format!("skepa_codegen_{name}_{nanos}.{ext}"))
}

fn assemble_llvm_ir(llvm_ir: &str, label: &str) {
    common::require_llvm_tool("llvm-as");
    let ll_path = temp_file(label, "ll");
    let bc_path = temp_file(label, "bc");
    fs::write(&ll_path, llvm_ir).expect("should write temporary llvm ir file");

    let output = Command::new("llvm-as")
        .arg(&ll_path)
        .arg("-o")
        .arg(&bc_path)
        .output()
        .expect("llvm-as should be available on PATH");

    let _ = fs::remove_file(&ll_path);
    let _ = fs::remove_file(&bc_path);

    assert!(
        output.status.success(),
        "llvm-as rejected generated IR: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn llvm_function_body<'a>(llvm_ir: &'a str, name: &str) -> &'a str {
    let marker = format!("define i64 @\"{name}\"");
    let start = llvm_ir
        .find(&marker)
        .unwrap_or_else(|| panic!("missing function {name} in llvm ir"));
    let body = &llvm_ir[start..];
    let end = body
        .find("\n}\n")
        .unwrap_or_else(|| panic!("missing end for function {name}"));
    &body[..end]
}

#[test]
fn llvm_codegen_emits_valid_int_only_module() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  let acc = 1;
  while (i < 4) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @\"main\"()"));
    assert!(llvm_ir.contains("icmp slt"));
    assert!(llvm_ir.contains("br i1"));

    assemble_llvm_ir(&llvm_ir, "valid");
}

#[test]
fn llvm_codegen_emits_valid_direct_calls() {
    let source = r#"
fn step(x: Int) -> Int {
  if (x < 10) {
    return x + 1;
  }
  return x;
}

fn main() -> Int {
  return step(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @\"step\"(i64 %arg0)"));
    assert!(llvm_ir.contains("call i64 @\"step\"(i64 4)"));

    assemble_llvm_ir(&llvm_ir, "direct_call");
}

#[test]
fn llvm_codegen_emits_valid_string_calls_and_constants() {
    let source = r#"
fn greet() -> String {
  return "hello";
}

fn main() -> String {
  return greet();
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_string_from_utf8(ptr, i64)"));
    assert!(llvm_ir.contains("define internal void @\"__skp_init_runtime_strings\"()"));
    assert!(llvm_ir.contains("@.rtstr."));
    assert!(llvm_ir.contains("define ptr @\"greet\"()"));
    assert_eq!(
        llvm_ir.matches("call ptr @skp_rt_string_from_utf8").count(),
        1
    );
    assert!(llvm_ir.contains("define ptr @\"main\"()"));

    assemble_llvm_ir(&llvm_ir, "string_call");
}

#[test]
fn llvm_codegen_emits_str_builtin_runtime_calls() {
    let source = r#"
import str;

fn main() -> Int {
  return str.len("hello") + str.indexOf("skepa", "epa");
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare i64 @skp_rt_builtin_str_len(ptr)"));
    assert!(llvm_ir.contains("declare i64 @skp_rt_builtin_str_index_of(ptr, ptr)"));
    assert!(llvm_ir.contains("call i64 @skp_rt_builtin_str_len(ptr"));
    assert!(llvm_ir.contains("call i64 @skp_rt_builtin_str_index_of(ptr"));

    assemble_llvm_ir(&llvm_ir, "str_builtin");
}

#[test]
fn llvm_codegen_folds_benchmark_shaped_constant_string_builtins() {
    let source = r#"
import str;

fn main() -> Int {
  let i = 0;
  let total = 0;
  while (i < 10) {
    let s = "skepa-language";
    total = total + str.len(s);
    total = total + str.indexOf(s, "lang");
    let cut = str.slice(s, 0, 5);
    if (str.contains(cut, "ske")) {
      total = total + 1;
    }
    i = i + 1;
  }
  if (total > 0) {
    return 0;
  }
  return 1;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(!main_body.contains("@skp_rt_builtin_str_len"));
    assert!(!main_body.contains("@skp_rt_builtin_str_index_of"));
    assert!(!main_body.contains("@skp_rt_builtin_str_slice"));
    assert!(!main_body.contains("@skp_rt_builtin_str_contains"));

    assemble_llvm_ir(&llvm_ir, "string_builtin_folded");
}

#[test]
fn llvm_codegen_scalarizes_read_only_local_string_arrays() {
    let source = r#"
import str;

fn main() -> Int {
  let words: [String; 4] = ["skepa", "language", "native", "speed"];
  let i = 0;
  let total = 0;
  while (i < 10) {
    let word = words[i % 4];
    total = total + str.len(word);
    i = i + 1;
  }
  return total;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(main_body.contains("%local0_elem0 = alloca ptr"));
    assert!(main_body.contains("%local0_elem3 = alloca ptr"));
    assert!(main_body.contains("array_get_case_"));
    assert!(!main_body.contains("@skp_rt_array_get"));

    assemble_llvm_ir(&llvm_ir, "string_array_scalarized");
}

#[test]
fn llvm_codegen_emits_project_entry_wrapper_calls() {
    let project = common::TempProject::new("project_codegen");
    let entry = project.file(
        "main.sk",
        r#"
fn helper(x: Int) -> Int {
  return x + 7;
}

fn main() -> Int {
  return helper(5);
}
"#,
    );

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @\"main::helper\"(i64 %arg0)"));
    assert!(llvm_ir.contains("define i64 @\"main\"()"));

    assemble_llvm_ir(&llvm_ir, "project_codegen");
}

#[test]
fn llvm_codegen_emits_module_init_via_global_ctors() {
    let project = common::TempProject::new("project_globals_codegen");
    let entry = project.file(
        "main.sk",
        r#"
let base: Int = 3;
let answer: Int = 7;

fn main() -> Int {
  return answer;
}
"#,
    );

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("@llvm.global_ctors = appending global"));
    assert!(llvm_ir.contains("@\"__skp_codegen_init\""));
    assert!(llvm_ir.contains("call void @\"__globals_init\"()"));
    assert!(llvm_ir.contains("store i64"));

    assemble_llvm_ir(&llvm_ir, "project_globals_codegen");
}

#[test]
fn llvm_codegen_emits_array_runtime_calls() {
    let source = r#"
fn main() -> Int {
  let arr: [Int; 3] = [0; 3];
  arr[1] = 7;
  arr[2] = arr[1] + 5;
  return arr[2];
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_array_new(i64)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_array_get(ptr, i64)"));
    assert!(llvm_ir.contains("declare void @skp_rt_array_set(ptr, i64, ptr)"));
    assert!(llvm_ir.contains("@skp_rt_value_from_int"));
    assert!(llvm_ir.contains("@skp_rt_value_to_int"));

    assemble_llvm_ir(&llvm_ir, "array_runtime");
}

#[test]
fn llvm_codegen_scalarizes_benchmark_shaped_local_int_arrays() {
    let source = r#"
fn main() -> Int {
  let arr: [Int; 8] = [0; 8];
  let i = 0;
  while (i < 10) {
    let idx = i % 8;
    arr[idx] = arr[idx] + 1;
    i = i + 1;
  }
  return arr[0] + arr[1];
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(main_body.contains("%local0_elem0 = alloca i64"));
    assert!(main_body.contains("%local0_elem7 = alloca i64"));
    assert!(main_body.contains("switch i64"));
    assert!(main_body.contains("array_get_case_"));
    assert!(main_body.contains("array_set_case_"));

    assemble_llvm_ir(&llvm_ir, "array_scalarized");
}

#[test]
fn llvm_codegen_emits_struct_runtime_calls_and_methods() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    if (x < 0) {
      return self.a;
    }
    return self.a + self.b + x;
  }
}

fn main() -> Int {
  let p = Pair { a: 2, b: 3 };
  p.a = 7;
  return p.mix(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_struct_new(i64, i64)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_struct_get(ptr, i64)"));
    assert!(llvm_ir.contains("declare void @skp_rt_struct_set(ptr, i64, ptr)"));
    assert!(llvm_ir.contains("define i64 @\"Pair::mix\"(ptr %arg0, i64 %arg1)"));

    assemble_llvm_ir(&llvm_ir, "struct_runtime");
}

#[test]
fn llvm_codegen_scalarizes_benchmark_shaped_local_struct_field_reads() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return ((self.a + x) * 3 + self.b) % 1000000007;
  }
}

fn main() -> Int {
  let p = Pair { a: 11, b: 7 };
  let i = 0;
  let total = 0;
  while (i < 10) {
    total = total + p.mix(i % 13);
    i = i + 1;
  }
  if (total > 0) {
    return 0;
  }
  return 1;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(main_body.contains("%local0_field0 = alloca i64"));
    assert!(main_body.contains("%local0_field1 = alloca i64"));
    assert!(main_body.contains("load i64, ptr %local0_field0"));
    assert!(main_body.contains("load i64, ptr %local0_field1"));
    assert!(!main_body.contains("@skp_rt_struct_get"));

    assemble_llvm_ir(&llvm_ir, "struct_scalarized");
}

#[test]
fn llvm_codegen_emits_vec_runtime_calls() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 10);
  vec.push(xs, 20);
  vec.set(xs, 1, 30);
  let first = vec.get(xs, 0);
  let removed = vec.delete(xs, 1);
  return first + removed + vec.len(xs);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_vec_new()"));
    assert!(llvm_ir.contains("declare i64 @skp_rt_vec_len(ptr)"));
    assert!(llvm_ir.contains("declare void @skp_rt_vec_push(ptr, ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_vec_get(ptr, i64)"));
    assert!(llvm_ir.contains("declare void @skp_rt_vec_set(ptr, i64, ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_vec_delete(ptr, i64)"));

    assemble_llvm_ir(&llvm_ir, "vec_runtime");
}

#[test]
fn llvm_codegen_emits_generic_runtime_builtin_dispatch() {
    let source = r#"
import datetime;
import fs;
import os;
import str;

fn main() -> Int {
  let now = datetime.nowUnix();
  let cwd = os.cwd();
  if (fs.exists("missing.txt")) {
    return now + str.len(cwd);
  }
  return now;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_call_builtin(ptr, ptr, i64, ptr)"));
    assert!(llvm_ir.contains("declare void @skp_rt_value_free(ptr)"));
    assert!(llvm_ir.contains("call ptr @skp_rt_call_builtin("));
    assert!(llvm_ir.contains("call void @skp_rt_value_free(ptr %v"));
    assert!(llvm_ir.contains("@.str."));

    assemble_llvm_ir(&llvm_ir, "generic_builtin_runtime");
}

#[test]
fn llvm_codegen_emits_indirect_call_trampoline() {
    let source = r#"
fn step(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let f: Fn(Int) -> Int = step;
  return f(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define internal ptr @__skp_rt_call_function_dispatch("));
    assert!(llvm_ir.contains("call ptr @__skp_rt_call_function_dispatch("));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_from_function(i32)"));
    assert!(llvm_ir.contains("declare i32 @skp_rt_value_to_function(ptr)"));
    assert!(llvm_ir.contains("declare void @skp_rt_value_free(ptr)"));
    assert!(llvm_ir.contains("icmp eq i64 %argc, 1"));
    assert!(llvm_ir.contains("call ptr @skp_rt_call_function(i32 0, i64 %argc, ptr %argv)"));
    assert!(llvm_ir.contains("call ptr @skp_rt_call_function(i32 -1, i64 %argc, ptr %argv)"));
    assert!(llvm_ir.contains("call void @skp_rt_value_free(ptr %v"));

    assemble_llvm_ir(&llvm_ir, "indirect_call_runtime");
}

#[test]
fn codegen_rejects_direct_call_return_type_mismatch_in_invalid_ir() {
    let mut builder = ir::IrBuilder::new();
    let mut program = builder.begin_program();

    let mut callee = builder.begin_function("callee", ir::IrType::Int);
    let callee_entry = callee.entry;
    builder.set_terminator(
        &mut callee,
        callee_entry,
        ir::Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(1)))),
    );
    let callee_id = callee.id;
    program.functions.push(callee);

    let mut main = builder.begin_function("main", ir::IrType::Int);
    let main_entry = main.entry;
    let dst = builder.push_temp(&mut main, ir::IrType::Bool);
    builder.push_instr(
        &mut main,
        main_entry,
        ir::Instr::CallDirect {
            dst: Some(dst),
            ret_ty: ir::IrType::Bool,
            function: callee_id,
            args: Vec::new(),
        },
    );
    builder.set_terminator(
        &mut main,
        main_entry,
        ir::Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
    );
    program.functions.push(main);

    let err = codegen::compile_program_to_llvm_ir(&program).expect_err("invalid direct call");
    let msg = err.to_string();
    assert!(msg.contains("call return type mismatch"));
}

#[test]
fn codegen_rejects_missing_parameter_backed_locals_in_invalid_ir() {
    let mut builder = ir::IrBuilder::new();
    let mut program = builder.begin_program();

    let mut func = builder.begin_function("main", ir::IrType::Int);
    let entry = func.entry;
    builder.push_param(&mut func, "x", ir::IrType::Int);
    builder.set_terminator(
        &mut func,
        entry,
        ir::Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
    );
    program.functions.push(func);

    let err =
        codegen::compile_program_to_llvm_ir(&program).expect_err("invalid param/local layout");
    let msg = err.to_string();
    assert!(msg.contains("missing parameter-backed locals"));
}

#[test]
fn codegen_rejects_reserved_internal_helper_function_names() {
    let source = r#"
fn __skp_rt_user_collision() -> Int {
  return 1;
}

fn main() -> Int {
  return __skp_rt_user_collision();
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let err = codegen::compile_program_to_llvm_ir(&program)
        .expect_err("reserved helper prefix should fail");
    let msg = err.to_string();
    assert!(msg.contains("reserved LLVM helper prefix"));
}

#[test]
fn codegen_rejects_panic_and_unreachable_terminators_in_invalid_ir() {
    let mut builder = ir::IrBuilder::new();
    let mut program = builder.begin_program();

    let mut panic_func = builder.begin_function("panic_path", ir::IrType::Int);
    let panic_entry = panic_func.entry;
    builder.set_terminator(
        &mut panic_func,
        panic_entry,
        ir::Terminator::Panic {
            message: "boom".into(),
        },
    );
    program.functions.push(panic_func);

    let err =
        codegen::compile_program_to_llvm_ir(&program).expect_err("panic terminator should fail");
    let msg = err.to_string();
    assert!(msg.contains("does not lower panic terminators"));

    let mut builder = ir::IrBuilder::new();
    let mut program = builder.begin_program();
    let unreachable_func = builder.begin_function("unreachable_path", ir::IrType::Int);
    program.functions.push(unreachable_func);
    let err = codegen::compile_program_to_llvm_ir(&program)
        .expect_err("unreachable terminator should fail");
    let msg = err.to_string();
    assert!(msg.contains("does not lower unreachable terminators"));
}

#[test]
fn codegen_builds_native_executable_for_indirect_calls() {
    let source = r#"
fn step(x: Int) -> Int {
  return x + 3;
}

fn main() -> Int {
  let f: Fn(Int) -> Int = step;
  return f(4);
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 7);
}

#[test]
fn llvm_codegen_emits_runtime_abi_boxing_and_unboxing_surface() {
    let source = r#"
fn pick(flag: Bool) -> Bool {
  return flag;
}

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 2);
  let ok = pick(true);
  if (ok) {
    return vec.get(xs, 0);
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_value_from_int(i64)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_from_bool(i1)"));
    assert!(llvm_ir.contains("declare i64 @skp_rt_value_to_int(ptr)"));
    assert!(llvm_ir.contains("declare i1 @skp_rt_value_to_bool(ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_from_vec(ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_to_vec(ptr)"));
    assert!(llvm_ir.contains("declare void @skp_rt_value_free(ptr)"));
    assert!(llvm_ir.contains("call void @skp_rt_value_free(ptr %v"));
}

#[test]
fn llvm_codegen_frees_boxed_values_after_runtime_container_ops() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

fn main() -> Int {
  let arr: [Int; 2] = [1, 2];
  let xs: Vec[Int] = vec.new();
  let p = Pair { a: 3, b: 4 };
  vec.push(xs, arr[0]);
  vec.set(xs, 0, p.a);
  return vec.get(xs, 0);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("call void @skp_rt_array_set("));
    assert!(llvm_ir.contains("call void @skp_rt_vec_push("));
    assert!(llvm_ir.contains("call void @skp_rt_vec_set("));
    assert!(llvm_ir.contains("call void @skp_rt_struct_set("));
    assert!(llvm_ir.contains("call void @skp_rt_value_free(ptr %v"));

    assemble_llvm_ir(&llvm_ir, "boxed_value_frees");
}

#[test]
fn llvm_codegen_caches_reused_runtime_string_literals_once_per_module() {
    let source = r#"
fn main() -> Int {
  let a = "alpha";
  let b = "alpha";
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert_eq!(
        llvm_ir.matches("call ptr @skp_rt_string_from_utf8").count(),
        1
    );
    assert!(llvm_ir.matches("load ptr, ptr @.rtstr.").count() >= 2);

    assemble_llvm_ir(&llvm_ir, "cached_runtime_strings");
}

#[test]
fn llvm_codegen_emits_bool_compare_using_i1() {
    let source = r#"
fn main() -> Int {
  let a = true;
  let b = false;
  if (a != b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("alloca i1"));
    assert!(llvm_ir.contains("load i1"));
    assert!(llvm_ir.contains("icmp ne i1"));
    assert!(!llvm_ir.contains("icmp ne i64"));
}

#[test]
fn llvm_codegen_emits_bool_equality_using_i1() {
    let source = r#"
fn main() -> Int {
  let a = true;
  let b = true;
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("load i1"));
    assert!(llvm_ir.contains("icmp eq i1"));
}

#[test]
fn llvm_codegen_emits_global_bool_compare_using_i1() {
    let source = r#"
let enabled: Bool = true;

fn main() -> Int {
  if (enabled == true) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("@g0 = global i1 0"));
    assert!(llvm_ir.contains("store i1 1, ptr @g0"));
    assert!(llvm_ir.contains("load i1, ptr @g0"));
    assert!(llvm_ir.contains("icmp eq i1"));
}

#[test]
fn llvm_codegen_keeps_int_compare_using_i64() {
    let source = r#"
fn main() -> Int {
  let a = 1;
  let b = 2;
  if (a < b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("load i64"));
    assert!(llvm_ir.contains("icmp slt i64"));
}

#[test]
fn llvm_codegen_emits_float_compare_using_double_and_fcmp() {
    let source = r#"
fn main() -> Int {
  let x = 1.5;
  let y = 2.0;
  if ((x + y) >= 3.5) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("fadd double"));
    assert!(llvm_ir.contains("fcmp oge double"));
    assert!(!llvm_ir.contains("unsupported codegen shape"));
}

#[test]
fn llvm_codegen_emits_global_float_compare_using_double_and_fcmp() {
    let source = r#"
let threshold: Float = 3.5;

fn main() -> Int {
  let value = 1.5 + 2.0;
  if (value >= threshold) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("@g0 = global double 0"));
    assert!(llvm_ir.contains("store double 3.5, ptr @g0"));
    assert!(llvm_ir.contains("load double, ptr @g0"));
    assert!(llvm_ir.contains("fcmp oge double"));
}

#[test]
fn llvm_codegen_lowers_string_equality_through_runtime_helper() {
    let source = r#"
fn main() -> Int {
  let a = "alpha";
  let b = "alpha";
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare i1 @skp_rt_string_eq(ptr, ptr)"));
    assert!(llvm_ir.contains("call i1 @skp_rt_string_eq(ptr"));
    assert!(llvm_ir.contains("load ptr, ptr @.rtstr."));
    assert!(!llvm_ir.contains("load i64, ptr %local0"));
    assert!(!llvm_ir.contains("load i64, ptr %local1"));
}

#[test]
fn llvm_codegen_lowers_string_inequality_and_global_string_compare_through_runtime_helper() {
    let source = r#"
let expected: String = "alpha";

fn main() -> Int {
  let a = "alpha";
  let b = "beta";
  if (a != b && a == expected) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare i1 @skp_rt_string_eq(ptr, ptr)"));
    assert!(llvm_ir.matches("call i1 @skp_rt_string_eq(ptr").count() >= 2);
    assert!(llvm_ir.contains("xor i1"));
    assert!(!llvm_ir.contains("icmp ne i64"));
}

#[test]
fn llvm_codegen_emits_runtime_abi_for_struct_layout_and_builtin_dispatch() {
    let source = r#"
import fs;

struct Pair {
  a: Int,
  b: Int
}

fn main() -> Int {
  let p = Pair { a: 3, b: 4 };
  if (fs.exists("missing.txt")) {
    return p.a;
  }
  return p.b;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare ptr @skp_rt_struct_new(i64, i64)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_struct_get(ptr, i64)"));
    assert!(llvm_ir.contains("declare void @skp_rt_struct_set(ptr, i64, ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_call_builtin(ptr, ptr, i64, ptr)"));
    assert!(llvm_ir.contains("call ptr @skp_rt_call_builtin("));
}

#[test]
fn codegen_emits_object_file_for_int_program() {
    let source = r#"
fn main() -> Int {
  return 7;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let obj_path = temp_file("object_only", object_ext());

    codegen::compile_program_to_object_file(&program, &obj_path)
        .expect("object emission should succeed");

    assert!(obj_path.exists());
    let _ = fs::remove_file(&obj_path);
}

#[test]
fn codegen_builds_native_executable_for_int_program() {
    let source = r#"
fn main() -> Int {
  return 7;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let exe_path = temp_file("native_exec", exe_ext());

    codegen::compile_program_to_executable(&program, &exe_path)
        .expect("native executable build should succeed");

    let output = Command::new(&exe_path)
        .output()
        .expect("built executable should run");

    let _ = fs::remove_file(&exe_path);

    assert_eq!(output.status.code(), Some(7));
}

#[test]
fn codegen_builds_native_executable_for_string_and_arr_builtins() {
    let source = r#"
import str;

fn main() -> Int {
  let s = "alpha-beta";
  return str.len(s) + str.indexOf(s, "beta");
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 16);
}

#[test]
fn codegen_builds_native_executable_for_arr_builtin_family() {
    let source = r#"
import arr;

fn main() -> Int {
  let xs: [Int; 3] = [1, 2, 3];
  if (arr.isEmpty(xs)) {
    return 0;
  }
  return arr.len(xs) + 4;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 7);
}

#[test]
fn codegen_builds_native_executable_for_arrays_vecs_and_struct_methods() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn total(self) -> Int {
    return self.a + self.b;
  }
}

fn main() -> Int {
  let arr: [Int; 2] = [2; 2];
  let xs: Vec[Int] = vec.new();
  vec.push(xs, arr[0]);
  vec.push(xs, arr[1] + 3);
  let p = Pair { a: vec.get(xs, 0), b: vec.get(xs, 1) };
  return p.total();
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 7);
}

#[test]
fn codegen_builds_native_executable_for_globals_and_module_init() {
    let source = r#"
let seed: Int = 4;
let answer: Int = seed + 3;

fn main() -> Int {
  return answer;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 7);
}

#[test]
fn codegen_builds_native_executable_for_io_and_datetime_builtins() {
    let source = r#"
import io;
import datetime;

fn main() -> Int {
  io.println("native-ok");
  if (datetime.nowMillis() > 0) {
    return 7;
  }
  return 0;
}
"#;

    let output = common::native_run_structured(source);
    assert_eq!(output.exit_code(), 7);
    assert!(
        output.stdout_lossy().contains("native-ok"),
        "expected io builtin output, got: {}",
        output.stdout_lossy()
    );
}

#[test]
fn codegen_builds_native_executable_for_random_builtins() {
    let source = r#"
import random;

fn main() -> Int {
  random.seed(7);
  let a = random.int(1, 10);
  random.seed(7);
  let b = random.int(1, 10);
  if (a == b) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

#[test]
fn codegen_builds_native_executable_for_fs_and_os_builtins() {
    let dir = temp_file("native_fs_os", "dir");
    fs::create_dir_all(&dir).expect("temporary dir should be created");
    let path = dir.join("note.txt");
    let path_text = path.to_string_lossy().replace('\\', "\\\\");
    let joined_left = dir.to_string_lossy().replace('\\', "\\\\");
    let shell = if cfg!(windows) {
        "echo hi"
    } else {
        "printf hi"
    };

    let source = format!(
        r#"
import fs;
import io;
import os;
import str;

fn main() -> Int {{
  fs.writeText("{path_text}", "a");
  fs.appendText("{path_text}", "b");
  let text = fs.readText("{path_text}");
  let joined = fs.join("{joined_left}", "note.txt");
  let cwd = os.cwd();
  let platform = os.platform();
  let shell = os.execShellOut("{shell}");
  io.print(text);
  io.println("");
  io.print(shell);
  io.println("");
  if (fs.exists("{path_text}") && str.len(text) == 2 && str.contains(joined, "note.txt") && str.len(cwd) > 0 && str.len(platform) > 0) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let output = common::native_run_structured(&source);
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir_all(&dir);

    assert_eq!(output.exit_code(), 0);
    assert!(
        output.stdout_lossy().contains("ab"),
        "expected fs output, got: {}",
        output.stdout_lossy()
    );
    assert!(
        output.stdout_lossy().contains("hi"),
        "expected shell output, got: {}",
        output.stdout_lossy()
    );
}

#[test]
fn codegen_builds_native_project_entry_wrapper_executable() {
    let project = common::TempProject::new("project_native_runtime");
    project.file(
        "util/math.sk",
        r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

export { add };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from util.math import add;

fn main() -> Int {
  return add(3, 4);
}
"#,
    );

    assert_eq!(common::native_run_project_structured(&entry).exit_code(), 7);
}

fn object_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}
