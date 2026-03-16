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
