use crate::ir::{IrFunction, IrProgram, Operand, Terminator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrVerifyError {
    MissingEntryBlock { function: String },
    MissingTerminator { function: String, block: String },
    UnknownTemp { function: String },
    UnknownLocal { function: String },
    UnknownGlobal,
}

pub struct IrVerifier;

impl IrVerifier {
    pub fn verify_program(program: &IrProgram) -> Result<(), IrVerifyError> {
        for func in &program.functions {
            Self::verify_function(program, func)?;
        }
        Ok(())
    }

    pub fn verify_function(program: &IrProgram, func: &IrFunction) -> Result<(), IrVerifyError> {
        if !func.blocks.iter().any(|block| block.id == func.entry) {
            return Err(IrVerifyError::MissingEntryBlock {
                function: func.name.clone(),
            });
        }

        for block in &func.blocks {
            if matches!(block.terminator, Terminator::Unreachable) && !block.instrs.is_empty() {
                return Err(IrVerifyError::MissingTerminator {
                    function: func.name.clone(),
                    block: block.name.clone(),
                });
            }

            for instr in &block.instrs {
                match instr {
                    crate::ir::Instr::Copy { src, .. }
                    | crate::ir::Instr::Unary { operand: src, .. } => {
                        Self::verify_operand(program, func, src)?;
                    }
                    crate::ir::Instr::Binary { left, right, .. }
                    | crate::ir::Instr::Compare { left, right, .. }
                    | crate::ir::Instr::Logic { left, right, .. } => {
                        Self::verify_operand(program, func, left)?;
                        Self::verify_operand(program, func, right)?;
                    }
                    crate::ir::Instr::StoreGlobal { value, .. }
                    | crate::ir::Instr::StoreLocal { value, .. } => {
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::VecPush { vec, value } => {
                        Self::verify_operand(program, func, vec)?;
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::MakeArray { items, .. } => {
                        for item in items {
                            Self::verify_operand(program, func, item)?;
                        }
                    }
                    crate::ir::Instr::MakeArrayRepeat { value, .. } => {
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::ArrayGet { array, index, .. }
                    | crate::ir::Instr::VecGet {
                        vec: array, index, ..
                    } => {
                        Self::verify_operand(program, func, array)?;
                        Self::verify_operand(program, func, index)?;
                    }
                    crate::ir::Instr::ArraySet {
                        array,
                        index,
                        value,
                        ..
                    }
                    | crate::ir::Instr::VecSet {
                        vec: array,
                        index,
                        value,
                        ..
                    } => {
                        Self::verify_operand(program, func, array)?;
                        Self::verify_operand(program, func, index)?;
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::MakeStruct { fields, .. } => {
                        for field in fields {
                            Self::verify_operand(program, func, field)?;
                        }
                    }
                    crate::ir::Instr::StructGet { base, .. } => {
                        Self::verify_operand(program, func, base)?;
                    }
                    crate::ir::Instr::StructSet { base, value, .. } => {
                        Self::verify_operand(program, func, base)?;
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::CallDirect { args, .. }
                    | crate::ir::Instr::CallBuiltin { args, .. } => {
                        for arg in args {
                            Self::verify_operand(program, func, arg)?;
                        }
                    }
                    crate::ir::Instr::CallIndirect { callee, args, .. } => {
                        Self::verify_operand(program, func, callee)?;
                        for arg in args {
                            Self::verify_operand(program, func, arg)?;
                        }
                    }
                    crate::ir::Instr::Const { .. }
                    | crate::ir::Instr::LoadGlobal { .. }
                    | crate::ir::Instr::LoadLocal { .. }
                    | crate::ir::Instr::MakeClosure { .. } => {}
                }
            }

            match &block.terminator {
                Terminator::Jump(_) | Terminator::Panic { .. } | Terminator::Unreachable => {}
                Terminator::Branch(branch) => {
                    Self::verify_operand(program, func, &branch.cond)?;
                }
                Terminator::Return(value) => {
                    if let Some(value) = value {
                        Self::verify_operand(program, func, value)?;
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
