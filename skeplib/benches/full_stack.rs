use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use skeplib::codegen;
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
    project_benches
);
criterion_main!(full_stack);
