use crate::ir::{IrFunction, IrProgram, Operand, Terminator};

mod helpers;
mod instrs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrVerifyError {
    MissingEntryBlock {
        function: String,
    },
    DuplicateBlockId {
        function: String,
    },
    DuplicateParamId {
        function: String,
    },
    DuplicateLocalId {
        function: String,
    },
    DuplicateTempId {
        function: String,
    },
    MissingTerminator {
        function: String,
        block: String,
    },
    UnknownBlockTarget {
        function: String,
        block: String,
    },
    UnknownTemp {
        function: String,
    },
    UnknownLocal {
        function: String,
    },
    UnknownGlobal,
    UnknownFunctionTarget {
        function: String,
    },
    UnknownStruct {
        function: String,
    },
    UnknownField {
        function: String,
        field: String,
    },
    BadCallSignature {
        function: String,
    },
    NonShareableConcurrencyValue {
        function: String,
        boundary: String,
        capability: crate::types::ConcurrencyCapability,
        path: Vec<String>,
        ty: String,
    },
    UnsafeTaskGlobalMutation {
        function: String,
        global: String,
    },
    UnsafeTaskGlobalRead {
        function: String,
        global: String,
    },
    UnverifiableTaskCodeEntry {
        function: String,
    },
    UnverifiableTaskCallTarget {
        function: String,
    },
    ReturnTypeMismatch {
        function: String,
    },
    OperandTypeMismatch {
        function: String,
    },
    UnknownModuleInitFunction,
}

pub struct IrVerifier;

impl IrVerifier {
    pub fn verify_program(program: &IrProgram) -> Result<(), IrVerifyError> {
        for func in &program.functions {
            Self::verify_function(program, func)?;
        }
        if let Some(init) = &program.module_init
            && !program
                .functions
                .iter()
                .any(|func| func.id == init.function)
        {
            return Err(IrVerifyError::UnknownModuleInitFunction);
        }
        Ok(())
    }

    pub fn verify_function(program: &IrProgram, func: &IrFunction) -> Result<(), IrVerifyError> {
        if !func.blocks.iter().any(|block| block.id == func.entry) {
            return Err(IrVerifyError::MissingEntryBlock {
                function: func.name.clone(),
            });
        }
        Self::verify_unique_ids(func)?;

        for block in &func.blocks {
            if matches!(block.terminator, Terminator::Unreachable) && !block.instrs.is_empty() {
                return Err(IrVerifyError::MissingTerminator {
                    function: func.name.clone(),
                    block: block.name.clone(),
                });
            }

            for instr in &block.instrs {
                Self::verify_instr(program, func, instr)?;
            }

            match &block.terminator {
                Terminator::Jump(target) => {
                    Self::verify_block_target(func, block.name.as_str(), *target)?;
                }
                Terminator::Panic { .. } | Terminator::Unreachable => {}
                Terminator::Branch(branch) => {
                    Self::verify_operand(program, func, &branch.cond)?;
                    if Self::operand_type(program, func, &branch.cond)
                        != Some(crate::ir::IrType::Bool)
                    {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    Self::verify_block_target(func, block.name.as_str(), branch.then_block)?;
                    Self::verify_block_target(func, block.name.as_str(), branch.else_block)?;
                }
                Terminator::Return(value) => {
                    if let Some(value) = value {
                        Self::verify_operand(program, func, value)?;
                        if let Some(ty) = Self::operand_type(program, func, value)
                            && !Self::types_compatible(&ty, &func.ret_ty)
                        {
                            return Err(IrVerifyError::ReturnTypeMismatch {
                                function: func.name.clone(),
                            });
                        }
                    } else if !func.ret_ty.is_void() {
                        return Err(IrVerifyError::ReturnTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_operand(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
    ) -> Result<(), IrVerifyError> {
        match operand {
            Operand::Const(_) => Ok(()),
            Operand::Temp(id) => {
                if func.temps.iter().any(|temp| temp.id == *id) {
                    Ok(())
                } else {
                    Err(IrVerifyError::UnknownTemp {
                        function: func.name.clone(),
                    })
                }
            }
            Operand::Local(id) => {
                if func.locals.iter().any(|local| local.id == *id) {
                    Ok(())
                } else {
                    Err(IrVerifyError::UnknownLocal {
                        function: func.name.clone(),
                    })
                }
            }
            Operand::Global(id) => {
                if program.globals.iter().any(|global| global.id == *id) {
                    Ok(())
                } else {
                    Err(IrVerifyError::UnknownGlobal)
                }
            }
        }
    }
}
