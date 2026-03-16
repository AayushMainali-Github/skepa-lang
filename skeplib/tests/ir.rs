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
