mod common_bench;

use common_bench::{
    compile_module, median_elapsed, parse_only, run_vm, sema_only, src_array_workload,
    src_function_call_chain, src_loop_accumulate, src_match_dispatch, src_recursive_fib,
    src_string_workload, src_struct_method_workload, src_vec_workload,
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
fn perf_runtime_function_call_chain_vm() {
    let src = src_function_call_chain(20_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(20_000));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_function_call_chain_vm", median, 250);
}

#[test]
#[ignore]
fn perf_runtime_recursive_fib_vm() {
    let src = src_recursive_fib(20);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(6765));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_recursive_fib_vm", median, 250);
}

#[test]
#[ignore]
fn perf_runtime_array_workload_vm() {
    let src = src_array_workload(24_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(24_000));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_array_workload_vm", median, 250);
}

#[test]
#[ignore]
fn perf_runtime_string_workload_vm() {
    let src = src_string_workload(3_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    assert_eq!(out, Value::Int(15_000));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_string_workload_vm", median, 400);
}

#[test]
#[ignore]
fn perf_runtime_struct_method_workload_vm() {
    let src = src_struct_method_workload(15_000);
    let module = compile_module(&src);
    let out = run_vm(&module);
    // Sum over i%5 for 15000 iterations = 3000 * (0+1+2+3+4) = 30000; plus base 3 each iter = 45000
    assert_eq!(out, Value::Int(75_000));

    let median = median_elapsed(2, 8, || {
        let _ = run_vm(&module);
    });
    assert_under("runtime_struct_method_workload_vm", median, 300);
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
