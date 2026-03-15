use super::*;

#[test]
fn sema_accepts_vec_type_and_typed_vec_new() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 10);
  vec.set(xs, 0, 20);
  return vec.get(xs, 0) + vec.len(xs);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(!result.has_errors, "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn sema_rejects_untyped_vec_new() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs = vec.new();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Cannot infer vector element type for let `xs`; annotate as `Vec[T]`")
    }));
}

#[test]
fn sema_rejects_vec_without_import() {
    let src = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("`vec.*` used without `import vec;`"))
    );
}

#[test]
fn sema_rejects_vec_push_set_value_type_mismatch() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, "x");
  vec.set(xs, 0, "y");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("vec.push argument 2 expects Int"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("vec.set argument 3 expects Int"))
    );
}

#[test]
fn sema_rejects_vec_index_arg_type_mismatch_and_assignment_mismatch() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let s: String = vec.delete(xs, "0");
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("vec.delete argument 2 expects Int"))
    );
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Type mismatch in let `s`: declared String, got Int")
    }));
}

#[test]
fn sema_rejects_vec_new_with_declared_non_vec_type() {
    let src = r#"
import vec;
fn main() -> Int {
  let x: Int = vec.new();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Type mismatch in let `x`: declared Int, got vec.new()")
    }));
}

#[test]
fn sema_rejects_vec_len_on_non_vec_and_bad_arity() {
    let src = r#"
import vec;
fn main() -> Int {
  let a = vec.len(1);
  let b = vec.len();
  return a + b;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("vec.len argument 1 expects Vec"))
    );
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("vec.len expects 1 argument(s), got 0"))
    );
}

#[test]
fn sema_rejects_vec_get_set_delete_arity_and_index_type_mismatches() {
    let src = r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let _a = vec.get(xs);
  let _b = vec.get(xs, false);
  vec.set(xs, "0", 1);
  vec.set(xs, 0);
  let _c = vec.delete(xs);
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    let msgs = diags
        .as_slice()
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        msgs.iter()
            .any(|m| m.contains("vec.get expects 2 argument(s), got 1"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("vec.get argument 2 expects Int"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("vec.set argument 2 expects Int"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("vec.set expects 3 argument(s), got 2"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("vec.delete expects 2 argument(s), got 1"))
    );
}
