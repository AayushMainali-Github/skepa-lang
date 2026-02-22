mod common;

use common::{assert_has_diag, sema_err, sema_ok};
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
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
fn sema_accepts_str_builtins() {
    let src = r#"
import str;
fn main() -> Int {
  let s: String = "  skepa-lang  ";
  let t: String = str.trim(s);
  if (str.startsWith(t, "sk") && str.endsWith(t, "lang") && str.contains(t, "epa")) {
    return str.len(t);
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_str_without_import() {
    let src = r#"
fn main() -> Int {
  return str.len("x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`str.*` used without `import str;`"))
    );
}

#[test]
fn sema_rejects_removed_universal_len_function() {
    let src = r#"
fn main() -> Int {
  return len("x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown function `len`"))
    );
}

#[test]
fn sema_accepts_str_case_conversion_builtins() {
    let src = r#"
import str;
fn main() -> Int {
  let a = str.toLower("SkEpA");
  let b = str.toUpper("laNg");
  if (a == "skepa" && b == "LANG") {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_str_case_conversion_type_mismatch() {
    let src = r#"
import str;
fn main() -> Int {
  let _a = str.toLower(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("str.toLower argument 1 expects String"))
    );
}

#[test]
fn sema_accepts_str_indexof_slice_isempty() {
    let src = r#"
import str;
fn main() -> Int {
  let s = "skepa";
  let idx = str.indexOf(s, "ep");
  let cut = str.slice(s, 1, 4);
  if (idx == 2 && cut == "kep" && !str.isEmpty(cut) && str.isEmpty("")) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_str_slice_signature_mismatch() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.slice("abc", "0", 2);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("str.slice argument 2 expects Int"))
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

#[test]
fn sema_accepts_typed_io_print_builtins() {
    let src = r#"
import io;
fn main() -> Int {
  io.printInt(7);
  io.printFloat(1.25);
  io.printBool(true);
  io.printString("ok");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_typed_io_print_type_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  io.printInt("bad");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("io.printInt argument 1 expects Int"))
    );
}

#[test]
fn sema_rejects_io_format_invalid_specifier_literal() {
    let src = r#"
import io;
fn main() -> Int {
  let _s = io.format("bad=%q", 1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.format format error: Unsupported format specifier `%q`")
    }));
}

#[test]
fn sema_rejects_io_printf_trailing_percent_literal() {
    let src = r#"
import io;
fn main() -> Int {
  io.printf("oops %", 1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.printf format error: Format string ends with `%`")
    }));
}

#[test]
fn sema_rejects_io_printf_non_string_format_arg() {
    let src = r#"
import io;
fn main() -> Int {
  io.printf(1, 2);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("io.printf argument 1 expects String"))
    );
}

#[test]
fn sema_rejects_typed_io_print_arity_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  io.printFloat();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("io.printFloat expects 1 argument(s), got 0")
    }));
}

#[test]
fn sema_accepts_arr_package_generic_ops_and_array_add() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 4] = [1, 2, 3, 2];
  let b: [Int; 2] = [9, 8];
  let c = a + b;
  if (arr.len(c) == 6 && !arr.isEmpty(c) && arr.contains(c, 8) && arr.indexOf(c, 2) == 1) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_arr_without_import() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  return arr.len(a);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`arr.*` used without `import arr;`"))
    );
}

#[test]
fn sema_rejects_arr_contains_mismatched_needle_type() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  if (arr.contains(a, "x")) {
    return 1;
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
            .any(|d| d.message.contains("arr.contains argument 2 expects Int"))
    );
}

#[test]
fn sema_rejects_array_add_with_different_element_types() {
    let src = r#"
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  let b: [Float; 2] = [1.0, 2.0];
  let _c = a + b;
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
fn sema_accepts_arr_count_first_last() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 5] = [2, 9, 2, 3, 2];
  let c = arr.count(a, 2);
  let f = arr.first(a);
  let l = arr.last(a);
  if (c == 3 && f == 2 && l == 2) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_arr_count_type_mismatch() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  return arr.count(a, "x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("arr.count argument 2 expects Int"))
    );
}

#[test]
fn sema_accepts_str_lastindexof_and_replace() {
    let src = r#"
import str;
fn main() -> Int {
  let s = "a-b-a-b";
  let i = str.lastIndexOf(s, "a");
  let r = str.replace(s, "-", "_");
  if (i == 4 && r == "a_b_a_b") {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_str_replace_type_mismatch() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.replace("abc", 1, "x");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("str.replace argument 2 expects String"))
    );
}

#[test]
fn sema_accepts_str_repeat() {
    let src = r#"
import str;
fn main() -> Int {
  let s = str.repeat("ab", 3);
  if (s == "ababab") {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_str_repeat_type_mismatch() {
    let src = r#"
import str;
fn main() -> Int {
  let _s = str.repeat("ab", "3");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("str.repeat argument 2 expects Int"))
    );
}

#[test]
fn sema_accepts_arr_join_for_string_arrays() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [String; 3] = ["a", "b", "c"];
  let s = arr.join(a, "-");
  if (s == "a-b-c") { return 1; }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_arr_join_for_non_string_array() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 2] = [1, 2];
  let _s = arr.join(a, ",");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("arr.join argument 1 expects Array[String]")
    }));
}

#[test]
fn sema_accepts_datetime_now_builtins() {
    let src = r#"
import datetime;
fn main() -> Int {
  let s: Int = datetime.nowUnix();
  let ms: Int = datetime.nowMillis();
  if (ms >= s * 1000) {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_datetime_without_import() {
    let src = r#"
fn main() -> Int {
  return datetime.nowUnix();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("`datetime.*` used without `import datetime;`")
    }));
}

#[test]
fn sema_rejects_datetime_nowunix_arity_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  return datetime.nowUnix(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.nowUnix expects 0 argument(s), got 1")
    }));
}

#[test]
fn sema_accepts_datetime_from_unix_and_millis() {
    let src = r#"
import datetime;
fn main() -> Int {
  let a: String = datetime.fromUnix(0);
  let b: String = datetime.fromMillis(1234);
  if (a == "1970-01-01T00:00:00Z" && b == "1970-01-01T00:00:01.234Z") {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_datetime_fromunix_type_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.fromUnix("0");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.fromUnix argument 1 expects Int")
    }));
}

#[test]
fn sema_accepts_datetime_parse_unix() {
    let src = r#"
import datetime;
fn main() -> Int {
  let ts: Int = datetime.parseUnix("1970-01-01T00:00:00Z");
  return ts;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_datetime_parse_unix_type_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  return datetime.parseUnix(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.parseUnix argument 1 expects String")
    }));
}

#[test]
fn sema_accepts_datetime_component_extractors() {
    let src = r#"
import datetime;
fn main() -> Int {
  let ts: Int = 1704112496;
  let y: Int = datetime.year(ts);
  let m: Int = datetime.month(ts);
  let d: Int = datetime.day(ts);
  let h: Int = datetime.hour(ts);
  let mi: Int = datetime.minute(ts);
  let s: Int = datetime.second(ts);
  if (y == 2024 && m == 1 && d == 1 && h == 12 && mi == 34 && s == 56) {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_datetime_year_type_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  return datetime.year("0");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("datetime.year argument 1 expects Int"))
    );
}

#[test]
fn sema_rejects_datetime_frommillis_arity_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  let _x = datetime.fromMillis();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.fromMillis expects 1 argument(s), got 0")
    }));
}

#[test]
fn sema_rejects_datetime_month_arity_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  return datetime.month(1, 2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.month expects 1 argument(s), got 2")
    }));
}

#[test]
fn sema_accepts_random_seed() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed(42);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_random_without_import() {
    let src = r#"
fn main() -> Int {
  random.seed(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("`random.*` used without `import random;`")
    }));
}

#[test]
fn sema_rejects_random_seed_type_mismatch() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed("x");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("random.seed argument 1 expects Int"))
    );
}

#[test]
fn sema_accepts_random_int_and_float() {
    let src = r#"
import random;
fn main() -> Int {
  random.seed(7);
  let i: Int = random.int(10, 20);
  let f: Float = random.float();
  if (i >= 10 && i <= 20 && f >= 0.0 && f < 1.0) {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_minimal_os_builtin_signatures() {
    let src = r#"
import str;
import os;
fn main() -> Int {
  let cwd: String = os.cwd();
  let plat: String = os.platform();
  os.sleep(1);
  let code: Int = os.execShell("echo hi");
  let out: String = os.execShellOut("echo hi");
  if (str.len(cwd) >= 0 && str.len(plat) >= 0 && code >= 0 && str.len(out) >= 0) {
    return 0;
  }
  return 1;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_random_int_arity_mismatch() {
    let src = r#"
import random;
fn main() -> Int {
  return random.int(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("random.int expects 2 argument(s), got 1")
    }));
}

#[test]
fn sema_rejects_random_int_arg2_type_mismatch() {
    let src = r#"
import random;
fn main() -> Int {
  return random.int(1, "2");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("random.int argument 2 expects Int") })
    );
}

#[test]
fn sema_rejects_random_float_arity_mismatch() {
    let src = r#"
import random;
fn main() -> Int {
  let _x = random.float(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("random.float expects 0 argument(s), got 1")
    }));
}

#[test]
fn sema_rejects_datetime_parseunix_arity_mismatch() {
    let src = r#"
import datetime;
fn main() -> Int {
  return datetime.parseUnix();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("datetime.parseUnix expects 1 argument(s), got 0")
    }));
}

#[test]
fn sema_rejects_duplicate_struct_declarations() {
    let src = r#"
struct User { id: Int }
struct User { name: String }
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate struct declaration `User`"))
    );
}

#[test]
fn sema_rejects_duplicate_fields_in_struct() {
    let src = r#"
struct User {
  id: Int,
  id: Int,
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate field `id` in struct `User`"))
    );
}

#[test]
fn sema_rejects_unknown_type_in_struct_field() {
    let src = r#"
struct User {
  profile: Profile,
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown type in struct `User` field `profile`: `Profile`")
    }));
}

#[test]
fn sema_accepts_struct_field_type_referencing_other_struct() {
    let src = r#"
struct Profile { age: Int }
struct User { profile: Profile }
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_unknown_impl_target() {
    let src = r#"
impl User {
  fn id(self) -> Int { return 1; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown impl target struct `User`"))
    );
}

#[test]
fn sema_rejects_duplicate_methods_in_impl_block() {
    let src = r#"
struct User { id: Int }
impl User {
  fn id(self) -> Int { return 1; }
  fn id(self) -> Int { return 2; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Duplicate method `id` in impl `User`") })
    );
}

#[test]
fn sema_rejects_unknown_type_in_method_signature() {
    let src = r#"
struct User { id: Int }
impl User {
  fn setProfile(self, p: Profile) -> Void { return; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown type in method `setProfile` parameter `p`: `Profile`")
    }));
}

#[test]
fn sema_rejects_duplicate_method_across_multiple_impl_blocks() {
    let src = r#"
struct User { id: Int }
impl User {
  fn id(self) -> Int { return self.id; }
}
impl User {
  fn id(self) -> Int { return 0; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate method `id` in impl `User`"))
    );
}

#[test]
fn sema_rejects_method_without_self_first_param() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(x: Int) -> Int { return x; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` must declare `self: User` as first parameter")
    }));
}

#[test]
fn sema_rejects_method_with_non_self_first_param_name() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(this: User) -> Int { return this.id; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` must declare `self: User` as first parameter")
    }));
}

#[test]
fn sema_accepts_struct_literal_field_access_and_field_assignment() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  let u: User = User { id: 7, name: "sam" };
  let v = u.id;
  if (v != 7) {
    return 1;
  }
  u.id = 9;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_struct_literal_unknown_and_missing_fields() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  let _u = User { id: 7, nope: "x" };
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown field `nope` in struct `User` literal")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Missing field `name` in struct `User` literal")
    }));
}

#[test]
fn sema_rejects_struct_literal_field_type_mismatch() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let _u = User { id: "x" };
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Type mismatch for field `id` in struct `User` literal")
    }));
}

#[test]
fn sema_rejects_unknown_field_access_and_assignment() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let u = User { id: 1 };
  let _x = u.nope;
  u.nope = 2;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown field `nope` on struct `User`") })
    );
}

#[test]
fn sema_accepts_struct_method_calls() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int {
    return self.id + x;
  }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add(5);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_unknown_struct_method() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let u = User { id: 7 };
  return u.nope(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown method `nope` on struct `User`") })
    );
}

#[test]
fn sema_rejects_struct_method_arity_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int { return self.id + x; }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Arity mismatch for method `User.add`") })
    );
}

#[test]
fn sema_rejects_struct_method_argument_type_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int { return self.id + x; }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add("x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Argument 1 for method `User.add`") })
    );
}

#[test]
fn sema_rejects_method_call_on_non_struct_value() {
    let src = r#"
fn main() -> Int {
  let x: Int = 1;
  return x.add(2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Method call requires struct receiver") })
    );
}

#[test]
fn sema_rejects_method_return_type_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    return "x";
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Return type mismatch") })
    );
}

#[test]
fn sema_rejects_method_missing_return_for_non_void() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    let x = self.id + 1;
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` may exit without returning")
    }));
}

#[test]
fn sema_rejects_unknown_variable_inside_method_body() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    return nope;
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown variable `nope`") })
    );
}

#[test]
fn sema_accepts_method_using_self_field_in_body() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, delta: Int) -> Int {
    let n: Int = self.id + delta;
    return n;
  }
}
fn main() -> Int {
  let u = User { id: 5 };
  return u.add(3);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_method_call_on_call_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn makeUser(x: Int) -> User {
  return User { id: x };
}
fn main() -> Int {
  return makeUser(9).bump(4);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_method_call_on_index_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn main() -> Int {
  let users: [User; 2] = [User { id: 2 }, User { id: 5 }];
  return users[1].bump(7);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_method_call_on_non_struct_chained_receiver() {
    let src = r#"
fn num() -> Int { return 1; }
fn main() -> Int {
  return num().bump(2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Method call requires struct receiver"))
    );
}

#[test]
fn sema_accepts_global_variable_usage_and_mutation() {
    let src = r#"
let counter: Int = 1;
fn bump() -> Int {
  counter = counter + 1;
  return counter;
}
fn main() -> Int {
  let a = bump();
  let b = bump();
  return a + b;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_duplicate_global_declarations() {
    let src = r#"
let x: Int = 1;
let x: Int = 2;
fn main() -> Int { return x; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Duplicate global variable declaration `x`")
    }));
}

#[test]
fn sema_accepts_global_initialized_from_previous_global() {
    let src = r#"
let a: Int = 2;
let b: Int = a + 3;
fn main() -> Int { return b; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_global_initialized_from_later_global() {
    let src = r#"
let b: Int = a + 1;
let a: Int = 2;
fn main() -> Int { return b; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown variable `a`"))
    );
}

#[test]
fn sema_rejects_exporting_unknown_name() {
    let src = r#"
fn main() -> Int { return 0; }
export { nope };
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Exported name `nope` does not exist in this module")
    }));
}

#[test]
fn sema_rejects_duplicate_export_aliases() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
fn sub(a: Int, b: Int) -> Int { return a - b; }
export { add as calc, sub as calc };
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate exported target name `calc`"))
    );
}

#[test]
fn sema_accepts_call_via_direct_from_import_binding() {
    let src = r#"
from utils.math import add;
fn main() -> Int {
  return add(1, 2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_call_via_qualified_import_namespace() {
    let src = r#"
import utils.math;
fn main() -> Int {
  return utils.math.add(1, 2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_wrong_namespace_level_for_folder_style_import() {
    let src = r#"
import string;
fn main() -> Int {
  return string.toUpper("x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Invalid namespace call `string.toUpper`")
    }));
}
