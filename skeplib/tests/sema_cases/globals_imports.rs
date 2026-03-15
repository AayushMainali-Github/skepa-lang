use super::*;

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
