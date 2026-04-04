use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::codegen;
use skeplib::ir;

#[path = "../../common.rs"]
mod common;

#[cfg(windows)]
fn ffi_test_library_path() -> &'static str {
    "kernel32.dll"
}

#[cfg(windows)]
fn ffi_test_symbol_name() -> &'static str {
    "GetCurrentProcessId"
}

#[cfg(windows)]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "lstrlenA"
}

#[cfg(windows)]
fn ffi_test_call1_int_value() -> i64 {
    0
}

#[cfg(windows)]
fn ffi_test_call1_int_expected() -> i64 {
    0
}

#[cfg(windows)]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "lstrlenA"
}

#[cfg(windows)]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(windows)]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(windows)]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "kernel32.dll"
}

#[cfg(windows)]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "OutputDebugStringA"
}

#[cfg(windows)]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "lstrcmpA"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(windows)]
fn ffi_test_call0_void_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "_tzset"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_symbol_name() -> &'static str {
    "getpid"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "abs"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_value() -> i64 {
    -9
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_expected() -> i64 {
    9
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "strlen"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "perror"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "strcmp"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call0_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "tzset"
}

#[cfg(target_os = "macos")]
fn ffi_test_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_symbol_name() -> &'static str {
    "getpid"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "abs"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_value() -> i64 {
    -9
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_expected() -> i64 {
    9
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "strlen"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "perror"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "strcmp"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(target_os = "macos")]
fn ffi_test_call0_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "tzset"
}

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
fn llvm_codegen_emits_bitwise_integer_ops() {
    let source = r#"
fn main() -> Int {
  let a = 12;
  let b = 10;
  let c = ~a;
  let d = a & b;
  let e = a | b;
  let f = a ^ b;
  let g = a << 2;
  let h = a >> 1;
  if (c == -13 && d == 8 && e == 14 && f == 6 && g == 48 && h == 6) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(main_body.contains("xor i64"));
    assert!(main_body.contains("and i64"));
    assert!(main_body.contains("or i64"));
    assert!(main_body.contains("shl i64"));
    assert!(main_body.contains("ashr i64"));

    assemble_llvm_ir(&llvm_ir, "bitwise_scalar");
}

#[test]
fn codegen_builds_native_executable_for_user_defined_operator_program() {
    let source = r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}

fn main() -> Int {
  return 5 `xoxo` 4 + 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 55);
}

#[test]
fn llvm_codegen_rewrites_known_indirect_calls_to_direct_calls() {
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

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert!(main_body.contains("call i64 @\"step\"(i64 4)"));
    assert!(!main_body.contains("@__skp_rt_call_function_dispatch"));

    assemble_llvm_ir(&llvm_ir, "known_indirect_direct");
}

#[test]
fn llvm_codegen_lowers_qualified_function_calls_directly() {
    let project = common::TempProject::new("qualified_direct_call");
    project.file(
        "math/util.sk",
        r#"
fn step(x: Int) -> Int {
  return x + 1;
}

export { step };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from math.util import step;

fn main() -> Int {
  return math.util.step(4) + step(5);
}
"#,
    );

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    let main_body = llvm_function_body(&llvm_ir, "main");
    assert_eq!(main_body.matches("call i64 @\"math.util::step\"").count(), 2);
    assert!(!main_body.contains("@__skp_rt_call_function_dispatch"));

    assemble_llvm_ir(&llvm_ir, "qualified_direct_call");
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
fn llvm_codegen_folds_hot_path_constant_string_builtins() {
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
fn llvm_codegen_wraps_void_main_with_i32_process_entry() {
    let source = r#"
fn main() -> Void {
  return;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define void @\"__skp_user_main\"()"));
    assert!(llvm_ir.contains("define i32 @\"main\"()"));
    assert!(llvm_ir.contains("call void @\"__skp_user_main\"()"));
    assert!(llvm_ir.contains("ret i32 0"));

    assemble_llvm_ir(&llvm_ir, "void_main_wrapper");
}

#[test]
fn native_void_main_exits_zero() {
    let source = r#"
fn main() -> Void {
  return;
}
"#;

    assert_eq!(common::native_run_exit_code_ok(source), 0);
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
fn llvm_codegen_scalarizes_hot_path_local_int_arrays() {
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
fn llvm_codegen_scalarizes_hot_path_local_float_arrays() {
    let source = r#"
fn main() -> Float {
  let arr: [Float; 8] = [0.0; 8];
  let i = 0;
  while (i < 10) {
    let idx = i % 8;
    arr[idx] = arr[idx] + 1.5;
    i = i + 1;
  }
  return arr[0] + arr[1];
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("%local0_data = alloca [8 x double]"));
    assert!(llvm_ir.contains("getelementptr inbounds [8 x double], ptr %local0_data"));
    assert!(llvm_ir.contains("load double, ptr"));
    assert!(llvm_ir.contains("store double"));

    assemble_llvm_ir(&llvm_ir, "float_array_scalarized");
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
fn llvm_codegen_scalarizes_hot_path_local_struct_field_reads() {
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
  let platform = os.platform();
  if (fs.exists("missing.txt")) {
    return now + str.len(platform);
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

    assert!(llvm_ir.contains("define internal ptr @__skp_rt_fnwrap_0("));
    assert!(llvm_ir.contains("call ptr %"));
    assert!(!llvm_ir.contains("@__skp_rt_call_function_dispatch"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_from_function(ptr)"));
    assert!(llvm_ir.contains("declare ptr @skp_rt_value_to_function(ptr)"));
    assert!(llvm_ir.contains("declare void @skp_rt_value_free(ptr)"));
    assert!(llvm_ir.contains("icmp eq i64 %argc, 1"));
    assert!(llvm_ir.contains("call ptr @skp_rt_call_function(ptr null, i64 %argc, ptr %argv)"));
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
fn llvm_codegen_lowers_bytes_equality_through_runtime_helper() {
    let source = r#"
import bytes;

fn main() -> Int {
  let a: Bytes = bytes.fromString("alpha");
  let b: Bytes = bytes.fromString("alpha");
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("declare i1 @skp_rt_bytes_eq(ptr, ptr)"));
    assert!(llvm_ir.contains("call i1 @skp_rt_bytes_eq(ptr"));
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
    let (exec_name, exec_arg) = if cfg!(windows) {
        ("where.exe", "cmd")
    } else {
        ("which", "sh")
    };

    let source = format!(
        r#"
import fs;
import io;
import os;
import str;
import vec;

fn main() -> Int {{
  fs.writeText("{path_text}", "a");
  fs.appendText("{path_text}", "b");
  let text = fs.readText("{path_text}");
  let joined = fs.join("{joined_left}", "note.txt");
  let platform = os.platform();
  let arch = os.arch();
  let args: Vec[String] = vec.new();
  vec.push(args, "{exec_arg}");
  let out = os.execOut("{exec_name}", args);
  io.print(text);
  io.println("");
  io.print(out);
  io.println("");
  if (fs.exists("{path_text}") && str.len(text) == 2 && str.contains(joined, "note.txt") && str.len(platform) > 0 && str.len(arch) > 0) {{
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
        output.stdout_lossy().lines().count() >= 2,
        "expected exec output, got: {}",
        output.stdout_lossy()
    );
}

#[test]
fn codegen_builds_native_executable_for_bytes_equality() {
    let source = r#"
import bytes;

fn main() -> Int {
  let a: Bytes = bytes.append(bytes.fromString("he"), bytes.fromString("llo"));
  let b: Bytes = bytes.push(bytes.fromString("hell"), 111);
  if (a == b && bytes.get(a, 0) == 104) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

#[test]
fn codegen_builds_native_executable_for_map_builtins() {
    let source = r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  let same = headers;
  map.insert(headers, "content-length", 12);
  let value = map.get(same, "content-length");
  let removed = map.remove(headers, "content-length");
  if (!map.has(same, "content-length") && map.len(headers) == 0) {
    return value + removed;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 24);
}

#[test]
fn codegen_reports_runtime_failure_for_missing_map_key() {
    let source = r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  return map.get(headers, "missing");
}
"#;

    let output = common::native_run_structured(source);
    assert_ne!(output.exit_code(), 0);
    assert!(
        output.stderr_lossy().contains("missing map key"),
        "expected missing-key runtime failure, got: {}",
        output.stderr_lossy()
    );
}

#[test]
fn codegen_builds_native_executable_for_option_values() {
    let source = r#"
fn wrap(x: Int) -> Option[Int] {
  return Some(x);
}

#[test]
fn codegen_builds_native_executable_for_result_values() {
    let source = r#"
fn wrap(x: Int) -> Result[Int, String] {
  return Ok(x);
}

fn fail() -> Result[Int, String] {
  return Err("bad");
}

fn main() -> Int {
  let a: Result[Int, String] = wrap(7);
  let b: Result[Int, String] = Ok(7);
  let c: Result[Int, String] = fail();
  let d: Result[Int, String] = Err("bad");
  if (a == b && c == d && a != c) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

fn missing() -> Option[Int] {
  return None();
}

fn main() -> Int {
  let a: Option[Int] = wrap(7);
  let b: Option[Int] = Some(7);
  let c: Option[Int] = missing();
  if (a == b && a != c && c == None()) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

#[test]
fn codegen_builds_native_executable_for_new_os_builtins() {
    let (exec_name, exec_arg) = if cfg!(windows) {
        ("where.exe", "cmd")
    } else {
        ("which", "sh")
    };
    let source = format!(
        r#"
import os;
import str;
import vec;

fn main() -> Int {{
  let plat = os.platform();
  let arch = os.arch();
  let arg0 = os.arg(0);
  os.envSet("SKEPA_TMP_ENV", "ok");
  let tmp = os.envGet("SKEPA_TMP_ENV");
  let has = os.envHas("SKEPA_TMP_ENV");
  os.envRemove("SKEPA_TMP_ENV");
  let removed = !os.envHas("SKEPA_TMP_ENV");
  let args: Vec[String] = vec.new();
  vec.push(args, "{exec_arg}");
  let code = os.exec("{exec_name}", args);
  let out = os.execOut("{exec_name}", args);
  if (str.len(plat) > 0 && str.len(arch) > 0 && str.len(arg0) > 0 && tmp == "ok" && has && removed && code == 0 && str.len(out) > 0) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    assert_eq!(common::native_run_structured(&source).exit_code(), 0);
}

#[test]
fn codegen_builds_native_executable_for_os_exit() {
    let source = r#"
import os;

fn main() -> Void {
  os.exit(7);
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 7);
}

#[test]
fn codegen_reports_runtime_failure_for_division_by_zero_natively() {
    let source = r#"
fn main() -> Int {
  return 1 / 0;
}
"#;

    let output = common::native_run_structured(source);
    assert_ne!(output.exit_code(), 0);
    assert!(
        output.stderr_lossy().contains("division by zero"),
        "expected division by zero runtime failure, got: {}",
        output.stderr_lossy()
    );
}

#[test]
fn codegen_reports_runtime_failure_for_negative_shift_count_natively() {
    let source = r#"
fn main() -> Int {
  return 1 << -1;
}
"#;

    let output = common::native_run_structured(source);
    assert_ne!(output.exit_code(), 0);
    assert!(
        output.stderr_lossy().contains("negative shift count"),
        "expected negative shift runtime failure, got: {}",
        output.stderr_lossy()
    );
}

#[test]
fn codegen_matches_interpreter_for_large_shift_counts() {
    let source = r#"
fn main() -> Int {
  let a = 1 << 65;
  let b = 8 >> 65;
  return a + b;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 6);
}

#[test]
fn codegen_builds_native_executable_for_minimal_net_listener_builtin() {
    let source = r#"
import net;

fn main() -> Int {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  return 0;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

#[test]
fn codegen_builds_native_executable_for_net_connect_read_write() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read ping");
        assert_eq!(&buf, b"ping");
        stream.write_all(b"pong").expect("write pong");
    });

    let source = format!(
        r#"
import net;
import str;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  net.write(socket, "ping");
  let msg = net.read(socket);
  net.close(socket);
  if (msg == "pong" && str.len(msg) == 4) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_connect_readbytes_writebytes() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read bytes");
        assert_eq!(&buf, &[1_u8, 2, 3, 4]);
        stream.write_all(&[5_u8, 6, 7]).expect("write bytes");
    });

    let source = format!(
        r#"
import net;
import bytes;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  let payload0: Bytes = bytes.fromString("");
  let payload1: Bytes = bytes.push(payload0, 1);
  let payload2: Bytes = bytes.push(payload1, 2);
  let payload3: Bytes = bytes.push(payload2, 3);
  let payload4: Bytes = bytes.push(payload3, 4);
  net.writeBytes(socket, payload4);
  let raw: Bytes = net.readBytes(socket);
  net.close(socket);
  if (bytes.len(raw) == 3 && bytes.get(raw, 0) == 5 && bytes.get(raw, 2) == 7) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_listen_accept_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    drop(listener);

    let source = format!(
        r#"
import net;
import str;

fn main() -> Int {{
  let listener: net.Listener = net.listen("{addr}");
  let socket: net.Socket = net.accept(listener);
  let msg = net.read(socket);
  net.write(socket, "pong");
  net.close(socket);
  net.closeListener(listener);
  if (msg == "ping" && str.len(msg) == 4) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let addr_text = addr.to_string();
    let peer = thread::spawn(move || {
        let mut stream = TcpStream::connect(&addr_text).expect("connect server");
        stream.write_all(b"ping").expect("write ping");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read pong");
        assert_eq!(&buf, b"pong");
    });

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_listen_accept_byte_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    drop(listener);

    let source = format!(
        r#"
import net;
import bytes;

fn main() -> Int {{
  let listener: net.Listener = net.listen("{addr}");
  let socket: net.Socket = net.accept(listener);
  let raw: Bytes = net.readBytes(socket);
  let out0: Bytes = bytes.fromString("");
  let out1: Bytes = bytes.push(out0, 9);
  let out2: Bytes = bytes.push(out1, 8);
  net.writeBytes(socket, out2);
  net.close(socket);
  net.closeListener(listener);
  if (bytes.len(raw) == 4 && bytes.get(raw, 0) == 1 && bytes.get(raw, 3) == 4) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let peer = thread::spawn(move || {
        let mut stream = TcpStream::connect(addr).expect("connect server");
        stream.write_all(&[1_u8, 2, 3, 4]).expect("write bytes");
        let mut buf = [0_u8; 2];
        stream.read_exact(&mut buf).expect("read response bytes");
        assert_eq!(&buf, &[9_u8, 8]);
    });

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_read_n() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        stream.write_all(&[4_u8, 5, 6, 7]).expect("write bytes");
    });

    let source = format!(
        r#"
import net;
import bytes;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  let raw: Bytes = net.readN(socket, 3);
  net.close(socket);
  if (bytes.len(raw) == 3 && bytes.get(raw, 0) == 4 && bytes.get(raw, 2) == 6) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_resolve() {
    let source = r#"
import net;
import str;

fn main() -> Int {
  let ip: String = net.resolve("127.0.0.1");
  if ((ip == "127.0.0.1") && (str.len(ip) == 9)) {
    return 0;
  }
  return 1;
}
"#;

    let result = common::native_run_structured(source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_parse_url() {
    let source = r#"
import net;
import map;
import str;

fn main() -> Int {
  let parts: Map[String, String] = net.parseUrl("https://example.com:8443/api?q=1#frag");
  let scheme: String = map.get(parts, "scheme");
  let host: String = map.get(parts, "host");
  let port: String = map.get(parts, "port");
  let path: String = map.get(parts, "path");
  let query: String = map.get(parts, "query");
  let fragment: String = map.get(parts, "fragment");
  if ((scheme == "https")
      && (host == "example.com")
      && (port == "8443")
      && (path == "/api")
      && (query == "q=1")
      && (fragment == "frag")
      && (str.len(host) == 11)) {
    return 0;
  }
  return 1;
}
"#;

    let result = common::native_run_structured(source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_fetch() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind http listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0_u8; 512];
        let read = stream.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..read]);
        assert!(request.contains("POST /fetch HTTP/1.0"));
        assert!(request.contains("Content-Type: application/json"));
        assert!(request.ends_with("\r\n\r\n{\"ok\":true}"));
        stream
            .write_all(
                b"HTTP/1.0 201 Created\r\nContent-Type: application/json\r\nContent-Length: 12\r\n\r\n{\"ok\":true}",
            )
            .expect("write response");
    });

    let source = format!(
        r#"
import map;
import net;
import str;

fn main() -> Int {{
  let options: Map[String, String] = map.new();
  map.insert(options, "method", "POST");
  map.insert(options, "body", "{{\"ok\":true}}");
  map.insert(options, "contentType", "application/json");
  let response: Map[String, String] = net.fetch("http://{addr}/fetch", options);
  let status: String = map.get(response, "status");
  let body: String = map.get(response, "body");
  let contentType: String = map.get(response, "contentType");
  if ((status == "201") && (body == "{{\"ok\":true}}") && (contentType == "application/json") && (str.len(body) == 11)) {{
    return 0;
  }}
  return 1;
}}
"#
    );

    let result = common::native_run_structured(&source);
    server.join().expect("server thread");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_ffi_open_and_bind() {
    let source = format!(
        r#"
import ffi;

fn main() -> Int {{
  let lib: ffi.Library = ffi.open("{library}");
  let sym: ffi.Symbol = ffi.bind(lib, "{symbol}");
  ffi.closeSymbol(sym);
  ffi.closeLibrary(lib);
  return 0;
}}
"#,
        library = ffi_test_library_path(),
        symbol = ffi_test_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_borrowed_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(s: String) -> Int;
extern("{library}") fn {int_sym}(seed: Int) -> Int;

fn main() -> Int {{
  let a: Int = {sym}("{arg}");
  let b: Int = {int_sym}({int_arg});
  if ((a == {expected}) && (b == {int_expected})) {{
    return 0;
  }}
  return 1;
}}
"#,
        library = ffi_test_library_path(),
        sym = ffi_test_call1_string_int_symbol_name(),
        arg = ffi_test_call1_string_int_value(),
        expected = ffi_test_call1_string_int_expected(),
        int_sym = ffi_test_call1_int_symbol_name(),
        int_arg = ffi_test_call1_int_value(),
        int_expected = ffi_test_call1_int_expected(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(s: String) -> Int;

fn main() -> Int {{
  let value: Int = {sym}("{arg}");
  if (value == {expected}) {{
    return 0;
  }}
  return 1;
}}
"#,
        library = ffi_test_library_path(),
        sym = ffi_test_call1_string_int_symbol_name(),
        arg = ffi_test_call1_string_int_value(),
        expected = ffi_test_call1_string_int_expected(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_void_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(s: String) -> Void;

fn main() -> Int {{
  {sym}("hello");
  return 0;
}}
"#,
        library = ffi_test_call1_string_void_library_path(),
        sym = ffi_test_call1_string_void_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_int_void_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(seed: Int) -> Void;

fn main() -> Int {{
  {sym}(123);
  return 0;
}}
"#,
        library = ffi_test_call1_int_void_library_path(),
        sym = ffi_test_call1_int_void_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_two_string_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(a: String, b: String) -> Int;

fn main() -> Int {{
  let value: Int = {sym}("same", "same");
  if (value == 0) {{
    return 0;
  }}
  return 1;
}}
"#,
        library = ffi_test_library_path(),
        sym = ffi_test_call2_string_int_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_string_int_calls() {
    let source = format!(
        r#"
extern("{library}") fn {sym}(s: String, n: Int) -> Int;

fn main() -> Int {{
  let value: Int = {sym}("hello", 3);
  if (value == 3) {{
    return 0;
  }}
  return 1;
}}
"#,
        library = ffi_test_call2_string_int_int_library_path(),
        sym = ffi_test_call2_string_int_int_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_linked_extern_zero_void_calls() {
    let source = format!(
        r#"
extern("{void_library}") fn {void_sym}() -> Void;

fn main() -> Int {{
  {void_sym}();
  return 0;
}}
"#,
        void_library = ffi_test_call0_void_library_path(),
        void_sym = ffi_test_call0_void_symbol_name(),
    );

    let result = common::native_run_structured(&source);
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_flush() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read ping");
        assert_eq!(&buf, b"ping");
    });

    let source = format!(
        r#"
import net;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  net.write(socket, "ping");
  net.flush(socket);
  net.close(socket);
  return 0;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_builds_native_executable_for_net_timeout_setters() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read ping");
        assert_eq!(&buf, b"ping");
    });

    let source = format!(
        r#"
import net;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  net.setReadTimeout(socket, 25);
  net.setWriteTimeout(socket, 50);
  net.write(socket, "ping");
  net.flush(socket);
  net.setReadTimeout(socket, 0);
  net.setWriteTimeout(socket, 0);
  net.close(socket);
  return 0;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr_lossy());
}

#[test]
fn codegen_reports_runtime_failure_for_non_utf8_net_reads() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        stream
            .write_all(&[0xFF, 0xFE, 0xFD])
            .expect("write invalid utf8");
    });

    let source = format!(
        r#"
import net;

fn main() -> Int {{
  let socket: net.Socket = net.connect("{addr}");
  let _msg = net.read(socket);
  net.close(socket);
  return 0;
}}
"#
    );

    let result = common::native_run_structured(&source);
    peer.join().expect("peer thread should finish");
    assert_ne!(result.exit_code(), 0);
    assert!(
        result.stderr_lossy().contains("valid UTF-8"),
        "expected utf8 runtime failure, got: {}",
        result.stderr_lossy()
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

#[test]
fn codegen_builds_native_executable_for_option_and_result_inspection_helpers() {
    let source = r#"
import option;
import result;

fn main() -> Int {
  let a: Option[Int] = Some(7);
  let b: Option[Int] = None();
  let c: Result[Int, String] = Ok(7);
  let d: Result[Int, String] = Err("bad");
  if (option.isSome(a) && option.isNone(b) && result.isOk(c) && result.isErr(d)) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}

fn object_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

#[test]
fn codegen_builds_native_executable_for_lowercase_option_and_result_constructor_aliases() {
    let source = r#"
fn wrap(x: Int) -> Option[Int] {
  return some(x);
}

fn fail() -> Result[Int, String] {
  return err("bad");
}

fn main() -> Int {
  let a: Option[Int] = wrap(7);
  let b: Option[Int] = some(7);
  let c: Option[Int] = none();
  let d: Result[Int, String] = ok(7);
  let e: Result[Int, String] = fail();
  if (a == b && c == none() && d == ok(7) && e == err("bad")) {
    return 0;
  }
  return 1;
}
"#;

    assert_eq!(common::native_run_structured(source).exit_code(), 0);
}
