mod common;

use skeplib::bytecode::{Value, compile_project_entry};
use skeplib::vm::Vm;
use std::fs;
use std::path::PathBuf;

fn bytecode_project_fixture_root() -> PathBuf {
    common::fixtures_dir("bytecode_project")
}

fn parse_expected_value(s: &str) -> Value {
    let t = s.trim();
    if let Some(v) = t.strip_prefix("Int:") {
        return Value::Int(v.trim().parse::<i64>().expect("valid Int value"));
    }
    if let Some(v) = t.strip_prefix("Bool:") {
        return Value::Bool(v.trim().parse::<bool>().expect("valid Bool value"));
    }
    if let Some(v) = t.strip_prefix("Float:") {
        return Value::Float(v.trim().parse::<f64>().expect("valid Float value"));
    }
    if let Some(v) = t.strip_prefix("String:") {
        return Value::String(v.trim().to_string());
    }
    panic!("unsupported expected value format `{t}`");
}

#[test]
fn all_valid_bytecode_project_fixtures_run_to_expected_output() {
    let root = bytecode_project_fixture_root().join("valid");
    let entries = fs::read_dir(&root).expect("valid bytecode_project fixtures dir exists");
    for entry in entries {
        let case_dir = entry.expect("dir entry").path();
        if !case_dir.is_dir() {
            continue;
        }
        let entry_file = case_dir.join("main.sk");
        let expected_raw = fs::read_to_string(case_dir.join("expected.txt")).expect("expected.txt exists");
        let expected = parse_expected_value(&expected_raw);
        let module = compile_project_entry(&entry_file).expect("compile project fixture");
        let got = Vm::run_module_main(&module).expect("run project fixture");
        assert_eq!(
            got, expected,
            "fixture {} expected {:?}, got {:?}",
            case_dir.display(), expected, got
        );
    }
}
