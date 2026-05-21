use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use skeplib::codegen;
use skeplib::codegen::llvm::LlvmEmitSection;
use skeplib::ir::{IrInterpreter, lowering};
use skeplib::parser::Parser;
use skeplib::sema::analyze_source;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("fixtures")
}

fn single_source() -> String {
    fs::read_to_string(fixture_root().join("heavy_single.sk")).expect("read single benchmark fixture")
}

fn project_entry() -> PathBuf {
    fixture_root().join("heavy_project").join("main.sk")
}

fn exe_ext() -> &'static str {
    if cfg!(windows) { "exe" } else { "out" }
}

fn obj_ext() -> &'static str {
    if cfg!(windows) { "obj" } else { "o" }
}

fn runtime_archive_name(name: &str) -> bool {
    if cfg!(windows) {
        (name.starts_with("libskepart-") && name.ends_with(".a"))
            || (name.starts_with("skepart-") && name.ends_with(".lib"))
            || name == "skepart.lib"
    } else {
        name.starts_with("libskepart-") && name.ends_with(".a")
    }
}

fn runtime_library_path() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        }
    });
    let target_dir = workspace_root.join("target").join(profile);
    for dir in [target_dir.join("deps"), target_dir] {
        if !dir.exists() {
            continue;
        }
        let mut candidates = fs::read_dir(&dir)
            .expect("read target dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(runtime_archive_name)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        candidates.sort();
        if let Some(path) = candidates.pop() {
            return path;
        }
    }
    panic!("runtime archive missing under target profile dir");
}

fn run_tool(tool: &str, args: &[&str]) {
    let output = Command::new(tool)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run `{tool}`: {err}"));
    assert!(
        output.status.success(),
        "`{tool}` failed: stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unique_suffix() -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = std::process::id();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{pid}_{nanos}_{seq}")
}

fn temp_artifact_path(label: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("skepa_bench_{label}_{}.{ext}", unique_suffix()))
}

fn run_output(command: &mut Command) -> std::io::Result<std::process::Output> {
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

fn parser_and_sema_benches(c: &mut Criterion) {
    let source = single_source();
    let mut group = c.benchmark_group("frontend_single_file");

    group.bench_with_input(BenchmarkId::new("parse", "heavy_single"), &source, |b, src| {
        b.iter(|| {
            let (program, diags) = Parser::parse_source(black_box(src));
            assert!(diags.is_empty(), "unexpected parser diagnostics");
            black_box(program);
        });
    });

    group.bench_with_input(BenchmarkId::new("sema", "heavy_single"), &source, |b, src| {
        b.iter(|| {
            let (result, diags) = analyze_source(black_box(src));
            assert!(diags.is_empty(), "unexpected sema diagnostics");
            assert!(!result.has_errors, "unexpected sema failure");
            black_box(result);
        });
    });

    group.finish();
}

fn ir_and_codegen_benches(c: &mut Criterion) {
    let source = single_source();
    let ir = lowering::compile_source_unoptimized(&source).expect("lower single fixture");
    let mut group = c.benchmark_group("compiler_single_file");

    group.bench_function("ir_lowering/heavy_single", |b| {
        b.iter(|| {
            let lowered =
                lowering::compile_source_unoptimized(black_box(&source)).expect("lower valid source");
            black_box(lowered);
        });
    });

    group.bench_function("ir_interpreter/heavy_single", |b| {
        b.iter(|| {
            let value = IrInterpreter::new(black_box(&ir))
                .run_main()
                .expect("interpreter should run");
            black_box(value);
        });
    });

    group.bench_function("llvm_ir_emit/heavy_single", |b| {
        b.iter(|| {
            let llvm_ir =
                codegen::compile_program_to_llvm_ir(black_box(&ir)).expect("emit llvm ir");
            black_box(llvm_ir);
        });
    });

    group.bench_function("llvm_ir_emit_module/heavy_single", |b| {
        b.iter(|| {
            let llvm_ir = codegen::compile_program_llvm_ir_section(
                black_box(&ir),
                LlvmEmitSection::Module,
            )
            .expect("emit module llvm ir section");
            black_box(llvm_ir);
        });
    });

    group.bench_function("llvm_ir_emit_runtime/heavy_single", |b| {
        b.iter(|| {
            let llvm_ir = codegen::compile_program_llvm_ir_section(
                black_box(&ir),
                LlvmEmitSection::Runtime,
            )
            .expect("emit runtime llvm ir section");
            black_box(llvm_ir);
        });
    });

    group.bench_function("llvm_ir_emit_functions/heavy_single", |b| {
        b.iter(|| {
            let llvm_ir = codegen::compile_program_llvm_ir_section(
                black_box(&ir),
                LlvmEmitSection::Functions,
            )
            .expect("emit functions llvm ir section");
            black_box(llvm_ir);
        });
    });

    group.bench_function("object_codegen/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_obj", obj_ext()),
            |path| {
                codegen::compile_program_to_object_file(black_box(&ir), &path)
                    .expect("emit object");
                let _ = fs::remove_file(path);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("native_build_and_run/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_exe", exe_ext()),
            |path| {
                codegen::compile_program_to_executable(black_box(&ir), &path)
                    .expect("build executable");
                let output = run_output(&mut Command::new(&path)).expect("run executable");
                let _ = fs::remove_file(path);
                assert_eq!(output.status.code(), Some(0), "{output:?}");
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn native_pipeline_stage_benches(c: &mut Criterion) {
    let source = single_source();
    let ir = lowering::compile_source_unoptimized(&source).expect("lower single fixture");
    let llvm_ir = codegen::compile_program_to_llvm_ir(&ir).expect("emit llvm ir");
    let ll_input = temp_artifact_path("heavy_single_input", "ll");
    fs::write(&ll_input, &llvm_ir).expect("write llvm ir input");

    let bc_input = temp_artifact_path("heavy_single_input", "bc");
    run_tool(
        "llvm-as",
        &[
            ll_input.as_os_str().to_string_lossy().as_ref(),
            "-o",
            bc_input.as_os_str().to_string_lossy().as_ref(),
        ],
    );

    let opt_bc_input = temp_artifact_path("heavy_single_input_opt", "bc");
    run_tool(
        "opt",
        &[
            "-passes=mem2reg,instcombine,simplifycfg,loop-simplify,loop-unroll",
            "-unroll-threshold=10000",
            bc_input.as_os_str().to_string_lossy().as_ref(),
            "-o",
            opt_bc_input.as_os_str().to_string_lossy().as_ref(),
        ],
    );

    let obj_input = temp_artifact_path("heavy_single_input", obj_ext());
    run_tool(
        "llc",
        &[
            "-O3",
            "-filetype=obj",
            opt_bc_input.as_os_str().to_string_lossy().as_ref(),
            "-o",
            obj_input.as_os_str().to_string_lossy().as_ref(),
        ],
    );

    let runtime = runtime_library_path();
    let exe_input = temp_artifact_path("heavy_single_stage", exe_ext());
    codegen::compile_program_to_executable(&ir, &exe_input).expect("build reusable executable");

    let mut group = c.benchmark_group("native_pipeline_single_file");

    group.bench_function("llvm_as_bitcode/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_bc", "bc"),
            |path| {
                run_tool(
                    "llvm-as",
                    &[
                        ll_input.as_os_str().to_string_lossy().as_ref(),
                        "-o",
                        path.as_os_str().to_string_lossy().as_ref(),
                    ],
                );
                let _ = fs::remove_file(path);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("opt_bitcode/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_opt_bc", "bc"),
            |path| {
                run_tool(
                    "opt",
                    &[
                        "-passes=mem2reg,instcombine,simplifycfg,loop-simplify,loop-unroll",
                        "-unroll-threshold=10000",
                        bc_input.as_os_str().to_string_lossy().as_ref(),
                        "-o",
                        path.as_os_str().to_string_lossy().as_ref(),
                    ],
                );
                let _ = fs::remove_file(path);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("llc_object_emit/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_llc_obj", obj_ext()),
            |path| {
                run_tool(
                    "llc",
                    &[
                        "-O3",
                        "-filetype=obj",
                        opt_bc_input.as_os_str().to_string_lossy().as_ref(),
                        "-o",
                        path.as_os_str().to_string_lossy().as_ref(),
                    ],
                );
                let _ = fs::remove_file(path);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("clang_link/heavy_single", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_single_link_exe", exe_ext()),
            |path| {
                let (tool, args) =
                    codegen::link_command_for_executable(&obj_input, &path, &runtime, true)
                        .expect("link command");
                let args = args.iter().map(String::as_str).collect::<Vec<_>>();
                run_tool(&tool, &args);
                let _ = fs::remove_file(path);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("binary_run/heavy_single", |b| {
        b.iter(|| {
            let output = run_output(&mut Command::new(&exe_input)).expect("run existing executable");
            assert_eq!(output.status.code(), Some(0), "{output:?}");
            black_box(output);
        });
    });

    group.finish();

    let _ = fs::remove_file(ll_input);
    let _ = fs::remove_file(bc_input);
    let _ = fs::remove_file(opt_bc_input);
    let _ = fs::remove_file(obj_input);
    let _ = fs::remove_file(exe_input);
}

fn project_benches(c: &mut Criterion) {
    let entry = project_entry();
    let ir = lowering::compile_project_entry_unoptimized(&entry)
        .expect("lower project fixture");
    let mut group = c.benchmark_group("project_full_stack");

    group.bench_function("project_ir_lowering/heavy_project", |b| {
        b.iter(|| {
            let lowered =
                lowering::compile_project_entry_unoptimized(black_box(&entry)).expect("lower project");
            black_box(lowered);
        });
    });

    group.bench_function("project_ir_interpreter/heavy_project", |b| {
        b.iter(|| {
            let value = IrInterpreter::new(black_box(&ir))
                .run_main()
                .expect("interpreter should run project");
            black_box(value);
        });
    });

    group.bench_function("project_native_build_and_run/heavy_project", |b| {
        b.iter_batched(
            || temp_artifact_path("heavy_project_exe", exe_ext()),
            |path| {
                codegen::compile_program_to_executable(black_box(&ir), &path)
                    .expect("build project executable");
                let output = run_output(&mut Command::new(&path)).expect("run project executable");
                let _ = fs::remove_file(path);
                assert_eq!(output.status.code(), Some(0), "{output:?}");
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(stdout.contains("heavy project ready"), "stdout was: {stdout}");
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    full_stack,
    parser_and_sema_benches,
    ir_and_codegen_benches,
    native_pipeline_stage_benches,
    project_benches
);
criterion_main!(full_stack);
