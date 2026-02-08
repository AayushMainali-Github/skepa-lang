use std::fs;
use std::path::PathBuf;

use skeplib::parser::Parser;

#[test]
fn all_valid_parser_fixtures_have_no_diagnostics() {
    let dir = fixtures_dir().join("valid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let (_program, diags) = Parser::parse_source(&src);
        assert!(
            diags.is_empty(),
            "expected valid fixture {:?} to parse cleanly, got diagnostics: {:?}",
            path,
            diags.as_slice()
        );
    }
}

#[test]
fn all_invalid_parser_fixtures_have_diagnostics() {
    let dir = fixtures_dir().join("invalid");
    for path in sk_files_in(&dir) {
        let src = fs::read_to_string(&path).expect("read fixture");
        let (_program, diags) = Parser::parse_source(&src);
        assert!(
            !diags.is_empty(),
            "expected invalid fixture {:?} to report diagnostics",
            path
        );
    }
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("parser")
}

fn sk_files_in(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).expect("fixture directory exists");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "sk") {
            out.push(path);
        }
    }
    out.sort();
    out
}
