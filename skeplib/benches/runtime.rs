mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use common::{
    compile_module, run_vm, src_array_workload, src_function_call_chain, src_loop_accumulate,
    src_match_dispatch, src_string_workload, src_struct_method_workload, src_vec_workload,
};

fn bench_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm_runtime");

    let loop_mod = compile_module(&src_loop_accumulate(25_000));
    group.bench_function("loop_accumulate", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&loop_mod));
        })
    });

    let match_mod = compile_module(&src_match_dispatch(18_000));
    group.bench_function("match_dispatch", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&match_mod));
        })
    });

    let vec_mod = compile_module(&src_vec_workload(4_000));
    group.bench_function("vec_workload", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&vec_mod));
        })
    });

    let fn_mod = compile_module(&src_function_call_chain(20_000));
    group.bench_function("function_call_chain", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&fn_mod));
        })
    });

    let arr_mod = compile_module(&src_array_workload(24_000));
    group.bench_function("array_workload", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&arr_mod));
        })
    });

    let str_mod = compile_module(&src_string_workload(3_000));
    group.bench_function("string_workload", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&str_mod));
        })
    });

    let struct_mod = compile_module(&src_struct_method_workload(15_000));
    group.bench_function("struct_method_workload", |b| {
        b.iter(|| {
            let _ = run_vm(black_box(&struct_mod));
        })
    });

    group.finish();
}

criterion_group!(benches, bench_runtime);
criterion_main!(benches);

