use skeplib::ir::{self, IrValue, PrettyIr};

use super::common;

#[test]
fn optimizer_preserves_runtime_managed_semantics_across_arrays_vecs_structs_and_strings() {
    let source = r#"
import str;

struct Boxed {
  items: Vec[Int]
}

impl Boxed {
  fn total(self) -> Int {
    return vec.get(self.items, 0) + vec.get(self.items, 1);
  }
}

fn main() -> Int {
  let arr: [Int; 2] = [4; 2];
  let xs: Vec[Int] = vec.new();
  let alias = xs;
  vec.push(xs, arr[0]);
  vec.push(alias, arr[1] + str.len("ab"));
  let boxed = Boxed { items: xs };
  return boxed.total();
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(10));
    assert_eq!(common::native_run_exit_code_ok(source), 10);

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("VecPush"));
    assert!(printed.contains("MakeStruct"));
}

#[test]
fn optimizer_preserves_string_heavy_native_execution() {
    let source = r#"
import str;

fn main() -> Int {
  let i = 0;
  let total = 0;
  while (i < 5) {
    let s = "skepa-language-runtime";
    let cut = str.slice(s, 6, 14);
    total = total + str.len(s);
    total = total + str.indexOf(s, "time");
    if (str.contains(cut, "language")) {
      total = total + 1;
    }
    i = i + 1;
  }
  return total;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(200));
    assert_eq!(common::native_run_exit_code_ok(source), 200);
}

#[test]
fn optimizer_handles_mixed_runtime_program_without_changing_result() {
    let source = r#"
import io;
import str;

struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return ((self.a + x) * 3 + self.b) % 1000000007;
  }
}

fn step(x: Int) -> Int {
  return x + 1;
}

fn arithmetic_work(n: Int) -> Int {
  let i = 1;
  let acc = 17;
  while (i < n) {
    acc = acc + ((i * 3) % 97);
    acc = acc - (i % 11);
    acc = acc + ((acc / 3) % 29);
    i = i + 1;
  }
  return acc;
}

fn call_work(n: Int) -> Int {
  let i = 0;
  while (i < n) {
    i = step(i);
  }
  return i;
}

fn array_work(n: Int) -> Int {
  let arr: [Int; 8] = [0; 8];
  let i = 0;
  while (i < n) {
    let idx = i % 8;
    arr[idx] = arr[idx] + ((i % 7) + 1);
    i = i + 1;
  }
  return arr[0] + arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7];
}

fn string_work(n: Int) -> Int {
  let i = 0;
  let total = 0;
  while (i < n) {
    let s = "skepa-language-runtime";
    total = total + str.len(s);
    total = total + str.indexOf(s, "time");
    let cut = str.slice(s, 6, 18);
    if (str.contains(cut, "language")) {
      total = total + 1;
    }
    i = i + 1;
  }
  return total;
}

fn struct_work(n: Int) -> Int {
  let p = Pair { a: 11, b: 7 };
  let i = 0;
  let total = 0;
  while (i < n) {
    total = total + p.mix(i % 13);
    i = i + 1;
  }
  return total;
}

fn main() -> Int {
  let total = arithmetic_work(120)
    + call_work(70)
    + array_work(64)
    + string_work(12)
    + struct_work(30);
  io.printInt(total);
  io.println("");
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(0));
    assert_eq!(common::native_run_printed_int_ok(source), 8850);

    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("Copy {"));
}

#[test]
fn optimizer_keeps_runtime_managed_store_order_observable_through_aliases() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let alias = xs;
  vec.push(alias, 1);
  let first = xs;
  vec.push(alias, 2);
  let second = xs;
  return vec.len(first) * 10 + vec.len(second);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should preserve observable aliasing semantics");
    assert_eq!(value, IrValue::Int(22));
    assert_eq!(common::native_run_exit_code_ok(source), 22);
}
