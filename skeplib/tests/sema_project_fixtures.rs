mod common;

use skeplib::sema::analyze_project_entry;
use std::fs;
use std::path::PathBuf;

fn sema_project_fixture_root() -> PathBuf {
    common::fixtures_dir("sema_project")
}

#[test]
fn all_valid_sema_project_fixtures_succeed() {
    let root = sema_project_fixture_root().join("valid");
    let entries = fs::read_dir(&root).expect("valid sema_project fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let (res, diags) = analyze_project_entry(&entry_file).expect("resolver/sema should succeed");
        assert!(
            !res.has_errors && diags.is_empty(),
            "expected sema success for fixture {}, got {:?}",
            case_dir.display(),
            diags.as_slice()
        );
    }
}

#[test]
fn all_invalid_sema_project_fixtures_fail_with_expected_code() {
    let root = sema_project_fixture_root().join("invalid");
    let entries = fs::read_dir(&root).expect("invalid sema_project fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let expected_code_path = case_dir.join("expected_code.txt");
        let expected_code = fs::read_to_string(&expected_code_path)
            .expect("expected_code.txt exists")
            .trim()
            .to_string();
        let expected_phrase = case_dir.join("expected_phrase.txt");
        let maybe_phrase = fs::read_to_string(&expected_phrase).ok().map(|s| s.trim().to_string());

        let errs = analyze_project_entry(&entry_file).expect_err("expected resolver failure");
        assert!(
            errs.iter().any(|e| e.code == expected_code),
            "fixture {} expected code {}, got {:?}",
            case_dir.display(),
            expected_code,
            errs
        );
        if let Some(phrase) = maybe_phrase {
            assert!(
                errs.iter().any(|e| e.message.contains(&phrase)),
                "fixture {} expected phrase `{}` in {:?}",
                case_dir.display(),
                phrase,
                errs
            );
        }
    }
}
