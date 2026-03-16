use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use skeplib::bytecode;
use skeplib::bytecode::Value;
use skeplib::ir::{
    self, BasicBlock, BlockId, FieldRef, FunctionId, Instr, IrFunction, IrInterpError,
    IrInterpreter, IrLocal, IrProgram, IrStruct, IrTemp, IrType, IrValue, IrVerifier, PrettyIr,
    StructField, StructId, TempId, Terminator,
};
use skeplib::vm::{Vm, VmErrorKind};

enum ExpectedValue<'a> {
    Int(i64),
    Bool(bool),
    Float(f64),
    String(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedErrorKind {
    DivisionByZero,
    IndexOutOfBounds,
    TypeMismatch,
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

fn assert_bytecode_and_ir_reject_same_source(source: &str, expected: ExpectedErrorKind) {
    let module = bytecode::compile_source(source).expect("bytecode lowering should succeed");
    let vm_err = Vm::run_module_main(&module).expect_err("bytecode vm should fail");
    let vm_kind = match vm_err.kind {
        VmErrorKind::DivisionByZero => ExpectedErrorKind::DivisionByZero,
        VmErrorKind::IndexOutOfBounds => ExpectedErrorKind::IndexOutOfBounds,
        VmErrorKind::TypeMismatch => ExpectedErrorKind::TypeMismatch,
        other => panic!("unexpected VM error kind in comparison test: {other:?}"),
    };
    assert_eq!(vm_kind, expected);

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
fn compile_source_applies_constant_folding_and_branch_simplification() {
    let source = r#"
fn main() -> Int {
  let x = 1 + 2;
  if (true) {
    return x;
  }
  return 99;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("Const { dst: TempId("));
    assert!(printed.contains("value: Int(3)"));
    assert!(printed.contains("Jump(BlockId("));
    assert!(!printed.contains("Branch(BranchTerminator"));
}

#[test]
fn compile_source_applies_copy_propagation() {
    let source = r#"
fn main() -> Int {
  let x = 1;
  let y = x;
  let z = y;
  return z + 2;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(3));
    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("Copy {"));
}

#[test]
fn verifier_rejects_unknown_jump_target() {
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
            instrs: Vec::new(),
            terminator: Terminator::Jump(BlockId(99)),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::UnknownBlockTarget { .. }));
}

#[test]
fn verifier_rejects_unknown_direct_call_target() {
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
            instrs: vec![ir::Instr::CallDirect {
                dst: None,
                function: FunctionId(77),
                args: Vec::new(),
                ret_ty: IrType::Int,
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

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::UnknownFunctionTarget { .. }
    ));
}

#[test]
fn verifier_rejects_duplicate_block_ids() {
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
                terminator: Terminator::Jump(BlockId(0)),
            },
            BasicBlock {
                id: BlockId(0),
                name: "duplicate".into(),
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

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::DuplicateBlockId { .. }));
}

#[test]
fn verifier_rejects_unknown_struct_field_ref() {
    let strukt = IrStruct {
        id: StructId(0),
        name: "Pair".into(),
        fields: vec![StructField {
            name: "a".into(),
            ty: IrType::Int,
        }],
    };
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![IrLocal {
            id: ir::LocalId(0),
            name: "pair".into(),
            ty: IrType::Named("Pair".into()),
        }],
        temps: vec![IrTemp {
            id: TempId(0),
            ty: IrType::Int,
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::StructGet {
                dst: TempId(0),
                ty: IrType::Int,
                base: ir::Operand::Local(ir::LocalId(0)),
                field: FieldRef {
                    index: 1,
                    name: "b".into(),
                },
            }],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: vec![strukt],
        module_init: None,
    };

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::UnknownField { ref field, .. } if field == "b"
    ));
}

#[test]
fn verifier_rejects_unknown_temp_operand() {
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
            instrs: vec![Instr::Copy {
                dst: TempId(1),
                ty: IrType::Int,
                src: ir::Operand::Temp(TempId(99)),
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

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::UnknownTemp { .. }));
}

#[test]
fn verifier_rejects_unknown_local_operand() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![IrTemp {
            id: TempId(0),
            ty: IrType::Int,
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::StoreLocal {
                local: ir::LocalId(77),
                ty: IrType::Int,
                value: ir::Operand::Const(ir::ConstValue::Int(1)),
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

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::UnknownLocal { .. }));
}

#[test]
fn verifier_rejects_unknown_global_operand() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![IrTemp {
            id: TempId(0),
            ty: IrType::Int,
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::LoadGlobal {
                dst: TempId(0),
                ty: IrType::Int,
                global: ir::GlobalId(42),
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

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::UnknownGlobal));
}

#[test]
fn verifier_rejects_unknown_module_init_target() {
    let program = IrProgram {
        functions: vec![IrFunction {
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
                instrs: Vec::new(),
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            }],
        }],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: Some(ir::IrModuleInit {
            function: FunctionId(99),
        }),
    };

    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::UnknownModuleInitFunction));
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
fn bytecode_and_ir_reject_same_division_by_zero_source() {
    let source = r#"
fn main() -> Int {
  return 8 / 0;
}
"#;

    assert_bytecode_and_ir_reject_same_source(source, ExpectedErrorKind::DivisionByZero);
}

#[test]
fn bytecode_and_ir_reject_same_array_oob_source() {
    let source = r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#;

    assert_bytecode_and_ir_reject_same_source(source, ExpectedErrorKind::IndexOutOfBounds);
}

#[test]
fn bytecode_and_ir_reject_same_string_slice_oob_source() {
    let source = r#"
import str;

fn main() -> String {
  return str.slice("abc", 0, 99);
}
"#;

    assert_bytecode_and_ir_reject_same_source(source, ExpectedErrorKind::IndexOutOfBounds);
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
