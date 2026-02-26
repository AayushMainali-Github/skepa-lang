mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use common::{
    compile_module, parse_only, sema_only, src_loop_accumulate, src_match_dispatch, src_vec_workload,
};

fn bench_compile(c: &mut Criterion) {
    let medium_src = format!(
        "{}\n{}\n{}",
        src_loop_accumulate(2_000),
        src_match_dispatch(2_000),
        src_vec_workload(600)
    );

    let mut group = c.benchmark_group("compile_pipeline");
    group.bench_function("parse_only", |b| {
        b.iter(|| parse_only(black_box(&medium_src)))
    });
    group.bench_function("sema_only", |b| {
        b.iter(|| sema_only(black_box(&medium_src)))
    });
    group.bench_function("compile_source", |b| {
        b.iter(|| {
            let _ = compile_module(black_box(&medium_src));
        })
    });
    group.finish();
}

criterion_group!(benches, bench_compile);
criterion_main!(benches);

