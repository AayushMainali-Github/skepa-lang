use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::codegen::{self, CodegenError};
use skeplib::ir;

fn temp_file(name: &str, ext: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic enough for temp path")
        .as_nanos();
    std::env::temp_dir().join(format!("skepa_codegen_{name}_{nanos}.{ext}"))
}

#[test]
fn llvm_codegen_emits_valid_int_only_module() {
    let source = r#"
fn main() -> Int {
  let i = 0;
  let acc = 1;
  while (i < 4) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let llvm_ir =
        codegen::compile_program_to_llvm_ir(&program).expect("LLVM lowering should succeed");

    assert!(llvm_ir.contains("define i64 @main()"));
    assert!(llvm_ir.contains("icmp slt"));
    assert!(llvm_ir.contains("br i1"));

    let ll_path = temp_file("valid", "ll");
    let bc_path = temp_file("valid", "bc");
    fs::write(&ll_path, llvm_ir).expect("should write temporary llvm ir file");

    let output = Command::new("llvm-as")
        .arg(&ll_path)
        .arg("-o")
        .arg(&bc_path)
        .output()
        .expect("llvm-as should be available on PATH");

    let _ = fs::remove_file(&ll_path);
    let _ = fs::remove_file(&bc_path);

    assert!(
        output.status.success(),
        "llvm-as rejected generated IR: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn llvm_codegen_rejects_direct_calls_for_now() {
    let source = r#"
fn step(x: Int) -> Int {
  if (x < 10) {
    return x + 1;
  }
  return x;
}

fn main() -> Int {
  return step(4);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let err =
        codegen::compile_program_to_llvm_ir(&program).expect_err("calls should not be lowered yet");
    assert!(matches!(err, CodegenError::Unsupported(_)));
}
