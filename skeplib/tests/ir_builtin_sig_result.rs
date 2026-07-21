use skeplib::ir::{self, IrVerifier};

#[test]
fn ir_verify_accepts_fs_read_text_result_builtin() {
    let source = r#"
import fs;

fn main() -> Int {
  let missing = fs.readText("definitely-missing-skepa-file.txt");
  let kept = missing;
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    IrVerifier::verify_program(&program)
        .expect("IR verify should accept fs.readText Result return");
}

#[test]
fn ir_verify_accepts_datetime_parse_unix_result_builtin() {
    let source = r#"
import datetime;

fn main() -> Int {
  let parsed = datetime.parseUnix("1970-01-01T00:00:00Z");
  let kept = parsed;
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    IrVerifier::verify_program(&program)
        .expect("IR verify should accept datetime.parseUnix Result return");
}

#[test]
fn ir_verify_accepts_os_exec_vec_argv() {
    let source = r#"
import os;
import vec;

fn main() -> Int {
  let args: Vec[String] = vec.new();
  let status = os.exec("true", args);
  let kept = status;
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    IrVerifier::verify_program(&program).expect("IR verify should accept os.exec Vec[String] argv");
}
