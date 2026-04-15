use skeplib::ir::{PrettyIr, lowering};

#[test]
fn extern_call_lowering_closes_library_on_bind_failure_path() {
    let source = r#"
extern("test-lib") fn strlen(s: String) -> Int;

fn main() -> Int {
  return strlen("abc");
}
"#;

    let program = lowering::compile_source_unoptimized(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();

    assert!(printed.contains("extern_bind_err"));
    assert!(printed.contains("name: \"isErr\""));
    assert!(printed.contains("name: \"closeLibrary\""));
    assert!(printed.contains("Jump"));
    assert!(printed.contains("extern_bind_ok"));
    assert!(printed.contains("name: \"closeSymbol\""));
}
