mod common;

use skeplib::sema::analyze_source;

#[test]
fn rejects_ffi_builtin_path_as_value_with_builtin_diagnostic() {
    let src = r#"
fn main() -> Int {
  let opener = ffi.open;
  return 0;
}
"#;

    let (res, diags) = analyze_source(src);
    assert!(res.has_errors);
    common::assert_has_diag(
        &diags,
        "Builtin path `ffi.open` is not a value; call it as a function",
    );
}
