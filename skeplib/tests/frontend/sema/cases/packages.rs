use super::*;

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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
fn sema_accepts_bytes_builtins() {
    let src = r#"
import bytes;
import str;

fn main() -> Int {
  let b: Bytes = bytes.fromString("hello");
  let s: String = bytes.toString(b);
  let n: Int = bytes.len(b);
  let first: Int = bytes.get(b, 1);
  let cut: Bytes = bytes.slice(b, 1, 4);
  let joined: Bytes = bytes.concat(cut, bytes.fromString("o"));
  let pushed: Bytes = bytes.push(joined, 33);
  let appended: Bytes = bytes.append(cut, bytes.fromString("lo"));
  let same: Bool = appended == bytes.fromString("ello");
  if (s == "hello" && str.len(s) == n && first == 101 && bytes.toString(pushed) == "ello!" && same) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_map_builtins() {
    let src = r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  map.insert(headers, "content-length", 12);
  let has_len: Bool = map.has(headers, "content-length");
  let value: Int = map.get(headers, "content-length");
  let removed: Int = map.remove(headers, "content-length");
  if (has_len && value == removed && map.len(headers) == 0) {
    return 1;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_map_without_import() {
    let src = r#"
fn main() -> Int {
  let values: Map[String, Int] = map.new();
  return map.len(values);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`map.*` used without `import map;`"))
    );
}

#[test]
fn sema_rejects_map_type_mismatches_and_inferred_new() {
    let src = r#"
import map;

fn main() -> Int {
  let inferred = map.new();
  let values: Map[String, Int] = map.new();
  map.insert(values, "ok", false);
  let _x = map.get(values, 1);
  let _same: Bool = values == values;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("Cannot infer map value type for let `inferred`")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("map.insert argument 3 expects Int")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("map.get argument 2 expects String")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("Map values cannot be compared with `==` or `!=`")
    }));
}

#[test]
fn sema_rejects_bytes_without_import() {
    let src = r#"
fn main() -> Int {
  let b: Bytes = bytes.fromString("x");
  return bytes.len(b);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`bytes.*` used without `import bytes;`"))
    );
}

#[test]
fn sema_rejects_bytes_from_string_type_mismatch() {
    let src = r#"
import bytes;

fn main() -> Int {
  let _b = bytes.fromString(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.fromString argument 1 expects String"))
    );
}

#[test]
fn sema_rejects_bytes_to_string_type_mismatch() {
    let src = r#"
import bytes;

fn main() -> Int {
  let _s = bytes.toString("abc");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.toString argument 1 expects Bytes"))
    );
}

#[test]
fn sema_rejects_bytes_len_type_mismatch() {
    let src = r#"
import bytes;

fn main() -> Int {
  return bytes.len(false);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.len argument 1 expects Bytes"))
    );
}

#[test]
fn sema_rejects_bytes_assignment_type_mismatch() {
    let src = r#"
import bytes;

fn main() -> Int {
  let x: String = bytes.fromString("abc");
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
fn sema_rejects_bytes_get_and_slice_type_mismatches() {
    let src = r#"
import bytes;

fn main() -> Int {
  let raw: Bytes = bytes.fromString("abc");
  let _a = bytes.get(raw, "1");
  let _b = bytes.slice(raw, false, 2);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.get argument 2 expects Int"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.slice argument 2 expects Int"))
    );
}

#[test]
fn sema_rejects_bytes_concat_type_mismatch() {
    let src = r#"
import bytes;

fn main() -> Int {
  let raw: Bytes = bytes.fromString("abc");
  let _x = bytes.concat(raw, "z");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.concat argument 2 expects Bytes"))
    );
}

#[test]
fn sema_rejects_bytes_push_and_eq_type_mismatches() {
    let src = r#"
import bytes;

fn main() -> Int {
  let raw: Bytes = bytes.fromString("abc");
  let _a = bytes.push(raw, "!");
  let _b: Bool = raw == 1;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("bytes.push argument 2 expects Int"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Invalid equality operands: left Bytes, right Int"))
    );
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
fn sema_reports_builtin_missing_import_and_argument_errors_together() {
    let src = r#"
fn main() -> Int {
  return str.len(nope);
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
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown variable `nope`"))
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
fn sema_missing_variadic_format_args_do_not_invent_concrete_return_type() {
    let src = r#"
import io;
fn main() -> Int {
  let s: String = io.format();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "io.format expects at least 1 argument");
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `s`"))
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
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
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_minimal_os_builtin_signatures() {
    let src = r#"
import str;
import os;
import vec;
fn main() -> Int {
  let plat: String = os.platform();
  let arch: String = os.arch();
  let arg0: String = os.arg(0);
  let hasHome: Bool = os.envHas("HOME");
  let home: String = os.envGet("HOME");
  os.envSet("MODE", "debug");
  os.envRemove("MODE");
  os.sleep(1);
  let args: Vec[String] = vec.new();
  vec.push(args, "status");
  let code: Int = os.exec("git", args);
  let out: String = os.execOut("git", args);
  if (str.len(plat) >= 0 && str.len(arch) >= 0 && str.len(arg0) >= 0 && hasHome && str.len(home) >= 0 && code >= 0 && str.len(out) >= 0) {
    return 0;
  }
  return 1;
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_builtin_opaque_net_types_in_annotations() {
    let src = r#"
fn takes_socket(s: net.Socket) -> Void {
  return;
}

fn main() -> Void {
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_unknown_builtin_opaque_net_type_name() {
    let src = r#"
fn main(x: net.Server) -> Void {
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown type in function `main` parameter `x`: `net.Server`"))
    );
}

#[test]
fn sema_accepts_minimal_net_listener_and_socket_builtins() {
    let src = r#"
import net;
import bytes;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let socket: net.Socket = net.accept(listener);
  let client: net.Socket = net.connect("127.0.0.1:8080");
  let secure: net.Socket = net.tlsConnect("example.com", 443);
  let msg: String = net.read(socket);
  let raw: Bytes = net.readBytes(socket);
  let exact: Bytes = net.readN(socket, 4);
  let local: String = net.localAddr(client);
  let peer: String = net.peerAddr(secure);
  net.write(client, msg);
  net.writeBytes(client, raw);
  net.writeBytes(client, exact);
  net.flush(client);
  net.setReadTimeout(client, 25);
  net.setWriteTimeout(client, 50);
  if (local == peer) {
    return;
  }
  net.close(socket);
  net.close(client);
  net.closeListener(listener);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_builtin_opaque_task_types_and_dummy_builtins() {
    let src = r#"
import task;

fn main() -> Int {
  let t: task.Task[Int] = task.__testTask(9);
  let c: task.Channel = task.__testChannel();
  let also_t: task.Task[Int] = t;
  let also_c: task.Channel = c;
  let v: Int = task.join(also_t);
  let _ = also_c;
  return v;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_task_without_import() {
    let src = r#"
fn main() -> Int {
  let t: task.Task[Int] = task.__testTask(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`task.*` used without `import task;`"))
    );
}

#[test]
fn sema_rejects_unknown_builtin_opaque_task_type_name() {
    let src = r#"
fn main(x: task.Mutex) -> Void {
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown type in function `main` parameter `x`: `task.Mutex`"))
    );
}

#[test]
fn sema_accepts_typed_task_channels_and_message_flow() {
    let src = r#"
import task;

fn main() -> Int {
  let jobs: task.Channel[Int] = task.channel();
  task.send(jobs, 7);
  let value: Int = task.recv(jobs);
  return value;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_untyped_task_channel_and_send_type_mismatch() {
    let src = r#"
import task;

fn main() -> Int {
  let inferred = task.channel();
  let jobs: task.Channel[Int] = task.channel();
  task.send(jobs, "bad");
  return task.recv(jobs);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| d
        .message
        .contains("Cannot infer channel value type for let `inferred`; annotate as `task.Channel[T]`")));
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("task.send argument 2 expects Int")));
}

#[test]
fn sema_rejects_net_readbytes_and_writebytes_type_mismatches() {
    let src = r#"
import net;
import bytes;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let socket: net.Socket = net.__testSocket();
  let _x = net.readBytes(listener);
  net.writeBytes(socket, "bad");
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.readBytes argument 1 expects Opaque(\"net.Socket\")")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.writeBytes argument 2 expects Bytes")
    }));
}

#[test]
fn sema_rejects_net_localaddr_and_peeraddr_type_mismatches() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let _a = net.localAddr(listener);
  let _b = net.peerAddr(listener);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.localAddr argument 1 expects Opaque(\"net.Socket\")")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.peerAddr argument 1 expects Opaque(\"net.Socket\")")
    }));
}

#[test]
fn sema_rejects_net_readn_type_mismatches() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let socket: net.Socket = net.__testSocket();
  let _a = net.readN(listener, 4);
  let _b = net.readN(socket, false);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.readN argument 1 expects Opaque(\"net.Socket\")")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("net.readN argument 2 expects Int")
    }));
}

#[test]
fn sema_rejects_net_flush_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  net.flush(listener);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.flush argument 1 expects Opaque(\"net.Socket\")")
    }));
}

#[test]
fn sema_rejects_net_timeout_type_mismatches() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let socket: net.Socket = net.__testSocket();
  net.setReadTimeout(listener, 5);
  net.setWriteTimeout(socket, false);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.setReadTimeout argument 1 expects Opaque(\"net.Socket\")")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message.contains("net.setWriteTimeout argument 2 expects Int")
    }));
}

#[test]
fn sema_rejects_net_accept_listener_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let socket: net.Socket = net.__testSocket();
  let _x = net.accept(socket);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.accept argument 1 expects Opaque(\"net.Listener\")")
    }));
}

#[test]
fn sema_rejects_net_tls_connect_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let _x = net.tlsConnect(1, false);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.tlsConnect argument 1 expects String")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.tlsConnect argument 2 expects Int")
    }));
}

#[test]
fn sema_rejects_net_connect_argument_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let _x = net.connect(1);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.connect argument 1 expects String")
    }));
}

#[test]
fn sema_rejects_net_close_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  net.close(listener);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.close argument 1 expects Opaque(\"net.Socket\")")
    }));
}

#[test]
fn sema_rejects_net_read_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let _x = net.read(listener);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.read argument 1 expects Opaque(\"net.Socket\")")
    }));
}

#[test]
fn sema_rejects_net_write_type_mismatches() {
    let src = r#"
import net;

fn main() -> Void {
  let socket: net.Socket = net.__testSocket();
  net.write("bad", 1);
  net.write(socket, 1);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.write argument 1 expects Opaque(\"net.Socket\")")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.write argument 2 expects String")
    }));
}

#[test]
fn sema_rejects_net_close_listener_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let socket: net.Socket = net.__testSocket();
  net.closeListener(socket);
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("net.closeListener argument 1 expects Opaque(\"net.Listener\")")
    }));
}

#[test]
fn sema_rejects_net_usage_without_import() {
    let src = r#"
fn main() -> Void {
  let _x = net.connect("127.0.0.1:1");
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`net.*` used without `import net;`"))
    );
}

#[test]
fn sema_rejects_net_connect_assignment_type_mismatch() {
    let src = r#"
import net;

fn main() -> Void {
  let l: net.Listener = net.connect("127.0.0.1:1");
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `l`"))
    );
}

#[test]
fn sema_rejects_unknown_net_builtin_names() {
    let src = r#"
import net;

fn main() -> Void {
  let _a = net.socket();
  let _b = net.exec("127.0.0.1:1");
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `net.socket`"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `net.exec`"))
    );
}

#[test]
fn sema_accepts_dummy_net_builtin_returning_socket_handle() {
    let src = r#"
import net;

fn make() -> net.Socket {
  return net.__testSocket();
}

fn main() -> Void {
  let s: net.Socket = make();
  return;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_os_without_import() {
    let src = r#"
fn main() -> Int {
  let _x = os.platform();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`os.*` used without `import os;`"))
    );
}

#[test]
fn sema_rejects_os_platform_arity_mismatch() {
    let src = r#"
import os;
fn main() -> Int {
  let _x = os.platform(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("os.platform expects 0 argument(s), got 1") })
    );
}

#[test]
fn sema_rejects_os_sleep_type_mismatch() {
    let src = r#"
import os;
fn main() -> Int {
  os.sleep("1");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("os.sleep argument 1 expects Int"))
    );
}

#[test]
fn sema_rejects_os_exec_type_mismatch() {
    let src = r#"
import os;
import vec;
fn main() -> Int {
  let args: Vec[String] = vec.new();
  return os.exec(1, args);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("os.exec argument 1 expects String"))
    );
}

#[test]
fn sema_rejects_os_exec_out_type_mismatch() {
    let src = r#"
import os;
import vec;
fn main() -> Int {
  let args: Vec[String] = vec.new();
  let _x = os.execOut(false, args);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("os.execOut argument 1 expects String")
    }));
}

#[test]
fn sema_rejects_os_exec_vec_type_mismatch() {
    let src = r#"
import os;
import vec;
fn main() -> Int {
  let args: Vec[Int] = vec.new();
  return os.exec("git", args);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("os.exec argument 2 expects Vec[String]"))
    );
}

#[test]
fn sema_rejects_removed_os_legacy_builtins() {
    let src = r#"
import os;
fn main() -> Int {
  let _a = os.cwd();
  let _b = os.execShell("echo hi");
  let _c = os.execShellOut("echo hi");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `os.cwd`"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `os.execShell`"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown builtin `os.execShellOut`"))
    );
}

#[test]
fn sema_rejects_os_platform_assignment_type_mismatch() {
    let src = r#"
import os;
fn main() -> Int {
  let x: Int = os.platform();
  return x;
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
fn sema_accepts_minimal_fs_builtin_signatures() {
    let src = r#"
import fs;
fn main() -> Int {
  let ex: Bool = fs.exists("a");
  let p: String = fs.join("a", "b");
  let t: String = fs.readText("a.txt");
  fs.writeText("a.txt", "x");
  fs.appendText("a.txt", "y");
  fs.mkdirAll("tmp/a/b");
  fs.removeFile("a.txt");
  fs.removeDirAll("tmp");
  if (ex || fs.exists(p) || (t == "")) {
    return 0;
  }
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_fs_without_import() {
    let src = r#"
fn main() -> Int {
  let _x = fs.exists("a");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`fs.*` used without `import fs;`"))
    );
}

#[test]
fn sema_rejects_fs_exists_arity_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  let _x = fs.exists();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("fs.exists expects 1 argument(s), got 0") })
    );
}

#[test]
fn sema_rejects_fs_join_arity_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  let _x = fs.join("a");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("fs.join expects 2 argument(s), got 1") })
    );
}

#[test]
fn sema_rejects_fs_read_text_type_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  let _x = fs.readText(1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("fs.readText argument 1 expects String"))
    );
}

#[test]
fn sema_rejects_fs_write_text_arg2_type_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.writeText("a.txt", 1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("fs.writeText argument 2 expects String"))
    );
}

#[test]
fn sema_rejects_fs_mkdir_all_type_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  fs.mkdirAll(false);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("fs.mkdirAll argument 1 expects String"))
    );
}

#[test]
fn sema_rejects_fs_exists_assignment_type_mismatch() {
    let src = r#"
import fs;
fn main() -> Int {
  let x: Int = fs.exists("a");
  return x;
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
fn sema_rejects_io_println_arity_mismatch() {
    let src = r#"
import io;
fn main() -> Int {
  io.println();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "io.println expects 1 argument(s), got 0");
}

#[test]
fn sema_rejects_str_startswith_wrong_arity_and_type() {
    let src = r#"
import str;
fn main() -> Int {
  let a = str.startsWith("abc");
  let b = str.startsWith("abc", 1);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "str.startsWith expects 2 argument(s), got 1");
    assert_has_diag(&diags, "str.startsWith argument 2 expects String");
}

#[test]
fn sema_wrong_arity_builtin_does_not_invent_concrete_return_type() {
    let src = r#"
import str;
fn main() -> Int {
  let x: Int = str.len();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "str.len expects 1 argument(s), got 0");
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `x`"))
    );
}

#[test]
fn sema_rejects_arr_first_on_non_array_value() {
    let src = r#"
import arr;
fn main() -> Int {
  return arr.first(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(&diags, "arr.first argument 1 expects Array");
}

#[test]
fn sema_tracks_builtin_return_types_across_families() {
    let src = r#"
    import bytes;
    import io;
import str;
import arr;
import vec;
import datetime;
import random;
import fs;
import os;
import vec;

fn main() -> Int {
      let raw: Bytes = bytes.fromString("x");
      let s: String = io.format("%d", 1);
  let b: Bool = str.isEmpty("");
  let a: [Int; 2] = [1, 2];
  let first: Int = arr.first(a);
  let xs: Vec[Int] = vec.new();
  let now: String = datetime.fromUnix(0);
  let r: Float = random.float();
  let exists: Bool = fs.exists("a");
  let args: Vec[String] = vec.new();
  vec.push(args, "status");
  let code: Int = os.exec("git", args);
      if (bytes.len(raw) == 1 && (b || exists || code >= 0 || r >= 0.0 || str.len(now) >= 0 || first >= 0 || str.len(s) >= 0)) {
        return vec.len(xs);
      }
      return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}
