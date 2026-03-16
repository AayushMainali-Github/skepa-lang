use skeplib::ir::{
    self, BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrLocal, IrProgram, IrTemp, IrType,
    IrValue, PrettyIr, TempId, Terminator,
};

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
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(3));
    let printed = PrettyIr::new(&program).to_string();
    assert!(printed.contains("Jump(BlockId("));
    assert!(!printed.contains("Branch(BranchTerminator"));
}

#[test]
fn compile_source_applies_copy_propagation_and_eliminates_dead_temps() {
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
fn compile_source_inlines_trivial_direct_calls_and_methods() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

fn step(x: Int) -> Int {
  return x + 1;
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + x + self.b;
  }
}

fn main() -> Int {
  let p = Pair { a: 10, b: 5 };
  return step(p.mix(7));
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = ir::IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run optimized source");
    assert_eq!(value, IrValue::Int(23));
    let printed = PrettyIr::new(&program).to_string();
    assert!(!printed.contains("CallDirect"));
}

#[test]
fn optimize_program_eliminates_overwritten_local_stores() {
    let local = IrLocal {
        id: skeplib::ir::LocalId(0),
        name: "x".into(),
        ty: IrType::Int,
    };
    let temp = IrTemp {
        id: TempId(0),
        ty: IrType::Int,
    };
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![local.clone()],
        temps: vec![temp],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::StoreLocal {
                    local: local.id,
                    ty: IrType::Int,
                    value: ir::Operand::Const(ir::ConstValue::Int(1)),
                },
                Instr::StoreLocal {
                    local: local.id,
                    ty: IrType::Int,
                    value: ir::Operand::Const(ir::ConstValue::Int(2)),
                },
                Instr::LoadLocal {
                    dst: TempId(0),
                    ty: IrType::Int,
                    local: local.id,
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Temp(TempId(0)))),
        }],
    };
    let mut program = IrProgram {
        structs: Vec::new(),
        globals: Vec::new(),
        functions: vec![func],
        module_init: None,
    };
    ir::opt::optimize_program(&mut program);
    let main = &program.functions[0];
    let store_count = main.blocks[0]
        .instrs
        .iter()
        .filter(|instr| matches!(instr, Instr::StoreLocal { .. }))
        .count();
    assert_eq!(store_count, 1);
}

#[test]
fn optimize_program_simplifies_loop_shapes_and_strength_reduction() {
    let cond = IrLocal {
        id: skeplib::ir::LocalId(0),
        name: "cond".into(),
        ty: IrType::Bool,
    };
    let sink = IrLocal {
        id: skeplib::ir::LocalId(1),
        name: "sink".into(),
        ty: IrType::Int,
    };
    let temp = IrTemp {
        id: TempId(0),
        ty: IrType::Int,
    };
    let exit_temp = IrTemp {
        id: TempId(1),
        ty: IrType::Int,
    };
    let math_temp = IrTemp {
        id: TempId(2),
        ty: IrType::Int,
    };
    let mut program = IrProgram {
        structs: Vec::new(),
        globals: Vec::new(),
        functions: vec![IrFunction {
            id: FunctionId(0),
            name: "main".into(),
            params: Vec::new(),
            locals: vec![cond, sink.clone()],
            temps: vec![temp, exit_temp, math_temp],
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    name: "entry".into(),
                    instrs: Vec::new(),
                    terminator: Terminator::Jump(BlockId(1)),
                },
                BasicBlock {
                    id: BlockId(1),
                    name: "while_cond".into(),
                    instrs: Vec::new(),
                    terminator: Terminator::Branch(ir::BranchTerminator {
                        cond: ir::Operand::Local(skeplib::ir::LocalId(0)),
                        then_block: BlockId(2),
                        else_block: BlockId(3),
                    }),
                },
                BasicBlock {
                    id: BlockId(2),
                    name: "while_body".into(),
                    instrs: vec![
                        Instr::Const {
                            dst: TempId(0),
                            ty: IrType::Int,
                            value: ir::ConstValue::Int(1),
                        },
                        Instr::Binary {
                            dst: TempId(2),
                            ty: IrType::Int,
                            op: ir::BinaryOp::Mul,
                            left: ir::Operand::Temp(TempId(0)),
                            right: ir::Operand::Const(ir::ConstValue::Int(2)),
                        },
                        Instr::StoreLocal {
                            local: sink.id,
                            ty: IrType::Int,
                            value: ir::Operand::Temp(TempId(2)),
                        },
                    ],
                    terminator: Terminator::Jump(BlockId(1)),
                },
                BasicBlock {
                    id: BlockId(3),
                    name: "while_exit".into(),
                    instrs: vec![Instr::LoadLocal {
                        dst: TempId(1),
                        ty: IrType::Int,
                        local: sink.id,
                    }],
                    terminator: Terminator::Return(Some(ir::Operand::Temp(TempId(1)))),
                },
            ],
        }],
        module_init: None,
    };
    ir::opt::optimize_program(&mut program);
    let main = &program.functions[0];
    let entry = main
        .blocks
        .iter()
        .find(|block| block.name == "entry")
        .expect("entry block should exist");
    let while_body = main
        .blocks
        .iter()
        .find(|block| block.name == "while_body")
        .expect("while body should still exist");
    assert!(
        !while_body
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::Const { .. }))
    );
    assert!(while_body.instrs.iter().all(|instr| !matches!(
        instr,
        Instr::Binary {
            op: ir::BinaryOp::Mul,
            ..
        }
    )));
    assert!(
        entry
            .instrs
            .iter()
            .all(|instr| !matches!(instr, Instr::StoreLocal { .. }))
    );
}
