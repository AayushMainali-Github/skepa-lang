use std::collections::HashSet;

use crate::ir::{IrFunction, IrProgram, IrType, Operand};

use super::{IrVerifier, IrVerifyError};

impl IrVerifier {
    pub(super) fn verify_unique_ids(func: &IrFunction) -> Result<(), IrVerifyError> {
        let mut param_ids = HashSet::new();
        for param in &func.params {
            if !param_ids.insert(param.id) {
                return Err(IrVerifyError::DuplicateParamId {
                    function: func.name.clone(),
                });
            }
        }
        let mut local_ids = HashSet::new();
        for local in &func.locals {
            if !local_ids.insert(local.id) {
                return Err(IrVerifyError::DuplicateLocalId {
                    function: func.name.clone(),
                });
            }
        }
        let mut temp_ids = HashSet::new();
        for temp in &func.temps {
            if !temp_ids.insert(temp.id) {
                return Err(IrVerifyError::DuplicateTempId {
                    function: func.name.clone(),
                });
            }
        }
        let mut block_ids = HashSet::new();
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                return Err(IrVerifyError::DuplicateBlockId {
                    function: func.name.clone(),
                });
            }
        }
        Ok(())
    }

    pub(super) fn verify_block_target(
        func: &IrFunction,
        block: &str,
        target: crate::ir::BlockId,
    ) -> Result<(), IrVerifyError> {
        if func.blocks.iter().any(|candidate| candidate.id == target) {
            Ok(())
        } else {
            Err(IrVerifyError::UnknownBlockTarget {
                function: func.name.clone(),
                block: block.to_string(),
            })
        }
    }

    pub(super) fn expect_index_operand_type(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
    ) -> Result<(), IrVerifyError> {
        Self::expect_operand_type(program, func, operand, &IrType::Int)
    }

    pub(super) fn expect_operand_type(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
        expected: &IrType,
    ) -> Result<(), IrVerifyError> {
        if let Some(actual) = Self::operand_type(program, func, operand)
            && !Self::types_compatible(&actual, expected)
        {
            return Err(IrVerifyError::OperandTypeMismatch {
                function: func.name.clone(),
            });
        }
        Ok(())
    }

    pub(super) fn expect_temp_type(
        func: &IrFunction,
        dst: crate::ir::TempId,
        expected: &IrType,
    ) -> Result<(), IrVerifyError> {
        let Some(actual) = func
            .temps
            .iter()
            .find(|temp| temp.id == dst)
            .map(|temp| temp.ty.clone())
        else {
            return Err(IrVerifyError::UnknownTemp {
                function: func.name.clone(),
            });
        };
        if !Self::types_compatible(&actual, expected) {
            return Err(IrVerifyError::OperandTypeMismatch {
                function: func.name.clone(),
            });
        }
        Ok(())
    }

    pub(super) fn expect_call_destination_type(
        func: &IrFunction,
        dst: Option<crate::ir::TempId>,
        ret_ty: &IrType,
        expected_ret: &IrType,
    ) -> Result<(), IrVerifyError> {
        match dst {
            Some(dst) => {
                Self::expect_temp_type(func, dst, ret_ty)?;
                if !matches!(expected_ret, IrType::Unknown)
                    && !Self::types_compatible(ret_ty, expected_ret)
                {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
            }
            None => {
                if !matches!(ret_ty, IrType::Void | IrType::Unknown)
                    || !matches!(expected_ret, IrType::Void | IrType::Unknown)
                {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn types_compatible(actual: &IrType, expected: &IrType) -> bool {
        if actual == expected {
            return true;
        }
        match (actual, expected) {
            (_, IrType::Unknown) | (IrType::Unknown, _) => true,
            (IrType::Option { value: a }, IrType::Option { value: b }) => {
                Self::types_compatible(a, b)
            }
            (
                IrType::Result {
                    ok: a_ok,
                    err: a_err,
                },
                IrType::Result {
                    ok: b_ok,
                    err: b_err,
                },
            ) => Self::types_compatible(a_ok, b_ok) && Self::types_compatible(a_err, b_err),
            (
                IrType::Array {
                    elem: a_elem,
                    size: a_size,
                },
                IrType::Array {
                    elem: b_elem,
                    size: b_size,
                },
            ) => a_size == b_size && Self::types_compatible(a_elem, b_elem),
            (IrType::Vec { elem: a }, IrType::Vec { elem: b }) => Self::types_compatible(a, b),
            (IrType::Map { value: a }, IrType::Map { value: b }) => Self::types_compatible(a, b),
            (
                IrType::Fn {
                    params: a_params,
                    ret: a_ret,
                },
                IrType::Fn {
                    params: b_params,
                    ret: b_ret,
                },
            ) => {
                a_params.len() == b_params.len()
                    && a_params
                        .iter()
                        .zip(b_params.iter())
                        .all(|(a, b)| Self::types_compatible(a, b))
                    && Self::types_compatible(a_ret, b_ret)
            }
            _ => false,
        }
    }

    pub(super) fn verify_field_ref(
        program: &IrProgram,
        func: &IrFunction,
        base: &Operand,
        field: &crate::ir::FieldRef,
    ) -> Result<(), IrVerifyError> {
        let Some(crate::ir::IrType::Named(struct_name)) = Self::operand_type(program, func, base)
        else {
            return Ok(());
        };
        let Some(strukt) = program
            .structs
            .iter()
            .find(|candidate| candidate.name == *struct_name)
        else {
            return Err(IrVerifyError::UnknownStruct {
                function: func.name.clone(),
            });
        };
        if field.index >= strukt.fields.len() || strukt.fields[field.index].name != field.name {
            return Err(IrVerifyError::UnknownField {
                function: func.name.clone(),
                field: field.name.clone(),
            });
        }
        Ok(())
    }

    pub(super) fn field_type(
        program: &IrProgram,
        func: &IrFunction,
        base: &Operand,
        field: &crate::ir::FieldRef,
    ) -> Option<crate::ir::IrType> {
        let crate::ir::IrType::Named(struct_name) = Self::operand_type(program, func, base)? else {
            return None;
        };
        let strukt = program
            .structs
            .iter()
            .find(|candidate| candidate.name == struct_name)?;
        strukt.fields.get(field.index).map(|entry| entry.ty.clone())
    }

    pub(super) fn container_elem_type(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
    ) -> crate::ir::IrType {
        match Self::operand_type(program, func, operand) {
            Some(crate::ir::IrType::Array { elem, .. }) => *elem,
            Some(crate::ir::IrType::Vec { elem }) => *elem,
            _ => crate::ir::IrType::Unknown,
        }
    }

    pub(super) fn operand_type(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
    ) -> Option<crate::ir::IrType> {
        match operand {
            Operand::Const(value) => Some(match value {
                crate::ir::ConstValue::Int(_) => crate::ir::IrType::Int,
                crate::ir::ConstValue::Float(_) => crate::ir::IrType::Float,
                crate::ir::ConstValue::Bool(_) => crate::ir::IrType::Bool,
                crate::ir::ConstValue::String(_) => crate::ir::IrType::String,
                crate::ir::ConstValue::Unit => crate::ir::IrType::Void,
            }),
            Operand::Temp(id) => func
                .temps
                .iter()
                .find(|temp| temp.id == *id)
                .map(|temp| temp.ty.clone()),
            Operand::Local(id) => func
                .locals
                .iter()
                .find(|local| local.id == *id)
                .map(|local| local.ty.clone()),
            Operand::Global(id) => program
                .globals
                .iter()
                .find(|global| global.id == *id)
                .map(|global| global.ty.clone()),
        }
    }
}
