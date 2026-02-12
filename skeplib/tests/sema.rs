use skeplib::sema::analyze_source;

#[test]
fn sema_accepts_valid_program() {
    let src = r#"
import io;

fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let x: Int = add(1, 2);
  if (x > 0) {
    io.println("ok");
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_reports_return_type_mismatch() {
    let src = r#"
fn main() -> Int {
  return true;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Return type mismatch"))
    );
}

#[test]
fn sema_reports_assignment_type_mismatch() {
    let src = r#"
fn main() -> Int {
  let x: Int = 1;
  x = true;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Assignment type mismatch"))
    );
}

#[test]
fn sema_reports_non_bool_condition() {
    let src = r#"
fn main() -> Int {
  if (1) {
    return 0;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("if condition must be Bool"))
    );
}

#[test]
fn sema_reports_function_arity_mismatch() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let x = add(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Arity mismatch"))
    );
}

#[test]
fn sema_requires_import_for_io_calls() {
    let src = r#"
fn main() -> Int {
  io.println("hello");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("without `import io;`"))
    );
}

#[test]
fn sema_reports_unknown_variable() {
    let src = r#"
fn main() -> Int {
  let x = y;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown variable `y`"))
    );
}

#[test]
fn sema_reports_unknown_function() {
    let src = r#"
fn main() -> Int {
  let x = nope(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown function `nope`"))
    );
}

#[test]
fn sema_reports_io_print_type_error() {
    let src = r#"
import io;
fn main() -> Int {
  io.println(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("io.println argument 1 expects String"))
    );
}

#[test]
fn sema_reports_io_readline_arity_error() {
    let src = r#"
import io;
fn main() -> Int {
  let x = io.readLine(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.readLine expects 0 argument(s), got 1")
    }));
}

#[test]
fn sema_allows_shadowing_in_inner_block() {
    let src = r#"
fn main() -> Int {
  let x: Int = 1;
  if (true) {
    let x: Int = 2;
    return x;
  }
  return x;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_reports_let_declared_type_mismatch() {
    let src = r#"
fn main() -> Int {
  let x: Int = "s";
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `x`"))
    );
}

#[test]
fn sema_reports_invalid_binary_operands() {
    let src = r#"
fn main() -> Int {
  let x = true + 1;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Invalid operands for Add"))
    );
}

#[test]
fn sema_reports_invalid_logical_operands() {
    let src = r#"
fn main() -> Int {
  let x = 1 && 2;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Logical operators require Bool operands")
    }));
}

#[test]
fn sema_reports_while_condition_type_error() {
    let src = r#"
fn main() -> Int {
  while (1) {
    return 0;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("while condition must be Bool"))
    );
}

#[test]
fn sema_reports_unknown_io_method() {
    let src = r#"
import io;
fn main() -> Int {
  io.nope("x");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `io.nope`"))
    );
}

#[test]
fn sema_reports_function_argument_type_mismatch() {
    let src = r#"
fn take(x: Int) -> Int {
  return x;
}

fn main() -> Int {
  let y = take("x");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Argument 1 for `take`"))
    );
}

#[test]
fn sema_accepts_readline_as_string_value() {
    let src = r#"
import io;
fn main() -> Int {
  let s: String = io.readLine();
  io.println(s);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_float_arithmetic_and_comparison() {
    let src = r#"
fn main() -> Float {
  let x: Float = 1.5 + 2.5;
  if (x >= 4.0) {
    return x;
  }
  return 0.0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_mixed_int_and_float_operands() {
    let src = r#"
fn main() -> Int {
  let x = 1 + 2.0;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Invalid operands for Add"))
    );
}

#[test]
fn sema_rejects_break_outside_while() {
    let src = r#"
fn main() -> Int {
  break;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("`break` is only allowed inside a while loop")
    }));
}

#[test]
fn sema_rejects_continue_outside_while() {
    let src = r#"
fn main() -> Int {
  continue;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("`continue` is only allowed inside a while loop")
    }));
}

#[test]
fn sema_accepts_break_and_continue_inside_while() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  while (i < 10) {
    if (i == 5) {
      break;
    } else {
      continue;
    }
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}
