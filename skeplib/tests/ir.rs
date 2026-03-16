use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::ir::{self, PrettyIr};

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

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("CallDirect"));
    assert!(printed.contains("CallBuiltin"));
    assert!(printed.contains("StoreGlobal"));
}

#[test]
fn lower_static_array_ops_to_ir() {
    let source = r#"
fn main() -> Int {
  let arr: [Int; 4] = [0; 4];
  arr[1] = 7;
  arr[2] = arr[1] + 3;
  return arr[2];
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeArrayRepeat"));
    assert!(printed.contains("ArraySet"));
    assert!(printed.contains("ArrayGet"));
}

#[test]
fn lower_struct_literal_and_field_ops_to_ir() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

fn main() -> Int {
  let p = Pair { a: 2, b: 3 };
  p.a = 7;
  return p.a + p.b;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert_eq!(program.structs.len(), 1);
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeStruct"));
    assert!(printed.contains("StructSet"));
    assert!(printed.contains("StructGet"));
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
fn lower_named_function_values_and_indirect_calls_to_ir() {
    let source = r#"
fn inc(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let f = inc;
  return f(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeClosure"));
    assert!(printed.contains("CallIndirect"));
}

#[test]
fn lower_non_capturing_function_literals_to_ir() {
    let source = r#"
fn main() -> Int {
  let f = fn(x: Int) -> Int {
    return x + 2;
  };
  return f(5);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert!(
        program
            .functions
            .iter()
            .any(|func| func.name.starts_with("__fn_lit_"))
    );
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("MakeClosure"));
    assert!(printed.contains("CallIndirect"));
}

#[test]
fn lower_vec_ops_to_ir() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  vec.push(xs, 10);
  vec.push(xs, 20);
  vec.set(xs, 1, 30);
  let first = vec.get(xs, 0);
  let removed = vec.delete(xs, 1);
  return first + removed + vec.len(xs);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("VecNew"));
    assert!(printed.contains("VecPush"));
    assert!(printed.contains("VecSet"));
    assert!(printed.contains("VecGet"));
    assert!(printed.contains("VecDelete"));
    assert!(printed.contains("VecLen"));
}

#[test]
fn lower_struct_method_calls_to_ir() {
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

fn main() -> Int {
  let p = Pair { a: 2, b: 3 };
  return p.mix(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert!(
        program
            .functions
            .iter()
            .any(|func| func.name == "Pair::mix")
    );
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("fn Pair::mix"));
    let main_fn = program
        .functions
        .iter()
        .find(|func| func.name == "main")
        .expect("main should be lowered");
    assert!(main_fn.blocks.iter().any(|block| {
        block
            .instrs
            .iter()
            .any(|instr| matches!(instr, ir::Instr::CallDirect { .. }))
    }));
    assert!(!main_fn.blocks.iter().any(|block| {
        block
            .instrs
            .iter()
            .any(|instr| matches!(instr, ir::Instr::CallBuiltin { .. }))
    }));
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

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("fn util::inc"));
    assert!(printed.contains("fn main"));

    let _ = fs::remove_dir_all(&root);
}
