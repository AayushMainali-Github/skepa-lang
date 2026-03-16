use skeplib::ir::{
    self, BasicBlock, BlockId, FieldRef, FunctionId, Instr, IrFunction, IrLocal, IrProgram,
    IrStruct, IrTemp, IrType, IrVerifier, StructField, StructId, TempId, Terminator,
};

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
fn verifier_rejects_missing_entry_block_and_missing_terminator_shape() {
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
        entry: BlockId(7),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::Const {
                dst: TempId(0),
                ty: IrType::Int,
                value: ir::ConstValue::Int(1),
            }],
            terminator: Terminator::Unreachable,
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
        ir::IrVerifyError::MissingEntryBlock { .. } | ir::IrVerifyError::MissingTerminator { .. }
    ));
}

#[test]
fn verifier_rejects_unknown_direct_call_and_closure_targets() {
    for instr in [
        Instr::CallDirect {
            dst: None,
            function: FunctionId(77),
            args: Vec::new(),
            ret_ty: IrType::Int,
        },
        Instr::MakeClosure {
            dst: TempId(0),
            function: FunctionId(88),
        },
    ] {
        let func = IrFunction {
            id: FunctionId(0),
            name: "main".into(),
            params: Vec::new(),
            locals: Vec::new(),
            temps: vec![IrTemp {
                id: TempId(0),
                ty: IrType::Fn {
                    params: vec![IrType::Int],
                    ret: Box::new(IrType::Int),
                },
            }],
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: vec![instr.clone()],
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
}

#[test]
fn verifier_rejects_duplicate_block_ids_unknown_structs_and_fields() {
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
        blocks: vec![
            BasicBlock {
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
            },
            BasicBlock {
                id: BlockId(0),
                name: "duplicate".into(),
                instrs: vec![Instr::MakeStruct {
                    dst: TempId(0),
                    struct_id: StructId(99),
                    fields: Vec::new(),
                }],
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            },
        ],
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
        ir::IrVerifyError::DuplicateBlockId { .. }
            | ir::IrVerifyError::UnknownField { .. }
            | ir::IrVerifyError::UnknownStruct { .. }
    ));
}

#[test]
fn verifier_rejects_unknown_temp_local_global_and_module_init() {
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
            instrs: vec![
                Instr::Copy {
                    dst: TempId(1),
                    ty: IrType::Int,
                    src: ir::Operand::Temp(TempId(2)),
                },
                Instr::StoreLocal {
                    local: ir::LocalId(77),
                    ty: IrType::Int,
                    value: ir::Operand::Const(ir::ConstValue::Int(1)),
                },
                Instr::LoadGlobal {
                    dst: TempId(3),
                    ty: IrType::Int,
                    global: ir::GlobalId(42),
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: Some(ir::IrModuleInit {
            function: FunctionId(99),
        }),
    };
    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::UnknownTemp { .. }
            | ir::IrVerifyError::UnknownLocal { .. }
            | ir::IrVerifyError::UnknownGlobal
            | ir::IrVerifyError::UnknownModuleInitFunction
    ));
}
