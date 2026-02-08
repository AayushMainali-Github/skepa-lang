use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::bytecode::compile_source;

#[test]
fn run_valid_program_returns_success() {
    let tmp = make_temp_dir("skeparun_ok");
    let file = tmp.join("ok.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let x = 2;
  return x + 3;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(5));
}

#[test]
fn run_reports_semantic_errors() {
    let tmp = make_temp_dir("skeparun_sema_err");
    let file = tmp.join("bad_sema.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  return true;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Return type mismatch"));
    assert!(stderr.contains("[E-SEMA][sema]"));
}

#[test]
fn run_reports_runtime_errors() {
    let tmp = make_temp_dir("skeparun_runtime_err");
    let file = tmp.join("bad_runtime.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  return 1 / 0;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[runtime]"));
    assert!(stderr.contains("[E-VM-DIV-ZERO]"));
}

#[test]
fn run_bc_executes_compiled_artifact() {
    let tmp = make_temp_dir("skeparun_run_bc");
    let bc = tmp.join("main.skbc");
    let module = compile_source(
        r#"
fn main() -> Int {
  return 9;
}
"#,
    )
    .expect("compile");
    fs::write(&bc, module.to_bytes()).expect("write bc");

    let output = Command::new(skeparun_bin())
        .arg("run-bc")
        .arg(&bc)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(9));
}

fn skeparun_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skeparun")
}

fn make_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
