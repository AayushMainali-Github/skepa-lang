mod common_bench;

use common_bench::{
    compile_module, median_elapsed, parse_only, run_vm, sema_only, src_loop_accumulate,
    src_match_dispatch, src_vec_workload,
};
use skeplib::bytecode::Value;

fn assert_under(label: &str, dur: std::time::Duration, max_ms: u128) {
    assert!(
        dur.as_millis() <= max_ms,
        "{label} regressed: {:?} > {}ms",
        dur,
        max_ms
    );
}

#[test]
#[ignore]
fn perf_runtime_loop_accumulate_vm() {
    let src = src_loop_accumulate(25_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert!(matches!(out, Value::Int(_)));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_loop_accumulate_vm", median, 250);
}

#[test]
#[ignore]
fn perf_runtime_match_dispatch_vm() {
    let src = src_match_dispatch(18_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert!(matches!(out, Value::Int(_)));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_match_dispatch_vm", median, 250);
}

#[test]
#[ignore]
fn perf_runtime_vec_workload_vm() {
    let src = src_vec_workload(4_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(6006));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_vec_workload_vm", median, 300);
}

#[test]
#[ignore]
fn perf_compile_pipeline_parse_and_sema() {
    let src = format!(
        "{}\n{}\n{}",
        src_loop_accumulate(2_000),
        src_match_dispatch(2_000),
        src_vec_workload(600)
    );

    let parse_median = median_elapsed(2, 8, || parse_only(&src));
    let sema_median = median_elapsed(2, 8, || sema_only(&src));

    assert_under("compile_parse_medium", parse_median, 120);
    assert_under("compile_sema_medium", sema_median, 180);
}
