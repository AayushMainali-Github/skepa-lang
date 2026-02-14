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
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("`break` is only allowed inside a loop") })
    );
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
            .contains("`continue` is only allowed inside a loop")
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

#[test]
fn sema_accepts_int_modulo() {
    let src = r#"
fn main() -> Int {
  let x: Int = 9 % 4;
  return x;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_float_modulo() {
    let src = r#"
fn main() -> Int {
  let x = 9.0 % 4.0;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Invalid operands for Mod"))
    );
}

#[test]
fn sema_accepts_unary_plus_for_numeric() {
    let src = r#"
fn main() -> Int {
  let a: Int = +1;
  let b: Float = +2.5;
  return a;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_unary_plus_for_bool() {
    let src = r#"
fn main() -> Int {
  let x = +true;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unary `+` expects Int or Float"))
    );
}

#[test]
fn sema_rejects_missing_return_for_non_void_function() {
    let src = r#"
fn main() -> Int {
  let x = 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("may exit without returning"))
    );
}

#[test]
fn sema_accepts_if_else_when_both_paths_return() {
    let src = r#"
fn main() -> Int {
  if (true) {
    return 1;
  } else {
    return 2;
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_for_with_break_and_continue() {
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
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_reports_non_bool_for_condition() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; 1; i = i + 1) {
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
            .any(|d| d.message.contains("for condition must be Bool"))
    );
}

#[test]
fn sema_for_init_scope_does_not_escape_loop() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 2; i = i + 1) {
  }
  return i;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown variable `i`"))
    );
}

#[test]
fn sema_allows_shadowing_inside_for_loop_body() {
    let src = r#"
fn main() -> Int {
  let x: Int = 10;
  for (let i = 0; i < 1; i = i + 1) {
    let x: Int = 20;
    if (x == 20) {
      continue;
    }
  }
  return x;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_static_arrays_and_indexing() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 3] = [1, 2, 3];
  let x: Int = a[1];
  a[2] = x;
  return a[0] + a[1] + a[2];
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_array_literal_element_type_mismatch() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, true];
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("Array literal element type mismatch")
            || d.message.contains("Type mismatch in let `a`")
    }));
}

#[test]
fn sema_rejects_array_size_mismatch() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2, 3];
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `a`"))
    );
}

#[test]
fn sema_rejects_non_int_array_index() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  let x = a[true];
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Array index must be Int"))
    );
}

#[test]
fn sema_rejects_index_assignment_type_mismatch() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  a[0] = true;
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
fn sema_accepts_len_for_string_and_array() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 3] = [1, 2, 3];
  let s: String = "abcd";
  return len(a) + len(s);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_len_for_non_collection() {
    let src = r#"
fn main() -> Int {
  return len(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("len expects String or Array argument"))
    );
}

#[test]
fn sema_rejects_len_wrong_arity() {
    let src = r#"
fn main() -> Int {
  return len();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("len expects 1 argument"))
    );
}

#[test]
fn sema_accepts_multidimensional_arrays_any_depth() {
    let src = r#"
fn main() -> Int {
  let t: [[[Int; 2]; 2]; 2] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
  let q: [[[[Int; 2]; 1]; 1]; 1] = [[[[1, 2]]]];
  t[1][1][0] = q[0][0][0][1];
  return t[1][1][0];
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_io_format_and_printf_with_matching_specs() {
    let src = r#"
import io;
fn main() -> Int {
  let s = io.format("n=%d f=%f ok=%b name=%s %%\n", 7, 2.5, true, "sam");
  io.printf("%s\t%s\\", s, "done");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_io_format_type_mismatch_from_literal_spec() {
    let src = r#"
import io;
fn main() -> Int {
  let s = io.format("x=%d", "bad");
  io.println(s);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.format argument 2 expects Int for `%d`")
    }));
}

#[test]
fn sema_rejects_io_printf_arity_mismatch_from_literal_spec() {
    let src = r#"
import io;
fn main() -> Int {
  io.printf("x=%d y=%d", 1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.printf format expects 2 value argument(s), got 1")
    }));
}
