pub mod llvm;

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, io};

use crate::ir::IrProgram;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    Unsupported(&'static str),
    MissingBlock(String),
    InvalidIr(String),
    Io(String),
    Tool(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported codegen shape: {msg}"),
            Self::MissingBlock(name) => write!(f, "missing basic block `{name}`"),
            Self::InvalidIr(msg) => write!(f, "invalid IR for codegen: {msg}"),
            Self::Io(msg) => write!(f, "i/o failure during codegen: {msg}"),
            Self::Tool(msg) => write!(f, "native toolchain failure: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<io::Error> for CodegenError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn compile_program_to_llvm_ir(program: &IrProgram) -> Result<String, CodegenError> {
    llvm::compile_program(program)
}

pub fn write_program_llvm_ir(program: &IrProgram, path: &Path) -> Result<(), CodegenError> {
    let ir = compile_program_to_llvm_ir(program)?;
    fs::write(path, ir)?;
    Ok(())
}

pub fn compile_program_to_bitcode_file(
    program: &IrProgram,
    path: &Path,
) -> Result<(), CodegenError> {
    compile_program_to_bitcode_file_with_tool(program, path, "llvm-as")
}

fn compile_program_to_bitcode_file_with_tool(
    program: &IrProgram,
    path: &Path,
    llvm_as: &str,
) -> Result<(), CodegenError> {
    let ll_path = temp_codegen_path("module", "ll");
    write_program_llvm_ir(program, &ll_path)?;
    let result = run_tool(
        llvm_as,
        &[
            ll_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&ll_path);
    result
}

pub fn compile_program_to_object_file(
    program: &IrProgram,
    path: &Path,
) -> Result<(), CodegenError> {
    compile_program_to_object_file_with_tools(program, path, "llvm-as", "opt", "llc")
}

fn compile_program_to_object_file_with_tools(
    program: &IrProgram,
    path: &Path,
    llvm_as: &str,
    opt: &str,
    llc: &str,
) -> Result<(), CodegenError> {
    let bc_path = temp_codegen_path("module", "bc");
    let opt_bc_path = temp_codegen_path("module_opt", "bc");
    compile_program_to_bitcode_file_with_tool(program, &bc_path, llvm_as)?;
    let opt_result = run_tool(
        opt,
        &[
            "-passes=mem2reg,instcombine,simplifycfg,loop-simplify,loop-unroll",
            "-unroll-threshold=10000",
            bc_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            opt_bc_path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    if let Err(err) = opt_result {
        let _ = fs::remove_file(&bc_path);
        let _ = fs::remove_file(&opt_bc_path);
        return Err(err);
    }
    let result = run_tool(
        llc,
        &[
            "-O3",
            "-filetype=obj",
            opt_bc_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&bc_path);
    let _ = fs::remove_file(&opt_bc_path);
    result
}

pub fn compile_program_to_executable(program: &IrProgram, path: &Path) -> Result<(), CodegenError> {
    let obj_path = temp_codegen_path("module", object_extension());
    compile_program_to_object_file(program, &obj_path)?;
    let result = link_object_file_to_executable(&obj_path, path);
    let _ = fs::remove_file(&obj_path);
    result
}

pub fn link_object_file_to_executable(object_path: &Path, path: &Path) -> Result<(), CodegenError> {
    let runtime = runtime_library_path()?;
    link_object_file_to_executable_with_tool(object_path, path, &runtime, "clang")
}

fn link_object_file_to_executable_with_tool(
    object_path: &Path,
    path: &Path,
    runtime: &Path,
    clang: &str,
) -> Result<(), CodegenError> {
    let object = object_path.as_os_str().to_string_lossy().into_owned();
    let runtime = runtime.as_os_str().to_string_lossy().into_owned();
    let output = path.as_os_str().to_string_lossy().into_owned();
    let args = link_args_for_executable(&object, &runtime, &output);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_tool(clang, &args)
}

fn link_args_for_executable(object: &str, runtime: &str, output: &str) -> Vec<String> {
    let mut args = vec![object.to_string()];
    if cfg!(all(windows, target_env = "msvc")) {
        args.push(runtime.to_string());
    } else if cfg!(windows) {
        args.extend([
            "-Wl,--start-group".to_string(),
            runtime.to_string(),
            "-Wl,--end-group".to_string(),
        ]);
    } else {
        args.push(runtime.to_string());
        args.push("-no-pie".to_string());
    }
    args.extend(["-o".to_string(), output.to_string()]);
    args.extend(runtime_native_libraries().into_iter().map(str::to_string));
    args
}

fn run_tool(tool: &str, args: &[&str]) -> Result<(), CodegenError> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|err| CodegenError::Tool(format!("failed to run `{tool}`: {err}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("stdout: {stdout}; stderr: {stderr}"),
        (false, true) => format!("stdout: {stdout}"),
        (true, false) => format!("stderr: {stderr}"),
        (true, true) => "tool produced no output".to_string(),
    };
    Err(CodegenError::Tool(format!("`{tool}` failed: {detail}",)))
}

fn temp_codegen_path(name: &str, ext: &str) -> std::path::PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    let pid = std::process::id();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("skepa_codegen_{name}_{pid}_{nanos}_{seq}.{ext}"))
}

fn object_extension() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn runtime_library_path() -> Result<PathBuf, CodegenError> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| CodegenError::Tool("failed to locate workspace root".into()))?
        .to_path_buf();
    let profile = std::env::var("PROFILE")
        .ok()
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| {
                    let dir = path.parent()?;
                    let profile_dir =
                        if dir.file_name().and_then(|name| name.to_str()) == Some("deps") {
                            dir.parent()?
                        } else {
                            dir
                        };
                    profile_dir.file_name().map(|name| name.to_owned())
                })
                .and_then(|name| name.into_string().ok())
        })
        .unwrap_or_else(|| {
            if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            }
        });
    runtime_library_path_in_target_dir(&workspace_root.join("target").join(profile))
}

fn runtime_library_path_in_target_dir(target_dir: &Path) -> Result<PathBuf, CodegenError> {
    fn is_runtime_archive(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                if cfg!(windows) {
                    (name.starts_with("libskepart-") && name.ends_with(".a"))
                        || (name.starts_with("skepart-") && name.ends_with(".lib"))
                        || name == "skepart.lib"
                } else {
                    name.starts_with("libskepart-") && name.ends_with(".a")
                }
            })
            .unwrap_or(false)
    }

    let candidate_dirs = [target_dir.join("deps"), target_dir.to_path_buf()];
    let mut candidates = Vec::new();
    for dir in candidate_dirs {
        if !dir.exists() {
            continue;
        }
        let mut found = fs::read_dir(&dir)
            .map_err(|err| CodegenError::Io(err.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| is_runtime_archive(path))
            .collect::<Vec<_>>();
        candidates.append(&mut found);
    }
    candidates.sort_by(|left, right| {
        let left_key = left
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .map(std::cmp::Reverse);
        let right_key = right
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .map(std::cmp::Reverse);
        left_key
            .cmp(&right_key)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });
    if let Some(path) = candidates.into_iter().next() {
        Ok(path)
    } else {
        let deps_dir = target_dir.join("deps");
        Err(CodegenError::Tool(format!(
            "native runtime library missing under {}",
            deps_dir.display()
        )))
    }
}

fn runtime_native_libraries() -> Vec<&'static str> {
    if cfg!(windows) {
        vec![
            "-lkernel32",
            "-lntdll",
            "-luserenv",
            "-lws2_32",
            "-ldbghelp",
        ]
    } else if cfg!(target_os = "macos") {
        vec!["-framework", "Security", "-framework", "CoreFoundation"]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CodegenError, compile_program_to_bitcode_file_with_tool,
        compile_program_to_object_file_with_tools, link_args_for_executable,
        link_object_file_to_executable_with_tool, run_tool, runtime_library_path_in_target_dir,
    };
    use crate::ir;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_codegen_test_path(name: &str, ext: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("skepa_codegen_test_{name}_{nanos}.{ext}"))
    }

    fn simple_program() -> crate::ir::IrProgram {
        ir::lowering::compile_source(
            r#"
fn main() -> Int {
  return 7;
}
"#,
        )
        .expect("IR lowering should succeed")
    }

    #[test]
    fn native_link_args_disable_pie_on_non_windows() {
        let args = link_args_for_executable("input.o", "libskepart.a", "out");
        if cfg!(windows) {
            assert!(!args.iter().any(|arg| arg == "-no-pie"));
        } else {
            assert!(args.iter().any(|arg| arg == "-no-pie"));
        }
    }

    #[test]
    fn native_link_args_use_gnu_group_flags_only_on_windows_gnu() {
        let args = link_args_for_executable("input.o", "libskepart.a", "out");
        let has_start_group = args.iter().any(|arg| arg == "-Wl,--start-group");
        let has_end_group = args.iter().any(|arg| arg == "-Wl,--end-group");
        if cfg!(all(windows, target_env = "msvc")) {
            assert!(!has_start_group);
            assert!(!has_end_group);
        } else if cfg!(windows) {
            assert!(has_start_group);
            assert!(has_end_group);
        } else {
            assert!(!has_start_group);
            assert!(!has_end_group);
        }
    }

    #[test]
    fn native_link_args_include_windows_runtime_libraries_only_on_windows() {
        let args = link_args_for_executable("input.o", "libskepart.a", "out");
        let has_kernel = args.iter().any(|arg| arg == "-lkernel32");
        let has_dbghelp = args.iter().any(|arg| arg == "-ldbghelp");
        let has_security_framework = args.iter().any(|arg| arg == "Security");
        let has_core_foundation_framework = args.iter().any(|arg| arg == "CoreFoundation");
        if cfg!(windows) {
            assert!(has_kernel);
            assert!(has_dbghelp);
            assert!(!has_security_framework);
            assert!(!has_core_foundation_framework);
        } else if cfg!(target_os = "macos") {
            assert!(!has_kernel);
            assert!(!has_dbghelp);
            assert!(has_security_framework);
            assert!(has_core_foundation_framework);
        } else {
            assert!(!has_kernel);
            assert!(!has_dbghelp);
            assert!(!has_security_framework);
            assert!(!has_core_foundation_framework);
        }
    }

    #[test]
    fn missing_runtime_archive_reports_specific_deps_directory() {
        let target_dir = std::env::temp_dir().join(format!(
            "skepa_codegen_missing_runtime_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&target_dir).expect("temp target dir");
        let err =
            runtime_library_path_in_target_dir(&target_dir).expect_err("runtime should be missing");
        let _ = fs::remove_dir_all(&target_dir);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("native runtime library missing under"));
                assert!(msg.contains("deps"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn runtime_library_selection_is_deterministic_when_multiple_archives_exist() {
        let target_dir = std::env::temp_dir().join(format!(
            "skepa_codegen_runtime_pick_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let deps_dir = target_dir.join("deps");
        fs::create_dir_all(&deps_dir).expect("temp deps dir");
        let older = deps_dir.join("libskepart-aaaa1111.a");
        let newer = deps_dir.join("libskepart-zzzz9999.a");
        fs::write(&older, []).expect("older archive");
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&newer, []).expect("newer archive");
        let selected =
            runtime_library_path_in_target_dir(&target_dir).expect("runtime archive should exist");
        let _ = fs::remove_dir_all(&target_dir);
        assert_eq!(
            selected.file_name().and_then(|name| name.to_str()),
            Some("libskepart-zzzz9999.a")
        );
    }

    #[test]
    fn missing_llvm_as_reports_clean_tool_error() {
        let program = simple_program();
        let bc_path = temp_codegen_test_path("missing_llvm_as", "bc");
        let err = compile_program_to_bitcode_file_with_tool(
            &program,
            &bc_path,
            "definitely-missing-llvm-as-test-tool",
        )
        .expect_err("missing llvm-as should be reported");
        let _ = fs::remove_file(&bc_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-llvm-as-test-tool`"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn missing_llc_reports_clean_tool_error() {
        let program = simple_program();
        let obj_path =
            temp_codegen_test_path("missing_llc", if cfg!(windows) { "obj" } else { "o" });
        let err = compile_program_to_object_file_with_tools(
            &program,
            &obj_path,
            "llvm-as",
            "opt",
            "definitely-missing-llc-test-tool",
        )
        .expect_err("missing llc should be reported");
        let _ = fs::remove_file(&obj_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-llc-test-tool`"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn missing_clang_reports_clean_tool_error() {
        let object_path = temp_codegen_test_path(
            "missing_clang_input",
            if cfg!(windows) { "obj" } else { "o" },
        );
        let output_path = temp_codegen_test_path(
            "missing_clang_output",
            if cfg!(windows) { "exe" } else { "out" },
        );
        let runtime_path = temp_codegen_test_path("missing_clang_runtime", "a");
        fs::write(&object_path, []).expect("temp object file");
        fs::write(&runtime_path, []).expect("temp runtime archive");
        let err = link_object_file_to_executable_with_tool(
            &object_path,
            &output_path,
            &runtime_path,
            "definitely-missing-clang-test-tool",
        )
        .expect_err("missing clang should be reported");
        let _ = fs::remove_file(&object_path);
        let _ = fs::remove_file(&runtime_path);
        let _ = fs::remove_file(&output_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-clang-test-tool`"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn missing_opt_reports_clean_tool_error() {
        let program = simple_program();
        let obj_path =
            temp_codegen_test_path("missing_opt", if cfg!(windows) { "obj" } else { "o" });
        let err = compile_program_to_object_file_with_tools(
            &program,
            &obj_path,
            "llvm-as",
            "definitely-missing-opt-test-tool",
            "llc",
        )
        .expect_err("missing opt should be reported");
        let _ = fs::remove_file(&obj_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-opt-test-tool`"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn tool_failure_reports_stdout_and_stderr_context() {
        let err = if cfg!(windows) {
            run_tool("cmd", &["/C", "(echo out) & (echo err 1>&2) & exit /B 1"])
        } else {
            run_tool("sh", &["-c", "printf out; printf err >&2; exit 1"])
        }
        .expect_err("tool should fail");
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("stdout: out"));
                assert!(msg.contains("stderr: err"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }
}
