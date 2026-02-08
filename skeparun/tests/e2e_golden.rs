use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use skeplib::bytecode::compile_source;

#[test]
fn run_golden_fixtures() {
    run_cases("run", fixtures_root().join("run"));
}

#[test]
fn run_bc_golden_fixtures() {
    run_cases("run-bc", fixtures_root().join("run-bc"));
}

fn run_cases(cmd: &str, dir: PathBuf) {
    let mut entries = fs::read_dir(&dir)
        .expect("read fixture dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect fixture entries");
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sk") {
            continue;
        }
        let spec = parse_spec(&path.with_extension("expect"));
        let exec_path = materialize_input(cmd, &path);
        let output = Command::new(skeparun_bin())
            .arg(cmd)
            .arg(&exec_path)
            .output()
            .expect("run skeparun");

        assert_eq!(
            output.status.code(),
            Some(spec.exit_code),
            "case: {}",
            path.display()
        );

        for needle in &spec.stdout_contains {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains(needle),
                "case: {}, missing stdout fragment: {needle}\nstdout:\n{stdout}",
                path.display()
            );
        }
        for needle in &spec.stderr_contains {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains(needle),
                "case: {}, missing stderr fragment: {needle}\nstderr:\n{stderr}",
                path.display()
            );
        }
    }
}

fn materialize_input(cmd: &str, source_path: &Path) -> PathBuf {
    if cmd != "run-bc" {
        return source_path.to_path_buf();
    }
    let source = fs::read_to_string(source_path).expect("read source fixture");
    let module = compile_source(&source).expect("compile fixture source");
    let out = std::env::temp_dir().join(format!(
        "skeparun_fixture_{}.skbc",
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
    ));
    fs::write(&out, module.to_bytes()).expect("write fixture bytecode");
    out
}

#[derive(Debug)]
struct CaseSpec {
    exit_code: i32,
    stdout_contains: Vec<String>,
    stderr_contains: Vec<String>,
}

fn parse_spec(path: &Path) -> CaseSpec {
    let text = fs::read_to_string(path).expect("read .expect file");
    let mut spec = CaseSpec {
        exit_code: 0,
        stdout_contains: Vec::new(),
        stderr_contains: Vec::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            panic!("invalid spec line in {}: {line}", path.display());
        };
        let key = k.trim();
        let value = v.trim().to_string();
        match key {
            "exit" => spec.exit_code = value.parse::<i32>().expect("exit must be i32"),
            "stdout_contains" => spec.stdout_contains.push(value),
            "stderr_contains" => spec.stderr_contains.push(value),
            _ => panic!("unknown key `{key}` in {}", path.display()),
        }
    }
    spec
}

fn skeparun_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skeparun")
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}
