use skeplib::parser::Parser;

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
