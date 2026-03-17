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
    compile_program_to_object_file_with_tools(program, path, "llvm-as", "llc")
}

fn compile_program_to_object_file_with_tools(
    program: &IrProgram,
    path: &Path,
    llvm_as: &str,
    llc: &str,
) -> Result<(), CodegenError> {
    let bc_path = temp_codegen_path("module", "bc");
    compile_program_to_bitcode_file_with_tool(program, &bc_path, llvm_as)?;
    let result = run_tool(
        llc,
        &[
            "-filetype=obj",
            bc_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&bc_path);
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
    if cfg!(windows) {
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
    Err(CodegenError::Tool(format!(
        "`{tool}` failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
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
    let deps_dir = target_dir.join("deps");
    let candidates = if deps_dir.exists() {
        fs::read_dir(&deps_dir)
            .map_err(|err| CodegenError::Io(err.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("libskepart-") && name.ends_with(".a"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if let Some(path) = candidates.into_iter().next() {
        Ok(path)
    } else {
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
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CodegenError, compile_program_to_bitcode_file_with_tool,
        compile_program_to_object_file_with_tools, link_args_for_executable,
        link_object_file_to_executable_with_tool, runtime_library_path_in_target_dir,
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
    fn native_link_args_include_windows_runtime_libraries_only_on_windows() {
        let args = link_args_for_executable("input.o", "libskepart.a", "out");
        let has_kernel = args.iter().any(|arg| arg == "-lkernel32");
        let has_dbghelp = args.iter().any(|arg| arg == "-ldbghelp");
        if cfg!(windows) {
            assert!(has_kernel);
            assert!(has_dbghelp);
        } else {
            assert!(!has_kernel);
            assert!(!has_dbghelp);
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
}
