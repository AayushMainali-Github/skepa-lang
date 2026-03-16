use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::bytecode;
use skeplib::bytecode::Value;
use skeplib::ir::{self, IrInterpreter, IrValue, PrettyIr};
use skeplib::vm::Vm;

enum ExpectedValue<'a> {
    Int(i64),
    Bool(bool),
    Float(f64),
    String(&'a str),
}

fn assert_bytecode_and_ir_accept_same_source(source: &str, expected: ExpectedValue<'_>) {
    let module = bytecode::compile_source(source).expect("bytecode lowering should succeed");
    let value = Vm::run_module_main(&module).expect("bytecode vm should run source");
    match &expected {
        ExpectedValue::Int(expected) => assert_eq!(value, Value::Int(*expected)),
        ExpectedValue::Bool(expected) => assert_eq!(value, Value::Bool(*expected)),
        ExpectedValue::Float(expected) => assert_eq!(value, Value::Float(*expected)),
        ExpectedValue::String(expected) => assert_eq!(value, Value::String((*expected).into())),
    }

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    assert!(
        !program.functions.is_empty(),
        "IR lowering should emit at least one function"
    );
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run source");
    match expected {
        ExpectedValue::Int(expected) => assert_eq!(value, IrValue::Int(expected)),
        ExpectedValue::Bool(expected) => assert_eq!(value, IrValue::Bool(expected)),
        ExpectedValue::Float(expected) => assert_eq!(value, IrValue::Float(expected)),
        ExpectedValue::String(expected) => assert_eq!(value, IrValue::String(expected.into())),
    }
}

fn assert_bytecode_and_ir_accept_same_project_entry(
    entry: &std::path::Path,
    expected: ExpectedValue<'_>,
) {
    let module = bytecode::compile_project_entry(entry).expect("bytecode project compile");
    let value = Vm::run_module_main(&module).expect("bytecode vm should run project");
    match &expected {
        ExpectedValue::Int(expected) => assert_eq!(value, Value::Int(*expected)),
        ExpectedValue::Bool(expected) => assert_eq!(value, Value::Bool(*expected)),
        ExpectedValue::Float(expected) => assert_eq!(value, Value::Float(*expected)),
        ExpectedValue::String(expected) => assert_eq!(value, Value::String((*expected).into())),
    }

    let program = ir::lowering::compile_project_entry(entry).expect("project IR lowering");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    match expected {
        ExpectedValue::Int(expected) => assert_eq!(value, IrValue::Int(expected)),
        ExpectedValue::Bool(expected) => assert_eq!(value, IrValue::Bool(expected)),
        ExpectedValue::Float(expected) => assert_eq!(value, IrValue::Float(expected)),
        ExpectedValue::String(expected) => assert_eq!(value, IrValue::String(expected.into())),
    }
}

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

#[test]
fn interpret_project_entry_ir_for_cross_module_call_flow() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_exec_project_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("math.sk"),
        r#"
export { bump, seed_value };

let seed: Int = 9;

fn bump(x: Int) -> Int {
  return x + 2;
}

fn seed_value() -> Int {
  return seed;
}
"#,
    )
    .expect("math module should be written");
    fs::write(
        &entry,
        r#"
from math import bump, seed_value;

fn main() -> Int {
  return bump(seed_value()) + 1;
}
"#,
    )
    .expect("entry module should be written");

    let bytecode = bytecode::compile_project_entry(&entry).expect("bytecode project compile");
    let bytecode_value = Vm::run_module_main(&bytecode).expect("bytecode vm should run project");
    assert_eq!(bytecode_value, Value::Int(12));

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(12));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn interpret_project_entry_ir_for_cross_module_struct_method_flow() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_struct_project_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("pair.sk"),
        r#"
export { Pair, make };

struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn make() -> Pair {
  return Pair { a: 4, b: 6 };
}
"#,
    )
    .expect("pair module should be written");
    fs::write(
        &entry,
        r#"
from pair import Pair, make;

fn main() -> Int {
  let p = make();
  p.a = 7;
  return p.mix(5);
}
"#,
    )
    .expect("entry module should be written");

    let bytecode = bytecode::compile_project_entry(&entry).expect("bytecode project compile");
    let bytecode_value = Vm::run_module_main(&bytecode).expect("bytecode vm should run project");
    assert_eq!(bytecode_value, Value::Int(18));

    let program =
        ir::lowering::compile_project_entry(&entry).expect("project IR lowering should succeed");
    let ir_value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run project");
    assert_eq!(ir_value, IrValue::Int(18));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bytecode_and_ir_accept_same_project_with_globals_and_string_builtins() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for temp name")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("skepa_ir_string_project_{unique}"));
    fs::create_dir_all(&root).expect("temp project dir should be created");

    let entry = root.join("main.sk");
    fs::write(
        root.join("words.sk"),
        r#"
export { get_base, get_needle, bonus };

let base: String = "skepa-language-benchmark";
let needle: String = "bench";
let extra: Int = 1;

fn get_base() -> String {
  return base;
}

fn get_needle() -> String {
  return needle;
}

fn bonus() -> Int {
  return extra;
}
"#,
    )
    .expect("words module should be written");
    fs::write(
        &entry,
        r#"
import str;
from words import get_base, get_needle, bonus;

fn main() -> Int {
  let base = get_base();
  let needle = get_needle();
  let total = str.len(base) + str.indexOf(base, needle);
  let cut = str.slice(base, 6, 14);
  if (str.contains(cut, "language")) {
    return total + bonus();
  }
  return total;
}
"#,
    )
    .expect("entry module should be written");

    assert_bytecode_and_ir_accept_same_project_entry(&entry, ExpectedValue::Int(40));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bytecode_and_ir_accept_same_core_control_flow_source() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  let acc = 0;
  while (i < 6) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Int(15));
}

#[test]
fn bytecode_and_ir_accept_same_for_loop_source() {
    let source = r#"
fn main() -> Int {
  let acc = 0;
  for (let i = 0; i < 8; i = i + 1) {
    if (i == 2) {
      continue;
    }
    if (i == 6) {
      break;
    }
    acc = acc + i;
  }
  return acc;
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Int(13));
}

#[test]
fn bytecode_and_ir_accept_same_struct_and_method_source() {
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
  let p = Pair { a: 10, b: 5 };
  p.a = 7;
  return p.mix(4);
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Int(16));
}

#[test]
fn bytecode_and_ir_accept_same_array_and_vec_source() {
    let source = r#"
fn main() -> Int {
  let arr: [Int; 3] = [1; 3];
  arr[1] = 5;
  let xs: Vec[Int] = vec.new();
  vec.push(xs, arr[0]);
  vec.push(xs, arr[1]);
  return vec.get(xs, 0) + vec.get(xs, 1) + arr[2];
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Int(7));
}

#[test]
fn bytecode_and_ir_accept_same_string_builtin_source() {
    let source = r#"
fn main() -> Int {
  let s = "skepa-language-benchmark";
  let total = 0;
  total = total + str.len(s);
  total = total + str.indexOf(s, "bench");
  let cut = str.slice(s, 6, 14);
  if (str.contains(cut, "language")) {
    total = total + 1;
  }
  return total;
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Int(40));
}

#[test]
fn bytecode_and_ir_accept_same_float_source() {
    let source = r#"
fn main() -> Float {
  let x = 1.5;
  let y = 2.0;
  return (x + y) * 2.0;
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Float(7.0));
}

#[test]
fn bytecode_and_ir_accept_same_bool_short_circuit_source() {
    let source = r#"
fn main() -> Bool {
  let a = true;
  let b = false;
  return (a && b) || !b;
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::Bool(true));
}

#[test]
fn bytecode_and_ir_accept_same_string_builtin_output_source() {
    let source = r#"
fn main() -> String {
  let s = "alpha-beta";
  let cut = str.slice(s, 0, 5);
  if (str.contains(s, "beta")) {
    return cut + "-ok";
  }
  return "bad";
}
"#;

    assert_bytecode_and_ir_accept_same_source(source, ExpectedValue::String("alpha-ok"));
}
