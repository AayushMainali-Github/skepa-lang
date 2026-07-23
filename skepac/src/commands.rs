use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{
    collections::HashSet,
    collections::hash_map::DefaultHasher,
    time::{Instant, UNIX_EPOCH},
};

use skeplib::codegen;
use skeplib::ir;
use skeplib::ir::{FunctionId, GlobalId};
use skeplib::resolver::{ModuleGraph, ResolveError, resolve_project};
use skeplib::sema::analyze_project_graph_phased;

use crate::cli::{EXIT_CODEGEN, EXIT_IO, EXIT_OK, EXIT_PARSE, EXIT_RESOLVE, EXIT_SEMA};
use crate::output::{print_diag, print_resolve_errors};

pub fn check_file(path: &str) -> Result<i32, String> {
    let graph = match resolve_project_or_report(path) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    match analyze_project_graph_phased(&graph) {
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
    let mut timings = BuildTimings::new("build-obj");
    let phase_start = Instant::now();
    let graph = match load_frontend_valid_graph(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    timings.record("frontend", phase_start.elapsed());
    let input_path = Path::new(input);
    let output_path = Path::new(output);
    let source_fingerprint = project_source_fingerprint(&graph);
    let cache_object = cached_object_path(input_path, &source_fingerprint);
    if cache_object.exists() {
        let copy_start = Instant::now();
        materialize_cached_artifact(&cache_object, output_path).map_err(|err| err.to_string())?;
        timings.record("copy_cached_object", copy_start.elapsed());
        println!("built object (cached): {output}");
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }
    let lower_start = Instant::now();
    let program = match compile_project_graph_or_report(&graph, input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    timings.record("ir_lowering", lower_start.elapsed());
    let ir_fingerprint = ir_program_fingerprint(&program);
    let ir_cache_object = cached_ir_object_path(input_path, &ir_fingerprint);
    if ir_cache_object.exists() {
        let copy_start = Instant::now();
        write_object_identity(&ir_cache_object, &ir_fingerprint);
        materialize_cached_artifact(&ir_cache_object, output_path)
            .map_err(|err| err.to_string())?;
        timings.record("reuse_cached_ir_object", copy_start.elapsed());
        println!("built object (cached ir): {output}");
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }
    if let Some(parent) = cache_object.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    if let Some(parent) = ir_cache_object.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let codegen_start = Instant::now();
    if let Err(err) = codegen::compile_program_to_object_file(&program, &ir_cache_object) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    timings.record("object_codegen", codegen_start.elapsed());
    write_object_identity(&ir_cache_object, &ir_fingerprint);
    store_cached_artifact(&ir_cache_object, &cache_object).map_err(|err| err.to_string())?;
    write_object_identity(&cache_object, &ir_fingerprint);
    let copy_start = Instant::now();
    materialize_cached_artifact(&ir_cache_object, output_path).map_err(|err| err.to_string())?;
    timings.record("copy_output", copy_start.elapsed());
    println!("built object: {output}");
    timings.finish_and_print();
    Ok(EXIT_OK as i32)
}

pub fn build_native_file(input: &str, output: &str) -> Result<i32, String> {
    let mut timings = BuildTimings::new("build-native");
    let phase_start = Instant::now();
    let graph = match load_frontend_valid_graph(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    timings.record("frontend", phase_start.elapsed());
    let input_path = Path::new(input);
    let output_path = Path::new(output);
    let source_fingerprint = project_source_fingerprint(&graph);
    if graph.modules.len() > 1 {
        return build_native_multi_module(&graph, input, input_path, output_path, timings);
    }
    let cache_object = cached_object_path(input_path, &source_fingerprint);
    let mut object_for_build = cache_object.clone();
    let mut object_identity = read_object_identity(&cache_object);
    let mut ir_artifact_identity: Option<String> = None;
    let mut lowered_program = None;
    let mut had_cached_object = cache_object.exists();
    if !had_cached_object {
        let lower_start = Instant::now();
        let program = match compile_project_graph_or_report(&graph, input) {
            Ok(program) => program,
            Err(code) => return Ok(code),
        };
        timings.record("ir_lowering", lower_start.elapsed());
        let ir_fingerprint = ir_program_fingerprint(&program);
        ir_artifact_identity = Some(ir_fingerprint.clone());
        let ir_cache_object = cached_ir_object_path(input_path, &ir_fingerprint);
        if ir_cache_object.exists() {
            let reuse_start = Instant::now();
            object_for_build = ir_cache_object;
            object_identity = Some(ir_fingerprint);
            timings.record("reuse_cached_ir_object", reuse_start.elapsed());
            had_cached_object = true;
        } else {
            lowered_program = Some(program);
        }
    }

    let fingerprint_start = Instant::now();
    let Some(runtime_inputs) = runtime_link_inputs() else {
        eprintln!("[E-CODEGEN][codegen] native runtime library missing");
        return Ok(EXIT_CODEGEN as i32);
    };
    let artifact_fingerprint = if let Some(ir_identity) = &ir_artifact_identity {
        native_ir_artifact_fingerprint(ir_identity, &runtime_inputs)
    } else {
        let object_identity =
            object_identity.unwrap_or_else(|| fallback_object_identity(&object_for_build));
        native_link_artifact_fingerprint(&object_identity, &runtime_inputs)
    };
    let output_fingerprint = native_output_fingerprint(output_path, &artifact_fingerprint);
    timings.record("fingerprint", fingerprint_start.elapsed());
    if build_output_cache_hit(output_path, &output_fingerprint) {
        codegen::sync_runtime_sidecars_for_output(output_path).map_err(|err| err.to_string())?;
        println!("built native (cached): {output}");
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }

    let cached_native = cached_native_path(input_path, &artifact_fingerprint);
    if cached_native.exists() {
        let copy_start = Instant::now();
        materialize_cached_artifact(&cached_native, output_path).map_err(|err| err.to_string())?;
        codegen::sync_runtime_sidecars_for_output(output_path).map_err(|err| err.to_string())?;
        write_build_output_cache(output_path, &output_fingerprint);
        timings.record("restore_cached_link", copy_start.elapsed());
        println!("built native (cached link): {output}");
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }

    if let Some(program) = lowered_program {
        let codegen_start = Instant::now();
        prepare_output_path(output_path).map_err(|err| err.to_string())?;
        if let Err(err) = codegen::compile_program_to_executable(&program, output_path) {
            eprintln!("[E-CODEGEN][codegen] {err}");
            return Ok(EXIT_CODEGEN as i32);
        }
        timings.record("native_codegen", codegen_start.elapsed());
    } else {
        let link_start = Instant::now();
        prepare_output_path(output_path).map_err(|err| err.to_string())?;
        if let Err(err) = codegen::link_object_file_to_executable(&object_for_build, output_path) {
            eprintln!("[E-CODEGEN][codegen] {err}");
            return Ok(EXIT_CODEGEN as i32);
        }
        timings.record("native_link", link_start.elapsed());
    }
    let copy_start = Instant::now();
    store_cached_artifact(output_path, &cached_native).map_err(|err| err.to_string())?;
    write_build_output_cache(output_path, &output_fingerprint);
    timings.record("store_cached_link", copy_start.elapsed());
    if had_cached_object {
        println!("built native (cached object): {output}");
    } else {
        println!("built native: {output}");
    }
    timings.finish_and_print();
    Ok(EXIT_OK as i32)
}

fn build_native_multi_module(
    graph: &ModuleGraph,
    input: &str,
    input_path: &Path,
    output_path: &Path,
    mut timings: BuildTimings,
) -> Result<i32, String> {
    let lower_start = Instant::now();
    let mut program = match compile_project_graph_unoptimized_or_report(graph, input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    // Apply the shared opt pipeline, but skip inlining so module partitions stay
    // independently cacheable (cross-module inlining couples fingerprints).
    ir::opt::optimize_program_for_partitions(&mut program);
    timings.record("ir_lowering", lower_start.elapsed());

    let partition_start = Instant::now();
    let partitions = project_native_partitions(graph, &program);
    let runtime_inputs = match runtime_link_inputs() {
        Some(inputs) => inputs,
        None => {
            eprintln!("[E-CODEGEN][codegen] native runtime library missing");
            return Ok(EXIT_CODEGEN as i32);
        }
    };
    timings.record("partition_plan", partition_start.elapsed());

    let mut object_paths = Vec::with_capacity(partitions.len());
    let mut object_identities = Vec::with_capacity(partitions.len());
    let mut reused_count = 0usize;
    let mut compiled = std::time::Duration::ZERO;
    for partition in &partitions {
        let llvm_ir = match codegen::compile_program_partition_to_llvm_ir(
            &program,
            partition.owned_functions.clone(),
            partition.owned_globals.clone(),
            partition.ctor_priority,
            partition.module_init_function,
        ) {
            Ok(ir) => ir,
            Err(err) => {
                eprintln!("[E-CODEGEN][codegen] {err}");
                return Ok(EXIT_CODEGEN as i32);
            }
        };
        let fingerprint = text_fingerprint(&llvm_ir);
        let cache_object = cached_partition_object_path(input_path, &partition.label, &fingerprint);
        maybe_write_partition_debug_ir(input_path, &partition.label, &llvm_ir)
            .map_err(|err| err.to_string())?;
        if cache_object.exists() {
            object_paths.push(cache_object);
            object_identities.push(fingerprint);
            reused_count += 1;
            continue;
        }
        let compile_start = Instant::now();
        if let Some(parent) = cache_object.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        if let Err(err) = codegen::compile_llvm_ir_to_object_file(&llvm_ir, &cache_object) {
            eprintln!("[E-CODEGEN][codegen] {err}");
            return Ok(EXIT_CODEGEN as i32);
        }
        compiled += compile_start.elapsed();
        object_paths.push(cache_object);
        object_identities.push(fingerprint);
    }
    if !compiled.is_zero() {
        timings.record("module_object_codegen", compiled);
    }
    if reused_count != 0 {
        timings.record_count("reused_cached_module_objects", reused_count);
    }

    let fingerprint_start = Instant::now();
    let artifact_fingerprint =
        native_link_artifact_fingerprint_many(&object_identities, &runtime_inputs);
    let output_fingerprint = native_output_fingerprint(output_path, &artifact_fingerprint);
    timings.record("fingerprint", fingerprint_start.elapsed());
    if build_output_cache_hit(output_path, &output_fingerprint) {
        codegen::sync_runtime_sidecars_for_output(output_path).map_err(|err| err.to_string())?;
        println!("built native (cached): {}", output_path.display());
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }

    let cached_native = cached_native_path(input_path, &artifact_fingerprint);
    if cached_native.exists() {
        let copy_start = Instant::now();
        materialize_cached_artifact(&cached_native, output_path).map_err(|err| err.to_string())?;
        codegen::sync_runtime_sidecars_for_output(output_path).map_err(|err| err.to_string())?;
        write_build_output_cache(output_path, &output_fingerprint);
        timings.record("restore_cached_link", copy_start.elapsed());
        println!("built native (cached link): {}", output_path.display());
        timings.finish_and_print();
        return Ok(EXIT_OK as i32);
    }

    let link_start = Instant::now();
    prepare_output_path(output_path).map_err(|err| err.to_string())?;
    if let Err(err) = codegen::link_object_files_to_executable(&object_paths, output_path) {
        eprintln!("[E-CODEGEN][codegen] {err}");
        return Ok(EXIT_CODEGEN as i32);
    }
    timings.record("native_link", link_start.elapsed());

    let copy_start = Instant::now();
    store_cached_artifact(output_path, &cached_native).map_err(|err| err.to_string())?;
    write_build_output_cache(output_path, &output_fingerprint);
    timings.record("store_cached_link", copy_start.elapsed());
    println!("built native (partitioned): {}", output_path.display());
    timings.finish_and_print();
    Ok(EXIT_OK as i32)
}

pub fn build_llvm_ir_file(input: &str, output: &str) -> Result<i32, String> {
    let graph = match load_frontend_valid_graph(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    let program = match compile_project_graph_or_report(&graph, input) {
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
    let graph = match load_frontend_valid_graph(input) {
        Ok(graph) => graph,
        Err(code) => return Ok(code),
    };
    let program = match compile_project_graph_or_report(&graph, input) {
        Ok(program) => program,
        Err(code) => return Ok(code),
    };
    let exe_path = temp_native_path();
    if let Some(parent) = exe_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
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

fn load_frontend_valid_graph(input: &str) -> Result<ModuleGraph, i32> {
    let graph = resolve_project_or_report(input)?;
    match analyze_project_graph_phased(&graph) {
        Ok((_sema, parse_diags, sema_diags)) => {
            if !parse_diags.is_empty() {
                for d in parse_diags.as_slice() {
                    print_diag("parse", d);
                }
                return Err(EXIT_PARSE as i32);
            }
            if !sema_diags.is_empty() {
                for d in sema_diags.as_slice() {
                    print_diag("sema", d);
                }
                return Err(EXIT_SEMA as i32);
            }
            Ok(graph)
        }
        Err(errs) => {
            if has_io_resolve_error(&errs) {
                print_resolve_errors(&errs);
                return Err(EXIT_IO as i32);
            }
            print_resolve_errors(&errs);
            Err(EXIT_RESOLVE as i32)
        }
    }
}

fn has_io_resolve_error(errs: &[ResolveError]) -> bool {
    errs.iter().any(|err| err.code == "E-MOD-IO")
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
    match ir::lowering::compile_project_graph_after_frontend(graph, Path::new(input)) {
        Ok(program) => Ok(program),
        Err(message) => {
            eprintln!("[E-CODEGEN][codegen] {message}");
            Err(EXIT_CODEGEN as i32)
        }
    }
}

fn compile_project_graph_unoptimized_or_report(
    graph: &ModuleGraph,
    input: &str,
) -> Result<ir::IrProgram, i32> {
    match ir::lowering::compile_project_graph_after_frontend_unoptimized(graph, Path::new(input)) {
        Ok(program) => Ok(program),
        Err(message) => {
            eprintln!("[E-CODEGEN][codegen] {message}");
            Err(EXIT_CODEGEN as i32)
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

#[derive(Clone)]
struct ProjectNativePartition {
    label: String,
    owned_functions: HashSet<FunctionId>,
    owned_globals: HashSet<GlobalId>,
    ctor_priority: u32,
    module_init_function: Option<FunctionId>,
}

fn project_native_partitions(
    graph: &ModuleGraph,
    program: &ir::IrProgram,
) -> Vec<ProjectNativePartition> {
    let mut partitions = Vec::new();
    let mut module_ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    module_ids.sort();
    for module_id in module_ids {
        let owned_functions = program
            .functions
            .iter()
            .filter(|func| func.name.starts_with(&format!("{module_id}::")))
            .map(|func| func.id)
            .collect::<HashSet<_>>();
        let owned_globals = program
            .globals
            .iter()
            .filter(|global| global.name.starts_with(&format!("{module_id}::")))
            .map(|global| global.id)
            .collect::<HashSet<_>>();
        partitions.push(ProjectNativePartition {
            label: module_id.clone(),
            owned_functions,
            owned_globals,
            ctor_priority: 65_534,
            module_init_function: None,
        });
    }

    let wrapper_functions = program
        .functions
        .iter()
        .filter(|func| func.name == "__globals_init" || func.name == "main")
        .map(|func| func.id)
        .collect::<HashSet<_>>();
    let wrapper_module_init = program.module_init.as_ref().map(|init| init.function);
    partitions.push(ProjectNativePartition {
        label: "__wrapper".to_string(),
        owned_functions: wrapper_functions,
        owned_globals: HashSet::new(),
        ctor_priority: 65_535,
        module_init_function: wrapper_module_init,
    });

    partitions
}

fn native_link_artifact_fingerprint(
    object_identity: &str,
    runtime: &codegen::RuntimeLinkInputs,
) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-link-artifact-cache-v4".hash(&mut hasher);
    object_identity.hash(&mut hasher);
    for (path, modified, len) in &runtime.cache_inputs {
        path.to_string_lossy().hash(&mut hasher);
        modified.hash(&mut hasher);
        len.hash(&mut hasher);
    }
    let placeholder_object = Path::new("__skepa_cached_object__");
    let placeholder_output = Path::new("__skepa_cached_output__");
    if let Ok((tool, args)) = codegen::link_command_for_executable(
        placeholder_object,
        placeholder_output,
        &runtime.link_path,
    ) {
        tool.hash(&mut hasher);
        for arg in normalized_link_args(args) {
            arg.hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

fn native_link_artifact_fingerprint_many(
    object_identities: &[String],
    runtime: &codegen::RuntimeLinkInputs,
) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-link-artifact-cache-v5".hash(&mut hasher);
    for identity in object_identities {
        identity.hash(&mut hasher);
    }
    for (path, modified, len) in &runtime.cache_inputs {
        path.to_string_lossy().hash(&mut hasher);
        modified.hash(&mut hasher);
        len.hash(&mut hasher);
    }
    let placeholder_objects = [PathBuf::from("__skepa_cached_object__.o")];
    let placeholder_output = Path::new("__skepa_cached_output__");
    if let Ok((tool, args)) = codegen::link_command_for_objects(
        &placeholder_objects,
        placeholder_output,
        &runtime.link_path,
    ) {
        tool.hash(&mut hasher);
        for arg in normalized_link_args(args) {
            arg.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

fn native_ir_artifact_fingerprint(
    ir_identity: &str,
    runtime: &codegen::RuntimeLinkInputs,
) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-ir-artifact-cache-v2".hash(&mut hasher);
    ir_identity.hash(&mut hasher);
    for (path, modified, len) in &runtime.cache_inputs {
        path.to_string_lossy().hash(&mut hasher);
        modified.hash(&mut hasher);
        len.hash(&mut hasher);
    }
    let placeholder_input = Path::new("__skepa_cached_input__.ll");
    let placeholder_output = Path::new("__skepa_cached_output__");
    if let Ok((tool, args)) = codegen::native_command_for_llvm_ir(
        placeholder_input,
        placeholder_output,
        &runtime.link_path,
    ) {
        tool.hash(&mut hasher);
        for arg in normalized_link_args(args) {
            arg.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

fn native_output_fingerprint(output: &Path, artifact_fingerprint: &str) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-native-output-cache-v1".hash(&mut hasher);
    output.to_string_lossy().hash(&mut hasher);
    artifact_fingerprint.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn prepare_output_path(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn materialize_cached_artifact(source: &Path, destination: &Path) -> std::io::Result<()> {
    prepare_output_path(destination)?;
    match fs::hard_link(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, destination)?;
            Ok(())
        }
    }
}

fn store_cached_artifact(source: &Path, cache_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if cache_path.exists() {
        fs::remove_file(cache_path)?;
    }
    match fs::hard_link(source, cache_path) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, cache_path)?;
            Ok(())
        }
    }
}

fn normalized_link_args(args: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(args.len());
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-o" {
            normalized.push(arg);
            normalized.push("<output>".to_string());
            skip_next = true;
            continue;
        }
        normalized.push(arg);
    }
    normalized
}

fn runtime_link_inputs() -> Option<codegen::RuntimeLinkInputs> {
    codegen::runtime_link_inputs().ok()
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

fn cached_ir_object_path(input: &Path, ir_fingerprint: &str) -> PathBuf {
    cache_root_for_input(input)
        .join("objects-ir")
        .join(format!("{ir_fingerprint}.{}", object_cache_extension()))
}

fn cached_native_path(input: &Path, link_fingerprint: &str) -> PathBuf {
    cache_root_for_input(input)
        .join("native")
        .join(format!("{link_fingerprint}.{}", native_cache_extension()))
}

fn cached_partition_object_path(input: &Path, label: &str, fingerprint: &str) -> PathBuf {
    let safe_label = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    cache_root_for_input(input)
        .join("partition-objects")
        .join(format!(
            "{safe_label}_{fingerprint}.{}",
            object_cache_extension()
        ))
}

fn maybe_write_partition_debug_ir(input: &Path, label: &str, llvm_ir: &str) -> std::io::Result<()> {
    if std::env::var_os("SKEPAC_DEBUG_PARTITION_LLVM").is_none() {
        return Ok(());
    }
    let safe_label = label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let path = cache_root_for_input(input)
        .join("partition-llvm")
        .join(format!("{safe_label}.ll"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, llvm_ir)
}

fn object_identity_path(object_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.identity", object_path.to_string_lossy()))
}

fn write_object_identity(object_path: &Path, identity: &str) {
    let identity_path = object_identity_path(object_path);
    if let Some(parent) = identity_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(identity_path, identity);
}

fn read_object_identity(object_path: &Path) -> Option<String> {
    fs::read_to_string(object_identity_path(object_path))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn fallback_object_identity(object_path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-object-identity-fallback-v1".hash(&mut hasher);
    if let Ok(bytes) = fs::read(object_path) {
        bytes.hash(&mut hasher);
    } else if let Ok(meta) = fs::metadata(object_path) {
        if let Ok(modified) = meta.modified()
            && let Ok(duration) = modified.duration_since(UNIX_EPOCH)
        {
            duration.as_nanos().hash(&mut hasher);
        }
        meta.len().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn cache_root_for_input(input: &Path) -> PathBuf {
    input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".skepac-cache")
}

fn object_cache_extension() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn native_cache_extension() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

fn ir_program_fingerprint(program: &ir::IrProgram) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-ir-object-cache-v1".hash(&mut hasher);
    format!("{program:#?}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn text_fingerprint(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    "skepac-text-fingerprint-v1".hash(&mut hasher);
    canonicalize_llvm_ssa_names(text).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn canonicalize_llvm_ssa_names(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut temp_ids = std::collections::HashMap::<String, usize>::new();
    let mut value_ids = std::collections::HashMap::<String, usize>::new();
    let mut block_ids = std::collections::HashMap::<String, usize>::new();

    while i < bytes.len() {
        let Some(prefix) = bytes.get(i..i + 2) else {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        };
        let is_temp = prefix == b"%t";
        let is_value = prefix == b"%v";
        if is_temp || is_value {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 2 {
                let name = &text[i..j];
                let next_index = if is_temp {
                    let id = temp_ids.len();
                    *temp_ids.entry(name.to_string()).or_insert(id)
                } else {
                    let id = value_ids.len();
                    *value_ids.entry(name.to_string()).or_insert(id)
                };

                if is_temp {
                    out.push_str(&format!("%t{next_index}"));
                } else {
                    out.push_str(&format!("%v{next_index}"));
                }
                i = j;
                continue;
            }
        }

        if bytes[i] == b'b' && bytes.get(i + 1) == Some(&b'b') {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 2 && bytes.get(j) == Some(&b'_') {
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
                    k += 1;
                }
                let name = &text[i..k];
                let suffix = &text[j..k];
                let id = block_ids.len();
                let next_index = *block_ids.entry(name.to_string()).or_insert(id);
                out.push_str(&format!("bb{next_index}{suffix}"));
                i = k;
                continue;
            }
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

struct BuildTimings {
    label: &'static str,
    enabled: bool,
    started: Instant,
    phases: Vec<(&'static str, u128)>,
    counters: Vec<(&'static str, usize)>,
}

impl BuildTimings {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            enabled: std::env::var_os("SKEPAC_TIMINGS").is_some(),
            started: Instant::now(),
            phases: Vec::new(),
            counters: Vec::new(),
        }
    }

    fn record(&mut self, phase: &'static str, elapsed: std::time::Duration) {
        if self.enabled {
            self.phases.push((phase, elapsed.as_micros()));
        }
    }

    fn record_count(&mut self, phase: &'static str, value: usize) {
        if self.enabled {
            self.counters.push((phase, value));
        }
    }

    fn finish_and_print(&self) {
        if !self.enabled {
            return;
        }
        for (phase, micros) in &self.phases {
            println!("timing[{}] {}={}us", self.label, phase, micros);
        }
        for (phase, value) in &self.counters {
            println!("timing[{}] {}={}", self.label, phase, value);
        }
        println!(
            "timing[{}] total={}us",
            self.label,
            self.started.elapsed().as_micros()
        );
    }
}

struct TempPathGuard(PathBuf);

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        if let Some(parent) = self.0.parent() {
            let _ = fs::remove_dir_all(parent);
        } else {
            let _ = fs::remove_file(&self.0);
        }
    }
}

fn temp_native_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    let ext = if cfg!(windows) { "exe" } else { "out" };
    std::env::temp_dir()
        .join(format!("skepac_run_{nanos}"))
        .join(format!("main.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_llvm_ssa_names, materialize_cached_artifact, prepare_output_path,
        store_cached_artifact, text_fingerprint,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("skepac_commands_{name}_{nanos}"))
    }

    #[test]
    fn cached_artifact_helpers_replace_existing_files() {
        let dir = temp_test_dir("cached_artifact_helpers");
        fs::create_dir_all(&dir).expect("temp dir");
        let source = dir.join("source.bin");
        let destination = dir.join("destination.bin");
        let cache = dir.join("cache.bin");

        fs::write(&source, b"fresh").expect("source");
        fs::write(&destination, b"stale").expect("destination");
        fs::write(&cache, b"old").expect("cache");

        materialize_cached_artifact(&source, &destination).expect("materialize");
        store_cached_artifact(&source, &cache).expect("store");

        assert_eq!(fs::read(&destination).expect("destination read"), b"fresh");
        assert_eq!(fs::read(&cache).expect("cache read"), b"fresh");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepare_output_path_removes_existing_file() {
        let dir = temp_test_dir("prepare_output_path");
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("output.bin");
        fs::write(&path, b"old").expect("output");
        prepare_output_path(&path).expect("prepare");
        assert!(!path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn llvm_ssa_name_canonicalization_stabilizes_fingerprint_noise() {
        let first = "define i64 @main() {\nbb13_entry:\n  %t2 = call i64 @foo()\n  %v8 = add i64 %t2, 1\n  br label %bb14_exit\nbb14_exit:\n  ret i64 %v8\n}\n";
        let second = "define i64 @main() {\nbb16_entry:\n  %t4 = call i64 @foo()\n  %v11 = add i64 %t4, 1\n  br label %bb19_exit\nbb19_exit:\n  ret i64 %v11\n}\n";

        assert_eq!(
            canonicalize_llvm_ssa_names(first),
            canonicalize_llvm_ssa_names(second)
        );
        assert_eq!(text_fingerprint(first), text_fingerprint(second));
    }
}
