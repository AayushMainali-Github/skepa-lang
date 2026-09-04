mod common;

use skeplib::{codegen, ir};

#[test]
fn llvm_codegen_emits_function_valued_global_initializer() {
    let source = r#"
fn inc(x: Int) -> Int { return x + 1; }
let f: Fn(Int) -> Int = inc;
fn main() -> Int { return f(41); }
"#;
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir = codegen::compile_program_to_llvm_ir(&program)
        .expect("function-valued global should lower to LLVM");
    assert!(llvm_ir.contains("@g0 = global ptr null"));
    assert_eq!(common::native_run_exit_code_ok(source), 42);
}

#[test]
fn native_string_builtins_execute_through_runtime_dispatch() {
    let source = r#"
import str;

fn main() -> Int {
  let value = "  SkEpA  ";
  if (!str.startsWith(value, "  Sk") || !str.endsWith(value, "A  ")) { return 1; }
  if (str.trim(value) != "SkEpA") { return 2; }
  if (str.toLower(value) != "  skepa  " || str.toUpper(value) != "  SKEPA  ") { return 3; }
  if (str.lastIndexOf("abca", "a") != 3) { return 4; }
  if (str.replace("a-b-b", "b", "x") != "a-x-x") { return 5; }
  if (str.repeat("ab", 3) != "ababab") { return 6; }
  if (!str.isEmpty("")) { return 7; }
  return 0;
}
"#;

    assert_eq!(common::native_run_exit_code_ok(source), 0);
}

#[test]
fn native_call_resolution_keeps_branch_dependent_function_values_dynamic() {
    let source = r#"
fn inc(x: Int) -> Int { return x + 1; }
fn dec(x: Int) -> Int { return x - 1; }

fn main() -> Int {
  let f: Fn(Int) -> Int = inc;
  if (false) {
    f = dec;
  }
  return f(10);
}
"#;

    assert_eq!(common::native_run_exit_code_ok(source), 11);
}

#[test]
fn llvm_codegen_uses_imported_operator_symbol_for_project_infix_calls() {
    let project = common::TempProject::new("codegen_imported_operator_symbol");
    project.file(
        "ops/math.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}
export { xoxo };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from ops.math import xoxo;
fn main() -> Int {
  return 4 `xoxo` 2;
}
"#,
    );

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @\"ops.math::xoxo\""));
    assert!(llvm_ir.contains("call i64 @\"ops.math::xoxo\""));
    assert!(!llvm_ir.contains("@\"main::xoxo\""));
}

#[test]
fn llvm_codegen_preserves_extern_bind_failure_cleanup_path() {
    let source = r#"
extern("test-lib") fn strlen(s: String) -> Int;

fn main() -> Int {
  return strlen("abc");
}
"#;

    let program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("extern_bind_err"));
    assert!(llvm_ir.contains("extern_bind_ok"));
    assert!(llvm_ir.contains("@skp_rt_call_builtin"));
    assert!(llvm_ir.contains("closeLibrary"));
    assert!(llvm_ir.contains("closeSymbol"));
}

#[test]
fn llvm_codegen_uses_option_aware_vec_get_runtime_helper() {
    let source = r#"
import option;
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 5);
  return option.unwrapSome(vec.get(xs, 0));
}
"#;

    let program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("call ptr @skp_rt_vec_get_option"));
}

#[test]
fn native_container_results_free_owned_boxed_values() {
    let source = r#"
import option;
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 7);
  let first = option.unwrapSome(vec.get(xs, 0));
  let removed = vec.delete(xs, 0);
  return first + removed;
}
"#;

    let program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("call ptr @skp_rt_vec_get_option"));
    assert!(llvm_ir.contains("call ptr @skp_rt_vec_delete"));
    assert!(llvm_ir.matches("call void @skp_rt_value_free").count() >= 2);
    assert_eq!(common::native_run_exit_code_ok(source), 14);
}

#[test]
fn llvm_codegen_lowers_str_slice_via_generic_result_dispatch() {
    let source = r#"
import result;
import str;

fn main() -> Int {
  let cut = result.unwrapOk(str.slice("skepa-language-runtime", 6, 14));
  return str.len(cut);
}
"#;

    let program =
        ir::lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("call ptr @skp_rt_call_builtin("));
    assert!(!llvm_ir.contains("call ptr @skp_rt_builtin_str_slice("));
}

#[test]
fn llvm_codegen_emits_unordered_fcmp_for_float_inequality() {
    let source = r#"
fn main() -> Int {
  let x = 1.5;
  let y = 2.0;
  if (x != y) {
    return 1;
  }
  return 0;
}

"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(
        llvm_ir.contains("fcmp une double"),
        "expected unordered float != lowering, got:\n{llvm_ir}"
    );
    assert!(
        !llvm_ir.contains("fcmp one double"),
        "ordered fcmp one must not be used for float !="
    );
}

#[test]
fn inlined_method_field_mutation_keeps_struct_pointer_valid() {
    let source = r#"
struct Counter {
  n: Int
}

impl Counter {
  fn bump(self) -> Int {
    self.n = self.n + 1;
    return self.n;
  }
}

fn main() -> Int {
  let c: Counter = Counter { n: 0 };
  return c.bump();
}
"#;

    assert_eq!(common::native_run_exit_code_ok(source), 1);
}
