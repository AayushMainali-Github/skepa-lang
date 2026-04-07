use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::ir::{self, IrInterpreter, IrValue, PrettyIr};

#[path = "../../common.rs"]
mod common;

#[test]
fn lower_simple_function_to_ir() {
    let source = r#"
fn add_loop(n: Int) -> Int {
  let i = 0;
  let acc = 0;
  while (i < n) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert_eq!(program.functions.len(), 1);
    let func = &program.functions[0];
    assert_eq!(func.name, "add_loop");
    assert!(func.blocks.len() >= 3);
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("fn add_loop"));
    assert!(printed.contains("while_cond") || printed.contains("Branch"));
}

#[test]
fn lower_globals_and_direct_calls_to_ir() {
    let source = r#"
let seed: Int = 41;

fn inc(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let x = inc(seed);
  let y = str.len("abc");
  return x + y;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert_eq!(program.globals.len(), 1);
    assert!(program.module_init.is_some());
    assert!(program.functions.iter().any(|f| f.name == "__globals_init"));
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(45));

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("CallBuiltin"));
    assert!(printed.contains("StoreGlobal"));
}

#[test]
fn lower_assignment_targets_cover_local_array_and_struct_paths() {
    let source = r#"
struct Boxed {
  value: Int
}

fn main() -> Int {
  let arr: [Int; 2] = [0; 2];
  let b = Boxed { value: 4 };
  let seed = 3;
  arr[0] = seed;
  b.value = arr[0] + 2;
  return b.value;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("ArraySet"));
    assert!(printed.contains("StructSet"));
    assert!(printed.contains("StoreLocal"));
}

#[test]
fn lower_static_array_vec_struct_method_and_function_value_ops_to_ir() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn inc(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  import option;
  let arr: [Int; 4] = [0; 4];
  let xs: Vec[Int] = vec.new();
  let p = Pair { a: 2, b: 3 };
  let f: Fn(Int) -> Int = inc;
  arr[1] = 7;
  vec.push(xs, arr[1]);
  return f(p.mix(option.unwrapSome(vec.get(xs, 0))));
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeArrayRepeat"));
    assert!(printed.contains("ArraySet"));
    assert!(printed.contains("VecNew"));
    assert!(printed.contains("VecPush"));
    assert!(printed.contains("VecGet"));
    assert!(printed.contains("MakeStruct"));
    assert!(printed.contains("fn Pair::mix"));
    assert!(printed.contains("MakeClosure"));
    assert!(printed.contains("CallIndirect"));
}

#[test]
fn lower_short_circuit_bool_ops_to_ir() {
    let source = r#"
fn main() -> Bool {
  let a = true;
  let b = false;
  return (a && b) || a;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("sc_rhs"));
    assert!(printed.contains("sc_short"));
    assert!(printed.contains("Branch"));
}

#[test]
fn lower_for_loop_blocks_and_builtin_families_to_ir() {
    let source = r#"
import arr;
import datetime;
import io;

fn main() -> Int {
  let xs: [Int; 2] = [1; 2];
  let total = 0;
  for (let i = 0; i < arr.len(xs); i = i + 1) {
    if (i == 1) {
      continue;
    }
    total = total + i;
  }
  io.printInt(total);
  return total + datetime.nowUnix();
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("for_cond") || printed.contains("loop_cond"));
    assert!(printed.contains("for_step") || printed.contains("loop_step"));
    assert!(printed.contains("CallBuiltin"));
    assert!(printed.contains("datetime"));
    assert!(printed.contains("io"));
    assert!(printed.contains("arr"));
}

#[test]
fn lower_project_entry_to_ir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_project_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("util.sk"),
        r#"
export { inc };

fn inc(x: Int) -> Int {
  return x + 1;
}
"#,
    )
    .expect("util module should be written");
    fs::write(
        &entry,
        r#"
from util import inc;

fn main() -> Int {
  return inc(41);
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    assert!(
        program
            .functions
            .iter()
            .any(|func| func.name == "util::inc")
    );
    assert!(program.functions.iter().any(|func| func.name == "main"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn lower_project_qualified_function_value_path_to_closure_ir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_project_fn_value_{unique}"));
    fs::create_dir_all(root.join("utils")).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("utils").join("math.sk"),
        r#"
fn add(a: Int, b: Int) -> Int {
  return a + b;
}

export { add };
"#,
    )
    .expect("util module should be written");
    fs::write(
        &entry,
        r#"
import utils.math;

fn main() -> Int {
  let f: Fn(Int, Int) -> Int = utils.math.add;
  return f(20, 22);
}
"#,
    )
    .expect("entry module should be written");

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeClosure"));
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project source");
    assert_eq!(value, IrValue::Int(42));
    let _ = fs::remove_dir_all(&root);
}
