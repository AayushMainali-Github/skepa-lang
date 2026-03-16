mod common;

use skeplib::codegen;
use skeplib::ir;
use skeplib::resolver::ResolveErrorKind;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skepa_native_project_{label}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn native_multi_module_program_executes_correctly() {
    let root = make_temp_dir("multi_module");
    fs::create_dir_all(root.join("utils")).expect("create folder");
    fs::write(
        root.join("utils").join("math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write util");
    fs::write(
        root.join("main.sk"),
        r#"
from utils.math import add;
fn main() -> Int { return add(20, 22); }
"#,
    )
    .expect("write main");

    let output = common::native_run_project_ok(&root.join("main.sk"));
    assert_eq!(output.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn duplicate_symbol_names_in_different_modules_do_not_collide_in_native_codegen() {
    let root = make_temp_dir("duplicate_names");
    fs::create_dir_all(root.join("a")).expect("create a");
    fs::create_dir_all(root.join("b")).expect("create b");
    fs::write(
        root.join("a").join("mod.sk"),
        r#"
fn id() -> Int { return 7; }
export { id };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b").join("mod.sk"),
        r#"
fn id() -> Int { return 9; }
export { id };
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from a.mod import id as aid;
from b.mod import id as bid;
fn main() -> Int { return aid() * 10 + bid(); }
"#,
    )
    .expect("write main");

    let program = common::compile_project_ir_ok(&root.join("main.sk"));
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");
    assert!(llvm_ir.contains("define i64 @\"a.mod::id\"()"));
    assert!(llvm_ir.contains("define i64 @\"b.mod::id\"()"));

    let output = common::native_run_project_ok(&root.join("main.sk"));
    assert_eq!(output.status.code(), Some(79));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn explicit_import_from_reexported_module_executes_natively() {
    let root = make_temp_dir("explicit_from_reexport");
    fs::write(
        root.join("a.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write a");
    fs::write(root.join("b.sk"), "export * from a;\n").expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from b import add;
fn main() -> Int { return add(40, 2); }
"#,
    )
    .expect("write main");

    let output = common::native_run_project_ok(&root.join("main.sk"));
    assert_eq!(output.status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_compile_failure_is_reported_as_codegen_error_for_native_path() {
    let root = make_temp_dir("codegen_error_kind");
    fs::write(
        root.join("main.sk"),
        r#"
fn main( -> Int { return 0; }
"#,
    )
    .expect("write malformed main");

    let errs =
        ir::lowering::compile_project_entry(&root.join("main.sk")).expect_err("expected failure");
    assert!(
        errs.iter()
            .any(|e| { e.kind == ResolveErrorKind::Codegen && e.code == "E-CODEGEN" })
    );
    let _ = fs::remove_dir_all(root);
}
