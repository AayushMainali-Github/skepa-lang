pub mod llvm;

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs, io,
    time::{Duration, Instant},
};

use crate::ir::IrProgram;
use crate::ir::{FunctionId, GlobalId};

#[derive(Debug, Clone)]
pub struct RuntimeLinkInputs {
    pub link_path: PathBuf,
    pub cache_inputs: Vec<(PathBuf, u128, u64)>,
}

#[derive(Debug, Clone)]
struct RuntimeLinkArtifacts {
    link_lib: PathBuf,
    sidecar_lib: Option<PathBuf>,
}

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

fn compile_program_to_llvm_ir_with_ownership(
    program: &IrProgram,
    owned_functions: std::collections::HashSet<FunctionId>,
    owned_globals: std::collections::HashSet<GlobalId>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
) -> Result<String, CodegenError> {
    llvm::compile_program_with_ownership(
        program,
        llvm::OwnershipPlan::partitioned(
            owned_functions,
            owned_globals,
            ctor_priority,
            module_init_function,
        ),
    )
}

pub fn compile_program_partition_to_llvm_ir(
    program: &IrProgram,
    owned_functions: std::collections::HashSet<FunctionId>,
    owned_globals: std::collections::HashSet<GlobalId>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
) -> Result<String, CodegenError> {
    compile_program_to_llvm_ir_with_ownership(
        program,
        owned_functions,
        owned_globals,
        ctor_priority,
        module_init_function,
    )
}

pub fn compile_program_llvm_ir_section(
    program: &IrProgram,
    section: llvm::LlvmEmitSection,
) -> Result<String, CodegenError> {
    llvm::compile_program_section(program, section)
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
    let mut timings = CodegenTimings::new("bitcode");
    let ll_path = temp_codegen_path("module", "ll");
    let emit_start = Instant::now();
    write_program_llvm_ir(program, &ll_path)?;
    timings.record("llvm_ir_emit", emit_start.elapsed());
    let assemble_start = Instant::now();
    let result = run_tool(
        llvm_as,
        &[
            ll_path.as_os_str().to_string_lossy().as_ref(),
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    timings.record("llvm_as", assemble_start.elapsed());
    let _ = fs::remove_file(&ll_path);
    timings.finish_and_print();
    result
}

pub fn compile_program_to_object_file(
    program: &IrProgram,
    path: &Path,
) -> Result<(), CodegenError> {
    compile_program_to_object_file_with_tools(program, path, "clang")
}

fn compile_program_to_object_file_with_tools(
    program: &IrProgram,
    path: &Path,
    clang: &str,
) -> Result<(), CodegenError> {
    let mut timings = CodegenTimings::new("object");
    let emit_start = Instant::now();
    let ir = compile_program_to_llvm_ir(program)?;
    timings.record("llvm_ir_emit", emit_start.elapsed());
    let clang_start = Instant::now();
    let result = run_tool_with_stdin(
        clang,
        &ir,
        &[
            "-O3",
            "-c",
            "-x",
            "ir",
            "-",
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    timings.record("clang_codegen", clang_start.elapsed());
    timings.finish_and_print();
    result
}

pub fn compile_llvm_ir_to_object_file(llvm_ir: &str, path: &Path) -> Result<(), CodegenError> {
    run_tool_with_stdin(
        "clang",
        llvm_ir,
        &[
            "-O3",
            "-c",
            "-x",
            "ir",
            "-",
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    )
}

fn compile_program_to_object_file_with_ownership(
    program: &IrProgram,
    path: &Path,
    clang: &str,
    owned_functions: std::collections::HashSet<FunctionId>,
    owned_globals: std::collections::HashSet<GlobalId>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
) -> Result<(), CodegenError> {
    let mut timings = CodegenTimings::new("object");
    let emit_start = Instant::now();
    let ir = compile_program_to_llvm_ir_with_ownership(
        program,
        owned_functions,
        owned_globals,
        ctor_priority,
        module_init_function,
    )?;
    timings.record("llvm_ir_emit", emit_start.elapsed());
    let clang_start = Instant::now();
    let result = run_tool_with_stdin(
        clang,
        &ir,
        &[
            "-O3",
            "-c",
            "-x",
            "ir",
            "-",
            "-o",
            path.as_os_str().to_string_lossy().as_ref(),
        ],
    );
    timings.record("clang_codegen", clang_start.elapsed());
    timings.finish_and_print();
    result
}

pub fn compile_program_to_executable(program: &IrProgram, path: &Path) -> Result<(), CodegenError> {
    let mut timings = CodegenTimings::new("native");
    let emit_start = Instant::now();
    let ir = compile_program_to_llvm_ir(program)?;
    timings.record("llvm_ir_emit", emit_start.elapsed());
    let clang_start = Instant::now();
    let result = compile_llvm_ir_to_executable_with_tool(&ir, path, "clang");
    timings.record("clang_native", clang_start.elapsed());
    timings.finish_and_print();
    result
}

pub fn link_object_file_to_executable(object_path: &Path, path: &Path) -> Result<(), CodegenError> {
    link_object_files_to_executable(&[object_path.to_path_buf()], path)
}

pub fn link_object_files_to_executable(
    object_paths: &[PathBuf],
    path: &Path,
) -> Result<(), CodegenError> {
    let runtime = runtime_link_artifacts()?;
    let (tool, args) = link_command_for_objects(object_paths, path, &runtime.link_lib)?;
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_tool(&tool, &args)?;
    sync_runtime_sidecars(path, &runtime)
}

pub fn link_command_for_executable(
    object_path: &Path,
    path: &Path,
    runtime: &Path,
) -> Result<(String, Vec<String>), CodegenError> {
    link_command_for_objects(&[object_path.to_path_buf()], path, runtime)
}

pub fn link_command_for_objects(
    object_paths: &[PathBuf],
    path: &Path,
    runtime: &Path,
) -> Result<(String, Vec<String>), CodegenError> {
    let objects = object_paths
        .iter()
        .map(|path| path.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let runtime = runtime.as_os_str().to_string_lossy().into_owned();
    let output = path.as_os_str().to_string_lossy().into_owned();
    let args = link_args_for_executable(&objects, &runtime, &output);
    Ok(("clang".to_string(), args))
}

#[cfg_attr(not(test), allow(dead_code))]
fn link_object_file_to_executable_with_tool(
    object_path: &Path,
    path: &Path,
    runtime: &Path,
    clang: &str,
) -> Result<(), CodegenError> {
    let object = object_path.as_os_str().to_string_lossy().into_owned();
    let runtime = runtime.as_os_str().to_string_lossy().into_owned();
    let output = path.as_os_str().to_string_lossy().into_owned();
    let args = link_args_for_executable(&[object], &runtime, &output);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_tool(clang, &args)
}

fn link_args_for_executable(objects: &[String], runtime: &str, output: &str) -> Vec<String> {
    let mut args = objects.to_vec();
    if cfg!(all(windows, target_env = "msvc")) {
        args.push(runtime.to_string());
        args.extend([
            "-Xlinker".to_string(),
            "/NODEFAULTLIB:libucrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:ucrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:vcruntime".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:msvcrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:legacy_stdio_definitions".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:oldnames".to_string(),
        ]);
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
    map_tool_output(tool, output)
}

pub fn compile_program_partition_to_object_file(
    program: &IrProgram,
    path: &Path,
    owned_functions: std::collections::HashSet<FunctionId>,
    owned_globals: std::collections::HashSet<GlobalId>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
) -> Result<(), CodegenError> {
    compile_program_to_object_file_with_ownership(
        program,
        path,
        "clang",
        owned_functions,
        owned_globals,
        ctor_priority,
        module_init_function,
    )
}

fn run_tool_with_stdin(tool: &str, stdin_text: &str, args: &[&str]) -> Result<(), CodegenError> {
    let mut child = Command::new(tool)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| CodegenError::Tool(format!("failed to run `{tool}`: {err}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CodegenError::Tool(format!("failed to open stdin for `{tool}`")))?;
    stdin
        .write_all(stdin_text.as_bytes())
        .map_err(|err| CodegenError::Tool(format!("failed to write stdin for `{tool}`: {err}")))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|err| CodegenError::Tool(format!("failed to wait for `{tool}`: {err}")))?;
    map_tool_output(tool, output)
}

fn map_tool_output(tool: &str, output: std::process::Output) -> Result<(), CodegenError> {
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

fn native_args_for_llvm_ir(input: &str, runtime: &str, output: &str) -> Vec<String> {
    let mut args = vec![
        "-O3".to_string(),
        "-x".to_string(),
        "ir".to_string(),
        input.to_string(),
        "-x".to_string(),
        "none".to_string(),
    ];
    if cfg!(all(windows, target_env = "msvc")) {
        args.push(runtime.to_string());
        args.extend([
            "-Xlinker".to_string(),
            "/NODEFAULTLIB:libucrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:ucrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:vcruntime".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:msvcrt".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:legacy_stdio_definitions".to_string(),
            "-Xlinker".to_string(),
            "/DEFAULTLIB:oldnames".to_string(),
        ]);
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

pub fn native_command_for_llvm_ir(
    llvm_ir_path: &Path,
    path: &Path,
    runtime: &Path,
) -> Result<(String, Vec<String>), CodegenError> {
    let input = llvm_ir_path.as_os_str().to_string_lossy().into_owned();
    let runtime = runtime.as_os_str().to_string_lossy().into_owned();
    let output = path.as_os_str().to_string_lossy().into_owned();
    Ok((
        "clang".to_string(),
        native_args_for_llvm_ir(&input, &runtime, &output),
    ))
}

fn compile_llvm_ir_to_executable_with_tool(
    llvm_ir: &str,
    path: &Path,
    clang: &str,
) -> Result<(), CodegenError> {
    let runtime = runtime_link_artifacts()?;
    let stdin_input = Path::new("-");
    let (_tool, args) = native_command_for_llvm_ir(stdin_input, path, &runtime.link_lib)?;
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_tool_with_stdin(clang, llvm_ir, &args)?;
    sync_runtime_sidecars(path, &runtime)
}

struct CodegenTimings {
    label: &'static str,
    enabled: bool,
    started: Instant,
    phases: Vec<(&'static str, Duration)>,
}

impl CodegenTimings {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            enabled: std::env::var_os("SKEPA_CODEGEN_TIMINGS").is_some(),
            started: Instant::now(),
            phases: Vec::new(),
        }
    }

    fn record(&mut self, phase: &'static str, elapsed: Duration) {
        if self.enabled {
            self.phases.push((phase, elapsed));
        }
    }

    fn finish_and_print(&self) {
        if !self.enabled {
            return;
        }
        for (phase, elapsed) in &self.phases {
            println!(
                "{}",
                format_timing_line(self.label, phase, elapsed.as_micros())
            );
        }
        println!(
            "{}",
            format_timing_line(self.label, "total", self.started.elapsed().as_micros())
        );
    }
}

fn format_timing_line(label: &str, phase: &str, micros: u128) -> String {
    format!("timing[codegen:{label}] {phase}={micros}us")
}

pub fn runtime_link_inputs() -> Result<RuntimeLinkInputs, CodegenError> {
    let artifacts = runtime_link_artifacts()?;
    let mut cache_inputs = vec![runtime_file_details(&artifacts.link_lib)?];
    if let Some(sidecar) = &artifacts.sidecar_lib {
        cache_inputs.push(runtime_file_details(sidecar)?);
    }
    Ok(RuntimeLinkInputs {
        link_path: artifacts.link_lib,
        cache_inputs,
    })
}

pub fn sync_runtime_sidecars_for_output(path: &Path) -> Result<(), CodegenError> {
    let runtime = runtime_link_artifacts()?;
    sync_runtime_sidecars(path, &runtime)
}

fn runtime_link_artifacts() -> Result<RuntimeLinkArtifacts, CodegenError> {
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
    runtime_link_artifacts_in_target_dir(&workspace_root.join("target").join(profile))
}

fn runtime_link_artifacts_in_target_dir(
    target_dir: &Path,
) -> Result<RuntimeLinkArtifacts, CodegenError> {
    let candidate_dirs = [target_dir.join("deps"), target_dir.to_path_buf()];
    let mut static_candidates = Vec::new();
    let mut shared_import_candidates = Vec::new();
    let mut shared_sidecar_candidates = Vec::new();
    for dir in candidate_dirs {
        if !dir.exists() {
            continue;
        }
        let found = fs::read_dir(&dir)
            .map_err(|err| CodegenError::Io(err.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        for path in found {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if cfg!(all(windows, not(target_env = "msvc"))) {
                if name.starts_with("libskepart") && name.ends_with(".dll.a") {
                    shared_import_candidates.push(path);
                    continue;
                }
                if name.starts_with("skepart") && name.ends_with(".dll") {
                    shared_sidecar_candidates.push(path);
                    continue;
                }
            } else if cfg!(all(windows, target_env = "msvc")) {
                if name.starts_with("skepart") && name.ends_with(".dll.lib") {
                    shared_import_candidates.push(path);
                    continue;
                }
                if name.starts_with("skepart") && name.ends_with(".dll") {
                    shared_sidecar_candidates.push(path);
                    continue;
                }
            }

            let is_static = if cfg!(windows) {
                (name.starts_with("libskepart-") && name.ends_with(".a"))
                    || (name.starts_with("skepart-") && name.ends_with(".lib"))
                    || name == "skepart.lib"
            } else {
                name.starts_with("libskepart-") && name.ends_with(".a")
            };
            if is_static {
                static_candidates.push(path);
            }
        }
    }

    if cfg!(windows) {
        sort_runtime_candidates(&mut shared_import_candidates);
        sort_runtime_candidates(&mut shared_sidecar_candidates);
        if let (Some(link_lib), Some(sidecar_lib)) = (
            shared_import_candidates.into_iter().next(),
            shared_sidecar_candidates.into_iter().next(),
        ) {
            return Ok(RuntimeLinkArtifacts {
                link_lib,
                sidecar_lib: Some(sidecar_lib),
            });
        }
    }

    sort_runtime_candidates(&mut static_candidates);
    if let Some(path) = static_candidates.into_iter().next() {
        Ok(RuntimeLinkArtifacts {
            link_lib: path,
            sidecar_lib: None,
        })
    } else {
        missing_runtime_error(target_dir)
    }
}

fn sort_runtime_candidates(candidates: &mut [PathBuf]) {
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
}

fn missing_runtime_error(target_dir: &Path) -> Result<RuntimeLinkArtifacts, CodegenError> {
    let deps_dir = target_dir.join("deps");
    Err(CodegenError::Tool(format!(
        "native runtime library missing under {}",
        deps_dir.display()
    )))
}

fn runtime_file_details(path: &Path) -> Result<(PathBuf, u128, u64), CodegenError> {
    let meta = fs::metadata(path).map_err(|err| CodegenError::Io(err.to_string()))?;
    let modified = meta
        .modified()
        .map_err(|err| CodegenError::Io(err.to_string()))?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| CodegenError::Io(err.to_string()))?
        .as_nanos();
    Ok((path.to_path_buf(), modified, meta.len()))
}

fn sync_runtime_sidecars(path: &Path, runtime: &RuntimeLinkArtifacts) -> Result<(), CodegenError> {
    let Some(sidecar) = &runtime.sidecar_lib else {
        return Ok(());
    };
    let sidecar_name = sidecar
        .file_name()
        .ok_or_else(|| CodegenError::Tool("runtime sidecar missing file name".into()))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|err| CodegenError::Io(err.to_string()))?;
    let destination = parent.join(sidecar_name);
    if destination == *sidecar {
        return Ok(());
    }
    if destination.exists() {
        fs::remove_file(&destination).map_err(|err| CodegenError::Io(err.to_string()))?;
    }
    match fs::hard_link(sidecar, &destination) {
        Ok(()) => {}
        Err(_) => {
            fs::copy(sidecar, &destination).map_err(|err| CodegenError::Io(err.to_string()))?;
        }
    }
    Ok(())
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
        CodegenError, RuntimeLinkArtifacts, compile_program_llvm_ir_section,
        compile_program_to_bitcode_file_with_tool, compile_program_to_llvm_ir,
        compile_program_to_object_file_with_tools, format_timing_line, link_args_for_executable,
        link_command_for_executable, link_object_file_to_executable_with_tool, run_tool,
        runtime_link_artifacts_in_target_dir, sync_runtime_sidecars,
    };
    use crate::codegen::llvm::LlvmEmitSection;
    use crate::ir;
    use std::fs;
    use std::path::PathBuf;
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
        let args = link_args_for_executable(&["input.o".to_string()], "libskepart.a", "out");
        if cfg!(windows) {
            assert!(!args.iter().any(|arg| arg == "-no-pie"));
        } else {
            assert!(args.iter().any(|arg| arg == "-no-pie"));
        }
    }

    #[test]
    fn llvm_ir_sections_reconstruct_full_program_output() {
        let program = simple_program();
        let full = compile_program_to_llvm_ir(&program).expect("full llvm ir");
        let sections = [
            compile_program_llvm_ir_section(&program, LlvmEmitSection::Module)
                .expect("module section"),
            compile_program_llvm_ir_section(&program, LlvmEmitSection::Runtime)
                .expect("runtime section"),
            compile_program_llvm_ir_section(&program, LlvmEmitSection::Functions)
                .expect("functions section"),
        ];
        assert_eq!(sections.join("\n"), full);
    }

    #[test]
    fn native_link_args_use_gnu_group_flags_only_on_windows_gnu() {
        let args = link_args_for_executable(&["input.o".to_string()], "libskepart.a", "out");
        let has_start_group = args.iter().any(|arg| arg == "-Wl,--start-group");
        let has_end_group = args.iter().any(|arg| arg == "-Wl,--end-group");
        let has_nodefaultlib_libucrt = args.iter().any(|arg| arg == "/NODEFAULTLIB:libucrt");
        if cfg!(all(windows, target_env = "msvc")) {
            assert!(!has_start_group);
            assert!(!has_end_group);
            assert!(has_nodefaultlib_libucrt);
        } else if cfg!(windows) {
            assert!(has_start_group);
            assert!(has_end_group);
            assert!(!has_nodefaultlib_libucrt);
        } else {
            assert!(!has_start_group);
            assert!(!has_end_group);
            assert!(!has_nodefaultlib_libucrt);
        }
    }

    #[test]
    fn native_link_args_include_windows_runtime_libraries_only_on_windows() {
        let args = link_args_for_executable(&["input.o".to_string()], "libskepart.a", "out");
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
        let err = runtime_link_artifacts_in_target_dir(&target_dir)
            .expect_err("runtime should be missing");
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
        let selected = runtime_link_artifacts_in_target_dir(&target_dir)
            .expect("runtime archive should exist");
        let _ = fs::remove_dir_all(&target_dir);
        assert_eq!(
            selected.link_lib.file_name().and_then(|name| name.to_str()),
            Some("libskepart-zzzz9999.a")
        );
    }

    #[test]
    fn runtime_sidecar_sync_replaces_existing_destination() {
        let temp_dir = std::env::temp_dir().join(format!(
            "skepa_codegen_sidecar_sync_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).expect("temp dir");
        let runtime_dir = temp_dir.join("runtime");
        let output_dir = temp_dir.join("out");
        fs::create_dir_all(&runtime_dir).expect("runtime dir");
        fs::create_dir_all(&output_dir).expect("output dir");

        let sidecar = runtime_dir.join("skepart.dll");
        let exe = output_dir.join(if cfg!(windows) { "app.exe" } else { "app.out" });
        let destination = output_dir.join("skepart.dll");

        fs::write(&sidecar, b"fresh").expect("sidecar");
        fs::write(&destination, b"stale").expect("existing dest");

        let runtime = RuntimeLinkArtifacts {
            link_lib: runtime_dir.join("libskepart.dll.a"),
            sidecar_lib: Some(sidecar.clone()),
        };

        sync_runtime_sidecars(&exe, &runtime).expect("sync sidecar");
        assert_eq!(fs::read(&destination).expect("destination"), b"fresh");

        let _ = fs::remove_dir_all(temp_dir);
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
        let obj_path = temp_codegen_test_path(
            "missing_clang_object_codegen",
            if cfg!(windows) { "obj" } else { "o" },
        );
        let err = compile_program_to_object_file_with_tools(
            &program,
            &obj_path,
            "definitely-missing-clang-object-test-tool",
        )
        .expect_err("missing clang object codegen should be reported");
        let _ = fs::remove_file(&obj_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-clang-object-test-tool`"));
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
    fn link_command_uses_plain_clang_driver() {
        let object_path = PathBuf::from("input.o");
        let output_path = PathBuf::from("out");
        let runtime_path = PathBuf::from("libskepart.a");
        let (tool, args) = link_command_for_executable(&object_path, &output_path, &runtime_path)
            .expect("link command");
        assert_eq!(tool, "clang");
        assert_eq!(args.first().map(String::as_str), Some("input.o"));
    }

    #[test]
    fn codegen_timing_line_format_is_stable() {
        assert_eq!(
            format_timing_line("object", "clang_codegen", 1234),
            "timing[codegen:object] clang_codegen=1234us"
        );
    }

    #[test]
    fn missing_opt_reports_clean_tool_error() {
        let program = simple_program();
        let obj_path = temp_codegen_test_path(
            "missing_object_codegen",
            if cfg!(windows) { "obj" } else { "o" },
        );
        let err = compile_program_to_object_file_with_tools(
            &program,
            &obj_path,
            "definitely-missing-object-test-tool",
        )
        .expect_err("missing object codegen tool should be reported");
        let _ = fs::remove_file(&obj_path);
        match err {
            CodegenError::Tool(msg) => {
                assert!(msg.contains("failed to run `definitely-missing-object-test-tool`"));
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
