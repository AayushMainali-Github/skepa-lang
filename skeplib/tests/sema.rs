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
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Return type mismatch")));
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
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Assignment type mismatch")));
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
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("if condition must be Bool")));
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
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Arity mismatch")));
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
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("without `import io;`")));
}
