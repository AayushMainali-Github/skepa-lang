mod common_bench;

use common_bench::{
    compile_module, median_elapsed, parse_only, run_vm, sema_only, src_array_workload,
    src_function_call_chain, src_loop_accumulate, src_match_dispatch, src_recursive_fib,
    src_string_workload, src_struct_method_workload, src_vec_workload,
};
use skeplib::bytecode::Value;

const PERF_WARMUP_RUNS: usize = 4;
const PERF_MEASURED_RUNS: usize = 15;

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
    let src = src_loop_accumulate(250_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert!(matches!(out, Value::Int(_)));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_loop_accumulate_vm", median, 650);
}

#[test]
#[ignore]
fn perf_runtime_match_dispatch_vm() {
    let src = src_match_dispatch(120_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert!(matches!(out, Value::Int(_)));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_match_dispatch_vm", median, 900);
}

#[test]
#[ignore]
fn perf_runtime_vec_workload_vm() {
    let src = src_vec_workload(24_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(36_006));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_vec_workload_vm", median, 450);
}

#[test]
#[ignore]
fn perf_runtime_function_call_chain_vm() {
    let src = src_function_call_chain(120_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(120_000));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_function_call_chain_vm", median, 950);
}

#[test]
#[ignore]
fn perf_runtime_recursive_fib_vm() {
    let src = src_recursive_fib(22);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(17_711));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_recursive_fib_vm", median, 350);
}

#[test]
#[ignore]
fn perf_runtime_array_workload_vm() {
    let src = src_array_workload(200_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(200_000));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_array_workload_vm", median, 1_200);
}

#[test]
#[ignore]
fn perf_runtime_string_workload_vm() {
    let src = src_string_workload(20_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(100_000));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_string_workload_vm", median, 650);
}

#[test]
#[ignore]
fn perf_runtime_struct_method_workload_vm() {
    let src = src_struct_method_workload(100_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(500_000));

    let median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_struct_method_workload_vm", median, 700);
}

#[test]
#[ignore]
fn perf_compile_pipeline_parse_and_sema() {
    let src = format!(
        "{}\n{}\n{}",
        src_loop_accumulate(20_000),
        src_match_dispatch(20_000),
        src_vec_workload(6_000)
    );

    let parse_median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || parse_only(&src));
    let sema_median = median_elapsed(PERF_WARMUP_RUNS, PERF_MEASURED_RUNS, || sema_only(&src));

    assert_under("compile_parse_medium", parse_median, 250);
    assert_under("compile_sema_medium", sema_median, 350);
}
