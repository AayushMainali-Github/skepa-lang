#[path = "../../common.rs"]
mod common;

mod fixtures;

use skeplib::codegen;
use skeplib::ir;
use skeplib::resolver::ResolveErrorKind;

#[test]
fn native_multi_module_program_executes_correctly() {
    let project = common::TempProject::new("multi_module");
    project.file(
        "utils/math.sk",
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from utils.math import add;
fn main() -> Int { return add(20, 22); }
"#,
    );

    let output = common::native_run_project_ok(&entry);
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn duplicate_symbol_names_in_different_modules_do_not_collide_in_native_codegen() {
    let project = common::TempProject::new("duplicate_names");
    project.file(
        "a/mod.sk",
        r#"
fn id() -> Int { return 7; }
export { id };
"#,
    );
    project.file(
        "b/mod.sk",
        r#"
fn id() -> Int { return 9; }
export { id };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from a.mod import id as aid;
from b.mod import id as bid;
fn main() -> Int { return aid() * 10 + bid(); }
"#,
    );

    let program = common::compile_project_ir_ok(&entry);
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");
    assert!(llvm_ir.contains("define i64 @\"a.mod::id\"()"));
    assert!(llvm_ir.contains("define i64 @\"b.mod::id\"()"));

    let output = common::native_run_project_ok(&entry);
    assert_eq!(output.status.code(), Some(79));
}

#[test]
fn explicit_import_from_reexported_module_executes_natively() {
    let project = common::TempProject::new("explicit_from_reexport");
    project.file(
        "a.sk",
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    );
    project.file("b.sk", "export * from a;\n");
    let entry = project.file(
        "main.sk",
        r#"
from b import add;
fn main() -> Int { return add(40, 2); }
"#,
    );

    let output = common::native_run_project_ok(&entry);
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn imported_operator_callable_executes_natively() {
    let project = common::TempProject::new("imported_operator_callable");
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
fn main() -> Int { return xoxo(4, 2); }
"#,
    );

    let output = common::native_run_project_ok(&entry);
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn project_compile_failure_is_reported_as_parse_error_for_native_path() {
    let project = common::TempProject::new("codegen_error_kind");
    let entry = project.file(
        "main.sk",
        r#"
fn main( -> Int { return 0; }
"#,
    );

    let errs = ir::lowering::compile_project_entry(&entry).expect_err("expected failure");
    assert!(
        errs.iter()
            .any(|e| { e.kind == ResolveErrorKind::Parse && e.code == "E-PARSE" })
    );
}
