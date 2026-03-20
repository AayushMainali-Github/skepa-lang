use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{BenchCase, CaseKind};

pub fn native_exec_case(
    name: &'static str,
    skepac: PathBuf,
    source: &str,
) -> Result<BenchCase, String> {
    let source_file = TempSourceFile::new(source)?;
    Ok(BenchCase {
        name,
        kind: CaseKind::Library,
        runner: Box::new(move || {
            run_runtime_command(&skepac, &["run", path_str(source_file.path())?])
        }),
    })
}

pub fn skipped_case(name: &'static str, reason: &'static str) -> BenchCase {
    BenchCase {
        name,
        kind: CaseKind::Cli,
        runner: Box::new(move || Err(format!("SKIP:{reason}"))),
    }
}

pub fn cli_tools(profile: &str) -> Result<Option<PathBuf>, String> {
    let exe_dir = env::current_exe()
        .map_err(|err| err.to_string())?
        .parent()
        .ok_or_else(|| "failed to locate current executable directory".to_string())?
        .to_path_buf();

    let expected_profile = exe_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_default();
    if expected_profile != profile {
        return Ok(None);
    }

    let skepac = exe_dir.join(exe_name("skepac"));
    if skepac.exists() {
        Ok(Some(skepac))
    } else {
        Ok(None)
    }
}

pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

pub fn run_command(exe: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {}: {err}", exe.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{} {} failed with {}: {}",
            exe.display(),
            args.join(" "),
            output.status,
            stderr.trim()
        ))
    }
}

pub struct TempSourceFile {
    dir: PathBuf,
    path: PathBuf,
}

impl TempSourceFile {
    pub fn new(source: &str) -> Result<Self, String> {
        let dir = temp_artifact_dir("bench_src");
        fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let path = dir.join("main.sk");
        fs::write(&path, source).map_err(|err| err.to_string())?;
        Ok(Self { dir, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSourceFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub fn temp_artifact_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    env::temp_dir().join(format!("skepabench_{label}_{nanos}"))
}

pub fn temp_artifact_path(label: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    env::temp_dir().join(format!("skepabench_{label}_{nanos}.{ext}"))
}

pub fn object_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

pub fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

pub fn path_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-utf8 path: {}", path.display()))
}

pub fn run_runtime_command(exe: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {}: {err}", exe.display()))?;
    validate_runtime_output(exe, args, &output)
}

pub fn validate_runtime_output(exe: &Path, args: &[&str], output: &Output) -> Result<(), String> {
    match output.status.code() {
        Some(0) => Ok(()),
        Some(code) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                Ok(())
            } else {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Err(format!(
                    "{} {} exited with {}: {}{}{}",
                    exe.display(),
                    args.join(" "),
                    code,
                    stdout.trim(),
                    if !stdout.trim().is_empty() && !stderr.trim().is_empty() {
                        " | "
                    } else {
                        ""
                    },
                    stderr.trim()
                ))
            }
        }
        None => Err(format!(
            "{} {} terminated without an exit code",
            exe.display(),
            args.join(" ")
        )),
    }
}
