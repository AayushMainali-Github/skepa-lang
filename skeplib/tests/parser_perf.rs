use skeplib::bytecode::{Value, compile_source};
use skeplib::parser::Parser;
use skeplib::vm::Vm;

#[test]
#[ignore]
fn parses_large_generated_program_without_diagnostics() {
    let mut src = String::from("import io;\n");
    for i in 0..300 {
        src.push_str(&format!(
            "fn f{i}(x: Int) -> Int {{ if (x > 0) {{ return x - 1; }} else {{ return 0; }} }}\n"
        ));
    }
    src.push_str("fn main() -> Int {\n");
    src.push_str("  let v = 10;\n");
    for i in 0..300 {
        src.push_str(&format!("  io.println(\"tick\");\n  let v{i} = f{i}(v);\n"));
    }
    src.push_str("  return 0;\n}\n");

    let (_program, diags) = Parser::parse_source(&src);
    assert!(
        diags.is_empty(),
        "large generated input should parse cleanly: {:?}",
        diags.as_slice()
    );
}

#[test]
#[ignore]
fn vm_executes_loop_heavy_program_within_reasonable_time() {
    let src = r#"
fn main() -> Int {
  let outer = 0;
  let acc = 0;
  for (; outer < 400; outer = outer + 1) {
    let inner = 0;
    for (; inner < 400; inner = inner + 1) {
      if ((inner % 11) == 0) {
        continue;
      }
      acc = acc + 1;
    }
  }
  return acc;
}
"#;
    let module = compile_source(src).expect("compile");
    let start = std::time::Instant::now();
    let out = Vm::run_module_main(&module).expect("run");
    assert_eq!(out, Value::Int(145_200));
    assert!(
        start.elapsed().as_secs_f64() < 2.5,
        "loop-heavy VM execution regressed: {:?}",
        start.elapsed()
    );
}
