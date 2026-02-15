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
  if (arr.len(c) == 6 && !arr.isEmpty(c) && arr.contains(c, 8) && arr.indexOf(c, 2) == 1 && arr.sum(c) == 25) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_arr_sum_for_nested_arrays() {
    let src = r#"
import arr;
fn main() -> Int {
  let rows: [[Int; 2]; 3] = [[1, 2], [3, 4], [5, 6]];
  let flat = arr.sum(rows);
  return arr.len(flat);
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
fn sema_rejects_arr_sum_for_bool_elements() {
    let src = r#"
import arr;
fn main() -> Int {
  let b: [Bool; 2] = [true, false];
  let _x = arr.sum(b);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("arr.sum supports Int, Float, String, or Array")
    }));
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
fn sema_accepts_str_repeat_and_arr_reverse() {
    let src = r#"
import str;
import arr;
fn main() -> Int {
  let s = str.repeat("ab", 3);
  let a: [Int; 4] = [1, 2, 3, 4];
  let r = arr.reverse(a);
  if (s == "ababab" && arr.first(r) == 4 && arr.last(r) == 1) {
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
fn sema_accepts_arr_slice_min_max() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 5] = [7, 2, 9, 2, 5];
  let s = arr.slice(a, 1, 4);
  if (arr.len(s) == 3 && arr.min(a) == 2 && arr.max(a) == 9) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_arr_slice_signature_mismatch() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 3] = [1, 2, 3];
  let _s = arr.slice(a, "0", 2);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("arr.slice argument 2 expects Int"))
    );
}

#[test]
fn sema_rejects_arr_min_max_for_non_numeric_elements() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [String; 2] = ["a", "b"];
  let _m = arr.min(a);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("arr.min supports Int or Float elements"))
    );
}

#[test]
fn sema_accepts_arr_sort_for_supported_element_types() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Int; 4] = [3, 1, 2, 1];
  let b = arr.sort(a);
  if (arr.first(b) == 1 && arr.last(b) == 3) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_accepts_arr_sort_for_bool_elements() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [Bool; 4] = [true, false, true, false];
  let s = arr.sort(a);
  if (!arr.first(s) && arr.last(s)) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_arr_sort_for_unsupported_element_types() {
    let src = r#"
import arr;
fn main() -> Int {
  let a: [[Int; 1]; 2] = [[1], [2]];
  let _s = arr.sort(a);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("arr.sort supports Int, Float, String, or Bool elements"))
    );
}
