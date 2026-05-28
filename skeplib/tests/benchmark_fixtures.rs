mod common;

use std::path::PathBuf;

fn bench_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("fixtures")
}

#[test]
fn heavy_single_benchmark_fixture_compiles_and_runs() {
    let src = std::fs::read_to_string(bench_fixture_root().join("heavy_single.sk"))
        .expect("read single benchmark fixture");
    assert_eq!(common::native_run_structured(&src).exit_code(), 0);
}

#[test]
fn heavy_project_benchmark_fixture_compiles_and_runs() {
    let entry = bench_fixture_root().join("heavy_project").join("main.sk");
    let output = common::native_run_project_structured(&entry);
    assert_eq!(output.exit_code(), 0);
}
