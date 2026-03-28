#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFailureClass {
    Usage,
    Io,
    Parse,
    Sema,
    Codegen,
    Runtime,
}

pub fn skepac_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skepac")
}

pub fn make_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

pub fn write_temp_file(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write temp file");
    path
}

pub fn obj_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

pub fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

pub fn run_skepac(args: &[&str]) -> Output {
    run_output(Command::new(skepac_bin()).args(args)).expect("run skepac")
}

pub fn assert_cli_failure_class(output: &Output, class: CliFailureClass) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    match class {
        CliFailureClass::Usage => {
            assert_eq!(output.status.code(), Some(2), "{output:?}");
        }
        CliFailureClass::Io => {
            assert_eq!(output.status.code(), Some(3), "{output:?}");
            assert!(
                stderr.contains("Failed to read")
                    || stderr.contains("[E-MOD-IO][resolve]")
                    || stderr.contains("i/o")
                    || stderr.contains("The system cannot find"),
                "stderr was: {stderr}"
            );
        }
        CliFailureClass::Parse => {
            assert!(
                output.status.code() == Some(10) || output.status.code() == Some(15),
                "{output:?}"
            );
            assert!(
                stderr.contains("[E-PARSE][parse]") || stderr.contains("[E-PARSE][resolve]"),
                "stderr was: {stderr}"
            );
        }
        CliFailureClass::Sema => {
            assert_eq!(output.status.code(), Some(11), "{output:?}");
            assert!(stderr.contains("[E-SEMA][sema]"), "stderr was: {stderr}");
        }
        CliFailureClass::Codegen => {
            assert_eq!(output.status.code(), Some(12), "{output:?}");
            assert!(
                stderr.contains("[E-CODEGEN][codegen]"),
                "stderr was: {stderr}"
            );
        }
        CliFailureClass::Runtime => {
            assert!(
                !output.status.success(),
                "expected runtime failure but process succeeded"
            );
            assert!(
                stderr.contains("[E-RUNTIME][runtime]")
                    || stderr.is_empty()
                    || stderr.contains("out of bounds")
                    || stderr.contains("division by zero")
                    || stderr.contains("negative shift count")
                    || stderr.contains("InvalidArgument")
                    || stderr.contains("index")
                    || stderr.contains("panic"),
                "stderr was: {stderr}"
            );
        }
    }
}

pub fn assert_diag_code_and_message(stderr: &str, code: &str, message_fragment: &str) {
    if !code.is_empty() {
        assert!(stderr.contains(code), "stderr missing `{code}`: {stderr}");
    }
    assert!(
        stderr.contains(message_fragment),
        "stderr missing `{message_fragment}`: {stderr}"
    );
}

fn run_output(command: &mut Command) -> std::io::Result<Output> {
    #[cfg(windows)]
    {
        let previous = set_error_mode(SEM_NOGPFAULTERRORBOX | SEM_FAILCRITICALERRORS);
        let result = command.output();
        let _ = set_error_mode(previous);
        result
    }
    #[cfg(not(windows))]
    {
        command.output()
    }
}

#[cfg(windows)]
const SEM_FAILCRITICALERRORS: u32 = 0x0001;
#[cfg(windows)]
const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;

#[cfg(windows)]
fn set_error_mode(mode: u32) -> u32 {
    unsafe extern "system" {
        fn SetErrorMode(u_mode: u32) -> u32;
    }
    unsafe { SetErrorMode(mode) }
}
