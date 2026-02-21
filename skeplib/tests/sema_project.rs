use skeplib::sema::analyze_project_entry;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skepa_sema_project_{label}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn sema_project_accepts_cross_file_struct_construction_and_method_call() {
    let root = make_temp_dir("struct_method");
    fs::create_dir_all(root.join("models")).expect("create models folder");
    fs::write(
        root.join("models").join("user.sk"),
        r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
export { User };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
from models.user import User;
fn main() -> Int {
  let u: User = User { id: 5 };
  return u.bump(7);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_cross_file_function_value_param_and_return() {
    let root = make_temp_dir("fn_values");
    fs::create_dir_all(root.join("utils")).expect("create utils folder");
    fs::write(
        root.join("utils").join("math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
from utils.math import add;
fn apply(f: Fn(Int, Int) -> Int, x: Int, y: Int) -> Int {
  return f(x, y);
}
fn make() -> Fn(Int, Int) -> Int { return add; }
fn main() -> Int {
  let f = make();
  return apply(f, 20, 22);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_imported_function_in_array_and_struct_field() {
    let root = make_temp_dir("fn_in_array_struct");
    fs::create_dir_all(root.join("utils")).expect("create utils folder");
    fs::write(
        root.join("utils").join("math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
from utils.math import add;

struct Op {
  f: Fn(Int, Int) -> Int
}

fn main() -> Int {
  let arr: [Fn(Int, Int) -> Int; 1] = [add];
  let op: Op = Op { f: add };
  return arr[0](10, 11) + (op.f)(20, 1);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_module_qualified_named_type_annotation() {
    let root = make_temp_dir("qualified_type_annotation");
    fs::create_dir_all(root.join("models")).expect("create models folder");
    fs::write(
        root.join("models").join("user.sk"),
        r#"
struct User { id: Int }
impl User { fn bump(self, d: Int) -> Int { return self.id + d; } }
export { User };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
import models.user;
fn read(u: models.user.User) -> Int { return u.bump(2); }
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_rejects_unimported_module_qualified_type_annotation() {
    let root = make_temp_dir("unimported_qualified_type_annotation");
    fs::create_dir_all(root.join("models")).expect("create models folder");
    fs::write(
        root.join("models").join("user.sk"),
        r#"
struct User { id: Int }
export { User };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
fn read(u: models.user.User) -> Int { return 0; }
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(res.has_errors);
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Unknown type")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_impl_for_imported_struct_and_uses_method() {
    let root = make_temp_dir("impl_imported_struct");
    fs::create_dir_all(root.join("models")).expect("create models folder");
    fs::write(
        root.join("models").join("user.sk"),
        r#"
struct User { id: Int }
export { User };
"#,
    )
    .expect("write user module");
    fs::write(
        root.join("ext.sk"),
        r#"
from models.user import User;
impl User {
  fn extra(self, d: Int) -> Int { return self.id + d; }
}
fn run(u: User) -> Int { return u.extra(3); }
export { run };
"#,
    )
    .expect("write ext module");
    fs::write(
        root.join("main.sk"),
        r#"
from models.user import User;
from ext import run;
fn main() -> Int {
  let u: User = User { id: 9 };
  return run(u);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_wildcard_import_through_re_export() {
    let root = make_temp_dir("wildcard_reexport");
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
from b import *;
fn main() -> Int { return add(20, 22); }
"#,
    )
    .expect("write main");
    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_direct_re_export_alias_usage() {
    let root = make_temp_dir("direct_reexport_alias_usage");
    fs::write(
        root.join("a.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write a");
    fs::write(root.join("b.sk"), "export { add as plus } from a;\n").expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from b import plus;
fn main() -> Int { return plus(1, 2); }
"#,
    )
    .expect("write main");
    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    assert!(!res.has_errors, "diagnostics: {:?}", diags.as_slice());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_rejects_wildcard_and_explicit_import_binding_conflict() {
    let root = make_temp_dir("wildcard_explicit_conflict");
    fs::write(
        root.join("a.sk"),
        r#"
fn x() -> Int { return 1; }
export { x };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
fn y() -> Int { return 2; }
export { y };
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from a import *;
from b import y as x;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = analyze_project_entry(&root.join("main.sk")).expect_err("resolver error expected");
    assert!(errs
        .iter()
        .any(|e| e.message.contains("Duplicate imported binding `x`")));
    let _ = fs::remove_dir_all(root);
}
