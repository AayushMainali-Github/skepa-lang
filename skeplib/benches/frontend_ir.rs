use criterion::{Criterion, black_box, criterion_group, criterion_main};
use skeplib::ir::lowering::compile_source_unoptimized;
use skeplib::parser::Parser;
use skeplib::sema::analyze_source;

const CORE_SOURCE: &str = r#"
import option;
import vec;

struct Item {
  name: String,
  count: Int,
}

fn sum(items: Vec[Item]) -> Int {
  let total = 0;
  let first = option.unwrapSome(vec.get(items, 0));
  let second = option.unwrapSome(vec.get(items, 1));
  let third = option.unwrapSome(vec.get(items, 2));
  total = total + first.count;
  total = total + second.count;
  total = total + third.count;
  return total;
}

fn main() -> Int {
  let items: Vec[Item] = vec.new();
  vec.push(items, Item { name: "bolts", count: 4 });
  vec.push(items, Item { name: "nuts", count: 3 });
  vec.push(items, Item { name: "washers", count: 5 });
  return sum(items);
}
"#;

fn parser_bench(c: &mut Criterion) {
    c.bench_function("parser/core_source", |b| {
        b.iter(|| {
            let (program, diags) = Parser::parse_source(black_box(CORE_SOURCE));
            assert!(diags.is_empty(), "unexpected parser diagnostics");
            black_box(program);
        });
    });
}

fn sema_bench(c: &mut Criterion) {
    c.bench_function("sema/core_source", |b| {
        b.iter(|| {
            let (result, diags) = analyze_source(black_box(CORE_SOURCE));
            assert!(diags.is_empty(), "unexpected sema diagnostics");
            assert!(!result.has_errors, "unexpected sema error result");
            black_box(result);
        });
    });
}

fn ir_lowering_bench(c: &mut Criterion) {
    c.bench_function("ir_lowering/core_source", |b| {
        b.iter(|| {
            let ir =
                compile_source_unoptimized(black_box(CORE_SOURCE)).expect("lower valid source");
            black_box(ir);
        });
    });
}

criterion_group!(frontend_ir, parser_bench, sema_bench, ir_lowering_bench);
criterion_main!(frontend_ir);
