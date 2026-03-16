use skeplib::ir::{
    self, BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrInterpError, IrInterpreter,
    IrProgram, IrType, IrValue, Terminator,
};

#[path = "common.rs"]
mod common;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedErrorKind {
    DivisionByZero,
    IndexOutOfBounds,
    TypeMismatch,
}

fn assert_ir_rejects_source(source: &str, expected: ExpectedErrorKind) {
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let ir_err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("IR interpreter should fail");
    let ir_kind = match ir_err {
        IrInterpError::DivisionByZero => ExpectedErrorKind::DivisionByZero,
        IrInterpError::IndexOutOfBounds => ExpectedErrorKind::IndexOutOfBounds,
        IrInterpError::TypeMismatch(_) => ExpectedErrorKind::TypeMismatch,
        other => panic!("unexpected IR error kind in comparison test: {other:?}"),
    };
    assert_eq!(ir_kind, expected);
}

#[test]
fn interpreter_rejects_non_bool_branch_condition() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: Vec::new(),
                terminator: ir::Terminator::Branch(ir::BranchTerminator {
                    cond: ir::Operand::Const(ir::ConstValue::Int(1)),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                }),
            },
            BasicBlock {
                id: BlockId(1),
                name: "then".into(),
                instrs: Vec::new(),
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(1)))),
            },
            BasicBlock {
                id: BlockId(2),
                name: "else".into(),
                instrs: Vec::new(),
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            },
        ],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("interpreter should reject non-bool branch conditions");
    assert!(matches!(
        err,
        IrInterpError::TypeMismatch("branch condition must be bool")
    ));
}

#[test]
fn interpreter_rejects_indirect_call_on_non_closure() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::CallIndirect {
                dst: None,
                ret_ty: IrType::Int,
                callee: ir::Operand::Const(ir::ConstValue::Int(7)),
                args: Vec::new(),
            }],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("interpreter should reject non-closure indirect calls");
    assert!(matches!(
        err,
        IrInterpError::TypeMismatch("indirect call on non-closure")
    ));
}

#[test]
fn interpreter_handles_runtime_managed_values_and_function_values() {
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

fn add1(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  let xs: Vec[Int] = vec.new();
  let p = Pair { a: arr[0], b: 3 };
  let f = add1;
  vec.push(xs, p.mix(4));
  return f(vec.get(xs, 0));
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(9));
}

#[test]
fn interpreter_preserves_shared_aliasing_for_vecs_and_struct_handles() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let ys = xs;
  vec.push(xs, 3);
  vec.set(ys, 0, 9);
  return vec.get(xs, 0);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(9));
}

#[test]
fn interpreter_runs_globals_module_init_and_core_builtins() {
    let source = r#"
import datetime;
import str;

let base: String = "skepa-language-benchmark";

fn main() -> Int {
  let total = str.len(base) + str.indexOf(base, "bench");
  let cut = str.slice(base, 6, 14);
  if (str.contains(cut, "language")) {
    return total + 1;
  }
  return datetime.nowMillis();
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(40));
}

#[test]
fn interpreter_supports_io_and_datetime_builtins_through_runtime() {
    let source = r#"
import datetime;
import io;

fn main() -> Int {
  io.printInt(7);
  let now = datetime.nowUnix();
  if (now >= 0) {
    return 1;
  }
  return 0;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(1));
}

#[test]
fn interpreter_reports_runtime_error_cases() {
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  return 8 / 0;
}
"#,
        ExpectedErrorKind::DivisionByZero,
    );
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import str;

fn main() -> String {
  return str.slice("abc", 0, 99);
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
}
