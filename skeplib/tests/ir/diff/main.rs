use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use skepart::{RtErrorKind, RtValue};
use skeplib::ir::{self, IrInterpreter, IrValue};

#[path = "../../common.rs"]
mod common;

fn assert_native_and_ir_accept_same_int_source(source: &str, expected: i32) {
    common::assert_native_matches_ir_value(source, RtValue::Int(i64::from(expected)));
}

fn assert_native_and_ir_accept_same_source(source: &str, expected: IrValue) {
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, expected);
}

fn assert_ir_rejects_source(source: &str, expected: RtErrorKind) {
    common::assert_native_matches_ir_error_kind(source, expected);
}

#[test]
fn native_and_ir_accept_same_core_control_flow_source() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 6) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;
    assert_native_and_ir_accept_same_int_source(source, 15);
}

#[test]
fn native_and_ir_accept_same_for_loop_source() {
    let source = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 8; i = i + 1) {
    if (i == 2) {
      continue;
    }
    if (i == 6) {
      break;
    }
    acc = acc + i;
  }
  return acc;
}
"#;
    assert_native_and_ir_accept_same_int_source(source, 13);
}

#[test]
fn native_and_ir_accept_same_bool_and_string_semantics() {
    assert_native_and_ir_accept_same_source(
        r#"
fn main() -> Bool {
  let a = true;
  let b = false;
  return (a && b) || !b;
}
"#,
        IrValue::Bool(true),
    );
    assert_native_and_ir_accept_same_source(
        r#"
import result;
import str;

fn main() -> String {
  let s = "alpha-beta";
  let cut = result.unwrapOk(str.slice(s, 0, 5));
  if (str.contains(s, "beta")) {
    return cut + "-ok";
  }
  return "bad";
}
"#,
        IrValue::String("alpha-ok".into()),
    );
}

#[test]
fn native_and_ir_accept_same_array_vec_struct_method_and_builtin_sources() {
    assert_native_and_ir_accept_same_int_source(
        r#"
import option;
import vec;

fn main() -> Int {
  let arr: [Int; 3] = [1; 3];
  arr[1] = 5;
  let xs: Vec[Int] = vec.new();
  vec.push(xs, arr[0]);
  vec.push(xs, arr[1]);
  return option.unwrapSome(vec.get(xs, 0)) + option.unwrapSome(vec.get(xs, 1)) + arr[2];
}
"#,
        7,
    );
    assert_native_and_ir_accept_same_int_source(
        r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn main() -> Int {
  let p = Pair { a: 10, b: 5 };
  p.a = 7;
  return p.mix(4);
}
"#,
        16,
    );
    assert_native_and_ir_accept_same_int_source(
        r#"
import datetime;
import str;
import result;

fn main() -> Int {
  let s = "skepa-language-runtime";
  let total = 0;
  total = total + str.len(s);
  total = total + str.indexOf(s, "time");
  let cut = result.unwrapOk(str.slice(s, 6, 14));
  if (str.contains(cut, "language")) {
    return total + 1;
  }
  return datetime.nowMillis();
}
"#,
        41,
    );
}

#[test]
fn native_and_ir_accept_same_io_and_datetime_behaviour() {
    let source = r#"
import datetime;
import io;

fn main() -> Int {
  io.print("alpha");
  io.printInt(7);
  io.println("");
  if (datetime.nowUnix() >= 0) {
    return 3;
  }
  return 0;
}
"#;

    let native = common::native_run_ok(source);
    let native_out = String::from_utf8_lossy(&native.stdout).replace("\r\n", "\n");
    assert_eq!(native_out, "alpha7\n");

    let host = common::DeterministicHost::default();
    let captured = host.captured_output();
    let value = common::ir_run_ok_with_host(source, Box::new(host));
    assert_eq!(value, IrValue::Int(3));
    assert_eq!(
        &*captured.lock().expect("lock deterministic host output"),
        "alpha7\n"
    );
}

#[test]
fn native_and_ir_accept_same_arr_fs_and_struct_project_sources() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_diff_project_mix_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("pair.sk"),
        r#"
export { Pair, make };

struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn total(self) -> Int {
    return self.a + self.b;
  }
}

fn make() -> Pair {
  return Pair { a: 4, b: 5 };
}
"#,
    )
    .expect("pair module should be written");
    fs::write(
        &entry,
        r#"
from pair import Pair, make;

fn main() -> Int {
  let xs: [Int; 2] = [3; 2];
  let p = make();
  xs[1] = p.total();
  return xs[0] + xs[1];
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(12));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 12);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn native_and_ir_accept_same_project_sources() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_diff_project_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("pair.sk"),
        r#"
export { Pair, make, base };

let base: String = "skepa-language-runtime";

struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn make() -> Pair {
  return Pair { a: 4, b: 6 };
}
"#,
    )
    .expect("pair module should be written");
    fs::write(
        &entry,
        r#"
import str;
from pair import Pair, make, base;

fn main() -> Int {
  let p = make();
  return p.mix(str.indexOf(base, "time"));
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(28));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 28);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn native_and_ir_accept_same_namespace_folder_import_project_source() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_diff_namespace_folder_{unique}"));
    fs::create_dir_all(root.join("string/nested")).expect("temp project dirs should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("string/case.sk"),
        r#"
fn up(s: String) -> String { return s; }
export { up };
"#,
    )
    .expect("case module should be written");
    fs::write(
        root.join("string/nested/trim.sk"),
        r#"
fn trim(s: String) -> String { return s; }
export { trim };
"#,
    )
    .expect("trim module should be written");
    fs::write(
        &entry,
        r#"
import string;
fn main() -> Int {
  let a = string.case.up("x");
  let b = string.nested.trim("y");
  if (a == "x" && b == "y") { return 0; }
  return 1;
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(0));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn native_and_ir_accept_same_namespace_aliased_struct_literal_project_source() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_diff_namespace_struct_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("models.sk"),
        r#"
struct User { id: Int }
export { User };
"#,
    )
    .expect("models module should be written");
    fs::write(
        &entry,
        r#"
import models as m;
fn main() -> Int {
  let u = m.User { id: 7 };
  return u.id;
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(7));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 7);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn native_and_ir_accept_same_qualified_namespace_function_value_project_source() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_diff_namespace_fn_value_{unique}"));
    fs::create_dir_all(root.join("utils")).expect("temp project dirs should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("utils/math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("math module should be written");
    fs::write(
        &entry,
        r#"
import utils.math;
fn main() -> Int {
  let f: Fn(Int, Int) -> Int = utils.math.add;
  return f(1, 2);
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(3));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 3);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn native_and_ir_accept_vec_subscript_indexing() {
    let source = r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 10);
  vec.push(xs, 20);
  return xs[0] + xs[1];
}
"#;
    assert_native_and_ir_accept_same_int_source(source, 30);
}

#[test]
fn ir_rejects_runtime_error_sources() {
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  return 8 / 0;
}
"#,
        RtErrorKind::DivisionByZero,
    );
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#,
        RtErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  return xs[0];
}
"#,
        RtErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import str;
import result;

fn main() -> String {
  return result.unwrapOk(str.slice("abc", 0, 99));
}
"#,
        RtErrorKind::InvalidArgument,
    );
}

#[test]
fn native_and_ir_accept_same_global_ident_assignment() {
    let source = r#"
let counter: Int = 1;
fn bump() -> Int {
  counter = counter + 1;
  return counter;
}
fn main() -> Int {
  return bump();
}
"#;
    assert_native_and_ir_accept_same_int_source(source, 2);
}
