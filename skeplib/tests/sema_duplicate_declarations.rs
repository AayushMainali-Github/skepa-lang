mod common;

use skeplib::sema::{analyze_project_entry, analyze_source};

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

#[test]
fn rejects_duplicate_function_declaration_in_project_module() {
    let project = common::TempProject::new("duplicate_project_function_declaration");
    let entry = project.file(
        "main.sk",
        r#"
fn value() -> Int { return 1; }
fn value() -> Int { return 2; }
fn main() -> Int { return value(); }
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(&diags, "Duplicate function declaration `value`");
}
