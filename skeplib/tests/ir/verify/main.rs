use skeplib::ir::{
    self, BasicBlock, BlockId, FieldRef, FunctionId, Instr, IrFunction, IrLocal, IrParam,
    IrProgram, IrStruct, IrTemp, IrType, IrVerifier, StructField, StructId, TempId, Terminator,
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

#[test]
fn verifier_rejects_duplicate_param_local_and_temp_ids() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: vec![
            skeplib::ir::IrParam {
                id: skeplib::ir::ParamId(0),
                name: "a".into(),
                ty: IrType::Int,
            },
            skeplib::ir::IrParam {
                id: skeplib::ir::ParamId(0),
                name: "b".into(),
                ty: IrType::Int,
            },
        ],
        locals: vec![
            IrLocal {
                id: ir::LocalId(0),
                name: "x".into(),
                ty: IrType::Int,
            },
            IrLocal {
                id: ir::LocalId(0),
                name: "y".into(),
                ty: IrType::Int,
            },
        ],
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Int,
            },
            IrTemp {
                id: TempId(0),
                ty: IrType::Int,
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: Vec::new(),
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
        ir::IrVerifyError::DuplicateParamId { .. }
            | ir::IrVerifyError::DuplicateLocalId { .. }
            | ir::IrVerifyError::DuplicateTempId { .. }
    ));
}

#[test]
fn verifier_rejects_bad_call_signature_return_mismatch_and_bad_branch_operand_type() {
    let callee = IrFunction {
        id: FunctionId(1),
        name: "callee".into(),
        params: vec![skeplib::ir::IrParam {
            id: skeplib::ir::ParamId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: Vec::new(),
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(1)))),
        }],
    };
    let main = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![IrTemp {
            id: TempId(0),
            ty: IrType::Bool,
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: vec![Instr::CallDirect {
                    dst: Some(TempId(0)),
                    function: FunctionId(1),
                    args: Vec::new(),
                    ret_ty: IrType::Int,
                }],
                terminator: Terminator::Branch(ir::BranchTerminator {
                    cond: ir::Operand::Const(ir::ConstValue::Int(1)),
                    then_block: BlockId(1),
                    else_block: BlockId(1),
                }),
            },
            BasicBlock {
                id: BlockId(1),
                name: "exit".into(),
                instrs: Vec::new(),
                terminator: Terminator::Return(None),
            },
        ],
    };
    let program = IrProgram {
        functions: vec![main, callee],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::BadCallSignature { .. }
            | ir::IrVerifyError::ReturnTypeMismatch { .. }
            | ir::IrVerifyError::OperandTypeMismatch { .. }
    ));
}

#[test]
fn verifier_rejects_bad_direct_builtin_and_indirect_call_types() {
    let callee = IrFunction {
        id: FunctionId(1),
        name: "callee".into(),
        params: vec![skeplib::ir::IrParam {
            id: skeplib::ir::ParamId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        locals: Vec::new(),
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Fn {
                    params: vec![IrType::Int],
                    ret: Box::new(IrType::Int),
                },
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::Const {
                    dst: TempId(0),
                    ty: IrType::Bool,
                    value: ir::ConstValue::Bool(true),
                },
                Instr::MakeClosure {
                    dst: TempId(1),
                    function: FunctionId(1),
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(1)))),
        }],
    };
    let main = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Fn {
                    params: vec![IrType::Int],
                    ret: Box::new(IrType::Int),
                },
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::Const {
                    dst: TempId(0),
                    ty: IrType::Bool,
                    value: ir::ConstValue::Bool(true),
                },
                Instr::MakeClosure {
                    dst: TempId(1),
                    function: FunctionId(1),
                },
                Instr::CallDirect {
                    dst: None,
                    ret_ty: IrType::Int,
                    function: FunctionId(1),
                    args: vec![ir::Operand::Temp(TempId(0))],
                },
                Instr::CallBuiltin {
                    dst: Some(TempId(0)),
                    ret_ty: IrType::Bool,
                    builtin: ir::BuiltinCall {
                        package: "str".into(),
                        name: "len".into(),
                    },
                    args: vec![ir::Operand::Const(ir::ConstValue::Bool(true))],
                },
                Instr::CallIndirect {
                    dst: None,
                    ret_ty: IrType::Bool,
                    callee: ir::Operand::Temp(TempId(1)),
                    args: vec![ir::Operand::Const(ir::ConstValue::Bool(true))],
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![main, callee],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::BadCallSignature { .. } | ir::IrVerifyError::OperandTypeMismatch { .. }
    ));
}

#[test]
fn verifier_rejects_non_int_indexes_for_array_and_vec_ops() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![
            IrLocal {
                id: ir::LocalId(0),
                name: "arr".into(),
                ty: IrType::Array {
                    elem: Box::new(IrType::Int),
                    size: 2,
                },
            },
            IrLocal {
                id: ir::LocalId(1),
                name: "xs".into(),
                ty: IrType::Vec {
                    elem: Box::new(IrType::Int),
                },
            },
        ],
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Int,
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::Const {
                    dst: TempId(0),
                    ty: IrType::Bool,
                    value: ir::ConstValue::Bool(true),
                },
                Instr::ArrayGet {
                    dst: TempId(1),
                    elem_ty: IrType::Int,
                    array: ir::Operand::Local(ir::LocalId(0)),
                    index: ir::Operand::Temp(TempId(0)),
                },
                Instr::VecLen {
                    dst: TempId(1),
                    vec: ir::Operand::Local(ir::LocalId(0)),
                },
            ],
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
    assert!(matches!(err, ir::IrVerifyError::OperandTypeMismatch { .. }));
}

#[test]
fn verifier_rejects_non_shareable_task_payloads_with_field_path() {
    for (name, expected_boundary) in [
        ("send", "task.send payload"),
        ("recv", "task.recv payload"),
        ("spawn", "task.spawn result"),
        ("join", "task.join result"),
    ] {
        let (locals, args, ret_ty, dst) = match name {
            "send" => (
                vec![
                    IrLocal {
                        id: ir::LocalId(0),
                        name: "jobs".into(),
                        ty: IrType::Opaque("task.Channel[Envelope]".into()),
                    },
                    IrLocal {
                        id: ir::LocalId(1),
                        name: "payload".into(),
                        ty: IrType::Named("Envelope".into()),
                    },
                ],
                vec![
                    ir::Operand::Local(ir::LocalId(0)),
                    ir::Operand::Local(ir::LocalId(1)),
                ],
                IrType::Void,
                None,
            ),
            "recv" => (
                vec![IrLocal {
                    id: ir::LocalId(0),
                    name: "jobs".into(),
                    ty: IrType::Opaque("task.Channel[Envelope]".into()),
                }],
                vec![ir::Operand::Local(ir::LocalId(0))],
                IrType::Named("Envelope".into()),
                Some(TempId(0)),
            ),
            "spawn" => (
                vec![IrLocal {
                    id: ir::LocalId(0),
                    name: "job".into(),
                    ty: IrType::Fn {
                        params: Vec::new(),
                        ret: Box::new(IrType::Named("Envelope".into())),
                    },
                }],
                vec![ir::Operand::Local(ir::LocalId(0))],
                IrType::Result {
                    ok: Box::new(IrType::Opaque("task.Task[Envelope]".into())),
                    err: Box::new(IrType::String),
                },
                Some(TempId(0)),
            ),
            "join" => (
                vec![IrLocal {
                    id: ir::LocalId(0),
                    name: "job".into(),
                    ty: IrType::Opaque("task.Task[Envelope]".into()),
                }],
                vec![ir::Operand::Local(ir::LocalId(0))],
                IrType::Named("Envelope".into()),
                Some(TempId(0)),
            ),
            _ => unreachable!(),
        };
        let temps = dst
            .map(|id| {
                vec![IrTemp {
                    id,
                    ty: ret_ty.clone(),
                }]
            })
            .unwrap_or_default();
        let func = IrFunction {
            id: FunctionId(0),
            name: "main".into(),
            params: Vec::new(),
            locals,
            temps,
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: vec![Instr::CallBuiltin {
                    dst,
                    ret_ty,
                    builtin: ir::BuiltinCall {
                        package: "task".into(),
                        name: name.into(),
                    },
                    args,
                }],
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            }],
        };
        let program = IrProgram {
            functions: vec![func],
            globals: Vec::new(),
            structs: vec![IrStruct {
                id: StructId(0),
                name: "Envelope".into(),
                fields: vec![StructField {
                    name: "items".into(),
                    ty: IrType::Vec {
                        elem: Box::new(IrType::Int),
                    },
                }],
            }],
            module_init: None,
        };

        let error = IrVerifier::verify_program(&program)
            .expect_err("non-shareable concurrency value must fail");
        assert!(matches!(
            error,
            ir::IrVerifyError::NonShareableConcurrencyValue {
                boundary,
                capability: skeplib::types::ConcurrencyCapability::LocalOnly,
                path,
                ty,
                ..
            } if boundary == expected_boundary
                && path == vec!["items"]
                && ty == "Vec[Int]"
        ));
    }
}

#[test]
fn verifier_checks_spawned_code_global_effects_through_direct_calls() {
    let local_only_global = r#"
import task;
import vec;

let values: Vec[Int] = vec.new();

fn read_count() -> Int {
  return vec.len(values);
}

fn worker() -> Int {
  return read_count();
}

fn main() -> Int {
  let spawned = task.spawn(worker);
  return 0;
}
"#;
    let error = ir::lowering::compile_source(local_only_global)
        .expect_err("spawned code must not read local-only global state");
    let messages = error
        .as_slice()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("NonShareableConcurrencyValue"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("task.spawn global `values`"))
    );

    let global_mutation = r#"
import task;

let counter: Int = 0;

fn worker() -> Int {
  counter = counter + 1;
  return counter;
}

fn main() -> Int {
  let spawned = task.spawn(worker);
  return 0;
}
"#;
    let error = ir::lowering::compile_source(global_mutation)
        .expect_err("spawned code must not mutate global state");
    let messages = error
        .as_slice()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("UnsafeTaskGlobalMutation"))
    );
    assert!(messages.iter().any(|message| message.contains("counter")));
}

#[test]
fn verifier_allows_spawned_code_to_read_shareable_globals() {
    let source = r#"
import task;

let offset: Int = 5;

fn worker() -> Int {
  return offset;
}

fn main() -> Int {
  let spawned = task.spawn(worker);
  return 0;
}
"#;
    ir::lowering::compile_source(source).expect("shareable read-only global is safe");
}

#[test]
fn verifier_rejects_spawned_reads_of_globals_mutated_by_other_functions() {
    let source = r#"
import task;

let counter: Int = 0;

fn read_counter() -> Int {
  return counter;
}

fn update_counter() -> Void {
  counter = counter + 1;
}

fn main() -> Int {
  let spawned = task.spawn(read_counter);
  return 0;
}
"#;
    let error = ir::lowering::compile_source(source)
        .expect_err("task reads must not race with writes from another function");
    let messages = error
        .as_slice()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages.iter().any(|message| {
        message.contains("UnsafeTaskGlobalRead") && message.contains("counter")
    }));
}

#[test]
fn verifier_checks_spawn_argument_signatures_and_capabilities() {
    fn program(expected_arg: IrType, actual_arg: IrType) -> IrProgram {
        let is_vec = matches!(&actual_arg, IrType::Vec { .. });
        let arg_operand = if is_vec {
            ir::Operand::Local(ir::LocalId(0))
        } else {
            ir::Operand::Temp(TempId(1))
        };
        let mut main_temps = vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Fn {
                    params: vec![expected_arg.clone()],
                    ret: Box::new(IrType::Int),
                },
            },
            IrTemp {
                id: TempId(2),
                ty: IrType::Result {
                    ok: Box::new(IrType::Opaque("task.Task[Int]".into())),
                    err: Box::new(IrType::String),
                },
            },
        ];
        let mut main_instrs = vec![Instr::MakeClosure {
            dst: TempId(0),
            function: FunctionId(1),
        }];
        if !is_vec {
            main_temps.push(IrTemp {
                id: TempId(1),
                ty: actual_arg.clone(),
            });
            let value = match &actual_arg {
                IrType::Int => ir::ConstValue::Int(2),
                IrType::String => ir::ConstValue::String("bad".into()),
                _ => unreachable!(),
            };
            main_instrs.push(Instr::Const {
                dst: TempId(1),
                ty: actual_arg.clone(),
                value,
            });
        }
        main_instrs.push(Instr::CallBuiltin {
            dst: Some(TempId(2)),
            ret_ty: IrType::Result {
                ok: Box::new(IrType::Opaque("task.Task[Int]".into())),
                err: Box::new(IrType::String),
            },
            builtin: ir::BuiltinCall {
                package: "task".into(),
                name: "spawn".into(),
            },
            args: vec![ir::Operand::Temp(TempId(0)), arg_operand],
        });

        let main = IrFunction {
            id: FunctionId(0),
            name: "main".into(),
            params: Vec::new(),
            locals: if is_vec {
                vec![IrLocal {
                    id: ir::LocalId(0),
                    name: "values".into(),
                    ty: actual_arg,
                }]
            } else {
                Vec::new()
            },
            temps: main_temps,
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: main_instrs,
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            }],
        };
        let task_function = IrFunction {
            id: FunctionId(1),
            name: "worker".into(),
            params: vec![IrParam {
                id: ir::ParamId(0),
                name: "value".into(),
                ty: expected_arg,
            }],
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
        };
        IrProgram {
            functions: vec![main, task_function],
            globals: Vec::new(),
            structs: Vec::new(),
            module_init: None,
        }
    }

    IrVerifier::verify_program(&program(IrType::Int, IrType::Int))
        .expect("matching shareable spawn arguments are valid");

    let mismatch = IrVerifier::verify_program(&program(IrType::Int, IrType::String))
        .expect_err("spawn argument type mismatch must be rejected");
    assert!(matches!(
        mismatch,
        ir::IrVerifyError::BadCallSignature { .. }
    ));

    let local_only = IrVerifier::verify_program(&program(
        IrType::Vec {
            elem: Box::new(IrType::Int),
        },
        IrType::Vec {
            elem: Box::new(IrType::Int),
        },
    ))
    .expect_err("mutable spawn arguments must be rejected");
    assert!(matches!(
        local_only,
        ir::IrVerifyError::NonShareableConcurrencyValue {
            boundary,
            capability: skeplib::types::ConcurrencyCapability::LocalOnly,
            ..
        } if boundary == "task.spawn argument 1"
    ));
}

#[test]
fn verifier_rejects_bad_store_closure_array_struct_and_vec_element_types() {
    let callee = IrFunction {
        id: FunctionId(1),
        name: "callee".into(),
        params: vec![
            skeplib::ir::IrParam {
                id: skeplib::ir::ParamId(0),
                name: "x".into(),
                ty: IrType::Int,
            },
            skeplib::ir::IrParam {
                id: skeplib::ir::ParamId(1),
                name: "y".into(),
                ty: IrType::Int,
            },
        ],
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
    };
    let main = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![
            IrLocal {
                id: ir::LocalId(0),
                name: "x".into(),
                ty: IrType::Int,
            },
            IrLocal {
                id: ir::LocalId(1),
                name: "xs".into(),
                ty: IrType::Vec {
                    elem: Box::new(IrType::Int),
                },
            },
            IrLocal {
                id: ir::LocalId(2),
                name: "pair".into(),
                ty: IrType::Named("Pair".into()),
            },
        ],
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Fn {
                    params: vec![IrType::Int],
                    ret: Box::new(IrType::Int),
                },
            },
            IrTemp {
                id: TempId(2),
                ty: IrType::Array {
                    elem: Box::new(IrType::Int),
                    size: 1,
                },
            },
            IrTemp {
                id: TempId(3),
                ty: IrType::Named("Pair".into()),
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::StoreLocal {
                    local: ir::LocalId(0),
                    ty: IrType::Int,
                    value: ir::Operand::Const(ir::ConstValue::Bool(true)),
                },
                Instr::StoreGlobal {
                    global: ir::GlobalId(0),
                    ty: IrType::Int,
                    value: ir::Operand::Const(ir::ConstValue::Bool(true)),
                },
                Instr::MakeClosure {
                    dst: TempId(1),
                    function: FunctionId(1),
                },
                Instr::MakeArray {
                    dst: TempId(2),
                    elem_ty: IrType::Int,
                    items: vec![ir::Operand::Const(ir::ConstValue::Bool(true))],
                },
                Instr::VecPush {
                    vec: ir::Operand::Local(ir::LocalId(1)),
                    value: ir::Operand::Const(ir::ConstValue::Bool(true)),
                },
                Instr::MakeStruct {
                    dst: TempId(3),
                    struct_id: StructId(0),
                    fields: vec![ir::Operand::Const(ir::ConstValue::Bool(true))],
                },
                Instr::StructSet {
                    base: ir::Operand::Local(ir::LocalId(2)),
                    field: FieldRef {
                        index: 0,
                        name: "a".into(),
                    },
                    value: ir::Operand::Const(ir::ConstValue::Bool(true)),
                    ty: IrType::Int,
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![main, callee],
        globals: vec![skeplib::ir::IrGlobal {
            id: ir::GlobalId(0),
            name: "g".into(),
            ty: IrType::Int,
            init: None,
        }],
        structs: vec![IrStruct {
            id: StructId(0),
            name: "Pair".into(),
            fields: vec![StructField {
                name: "a".into(),
                ty: IrType::Int,
            }],
        }],
        module_init: None,
    };
    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(
        err,
        ir::IrVerifyError::OperandTypeMismatch { .. } | ir::IrVerifyError::BadCallSignature { .. }
    ));
}

#[test]
fn verifier_rejects_bad_load_get_and_operator_result_types() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![
            IrLocal {
                id: ir::LocalId(0),
                name: "flag".into(),
                ty: IrType::Bool,
            },
            IrLocal {
                id: ir::LocalId(1),
                name: "arr".into(),
                ty: IrType::Array {
                    elem: Box::new(IrType::Int),
                    size: 1,
                },
            },
            IrLocal {
                id: ir::LocalId(2),
                name: "pair".into(),
                ty: IrType::Named("Pair".into()),
            },
        ],
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Int,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(2),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(3),
                ty: IrType::Bool,
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::LoadLocal {
                    dst: TempId(0),
                    ty: IrType::Int,
                    local: ir::LocalId(0),
                },
                Instr::ArrayGet {
                    dst: TempId(1),
                    elem_ty: IrType::Bool,
                    array: ir::Operand::Local(ir::LocalId(1)),
                    index: ir::Operand::Const(ir::ConstValue::Int(0)),
                },
                Instr::StructGet {
                    dst: TempId(2),
                    ty: IrType::Bool,
                    base: ir::Operand::Local(ir::LocalId(2)),
                    field: FieldRef {
                        index: 0,
                        name: "a".into(),
                    },
                },
                Instr::Binary {
                    dst: TempId(3),
                    ty: IrType::Bool,
                    op: ir::BinaryOp::Add,
                    left: ir::Operand::Const(ir::ConstValue::Bool(true)),
                    right: ir::Operand::Const(ir::ConstValue::Bool(false)),
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: vec![IrStruct {
            id: StructId(0),
            name: "Pair".into(),
            fields: vec![StructField {
                name: "a".into(),
                ty: IrType::Int,
            }],
        }],
        module_init: None,
    };
    let err = IrVerifier::verify_program(&program).expect_err("verifier should fail");
    assert!(matches!(err, ir::IrVerifyError::OperandTypeMismatch { .. }));
}

#[test]
fn verifier_rejects_bitwise_operators_on_non_int_types() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![
            IrTemp {
                id: TempId(0),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(1),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(2),
                ty: IrType::Bool,
            },
            IrTemp {
                id: TempId(3),
                ty: IrType::Float,
            },
            IrTemp {
                id: TempId(4),
                ty: IrType::Float,
            },
        ],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::Const {
                    dst: TempId(0),
                    ty: IrType::Bool,
                    value: ir::ConstValue::Bool(true),
                },
                Instr::Const {
                    dst: TempId(3),
                    ty: IrType::Float,
                    value: ir::ConstValue::Float(1.0),
                },
                Instr::Unary {
                    dst: TempId(1),
                    ty: IrType::Bool,
                    op: ir::UnaryOp::BitNot,
                    operand: ir::Operand::Temp(TempId(0)),
                },
                Instr::Binary {
                    dst: TempId(2),
                    ty: IrType::Bool,
                    op: ir::BinaryOp::BitAnd,
                    left: ir::Operand::Temp(TempId(0)),
                    right: ir::Operand::Const(ir::ConstValue::Bool(false)),
                },
                Instr::Binary {
                    dst: TempId(4),
                    ty: IrType::Float,
                    op: ir::BinaryOp::Shl,
                    left: ir::Operand::Temp(TempId(3)),
                    right: ir::Operand::Const(ir::ConstValue::Float(2.0)),
                },
            ],
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
    assert!(matches!(err, ir::IrVerifyError::OperandTypeMismatch { .. }));
}
