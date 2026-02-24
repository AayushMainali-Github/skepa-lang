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

    assert_eq!(output.status.code(), Some(11));
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

    assert_eq!(output.status.code(), Some(14));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[runtime]"));
    assert!(stderr.contains("[E-VM-DIV-ZERO]"));
}

#[test]
fn run_missing_file_returns_io_exit_code() {
    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg("does_not_exist.sk")
        .output()
        .expect("run skeparun");
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn run_bc_decode_failure_returns_decode_exit_code() {
    let tmp = make_temp_dir("skeparun_bad_bc");
    let bc = tmp.join("bad.skbc");
    fs::write(&bc, b"not-bytecode").expect("write bad bc");
    let output = Command::new(skeparun_bin())
        .arg("run-bc")
        .arg(&bc)
        .output()
        .expect("run skeparun");
    assert_eq!(output.status.code(), Some(13));
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

#[test]
fn run_executes_break_continue_modulo_and_short_circuit() {
    let tmp = make_temp_dir("skeparun_new_features");
    let file = tmp.join("features.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let i = 0;
  let acc = +0;
  while (i < 8) {
    i = i + 1;
    if (i == 2) {
      continue;
    }
    acc = acc + (i % 3);
    if (i == 6 || false) {
      break;
    }
  }
  if (false && ((1 / 0) == 0)) {
    return 99;
  }
  return acc;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(4));
}

#[test]
fn run_executes_nested_for_break_continue() {
    let tmp = make_temp_dir("skeparun_for_features");
    let file = tmp.join("for_features.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 3; i = i + 1) {
    for (let j = 0; j < 4; j = j + 1) {
      if (j == 1) {
        continue;
      }
      if (i == 2 && j == 3) {
        break;
      }
      acc = acc + 1;
    }
  }
  return acc;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(8));
}

#[test]
fn run_multi_file_project_executes_entry_graph() {
    let tmp = make_temp_dir("skeparun_multi");
    fs::create_dir_all(tmp.join("utils")).expect("create utils");
    let main = tmp.join("main.sk");
    fs::write(
        tmp.join("utils").join("math.sk"),
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

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&main)
        .output()
        .expect("run skeparun");
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn run_multi_file_resolver_error_includes_context() {
    let tmp = make_temp_dir("skeparun_multi_resolve_err");
    let file = tmp.join("main.sk");
    fs::write(
        &file,
        r#"
import missing.dep;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-MOD-NOT-FOUND][resolve]"));
    assert!(stderr.contains("while resolving import `missing.dep`"));
}

#[test]
fn run_rejects_non_numeric_call_depth_env() {
    let tmp = make_temp_dir("skeparun_depth_env_non_numeric");
    let file = tmp.join("ok.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int { return 0; }
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .env("SKEPA_MAX_CALL_DEPTH", "abc")
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SKEPA_MAX_CALL_DEPTH must be an integer >= 1"));
}

#[test]
fn run_rejects_zero_call_depth_env() {
    let tmp = make_temp_dir("skeparun_depth_env_zero");
    let file = tmp.join("ok.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int { return 0; }
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .env("SKEPA_MAX_CALL_DEPTH", "0")
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SKEPA_MAX_CALL_DEPTH must be an integer >= 1"));
}

#[test]
fn run_depth_one_allows_main_but_blocks_nested_calls() {
    let tmp = make_temp_dir("skeparun_depth_one_nested_call");
    let file = tmp.join("nested.sk");
    fs::write(
        &file,
        r#"
fn child() -> Int { return 1; }
fn main() -> Int { return child(); }
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .env("SKEPA_MAX_CALL_DEPTH", "1")
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(14));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-VM-STACK-OVERFLOW]"));
}

#[test]
fn run_executes_minimal_os_builtins() {
    let tmp = make_temp_dir("skeparun_os_minimal");
    let file = tmp.join("os_minimal.sk");
    let shell_ok = if cfg!(target_os = "windows") {
        "exit /b 0"
    } else {
        "exit 0"
    };
    let shell_out = if cfg!(target_os = "windows") {
        "echo hello"
    } else {
        "printf hello"
    };
    let src = format!(
        r#"
import os;
import str;

fn main() -> Int {{
  let cwd = os.cwd();
  let p = os.platform();
  os.sleep(1);
  let code = os.execShell("{shell_ok}");
  let out = os.execShellOut("{shell_out}");
  if (str.len(cwd) > 0 && str.len(p) > 0 && code == 0 && str.contains(out, "hello")) {{
    return 0;
  }}
  return 1;
}}
"#
    );
    fs::write(&file, src).expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
}

#[test]
fn run_executes_minimal_fs_builtins() {
    let tmp = make_temp_dir("skeparun_fs_minimal");
    let file = tmp.join("fs_minimal.sk");
    let work = tmp.join("work");
    let file_txt = work.join("data.txt");
    let work_s = work.display().to_string().replace('\\', "\\\\");
    let file_txt_s = file_txt.display().to_string().replace('\\', "\\\\");

    let src = format!(
        r#"
import fs;
import str;

fn main() -> Int {{
  fs.mkdirAll("{work_s}");
  if (!fs.exists("{work_s}")) {{
    return 1;
  }}

  fs.writeText("{file_txt_s}", "a");
  fs.appendText("{file_txt_s}", "b");
  let txt = fs.readText("{file_txt_s}");
  let joined = fs.join("{work_s}", "data.txt");

  if (!(str.contains(txt, "ab") && fs.exists(joined))) {{
    return 2;
  }}

  fs.removeFile("{file_txt_s}");
  if (fs.exists("{file_txt_s}")) {{
    return 3;
  }}

  fs.removeDirAll("{work_s}");
  if (fs.exists("{work_s}")) {{
    return 4;
  }}

  return 0;
}}
"#
    );
    fs::write(&file, src).expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
}

#[test]
fn run_executes_match_statement_program() {
    let tmp = make_temp_dir("skeparun_match");
    let file = tmp.join("match_ok.sk");
    fs::write(
        &file,
        r#"
let hits: Int = 0;

fn next() -> Int {
  hits = hits + 1;
  return hits;
}

fn main() -> Int {
  let tag = "B";
  let score = matchValue(tag);
  if (score != 20) {
    return 91;
  }

  match (next()) {
    1 => { }
    _ => { return 92; }
  }

  if (hits != 1) {
    return 93;
  }
  return 0;
}

fn matchValue(s: String) -> Int {
  match (s) {
    "A" => { return 10; }
    "B" | "b" => { return 20; }
    _ => { return 30; }
  }
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
}

#[test]
fn run_executes_vec_program() {
    let tmp = make_temp_dir("skeparun_vec");
    let file = tmp.join("vec_ok.sk");
    fs::write(
        &file,
        r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 4);
  vec.push(xs, 8);
  vec.push(xs, 12);

  let alias = xs;
  vec.set(alias, 1, 9);

  let removed = vec.delete(xs, 0);
  if (removed != 4) {
    return 91;
  }

  if (vec.len(xs) != 2) {
    return 92;
  }

  if (vec.get(xs, 0) != 9 || vec.get(alias, 1) != 12) {
    return 93;
  }

  return 0;
}
"#,
    )
    .expect("write fixture");

    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&file)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
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
