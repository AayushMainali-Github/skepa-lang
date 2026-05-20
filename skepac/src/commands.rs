use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{collections::hash_map::DefaultHasher, time::UNIX_EPOCH};

use skeplib::codegen;
use skeplib::ir;
use skeplib::resolver::{ModuleGraph, ResolveError, resolve_project};
use skeplib::sema::analyze_project_entry_phased;

use crate::cli::{EXIT_CODEGEN, EXIT_IO, EXIT_OK, EXIT_PARSE, EXIT_RESOLVE, EXIT_SEMA};
use crate::output::{print_diag, print_resolve_errors};

pub fn check_file(path: &str) -> Result<i32, String> {
    match analyze_project_entry_phased(Path::new(path)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if parse_diags.is_empty() && sema_diags.is_empty() {
                println!("ok: {path}");
                return Ok(EXIT_OK as i32);
            }
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(EXIT_PARSE as i32);
            }
            for d in sema_diags.as_slice() {
                print_diag("sema", d);
            }
            Ok(EXIT_SEMA as i32)
        }
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                return Ok(EXIT_IO as i32);
            }
            print_resolve_errors(&errs);
            Ok(EXIT_RESOLVE as i32)
        }
    }
}

pub fn build_object_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let graph = match resolve_project_or_report(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    let input_path = Path::new(input);
    let output_path = Path::new(output);
    let source_fingerprint = project_source_fingerprint(&graph);
    let cache_object = cached_object_path(input_path, &source_fingerprint);
    if cache_object.exists() {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::copy(&cache_object, output_path).map_err(|err| err.to_string())?;
        println!("built object (cached): {output}");
        return Ok(EXIT_OK as i32);
    }
    let program = match compile_project_graph_or_report(&graph, input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Some(parent) = cache_object.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    if let Err(err) = codegen::compile_program_to_object_file(&program, &cache_object) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::copy(&cache_object, output_path).map_err(|err| err.to_string())?;
    println!("built object: {output}");
    Ok(EXIT_OK as i32)
}

pub fn build_native_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let graph = match resolve_project_or_report(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    let input_path = Path::new(input);
    let output_path = Path::new(output);
    let source_fingerprint = project_source_fingerprint(&graph);
    let fingerprint = native_build_fingerprint(output_path, &source_fingerprint);
    if build_output_cache_hit(output_path, &fingerprint) {
        println!("built native (cached): {output}");
        return Ok(EXIT_OK as i32);
    }
    let cache_object = cached_object_path(input_path, &source_fingerprint);
    if cache_object.exists() {
        if let Err(err) = codegen::link_object_file_to_executable(&cache_object, output_path) {
            eprintln!("[E-CODEGEN][codegen] {err}");
            return Ok(EXIT_CODEGEN as i32);
        }
        write_build_output_cache(output_path, &fingerprint);
        println!("built native (cached object): {output}");
        return Ok(EXIT_OK as i32);
    }
    let program = match compile_project_graph_or_report(&graph, input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Some(parent) = cache_object.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    if let Err(err) = codegen::compile_program_to_object_file(&program, &cache_object) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    if let Err(err) = codegen::link_object_file_to_executable(&cache_object, output_path) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    write_build_output_cache(output_path, &fingerprint);
    println!("built native: {output}");
    Ok(EXIT_OK as i32)
}

pub fn build_llvm_ir_file(input: &str, output: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    if let Err(err) = codegen::write_program_llvm_ir(&program, Path::new(output)) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    println!("built llvm ir: {output}");
    Ok(EXIT_OK as i32)
}

pub fn run_native_file(input: &str) -> Result<i32, String> {
    if let Some(code) = validate_frontend(input)? {
        return Ok(code);
    }
    let program = match compile_project_or_report(input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    let exe_path = temp_native_path();
    let _cleanup = TempPathGuard::new(exe_path.clone());
    if let Err(err) = codegen::compile_program_to_executable(&program, &exe_path) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    let output = Command::new(&exe_path).output();
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            eprintln!("[E-RUNTIME][runtime] failed to run native executable: {err}");
            return Ok(1);
        }
    };
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    let status = output.status;
    let Some(code) = status.code() else {
        eprintln!("[E-RUNTIME][runtime] native executable terminated without an exit code");
        return Ok(1);
    };
    Ok(code)
}

fn validate_frontend(input: &str) -> Result<Option<i32>, String> {
    match analyze_project_entry_phased(Path::new(input)) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Ok(Some(EXIT_PARSE as i32));
            }
            if !sema_diags.is_empty() {
                for d in sema_diags.as_slice() {
                    print_diag("sema", d);
                }
                return Ok(Some(EXIT_SEMA as i32));
            }
            Ok(None)
        }
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                return Ok(Some(EXIT_IO as i32));
            }
            print_resolve_errors(&errs);
            Ok(Some(EXIT_RESOLVE as i32))
        }
    }
}

fn has_io_resolve_error(errs: &[ResolveError]) -> bool {
    errs.iter().any(|err| err.code == "E-MOD-IO")
}

fn compile_project_or_report(input: &str) -> Result<ir::IrProgram, i32> {
    let graph = resolve_project_or_report(input)?;
    compile_project_graph_or_report(&graph, input)
}

fn resolve_project_or_report(input: &str) -> Result<ModuleGraph, i32> {
    match resolve_project(Path::new(input)) {
        Ok(graph) => Ok(graph),
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                Err(EXIT_IO as i32)
            } else {
                print_resolve_errors(&errs);
                Err(EXIT_RESOLVE as i32)
            }
        }
    }
}

fn compile_project_graph_or_report(graph: &ModuleGraph, input: &str) -> Result<ir::IrProgram, i32> {
    match ir::lowering::compile_project_graph(graph, Path::new(input)) {
        Ok(program) => Ok(program),
        Err(message) => {
            eprintln!("[E-CODEGEN][codegen] {message}");
            Err(EXIT_RESOLVE as i32)
        }
    }
}

fn project_source_fingerprint(graph: &ModuleGraph) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-source-cache-v1".hash(&mut hasher);

    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let module = &graph.modules[&id];
        id.hash(&mut hasher);
        module.path.to_string_lossy().hash(&mut hasher);
        module.source.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn native_build_fingerprint(output: &Path, source_fingerprint: &str) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-build-cache-v1".hash(&mut hasher);
    output.to_string_lossy().hash(&mut hasher);
    source_fingerprint.hash(&mut hasher);

    if let Some((runtime_path, modified, len)) = runtime_archive_fingerprint() {
        runtime_path.hash(&mut hasher);
        modified.hash(&mut hasher);
        len.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn runtime_archive_fingerprint() -> Option<(String, u128, u64)> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .to_path_buf();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let target_dir = workspace_root.join("target").join(profile);
    let candidate_dirs = [target_dir.join("deps"), target_dir];
    let mut candidates = Vec::new();
    for dir in candidate_dirs {
        if !dir.exists() {
            continue;
        }
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let matches = if cfg!(windows) {
                (name.starts_with("libskepart-") && name.ends_with(".a"))
                    || (name.starts_with("skepart-") && name.ends_with(".lib"))
                    || name == "skepart.lib"
            } else {
                name.starts_with("libskepart-") && name.ends_with(".a")
            };
            if matches {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    let path = candidates.pop()?;
    let meta = fs::metadata(&path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((path.to_string_lossy().into_owned(), modified, meta.len()))
}

fn build_output_cache_hit(output: &Path, fingerprint: &str) -> bool {
    if !output.exists() {
        return false;
    }
    let cache_path = build_output_cache_path(output);
    match fs::read_to_string(cache_path) {
        Ok(contents) => contents.trim() == fingerprint,
        Err(_) => false,
    }
}

fn write_build_output_cache(output: &Path, fingerprint: &str) {
    let cache_path = build_output_cache_path(output);
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(cache_path, fingerprint);
}

fn build_output_cache_path(output: &Path) -> PathBuf {
    let parent = output
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut hasher = DefaultHasher::new();
    output.to_string_lossy().hash(&mut hasher);
    let file_stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let safe_name = file_stem
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    parent
        .join(".skepac-cache")
        .join(format!("{safe_name}_{:016x}.fingerprint", hasher.finish()))
}

fn cached_object_path(input: &Path, source_fingerprint: &str) -> PathBuf {
    cache_root_for_input(input)
        .join("objects")
        .join(format!("{source_fingerprint}.{}", object_cache_extension()))
}

fn cache_root_for_input(input: &Path) -> PathBuf {
    input.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".skepac-cache")
}

fn object_cache_extension() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

struct TempPathGuard(PathBuf);

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn temp_native_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    let ext = if cfg!(windows) { "exe" } else { "out" };
    std::env::temp_dir().join(format!("skepac_run_{nanos}.{ext}"))
}
