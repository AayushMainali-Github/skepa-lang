mod common;

use skeplib::{codegen, ir};

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
