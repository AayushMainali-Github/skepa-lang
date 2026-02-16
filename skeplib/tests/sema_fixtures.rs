mod common;

use std::fs;

use common::{fixtures_dir, sema_err, sema_ok, sk_files_in};

#[test]
fn all_valid_sema_fixtures_have_no_diagnostics() {
    let dir = fixtures_dir("sema").join("valid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let _ = sema_ok(&src);
    }
}

#[test]
fn all_invalid_sema_fixtures_have_diagnostics() {
    let dir = fixtures_dir("sema").join("invalid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let _ = sema_err(&src);
    }
}
