use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn check_valid_program_returns_zero() {
    let tmp = make_temp_dir("skepac_ok");
    let file = tmp.join("ok.sk");
    fs::write(
        &file,
        r#"
import io;
fn main() -> Int {
  return 0;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run skepac");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ok:"));
}

#[test]
fn check_invalid_program_returns_non_zero() {
    let tmp = make_temp_dir("skepac_bad");
    let file = tmp.join("bad.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  return 0
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run skepac");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Expected `;` after return statement"));
}

#[test]
fn check_without_arguments_shows_usage_and_fails() {
    let output = Command::new(skepac_bin())
        .output()
        .expect("run skepac");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: skepac check <file.sk>"));
}

#[test]
fn unknown_command_fails() {
    let output = Command::new(skepac_bin())
        .arg("wat")
        .output()
        .expect("run skepac");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command"));
}

#[test]
fn missing_file_fails() {
    let output = Command::new(skepac_bin())
        .arg("check")
        .arg("does_not_exist.sk")
        .output()
        .expect("run skepac");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read"));
}

fn skepac_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skepac")
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
