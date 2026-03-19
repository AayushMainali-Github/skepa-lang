use skeplib::ir::{self, IrValue, PrettyIr};

#[test]
fn copy_prop_eliminates_simple_copy_chains() {
    let source = r#"
fn main() -> Int {
  let x = 1;
  let y = x;
  let z = y;
  let unused = z + 100;
  return z + 2;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(3));

    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("Copy {"));
    assert!(!printed.contains("Int(101)"));
}

#[test]
fn copy_prop_does_not_cross_runtime_managed_mutations() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let alias = xs;
  vec.push(xs, 4);
  vec.set(alias, 0, 9);
  return vec.get(xs, 0);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(9));

    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("VecPush"));
    assert!(printed.contains("VecSet"));
}
