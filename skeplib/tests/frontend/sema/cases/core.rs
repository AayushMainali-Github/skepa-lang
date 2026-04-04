use super::*;

#[test]
fn sema_accepts_valid_program() {
    let src = r#"
import io;

fn add(a: Int, b: Int) -> Int {
  return a + b;
}

#[test]
fn sema_accepts_bytes_type_in_signatures_and_locals() {
    let src = r#"
fn id(data: Bytes) -> Bytes {
  let copy: Bytes = data;
  return copy;
}

fn main() -> Int {
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_map_type_in_signatures_and_locals() {
    let src = r#"
fn id(data: Map[String, Int]) -> Map[String, Int] {
  let copy: Map[String, Int] = data;
  return copy;
}

fn main() -> Int {
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_option_type_in_signatures_and_locals() {
    let src = r#"
fn id(data: Option[Int]) -> Option[Int] {
  let copy: Option[Int] = data;
  return copy;
}

fn main() -> Int {
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_some_none_values_and_option_equality() {
    let src = r#"
fn wrap(x: Int) -> Option[Int] {
  return Some(x);
}

fn missing() -> Option[Int] {
  return None();
}

fn main() -> Int {
  let a: Option[Int] = wrap(7);
  let b: Option[Int] = Some(7);
  let c: Option[Int] = None();
  if (a == b && a != c && c == missing()) {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_bad_option_constructor_arity() {
    let src = r#"
fn main() -> Int {
  let a: Option[Int] = Some(1, 2);
  let b: Option[Int] = None(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Some expects 1 argument, got 2");
    assert_has_diag(&diags, "None expects 0 arguments, got 1");
}

fn main() -> Int {
  let x: Int = add(1, 2);
  if (x > 0) {
    io.println("ok");
  }
  return 0;
}
"#;
    let _ = sema_ok(src);
}

#[test]
fn sema_reports_return_type_mismatch() {
    let src = r#"
fn main() -> Int {
  return true;
}
"#;
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Return type mismatch");
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
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Assignment type mismatch");
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
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "if condition must be Bool");
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
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Arity mismatch");
}

#[test]
fn sema_wrong_arity_function_call_does_not_invent_concrete_return_type() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let x: Int = add(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Arity mismatch for `add`: expected 2, got 1");
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `x`"))
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
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "without `import io;`");
}

#[test]
fn sema_reports_unknown_variable() {
    let src = r#"
fn main() -> Int {
  let x = y;
  return 0;
}
"#;
    let (result, diags) = sema_err(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Unknown variable `y`");
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
fn sema_accepts_function_typed_param_and_indirect_call() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn apply(f: Fn(Int, Int) -> Int, x: Int, y: Int) -> Int {
  return f(x, y);
}

fn main() -> Int {
  return apply(add, 2, 3);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_stops_after_parse_errors_without_adding_cascaded_semantic_noise() {
    let src = r#"
fn main() -> Int {
  let x = ;
  return nope;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected expression"))
    );
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown variable `nope`"))
    );
}

#[test]
fn sema_accepts_function_typed_local_and_indirect_call() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = add;
  return f(4, 5);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_function_value_call_with_wrong_arity() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = add;
  return f(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Arity mismatch for function value call: expected 2, got 1")
    }));
}

#[test]
fn sema_wrong_arity_function_value_call_does_not_invent_concrete_return_type() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = add;
  let x: Int = f(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(
        &diags,
        "Arity mismatch for function value call: expected 2, got 1",
    );
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `x`"))
    );
}

#[test]
fn sema_accepts_non_capturing_function_literal() {
    let src = r#"
fn main() -> Int {
  let f: Fn(Int) -> Int = fn(x: Int) -> Int {
    return x + 1;
  };
  return f(41);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_capturing_function_literal() {
    let src = r#"
fn main() -> Int {
  let y = 2;
  let f: Fn(Int) -> Int = fn(x: Int) -> Int {
    return x + y;
  };
  return f(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Function literals cannot capture outer variable `y`")
    }));
}

#[test]
fn sema_accepts_immediate_function_literal_call() {
    let src = r#"
fn main() -> Int {
  return (fn(x: Int) -> Int { return x + 1; })(41);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_function_returning_function_literal_and_chained_call() {
    let src = r#"
fn makeInc() -> Fn(Int) -> Int {
  return fn(x: Int) -> Int { return x + 1; };
}

fn main() -> Int {
  return makeInc()(41);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_returned_function_literal_type_mismatch() {
    let src = r#"
fn makeBad() -> Fn(Int) -> Int {
  return fn(x: Int) -> Float { return 1.0; };
}

fn main() -> Int {
  return 0;
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
fn sema_function_literal_allows_calling_named_functions_without_capture() {
    let src = r#"
fn plus1(x: Int) -> Int { return x + 1; }

fn main() -> Int {
  let f: Fn(Int) -> Int = fn(v: Int) -> Int {
    return plus1(v);
  };
  return f(41);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_function_value_equality() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = add;
  if (f == add) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Function values cannot be compared with `==` or `!=`")
    }));
}

#[test]
fn sema_accepts_function_type_inside_struct_field_and_call_via_grouping() {
    let src = r#"
struct Op {
  apply: Fn(Int, Int) -> Int
}

fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let op: Op = Op { apply: add };
  return (op.apply)(20, 22);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_function_type_inside_array_and_returned_array_of_functions() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
fn mul(a: Int, b: Int) -> Int { return a * b; }

fn makeOps() -> [Fn(Int, Int) -> Int; 2] {
  return [add, mul];
}

fn main() -> Int {
  let ops = makeOps();
  return ops[0](2, 3) + ops[1](2, 3);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_method_style_call_on_function_field() {
    let src = r#"
struct Op {
  apply: Fn(Int, Int) -> Int
}

fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let op: Op = Op { apply: add };
  return op.apply(1, 2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown method `apply` on struct `Op`"))
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_bitwise_and_shift_for_ints() {
    let src = r#"
fn main() -> Int {
  let a: Int = ~1;
  let b: Int = 1 & 2;
  let c: Int = 1 | 2;
  let d: Int = 1 ^ 2;
  let e: Int = 1 << 3;
  let f: Int = 8 >> 1;
  return a + b + c + d + e + f;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_bitwise_operators_for_non_ints() {
    let src = r#"
fn main() -> Int {
  let a = ~true;
  let b = 1.0 & 2.0;
  let c = false | true;
  let d = 1 << 2.0;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Unary `~` expects Int");
    assert_has_diag(&diags, "Invalid operands for BitAnd");
    assert_has_diag(&diags, "Invalid operands for BitOr");
    assert_has_diag(&diags, "Invalid operands for Shl");
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
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_match_with_literals_or_and_wildcard() {
    let src = r#"
fn main() -> Int {
  let x: Int = 2;
  match (x) {
    0 | 1 => { return 10; }
    2 => { return 20; }
    _ => { return 30; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_match_on_string_literals() {
    let src = r#"
fn main() -> Int {
  let s: String = "go";
  match (s) {
    "go" => { return 1; }
    "stop" => { return 2; }
    _ => { return 0; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_match_without_wildcard_arm() {
    let src = r#"
fn main() -> Int {
  match (1) {
    1 => { return 1; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Match statement requires a wildcard arm `_`");
}

#[test]
fn sema_rejects_match_wildcard_not_last() {
    let src = r#"
fn main() -> Int {
  match (1) {
    _ => { return 1; }
    1 => { return 2; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Wildcard match arm `_` must be last");
}

#[test]
fn sema_rejects_match_duplicate_wildcard() {
    let src = r#"
fn main() -> Int {
  match (1) {
    1 => { return 1; }
    _ => { return 2; }
    _ => { return 3; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Match statement can contain only one wildcard arm");
}

#[test]
fn sema_rejects_match_pattern_type_mismatch() {
    let src = r#"
fn main() -> Int {
  match (1) {
    "x" => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Match pattern type mismatch");
}

#[test]
fn sema_rejects_match_duplicate_literal_patterns() {
    let src = r#"
fn main() -> Int {
  match (1) {
    1 => { return 1; }
    1 => { return 2; }
    _ => { return 0; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Duplicate match pattern Int literal `1`");
}

#[test]
fn sema_rejects_match_or_pattern_with_wildcard_member() {
    let src = r#"
fn main() -> Int {
  match (1) {
    1 | _ => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Match OR-pattern alternatives must be literals");
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_returns_through_nested_match_inside_loop() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 2; i = i + 1) {
    match (i) {
      0 => { }
      1 => { return 7; }
      _ => { return 0; }
    }
  }
  return 3;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_loop_control_inside_nested_match_and_if_shapes() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 3; i = i + 1) {
    match (i) {
      0 => { continue; }
      1 => {
        if (true) {
          break;
        }
      }
      _ => { }
    }
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_loop_control_in_function_literal_inside_loop() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 1; i = i + 1) {
    let f: Fn() -> Int = fn() -> Int {
      break;
      return 0;
    };
    let g: Fn() -> Int = fn() -> Int {
      continue;
      return 0;
    };
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "`break` is only allowed inside a loop");
    assert_has_diag(&diags, "`continue` is only allowed inside a loop");
}

#[test]
fn sema_accepts_inner_loop_control_inside_function_literal_nested_in_loop() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 1; i = i + 1) {
    let f: Fn() -> Int = fn() -> Int {
      let x = 0;
      while (x < 3) {
        if (x == 0) {
          x = x + 1;
          continue;
        }
        break;
      }
      return x;
    };
    return f();
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_inferred_empty_array_local() {
    let src = r#"
fn main() -> Int {
  let xs = [];
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Cannot infer type of empty array literal");
}

#[test]
fn sema_rejects_inferred_empty_array_global() {
    let src = r#"
let xs = [];
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Cannot infer type of empty array literal");
}

#[test]
fn sema_rejects_vector_value_equality_as_unsupported() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let ys: Vec[Int] = vec.new();
  if (xs == ys) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Vector values cannot be compared with `==` or `!=`");
}

#[test]
fn sema_accepts_higher_order_function_chain_through_multiple_layers() {
    let src = r#"
fn add1(x: Int) -> Int { return x + 1; }

fn wrap(f: Fn(Int) -> Int) -> Fn(Int) -> Int {
  return f;
}

fn makeWrapped() -> Fn(Int) -> Int {
  return wrap(add1);
}

fn applyTwice(f: Fn(Int) -> Int, x: Int) -> Int {
  return f(f(x));
}

fn main() -> Int {
  return applyTwice(makeWrapped(), 40);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_function_value_conversion_to_incompatible_signature() {
    let src = r#"
fn add1(x: Int) -> Int { return x + 1; }

fn main() -> Int {
  let f: Fn(Int) -> Float = add1;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Type mismatch in let `f`");
}

#[test]
fn sema_rejects_passing_incompatible_function_value_to_higher_order_param() {
    let src = r#"
fn takesFloatFn(f: Fn(Float) -> Float) -> Float {
  return f(1.0);
}

fn add1(x: Int) -> Int { return x + 1; }

fn main() -> Int {
  let x = takesFloatFn(add1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Argument 1 for `takesFloatFn`");
}

#[test]
fn sema_accepts_user_defined_operator_expression_in_same_module() {
    let src = r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 2 {
  return lhs + rhs;
}

fn main() -> Int {
  return 5 `xoxo` 4;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_unknown_user_defined_operator_and_invalid_operator_shape() {
    let src = r#"
opr bad(a: Int) -> Int precedence -1 {
  return a;
}

fn main() -> Int {
  return 5 `missing` 4;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "Operator `bad` must declare exactly 2 parameters");
    assert_has_diag(&diags, "Unknown operator `missing`");
}
