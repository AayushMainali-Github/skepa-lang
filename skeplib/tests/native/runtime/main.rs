#[path = "../../common.rs"]
mod common;

use skeplib::ir::IrInterpError;

#[test]
fn native_exec_runs_core_control_flow_program() {
    let src = r#"
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

    assert_eq!(common::native_run_exit_code_ok(src), 15);
    assert_eq!(common::ir_run_ok(src), skepart::value::RtValue::Int(15));
}

#[test]
fn native_exec_runs_for_loop_with_break_and_continue() {
    let src = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 8; i = i + 1) {
    if (i == 2) {
      continue;
    }
    if (i == 6) {
      break;
    }
    acc = acc + (i % 3);
  }
  return acc;
}
"#;

    assert_eq!(common::native_run_exit_code_ok(src), 4);
    assert_eq!(common::ir_run_ok(src), skepart::value::RtValue::Int(4));
}

#[test]
fn ir_interpreter_runs_struct_method_program_for_runtime_validation() {
    let src = r#"
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
"#;

    assert_eq!(common::ir_run_ok(src), skepart::value::RtValue::Int(16));
}

#[test]
fn ir_interpreter_runs_array_and_vec_program_for_runtime_validation() {
    let src = r#"
import option;

fn main() -> Int {
  let arr: [Int; 3] = [1; 3];
  arr[1] = 5;
  let xs: Vec[Int] = vec.new();
  vec.push(xs, arr[0]);
  vec.push(xs, arr[1]);
  return option.unwrapSome(vec.get(xs, 0)) + option.unwrapSome(vec.get(xs, 1)) + arr[2];
}
"#;

    assert_eq!(common::ir_run_ok(src), skepart::value::RtValue::Int(7));
}

#[test]
fn ir_interpreter_runs_function_value_program_for_runtime_validation() {
    let src = r#"
fn inc(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let f: Fn(Int) -> Int = inc;
  return f(4);
}
"#;

    assert_eq!(common::ir_run_ok(src), skepart::value::RtValue::Int(5));
}

#[test]
fn ir_interpreter_reports_division_by_zero_for_runtime_validation() {
    let src = r#"
fn main() -> Int {
  return 8 / 0;
}
"#;

    let err = common::ir_run_err(src);
    assert!(matches!(err, IrInterpError::DivisionByZero));
}

#[test]
fn ir_interpreter_reports_array_oob_for_runtime_validation() {
    let src = r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#;

    let err = common::ir_run_err(src);
    assert!(matches!(err, IrInterpError::IndexOutOfBounds));
}

#[test]
fn ir_interpreter_runs_string_and_float_shapes_for_internal_validation() {
    let string_src = r#"
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
"#;
    let float_src = r#"
fn main() -> Float {
  let x = 1.5;
  let y = 2.0;
  return (x + y) * 2.0;
}
"#;

    assert_eq!(
        common::ir_run_ok(string_src),
        skepart::value::RtValue::String("alpha-ok".into())
    );
    assert_eq!(
        common::ir_run_ok(float_src),
        skepart::value::RtValue::Float(7.0)
    );
}

#[test]
fn native_exec_matches_ir_for_float_compare_codegen() {
    let src = r#"
fn main() -> Int {
  let x = 1.5;
  let y = 2.0;
  if ((x + y) >= 3.5) {
    return 1;
  }
  return 0;
}
"#;

    common::assert_native_matches_ir_value(src, skepart::value::RtValue::Int(1));
}

#[test]
fn native_exec_matches_ir_for_string_equality_compare_codegen() {
    let src = r#"
fn main() -> Int {
  let a = "alpha";
  let b = "alpha";
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    common::assert_native_matches_ir_value(src, skepart::value::RtValue::Int(1));
}

#[test]
fn native_exec_matches_ir_for_global_float_and_string_compare_codegen() {
    let float_src = r#"
let threshold: Float = 3.5;

fn main() -> Int {
  let value = 1.5 + 2.0;
  if (value >= threshold) {
    return 1;
  }
  return 0;
}
"#;
    let string_src = r#"
let expected: String = "alpha";

fn main() -> Int {
  let actual = "alpha";
  let other = "beta";
  if (actual == expected && actual != other) {
    return 1;
  }
  return 0;
}
"#;

    common::assert_native_matches_ir_value(float_src, skepart::value::RtValue::Int(1));
    common::assert_native_matches_ir_value(string_src, skepart::value::RtValue::Int(1));
}
