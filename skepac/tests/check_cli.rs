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
    assert_eq!(output.status.code(), Some(10));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Expected `;` after return statement"));
    assert!(stderr.contains("[E-PARSE][parse]"));
}

#[test]
fn check_sema_invalid_program_returns_sema_exit_code() {
    let tmp = make_temp_dir("skepac_sema_bad");
    let file = tmp.join("bad_sema.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  return "oops";
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run skepac");
    assert_eq!(output.status.code(), Some(11));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-SEMA][sema]"));
}

#[test]
fn check_without_arguments_shows_usage_and_fails() {
    let output = Command::new(skepac_bin()).output().expect("run skepac");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: skepac check <entry.sk> | skepac build <entry.sk> <out.skbc>"));
}

#[test]
fn unknown_command_fails() {
    let output = Command::new(skepac_bin())
        .arg("wat")
        .output()
        .expect("run skepac");
    assert_eq!(output.status.code(), Some(2));
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
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read"));
}

#[test]
fn build_writes_bytecode_artifact() {
    let tmp = make_temp_dir("skepac_build");
    let source = tmp.join("main.sk");
    let out = tmp.join("main.skbc");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  return 7;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("build")
        .arg(&source)
        .arg(&out)
        .output()
        .expect("run skepac build");

    assert!(output.status.success(), "{:?}", output);
    assert!(out.exists());
}

#[test]
fn build_malformed_source_is_reported_as_parse_error() {
    let tmp = make_temp_dir("skepac_build_parse_bad");
    let source = tmp.join("bad.sk");
    let out = tmp.join("bad.skbc");
    fs::write(
        &source,
        r#"
fn main( -> Int {
  return 0;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("build")
        .arg(&source)
        .arg(&out)
        .output()
        .expect("run skepac build");

    assert_eq!(output.status.code(), Some(10));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-PARSE][parse]"));
}

#[test]
fn disasm_source_prints_bytecode_text() {
    let tmp = make_temp_dir("skepac_disasm_src");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  return 1 + 2;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("disasm")
        .arg(&source)
        .output()
        .expect("run skepac disasm");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fn main"));
    assert!(stdout.contains("Add"));
}

#[test]
fn disasm_malformed_source_is_reported_as_parse_error() {
    let tmp = make_temp_dir("skepac_disasm_parse_bad");
    let source = tmp.join("bad.sk");
    fs::write(
        &source,
        r#"
fn main( -> Int {
  return 0;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("disasm")
        .arg(&source)
        .output()
        .expect("run skepac disasm");

    assert_eq!(output.status.code(), Some(10));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-PARSE][parse]"));
}

#[test]
fn disasm_bytecode_file_prints_text() {
    let tmp = make_temp_dir("skepac_disasm_bc");
    let source = tmp.join("main.sk");
    let bc = tmp.join("main.skbc");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  return 3;
}
"#,
    )
    .expect("write source");

    let build_out = Command::new(skepac_bin())
        .arg("build")
        .arg(&source)
        .arg(&bc)
        .output()
        .expect("run build");
    assert!(build_out.status.success(), "{:?}", build_out);

    let disasm_out = Command::new(skepac_bin())
        .arg("disasm")
        .arg(&bc)
        .output()
        .expect("run disasm");
    assert!(disasm_out.status.success(), "{:?}", disasm_out);
    let stdout = String::from_utf8_lossy(&disasm_out.stdout);
    assert!(stdout.contains("LoadConst Int(3)"));
}

#[test]
fn check_accepts_new_language_features_program() {
    let tmp = make_temp_dir("skepac_check_new_features");
    let file = tmp.join("features.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let i = 0;
  let acc = +0;
  while (i < 10) {
    i = i + 1;
    if (i == 3) {
      continue;
    }
    acc = acc + (i % 4);
    if (i == 7 || false) {
      break;
    }
  }
  return acc;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn check_accepts_minimal_os_builtins_program() {
    let tmp = make_temp_dir("skepac_check_os_minimal");
    let file = tmp.join("os_minimal.sk");
    let shell_ok = if cfg!(target_os = "windows") {
        "exit /b 0"
    } else {
        "exit 0"
    };
    let shell_out = if cfg!(target_os = "windows") {
        "echo hi"
    } else {
        "printf hi"
    };
    let src = format!(
        r#"
import os;
import str;
fn main() -> Int {{
  let c = os.cwd();
  let p = os.platform();
  os.sleep(1);
  let code = os.execShell("{shell_ok}");
  let out = os.execShellOut("{shell_out}");
  if (str.len(c) > 0 && str.len(p) > 0 && code == 0 && str.contains(out, "hi")) {{
    return 0;
  }}
  return 1;
}}
"#
    );
    fs::write(&file, src).expect("write source");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
}

#[test]
fn check_accepts_minimal_fs_builtins_program() {
    let tmp = make_temp_dir("skepac_check_fs_minimal");
    let file = tmp.join("fs_minimal.sk");
    fs::write(
        &file,
        r#"
import fs;
import str;
fn main() -> Int {
  let ex: Bool = fs.exists("a");
  let p: String = fs.join("a", "b");
  let t: String = fs.readText("a.txt");
  fs.writeText("a.txt", "x");
  fs.appendText("a.txt", "y");
  fs.mkdirAll("tmp/a/b");
  fs.removeFile("a.txt");
  fs.removeDirAll("tmp");
  if (ex || fs.exists(p) || (t == "") || str.len(p) >= 0) {
    return 0;
  }
  return 0;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
}

#[test]
fn disasm_includes_mod_and_short_circuit_instructions() {
    let tmp = make_temp_dir("skepac_disasm_new_ops");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  if (true || false) {
    return 8 % 3;
  }
  return 0;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("disasm")
        .arg(&source)
        .output()
        .expect("run disasm");
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("JumpIfTrue"));
    assert!(stdout.contains("ModInt"));
}

#[test]
fn check_accepts_for_loop_control_flow_program() {
    let tmp = make_temp_dir("skepac_check_for_features");
    let file = tmp.join("for_features.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 8; i = i + 1) {
    if (i == 2) {
      continue;
    }
    if (i == 6) {
      break;
    }
    acc = acc + (i % 3);
  }
  return acc;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn multi_file_project_check_build_disasm_work() {
    let tmp = make_temp_dir("skepac_multi");
    fs::create_dir_all(tmp.join("utils")).expect("create utils");
    let main = tmp.join("main.sk");
    let util = tmp.join("utils").join("math.sk");
    let out = tmp.join("main.skbc");

    fs::write(
        &util,
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write util");
    fs::write(
        &main,
        r#"
from utils.math import add;
fn main() -> Int { return add(20, 22); }
"#,
    )
    .expect("write main");

    let check = Command::new(skepac_bin())
        .arg("check")
        .arg(&main)
        .output()
        .expect("run check");
    assert_eq!(check.status.code(), Some(0));

    let build = Command::new(skepac_bin())
        .arg("build")
        .arg(&main)
        .arg(&out)
        .output()
        .expect("run build");
    assert_eq!(build.status.code(), Some(0));
    assert!(out.exists());

    let disasm = Command::new(skepac_bin())
        .arg("disasm")
        .arg(&main)
        .output()
        .expect("run disasm");
    assert_eq!(disasm.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&disasm.stdout);
    assert!(stdout.contains("fn utils.math::add"));
}

#[test]
fn multi_file_project_resolver_error_reports_import_chain_like_context() {
    let tmp = make_temp_dir("skepac_multi_resolve_err");
    let main = tmp.join("main.sk");
    fs::write(
        &main,
        r#"
import missing.dep;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&main)
        .output()
        .expect("run check");
    assert_eq!(output.status.code(), Some(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-MOD-NOT-FOUND][resolve]"));
    assert!(stderr.contains("while resolving import `missing.dep`"));
}

#[test]
fn build_resolver_error_uses_resolver_code_not_io_code() {
    let tmp = make_temp_dir("skepac_build_resolve_err");
    let main = tmp.join("main.sk");
    let out = tmp.join("main.skbc");
    fs::write(
        &main,
        r#"
import missing.dep;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let output = Command::new(skepac_bin())
        .arg("build")
        .arg(&main)
        .arg(&out)
        .output()
        .expect("run build");
    assert_eq!(output.status.code(), Some(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-MOD-NOT-FOUND][resolve]"));
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
