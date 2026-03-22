use crate::ir::{Instr, IrProgram, NativeCallLowering, NativeCallPlan};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        let plan = NativeCallPlan::analyze(func);
        for block in &mut func.blocks {
            for instr in &mut block.instrs {
                let replacement = match instr {
                    Instr::CallIndirect {
                        dst,
                        ret_ty,
                        callee,
                        args,
                    } => match plan.operand_lowering(callee) {
                        NativeCallLowering::KnownFunction(function) => Some(Instr::CallDirect {
                            dst: *dst,
                            ret_ty: ret_ty.clone(),
                            function,
                            args: args.clone(),
                        }),
                        NativeCallLowering::Dynamic => None,
                    },
                    _ => None,
                };
                if let Some(new_instr) = replacement {
                    *instr = new_instr;
                    changed = true;
                }
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::ir::{
        BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrLocal, IrProgram, IrTemp, IrType,
        LocalId, Operand, TempId, Terminator,
    };

    #[test]
    fn resolves_known_indirect_calls_before_backend_lowering() {
        let fn_ty = IrType::Fn {
            params: vec![IrType::Int],
            ret: Box::new(IrType::Int),
        };
        let mut program = IrProgram::new();
        program.functions.push(IrFunction {
            id: FunctionId(0),
            name: "inc".into(),
            params: vec![],
            locals: vec![],
            temps: vec![],
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: vec![],
                terminator: Terminator::Return(Some(Operand::Const(crate::ir::ConstValue::Int(1)))),
            }],
        });
        program.functions.push(IrFunction {
            id: FunctionId(1),
            name: "main".into(),
            params: vec![],
            locals: vec![IrLocal {
                id: LocalId(0),
                name: "f".into(),
                ty: fn_ty.clone(),
            }],
            temps: vec![
                IrTemp {
                    id: TempId(0),
                    ty: fn_ty.clone(),
                },
                IrTemp {
                    id: TempId(1),
                    ty: fn_ty.clone(),
                },
                IrTemp {
                    id: TempId(2),
                    ty: IrType::Int,
                },
            ],
            ret_ty: IrType::Int,
            entry: BlockId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: vec![
                    Instr::MakeClosure {
                        dst: TempId(0),
                        function: FunctionId(0),
                    },
                    Instr::StoreLocal {
                        local: LocalId(0),
                        ty: fn_ty.clone(),
                        value: Operand::Temp(TempId(0)),
                    },
                    Instr::LoadLocal {
                        dst: TempId(1),
                        ty: fn_ty,
                        local: LocalId(0),
                    },
                    Instr::CallIndirect {
                        dst: Some(TempId(2)),
                        ret_ty: IrType::Int,
                        callee: Operand::Temp(TempId(1)),
                        args: vec![Operand::Const(crate::ir::ConstValue::Int(41))],
                    },
                ],
                terminator: Terminator::Return(Some(Operand::Temp(TempId(2)))),
            }],
        });

        let main_before = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .unwrap();
        assert!(main_before.blocks.iter().any(|block| {
            block
                .instrs
                .iter()
                .any(|instr| matches!(instr, Instr::CallIndirect { .. }))
        }));

        assert!(run(&mut program));

        let main_after = program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .unwrap();
        assert!(main_after.blocks.iter().all(|block| {
            block
                .instrs
                .iter()
                .all(|instr| !matches!(instr, Instr::CallIndirect { .. }))
        }));
        assert!(main_after.blocks.iter().any(|block| {
            block
                .instrs
                .iter()
                .any(|instr| matches!(instr, Instr::CallDirect { .. }))
        }));
    }
}
