mod common;

use std::fs;
use std::path::PathBuf;

fn native_project_fixture_root() -> PathBuf {
    common::fixtures_dir("native_project")
}

fn parse_expected_exit_code(s: &str) -> i32 {
    let t = s.trim();
    if let Some(v) = t.strip_prefix("Int:") {
        return v.trim().parse::<i32>().expect("valid Int exit code");
    }
    panic!("native project fixtures currently require `Int:` expected values, got `{t}`");
}

#[test]
fn all_valid_native_project_fixtures_run_to_expected_exit_code() {
    let root = native_project_fixture_root().join("valid");
    let entries = fs::read_dir(&root).expect("valid native project fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let expected_raw =
            fs::read_to_string(case_dir.join("expected.txt")).expect("expected.txt exists");
        let expected = parse_expected_exit_code(&expected_raw);
        let output = common::native_run_project_ok(&entry_file);
        assert_eq!(
            output.status.code(),
            Some(expected),
            "fixture {} expected exit code {:?}, got {:?}",
            case_dir.display(),
            expected,
            output.status.code()
        );
    }
}
