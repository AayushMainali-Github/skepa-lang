use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use skepart::{RtHost, RtResult, RtString};
use skeplib::ir::{self, IrInterpError, IrInterpreter, IrValue};

#[path = "common.rs"]
mod common;

struct DiffHost {
    out: Rc<RefCell<String>>,
}

impl Default for DiffHost {
    fn default() -> Self {
        Self {
            out: Rc::new(RefCell::new(String::new())),
        }
    }
}

impl RtHost for DiffHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        self.out.borrow_mut().push_str(text);
        Ok(())
    }

    fn datetime_now_unix(&mut self) -> RtResult<i64> {
        Ok(123)
    }

    fn datetime_now_millis(&mut self) -> RtResult<i64> {
        Ok(456_789)
    }

    fn fs_exists(&mut self, path: &str) -> RtResult<bool> {
        Ok(path == "exists.txt")
    }

    fn fs_read_text(&mut self, path: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("read:{path}")))
    }

    fn fs_join(&mut self, left: &str, right: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("{left}/{right}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedErrorKind {
    DivisionByZero,
    IndexOutOfBounds,
    TypeMismatch,
}

fn assert_native_and_ir_accept_same_int_source(source: &str, expected: i32) {
    let code = common::native_run_exit_code_ok(source);
    assert_eq!(code, expected);
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(i64::from(expected)));
}

fn assert_native_and_ir_accept_same_source(source: &str, expected: IrValue) {
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, expected);
}

fn assert_ir_rejects_source(source: &str, expected: ExpectedErrorKind) {
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let ir_err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("IR interpreter should fail");
    let ir_kind = match ir_err {
        IrInterpError::DivisionByZero => ExpectedErrorKind::DivisionByZero,
        IrInterpError::IndexOutOfBounds => ExpectedErrorKind::IndexOutOfBounds,
        IrInterpError::TypeMismatch(_) => ExpectedErrorKind::TypeMismatch,
        other => panic!("unexpected IR error kind in comparison test: {other:?}"),
    };
    assert_eq!(ir_kind, expected);
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
fn main() -> String {
  let s = "alpha-beta";
  let cut = str.slice(s, 0, 5);
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
fn main() -> Int {
  let arr: [Int; 3] = [1; 3];
  arr[1] = 5;
  let xs: Vec[Int] = vec.new();
  vec.push(xs, arr[0]);
  vec.push(xs, arr[1]);
  return vec.get(xs, 0) + vec.get(xs, 1) + arr[2];
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

fn main() -> Int {
  let s = "skepa-language-benchmark";
  let total = 0;
  total = total + str.len(s);
  total = total + str.indexOf(s, "bench");
  let cut = str.slice(s, 6, 14);
  if (str.contains(cut, "language")) {
    return total + 1;
  }
  return datetime.nowMillis();
}
"#,
        40,
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

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let host = DiffHost::default();
    let captured = host.out.clone();
    let value = IrInterpreter::with_host(&program, Box::new(host))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(3));
    assert_eq!(&*captured.borrow(), "alpha7\n");
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
from pair import make;

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

let base: String = "skepa-language-benchmark";

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
from pair import make, base;

fn main() -> Int {
  let p = make();
  return p.mix(str.indexOf(base, "bench"));
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(25));
    assert_eq!(common::native_run_project_exit_code_ok(&entry), 25);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ir_rejects_runtime_error_sources() {
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  return 8 / 0;
}
"#,
        ExpectedErrorKind::DivisionByZero,
    );
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import str;

fn main() -> String {
  return str.slice("abc", 0, 99);
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
}
