#[path = "../common.rs"]
mod common;

use skeplib::sema::{analyze_project_entry, analyze_project_entry_phased};
use std::fs;

fn make_temp_dir(label: &str) -> std::path::PathBuf {
    common::make_temp_dir(label)
}

#[test]
fn sema_project_accepts_cross_file_struct_construction_and_method_call() {
    let root = common::make_temp_dir("struct_method");
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
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_cross_file_function_value_param_and_return() {
    let root = common::make_temp_dir("fn_values");
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
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_function_value_via_qualified_import_namespace_path() {
    let root = common::make_temp_dir("qualified_import_fn_value");
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
import utils.math;
fn main() -> Int {
  let f: Fn(Int, Int) -> Int = utils.math.add;
  return f(1, 2);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_stops_after_parse_errors_without_project_sema_cascades() {
    let root = common::make_temp_dir("project_parse_short_circuit");
    fs::write(
        root.join("main.sk"),
        r#"
fn main() -> Int {
  let x = ;
  return nope;
}
"#,
    )
    .expect("write main");

    let errs = analyze_project_entry_phased(&root.join("main.sk")).expect_err("parse should fail");
    assert!(
        errs.iter()
            .any(|e| e.kind == skeplib::resolver::ResolveErrorKind::Parse)
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Expected expression"))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_preserves_inferred_type_for_imported_unannotated_global() {
    let root = common::make_temp_dir("imported_inferred_global");
    fs::create_dir_all(root.join("config")).expect("create config folder");
    fs::write(
        root.join("config").join("flags.sk"),
        r#"
let enabled = true;
export { enabled };
"#,
    )
    .expect("write module");
    fs::write(
        root.join("main.sk"),
        r#"
from config.flags import enabled;

fn main() -> Int {
  if (enabled) {
    return 1;
  }
  return 0;
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_imported_function_in_array_and_struct_field() {
    let root = common::make_temp_dir("fn_in_array_struct");
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
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_module_qualified_named_type_annotation() {
    let root = common::make_temp_dir("qualified_type_annotation");
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
    common::assert_sema_success(&res, &diags);
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
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown type"))
    );
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
    common::assert_sema_success(&res, &diags);
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
    common::assert_sema_success(&res, &diags);
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
    common::assert_sema_success(&res, &diags);
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
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate imported binding `x`"))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_rejects_namespace_root_from_import() {
    let root = make_temp_dir("sema_namespace_root_from_import");
    fs::create_dir_all(root.join("string")).expect("create string folder");
    fs::write(
        root.join("string").join("trim.sk"),
        r#"
fn trim(s: String) -> String { return s; }
export { trim };
"#,
    )
    .expect("write trim");
    fs::write(
        root.join("string").join("case.sk"),
        r#"
fn up(s: String) -> String { return s; }
export { up };
"#,
    )
    .expect("write case");
    fs::write(
        root.join("main.sk"),
        r#"
from string import trim;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = analyze_project_entry(&root.join("main.sk")).expect_err("resolver error expected");
    assert!(
        errs.iter()
            .any(|e| e.code == "E-MOD-AMBIG" && e.message.contains("namespace root"))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_rejects_missing_re_exported_symbol_import() {
    let root = make_temp_dir("sema_missing_reexported_symbol_import");
    fs::write(
        root.join("a.sk"),
        r#"
fn shown() -> Int { return 1; }
export { shown };
"#,
    )
    .expect("write a");
    fs::write(root.join("b.sk"), "export * from a;\n").expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from b import missing;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = analyze_project_entry(&root.join("main.sk")).expect_err("resolver error expected");
    assert!(errs.iter().any(|e| {
        e.code == "E-IMPORT-NOT-EXPORTED" && e.message.contains("Cannot import `missing`")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_large_multi_module_flow_with_globals_structs_methods_and_builtins() {
    let root = make_temp_dir("large_multi_module_happy_path");
    fs::create_dir_all(root.join("models")).expect("create models");
    fs::create_dir_all(root.join("utils")).expect("create utils");
    fs::create_dir_all(root.join("services")).expect("create services");

    fs::write(
        root.join("models").join("user.sk"),
        r#"
struct User { id: Int, name: String }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
export { User };
"#,
    )
    .expect("write user");
    fs::write(
        root.join("utils").join("math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write math");
    fs::write(
        root.join("services").join("pipeline.sk"),
        r#"
from models.user import User;
from utils.math import add;
import str;

fn run(u: User, bonus: Int) -> Int {
  let label = str.toUpper(u.name);
  if (label == "SAM") {
    return add(u.bump(bonus), 1);
  }
  return 0;
}

export { run };
"#,
    )
    .expect("write pipeline");
    fs::write(
        root.join("main.sk"),
        r#"
from models.user import User;
from services.pipeline import run;

let base: Int = 2;

fn main() -> Int {
  let u: User = User { id: 5, name: "sam" };
  return run(u, base);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_cross_module_method_style_call_on_imported_struct() {
    let root = make_temp_dir("cross_module_method_style_call");
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
  let u: User = User { id: 4 };
  return u.bump(5);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_imported_struct_alias_as_type_annotation() {
    let root = make_temp_dir("imported_struct_alias_type");
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
from models.user import User as AccountUser;
fn main() -> Int {
  let u: AccountUser = AccountUser { id: 4 };
  return u.bump(5);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sema_project_accepts_imported_struct_alias_in_struct_field_and_method_call() {
    let root = make_temp_dir("imported_struct_alias_struct_field");
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
from models.user import User as AccountUser;

struct Holder {
  user: AccountUser
}

fn main() -> Int {
  let h: Holder = Holder { user: AccountUser { id: 7 } };
  return h.user.bump(2);
}
"#,
    )
    .expect("write main");

    let (res, diags) = analyze_project_entry(&root.join("main.sk")).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
    let _ = fs::remove_dir_all(root);
}
