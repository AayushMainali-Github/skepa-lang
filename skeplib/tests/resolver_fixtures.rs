mod common;

use skeplib::resolver::{resolve_project, ResolveErrorKind};
use std::fs;
use std::path::PathBuf;

fn resolver_fixture_root() -> PathBuf {
    common::fixtures_dir("resolver")
}

#[test]
fn all_valid_resolver_fixtures_succeed() {
    let root = resolver_fixture_root().join("valid");
    let entries = fs::read_dir(&root).expect("valid resolver fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let result = resolve_project(&entry_file);
        assert!(
            result.is_ok(),
            "expected resolver success for fixture {}, got {:?}",
            case_dir.display(),
            result.err()
        );
    }
}

#[test]
fn all_invalid_resolver_fixtures_fail_with_expected_kind() {
    let root = resolver_fixture_root().join("invalid");
    let entries = fs::read_dir(&root).expect("invalid resolver fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let expected_kind_path = case_dir.join("expected_kind.txt");
        let expected_kind = fs::read_to_string(&expected_kind_path)
            .expect("expected_kind.txt exists")
            .trim()
            .to_string();
        let errs = resolve_project(&entry_file).expect_err("expected resolver failure");
        let matched = errs.iter().any(|e| match expected_kind.as_str() {
            "MissingModule" => e.kind == ResolveErrorKind::MissingModule,
            "AmbiguousModule" => e.kind == ResolveErrorKind::AmbiguousModule,
            "Cycle" => e.kind == ResolveErrorKind::Cycle,
            "DuplicateModuleId" => e.kind == ResolveErrorKind::DuplicateModuleId,
            "Io" => e.kind == ResolveErrorKind::Io,
            _ => false,
        });
        assert!(
            matched,
            "fixture {} expected kind {}, got {:?}",
            case_dir.display(),
            expected_kind,
            errs
        );
    }
}
