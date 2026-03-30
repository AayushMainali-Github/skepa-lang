use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::process::Command;
use std::thread;

mod common;

use common::{
    CliFailureClass, assert_cli_failure_class, assert_diag_code_and_message, exe_ext,
    make_temp_dir, obj_ext, skepac_bin, write_temp_file,
};

#[test]
fn check_valid_program_returns_zero() {
    let tmp = make_temp_dir("skepac_ok");
    let file = write_temp_file(
        &tmp,
        "ok.sk",
        r#"
import io;
fn main() -> Int {
  return 0;
}
"#,
    );

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
    let file = write_temp_file(
        &tmp,
        "bad.sk",
        r#"
fn main() -> Int {
  return 0
}
"#,
    );

    let output = Command::new(skepac_bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run skepac");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_cli_failure_class(&output, CliFailureClass::Parse);
    assert_diag_code_and_message(&stderr, "[E-PARSE]", "Expected `;` after return statement");
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_cli_failure_class(&output, CliFailureClass::Sema);
    assert_diag_code_and_message(&stderr, "[E-SEMA][sema]", "Return type mismatch");
}

#[test]
fn check_without_arguments_shows_usage_and_fails() {
    let output = Command::new(skepac_bin()).output().expect("run skepac");
    assert_cli_failure_class(&output, CliFailureClass::Usage);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage: skepac check <entry.sk> | skepac run <entry.sk> | skepac build-native <entry.sk> <out.exe> | skepac build-obj <entry.sk> <out.obj> | skepac build-llvm-ir <entry.sk> <out.ll>"));
}

#[test]
fn unknown_command_fails() {
    let output = Command::new(skepac_bin())
        .arg("wat")
        .output()
        .expect("run skepac");
    assert_cli_failure_class(&output, CliFailureClass::Usage);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command"));
}

#[test]
fn run_executes_native_temp_binary_and_returns_exit_code() {
    let tmp = make_temp_dir("skepac_run_native");
    let source = tmp.join("main.sk");
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
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(7), "{:?}", output);
}

#[test]
fn run_reports_runtime_failure_for_division_by_zero() {
    let tmp = make_temp_dir("skepac_run_div_zero");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  let x = 1 / 0;
  return x;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_cli_failure_class(&output, CliFailureClass::Runtime);
}

#[test]
fn run_reports_runtime_failure_for_array_out_of_bounds() {
    let tmp = make_temp_dir("skepac_run_array_oob");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  let xs: [Int; 2] = [1, 2];
  return xs[9];
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_cli_failure_class(&output, CliFailureClass::Runtime);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("out of bounds") || stderr.contains("index") || stderr.contains("panic"),
        "stderr was: {stderr}"
    );
}

#[test]
fn run_executes_bitwise_integer_program_end_to_end() {
    let tmp = make_temp_dir("skepac_run_bitwise");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  let a = 12;
  let b = 10;
  let c = ~a;
  let d = a & b;
  let e = a | b;
  let f = a ^ b;
  let g = a << 2;
  let h = a >> 1;
  if (c == -13 && d == 8 && e == 14 && f == 6 && g == 48 && h == 6) {
    return 7;
  }
  return 1;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(7), "{:?}", output);
}

#[test]
fn run_reports_runtime_failure_for_negative_shift_count() {
    let tmp = make_temp_dir("skepac_run_negative_shift");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
fn main() -> Int {
  return 1 << -1;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_cli_failure_class(&output, CliFailureClass::Runtime);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("negative shift count") || stderr.contains("runtime"),
        "stderr was: {stderr}"
    );
}

#[test]
fn run_executes_user_defined_operator_program_end_to_end() {
    let tmp = make_temp_dir("skepac_run_user_operator");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}

fn main() -> Int {
  return 5 `xoxo` 4 + 1;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(55), "{:?}", output);
}

#[test]
fn run_executes_net_client_program_on_loopback() {
    let tmp = make_temp_dir("skepac_run_net_client");
    let source = tmp.join("main.sk");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("loopback addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).expect("read client payload");
        buf
    });
    fs::write(
        &source,
        format!(
            r#"
import net;

fn main() -> Int {{
  let client: net.Socket = net.connect("{addr}");
  net.write(client, "ping");
  net.close(client);
  return 0;
}}
"#
        ),
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    assert_eq!(server.join().expect("join server"), *b"ping");
}

#[test]
fn run_executes_net_byte_client_program_on_loopback() {
    let tmp = make_temp_dir("skepac_run_net_byte_client");
    let source = tmp.join("main.sk");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("loopback addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).expect("read client payload");
        buf
    });
    fs::write(
        &source,
        format!(
            r#"
import net;
import bytes;

fn main() -> Int {{
  let raw0: Bytes = bytes.fromString("");
  let raw1: Bytes = bytes.push(raw0, 1);
  let raw2: Bytes = bytes.push(raw1, 2);
  let raw3: Bytes = bytes.push(raw2, 3);
  let raw4: Bytes = bytes.push(raw3, 4);
  let client: net.Socket = net.connect("{addr}");
  net.writeBytes(client, raw4);
  net.close(client);
  return 0;
}}
"#
        ),
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    assert_eq!(server.join().expect("join server"), [1_u8, 2, 3, 4]);
}

#[test]
fn run_executes_map_program_end_to_end() {
    let tmp = make_temp_dir("skepac_run_map");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  let same = headers;
  map.insert(headers, "content-length", 12);
  let value = map.get(same, "content-length");
  let removed = map.remove(headers, "content-length");
  if (!map.has(same, "content-length") && map.len(headers) == 0) {
    return value + removed;
  }
  return 1;
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_eq!(output.status.code(), Some(24), "{:?}", output);
}

#[test]
fn run_reports_runtime_failure_for_missing_map_key() {
    let tmp = make_temp_dir("skepac_run_map_missing_key");
    let source = tmp.join("main.sk");
    fs::write(
        &source,
        r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  return map.get(headers, "missing");
}
"#,
    )
    .expect("write source");

    let output = Command::new(skepac_bin())
        .arg("run")
        .arg(&source)
        .output()
        .expect("run skepac run");

    assert_ne!(output.status.code(), Some(0), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing map key") || stderr.contains("MissingField"),
        "stderr was: {stderr}"
    );
}

#[test]
fn build_llvm_ir_writes_ir_artifact() {
    let tmp = make_temp_dir("skepac_build_ll");
    let source = tmp.join("main.sk");
    let out = tmp.join("main.ll");
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
        .arg("build-llvm-ir")
        .arg(&source)
        .arg(&out)
        .output()
        .expect("run skepac build-llvm-ir");

    assert!(output.status.success(), "{:?}", output);
    assert!(out.exists());
    let ir = fs::read_to_string(&out).expect("read llvm ir");
    assert!(ir.contains("define i64 @\"main\"()"));
}

#[test]
fn missing_file_fails() {
    let output = Command::new(skepac_bin())
        .arg("check")
        .arg("does_not_exist.sk")
        .output()
        .expect("run skepac");
    assert_eq!(output.status.code(), Some(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_diag_code_and_message(
        &stderr,
        "[E-MOD-NOT-FOUND][resolve]",
        "Entry module not found",
    );
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
    let (exec_name, exec_arg) = if cfg!(windows) {
        ("where.exe", "cmd")
    } else {
        ("which", "sh")
    };
    let src = format!(
        r#"
import os;
import str;
import vec;
fn main() -> Int {{
  let p = os.platform();
  let a = os.arch();
  let arg0 = os.arg(0);
  let has = os.envHas("PATH");
  let path = os.envGet("PATH");
  os.envSet("SKEPA_TMP_ENV", "ok");
  os.envRemove("SKEPA_TMP_ENV");
  os.sleep(1);
  let args: Vec[String] = vec.new();
  vec.push(args, "{exec_arg}");
  let code = os.exec("{exec_name}", args);
  let out = os.execOut("{exec_name}", args);
  if (str.len(p) > 0 && str.len(a) > 0 && str.len(arg0) > 0 && has && str.len(path) > 0 && code == 0 && str.len(out) > 0) {{
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
fn check_accepts_minimal_net_builtins_program() {
    let tmp = make_temp_dir("skepac_check_net_minimal");
    let file = tmp.join("net_minimal.sk");
    fs::write(
        &file,
        r#"
import net;
import bytes;

fn main() -> Int {
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let socket: net.Socket = net.accept(listener);
  let client: net.Socket = net.connect("127.0.0.1:8080");
  let secure: net.Socket = net.tlsConnect("example.com", 443);
  let msg: String = net.read(socket);
  let raw: Bytes = net.readBytes(socket);
  let exact: Bytes = net.readN(socket, 4);
  let local: String = net.localAddr(client);
  let peer: String = net.peerAddr(secure);
  net.write(client, msg);
  net.writeBytes(client, raw);
  net.writeBytes(client, exact);
  net.flush(client);
  net.setReadTimeout(client, 25);
  net.setWriteTimeout(client, 50);
  net.close(socket);
  net.close(client);
  net.closeListener(listener);
  if (local != peer) {
    return 0;
  }
  return 1;
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
fn check_accepts_minimal_bytes_builtins_program() {
    let tmp = make_temp_dir("skepac_check_bytes_minimal");
    let file = tmp.join("bytes_minimal.sk");
    fs::write(
        &file,
        r#"
import bytes;
import str;

fn main() -> Int {
  let raw: Bytes = bytes.fromString("hello");
  let text: String = bytes.toString(raw);
  let n: Int = bytes.len(raw);
  let piece: Bytes = bytes.slice(raw, 1, 4);
  let first: Int = bytes.get(raw, 0);
  let joined: Bytes = bytes.concat(piece, bytes.fromString("lo"));
  let pushed: Bytes = bytes.push(joined, 33);
  let appended: Bytes = bytes.append(piece, bytes.fromString("lo"));
  if (text == "hello" && str.len(text) == n && first == 104 && bytes.toString(pushed) == "ello!" && appended == joined) {
    return 0;
  }
  return 1;
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
fn check_accepts_minimal_map_builtins_program() {
    let tmp = make_temp_dir("skepac_check_map_minimal");
    let file = tmp.join("map_minimal.sk");
    fs::write(
        &file,
        r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  map.insert(headers, "content-length", 12);
  let ok: Bool = map.has(headers, "content-length");
  let n: Int = map.get(headers, "content-length");
  let gone: Int = map.remove(headers, "content-length");
  if (ok && n == gone && map.len(headers) == 0) {
    return 0;
  }
  return 1;
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
fn check_accepts_minimal_task_builtins_program() {
    let tmp = make_temp_dir("skepac_check_task_minimal");
    let file = tmp.join("task_minimal.sk");
    fs::write(
        &file,
        r#"
import task;

fn main() -> Int {
  let t: task.Task = task.__testTask();
  let c: task.Channel = task.__testChannel();
  let t2: task.Task = t;
  let c2: task.Channel = c;
  let _ = t2;
  let _ = c2;
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
fn check_accepts_minimal_typed_task_channel_program() {
    let tmp = make_temp_dir("skepac_check_task_channel");
    let file = tmp.join("task_channel.sk");
    fs::write(
        &file,
        r#"
import task;

fn main() -> Int {
  let jobs: task.Channel[Int] = task.channel();
  task.send(jobs, 5);
  return task.recv(jobs);
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
fn check_accepts_match_statement_program() {
    let tmp = make_temp_dir("skepac_check_match");
    let file = tmp.join("match_ok.sk");
    fs::write(
        &file,
        r#"
fn main() -> Int {
  let s = "Y";
  match (s) {
    "y" | "Y" => { return 1; }
    _ => { return 0; }
  }
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
fn check_accepts_vec_program() {
    let tmp = make_temp_dir("skepac_check_vec");
    let file = tmp.join("vec_ok.sk");
    fs::write(
        &file,
        r#"
import vec;
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 10);
  vec.push(xs, 20);
  vec.set(xs, 1, 30);
  let y: Int = vec.get(xs, 0);
  let z: Int = vec.delete(xs, 1);
  if (vec.len(xs) == 1 && y == 10 && z == 30) {
    return 0;
  }
  return 1;
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
fn multi_file_project_check_build_native_and_ir_work() {
    let tmp = make_temp_dir("skepac_multi");
    fs::create_dir_all(tmp.join("utils")).expect("create utils");
    let main = tmp.join("main.sk");
    let util = tmp.join("utils").join("math.sk");
    let out = tmp.join(format!("main.{}", exe_ext()));
    let ir = tmp.join("main.ll");

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
        .arg("build-native")
        .arg(&main)
        .arg(&out)
        .output()
        .expect("run build");
    assert_eq!(build.status.code(), Some(0));
    assert!(out.exists());

    let llvm_ir = Command::new(skepac_bin())
        .arg("build-llvm-ir")
        .arg(&main)
        .arg(&ir)
        .output()
        .expect("run build-llvm-ir");
    assert_eq!(llvm_ir.status.code(), Some(0));
    let text = fs::read_to_string(&ir).expect("read llvm ir");
    assert!(text.contains("define i64 @\"utils.math::add\""));
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
    let out = tmp.join(format!("main.{}", exe_ext()));
    fs::write(
        &main,
        r#"
import missing.dep;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let output = Command::new(skepac_bin())
        .arg("build-native")
        .arg(&main)
        .arg(&out)
        .output()
        .expect("run build-native");
    assert_eq!(output.status.code(), Some(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[E-MOD-NOT-FOUND][resolve]"));
}

#[test]
fn build_obj_writes_native_object_artifact() {
    let tmp = make_temp_dir("skepac_build_obj");
    let source = tmp.join("main.sk");
    let out = tmp.join(format!("main.{}", obj_ext()));
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
        .arg("build-obj")
        .arg(&source)
        .arg(&out)
        .output()
        .expect("run skepac build-obj");

    assert!(output.status.success(), "{:?}", output);
    assert!(out.exists());
}

#[test]
fn build_native_writes_executable_and_runs() {
    let tmp = make_temp_dir("skepac_build_native");
    let source = tmp.join("main.sk");
    let out = tmp.join(format!("main.{}", exe_ext()));
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
        .arg("build-native")
        .arg(&source)
        .arg(&out)
        .output()
        .expect("run skepac build-native");

    assert!(output.status.success(), "{:?}", output);
    assert!(out.exists());

    let run = Command::new(&out)
        .output()
        .expect("native executable should run");
    assert_eq!(run.status.code(), Some(7));
}

#[test]
fn build_obj_reports_toolchain_failure_cleanly() {
    let tmp = make_temp_dir("skepac_build_obj_no_toolchain");
    let source = tmp.join("main.sk");
    let out = tmp.join(format!("main.{}", obj_ext()));
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
        .arg("build-obj")
        .arg(&source)
        .arg(&out)
        .env("PATH", "")
        .output()
        .expect("run skepac build-obj");

    assert_cli_failure_class(&output, CliFailureClass::Codegen);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("native toolchain failure"));
    assert!(stderr.contains("llvm-as") || stderr.contains("llc"));
}

#[test]
fn build_native_reports_toolchain_failure_cleanly() {
    let tmp = make_temp_dir("skepac_build_native_no_toolchain");
    let source = tmp.join("main.sk");
    let out = tmp.join(format!("main.{}", exe_ext()));
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
        .arg("build-native")
        .arg(&source)
        .arg(&out)
        .env("PATH", "")
        .output()
        .expect("run skepac build-native");

    assert_cli_failure_class(&output, CliFailureClass::Codegen);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("native toolchain failure"));
    assert!(stderr.contains("llvm-as") || stderr.contains("llc") || stderr.contains("clang"));
}
