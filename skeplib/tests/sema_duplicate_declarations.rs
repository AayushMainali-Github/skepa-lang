mod common;

use skeplib::sema::analyze_source;

#[test]
fn rejects_duplicate_function_declaration() {
    let src = r#"
fn value() -> Int { return 1; }
fn value() -> String { return "bad"; }
fn main() -> Int { return value(); }
"#;

    let (res, diags) = analyze_source(src);
    assert!(res.has_errors);
    common::assert_has_diag(&diags, "Duplicate function declaration `value`");
}
