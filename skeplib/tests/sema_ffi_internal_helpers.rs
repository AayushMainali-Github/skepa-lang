mod common;

use skeplib::sema::analyze_source;

#[test]
fn rejects_low_level_ffi_helpers_with_internal_policy() {
    let src = r#"
import ffi;
fn main() -> Int {
  let a = ffi.call0Bool(1);
  let b = ffi.call2IntInt(1, 2, 3);
  return 0;
}
"#;

    let (res, diags) = analyze_source(src);
    assert!(res.has_errors);
    common::assert_has_diag(
        &diags,
        "`ffi.call0Bool` is a low-level internal helper; use `extern(\"...\") fn ...;` declarations instead",
    );
    common::assert_has_diag(
        &diags,
        "`ffi.call2IntInt` is a low-level internal helper; use `extern(\"...\") fn ...;` declarations instead",
    );
}
