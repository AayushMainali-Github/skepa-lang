use skeplib::bytecode::{BytecodeModule, Value, compile_source};
use skeplib::parser::Parser;
use skeplib::sema::analyze_source;
use skeplib::vm::Vm;

pub fn src_loop_accumulate(iters: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iters}) {{
    acc = acc + i;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

pub fn src_match_dispatch(iters: usize) -> String {
    format!(
        r#"
fn score(x: Int) -> Int {{
  match (x % 4) {{
    0 => {{ return 1; }}
    1 | 2 => {{ return 2; }}
    _ => {{ return 3; }}
  }}
}}
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iters}) {{
    acc = acc + score(i);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

pub fn src_vec_workload(iters: usize) -> String {
    format!(
        r#"
import vec;
fn main() -> Int {{
  let i = 0;
  let xs: Vec[Int] = vec.new();
  while (i < {iters}) {{
    vec.push(xs, i);
    i = i + 1;
  }}
  vec.set(xs, 0, 7);
  let mid = vec.delete(xs, {mid});
  return vec.len(xs) + vec.get(xs, 0) + mid;
}}
"#,
        mid = iters / 2
    )
}

pub fn src_function_call_chain(iters: usize) -> String {
    format!(
        r#"
fn inc(x: Int) -> Int {{ return x + 1; }}
fn hop(x: Int) -> Int {{ return inc(x); }}
fn step(x: Int) -> Int {{ return hop(x); }}
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iters}) {{
    acc = step(acc);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

pub fn src_array_workload(iters: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let a: [Int; 8] = [0; 8];
  let i = 0;
  while (i < {iters}) {{
    let idx = i % 8;
    a[idx] = a[idx] + 1;
    i = i + 1;
  }}
  return a[0] + a[1] + a[2] + a[3] + a[4] + a[5] + a[6] + a[7];
}}
"#
    )
}

pub fn src_string_workload(iters: usize) -> String {
    format!(
        r#"
import str;
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iters}) {{
    let s = str.trim("  skepa  ");
    if (str.contains(s, "ke")) {{
      acc = acc + str.len(str.replace(s, "e", "E"));
    }}
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

pub fn src_struct_method_workload(iters: usize) -> String {
    format!(
        r#"
struct Counter {{ v: Int }}
impl Counter {{
  fn add(self, x: Int) -> Int {{
    return self.v + x;
  }}
}}
fn main() -> Int {{
  let c = Counter {{ v: 3 }};
  let i = 0;
  let acc = 0;
  while (i < {iters}) {{
    acc = acc + c.add(i % 5);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

pub fn compile_module(src: &str) -> BytecodeModule {
    compile_source(src).expect("compile bench source")
}

pub fn run_vm(module: &BytecodeModule) -> Value {
    Vm::run_module_main(module).expect("run bench module")
}

pub fn parse_only(src: &str) {
    let (_p, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "parser diags: {:?}", diags.as_slice());
}

pub fn sema_only(src: &str) {
    let (res, diags) = analyze_source(src);
    assert!(!res.has_errors, "sema diags: {:?}", diags.as_slice());
    assert!(diags.is_empty(), "sema diags: {:?}", diags.as_slice());
}

