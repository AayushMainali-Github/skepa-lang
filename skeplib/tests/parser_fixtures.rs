mod common;

use std::fs;

use common::{fixtures_dir, parse_err, parse_ok, sk_files_in};

#[test]
fn all_valid_parser_fixtures_have_no_diagnostics() {
    let dir = fixtures_dir("parser").join("valid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let _program = parse_ok(&src);
    }
}

#[test]
fn all_invalid_parser_fixtures_have_diagnostics() {
    let dir = fixtures_dir("parser").join("invalid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let _diags = parse_err(&src);
    }
}
