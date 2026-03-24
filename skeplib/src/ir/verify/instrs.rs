use crate::builtins::{BuiltinKind, find_builtin_spec};
use crate::ir::{Instr, IrFunction, IrProgram, IrType};

use super::{IrVerifier, IrVerifyError};

impl IrVerifier {
    pub(super) fn verify_instr(
        program: &IrProgram,
        func: &IrFunction,
        instr: &Instr,
    ) -> Result<(), IrVerifyError> {
        match instr {
            Instr::Copy { dst, src, ty } => {
                Self::verify_operand(program, func, src)?;
                Self::expect_temp_type(func, *dst, ty)?;
                Self::expect_operand_type(program, func, src, ty)?;
            }
            Instr::Unary {
                dst,
                ty,
                op,
                operand,
            } => {
                Self::verify_operand(program, func, operand)?;
                Self::expect_temp_type(func, *dst, ty)?;
                let expected_operand_ty = match op {
                    crate::ir::UnaryOp::Neg => ty,
                    crate::ir::UnaryOp::Not => &IrType::Bool,
                    crate::ir::UnaryOp::BitNot => &IrType::Int,
                };
                Self::expect_operand_type(program, func, operand, expected_operand_ty)?;
                match op {
                    crate::ir::UnaryOp::Neg
                        if !matches!(ty, IrType::Int | IrType::Float | IrType::Unknown) =>
                    {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    crate::ir::UnaryOp::Not if !matches!(ty, IrType::Bool | IrType::Unknown) => {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    crate::ir::UnaryOp::BitNot if !matches!(ty, IrType::Int | IrType::Unknown) => {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    _ => {}
                }
            }
            Instr::Binary {
                dst,
                ty,
                op,
                left,
                right,
            } => {
                Self::verify_operand(program, func, left)?;
                Self::verify_operand(program, func, right)?;
                Self::expect_temp_type(func, *dst, ty)?;
                Self::expect_operand_type(program, func, left, ty)?;
                Self::expect_operand_type(program, func, right, ty)?;
                match op {
                    crate::ir::BinaryOp::Add
                        if !matches!(
                            ty,
                            IrType::Int | IrType::Float | IrType::String | IrType::Unknown
                        ) =>
                    {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    crate::ir::BinaryOp::Sub
                    | crate::ir::BinaryOp::Mul
                    | crate::ir::BinaryOp::Div
                    | crate::ir::BinaryOp::Mod
                        if !matches!(ty, IrType::Int | IrType::Float | IrType::Unknown) =>
                    {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    crate::ir::BinaryOp::BitAnd
                    | crate::ir::BinaryOp::BitOr
                    | crate::ir::BinaryOp::BitXor
                    | crate::ir::BinaryOp::Shl
                    | crate::ir::BinaryOp::Shr
                        if !matches!(ty, IrType::Int | IrType::Unknown) =>
                    {
                        return Err(IrVerifyError::OperandTypeMismatch {
                            function: func.name.clone(),
                        });
                    }
                    _ => {}
                }
            }
            Instr::Compare {
                dst, left, right, ..
            } => {
                Self::verify_operand(program, func, left)?;
                Self::verify_operand(program, func, right)?;
                Self::expect_temp_type(func, *dst, &IrType::Bool)?;
                let Some(left_ty) = Self::operand_type(program, func, left) else {
                    return Ok(());
                };
                let Some(right_ty) = Self::operand_type(program, func, right) else {
                    return Ok(());
                };
                if !Self::types_compatible(&left_ty, &right_ty) {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::Logic {
                dst, left, right, ..
            } => {
                Self::verify_operand(program, func, left)?;
                Self::verify_operand(program, func, right)?;
                Self::expect_temp_type(func, *dst, &IrType::Bool)?;
                Self::expect_operand_type(program, func, left, &IrType::Bool)?;
                Self::expect_operand_type(program, func, right, &IrType::Bool)?;
            }
            Instr::StoreGlobal { global, value, .. } => {
                let Some(global_ty) = program
                    .globals
                    .iter()
                    .find(|candidate| candidate.id == *global)
                    .map(|candidate| candidate.ty.clone())
                else {
                    return Err(IrVerifyError::UnknownGlobal);
                };
                Self::verify_operand(program, func, value)?;
                Self::expect_operand_type(program, func, value, &global_ty)?;
            }
            Instr::StoreLocal { local, value, .. } => {
                let Some(local_ty) = func
                    .locals
                    .iter()
                    .find(|candidate| candidate.id == *local)
                    .map(|candidate| candidate.ty.clone())
                else {
                    return Err(IrVerifyError::UnknownLocal {
                        function: func.name.clone(),
                    });
                };
                Self::verify_operand(program, func, value)?;
                Self::expect_operand_type(program, func, value, &local_ty)?;
            }
            Instr::VecPush { vec, value } => {
                Self::verify_operand(program, func, vec)?;
                Self::verify_operand(program, func, value)?;
                let expected_elem_ty = Self::container_elem_type(program, func, vec);
                if !matches!(expected_elem_ty, IrType::Unknown) {
                    Self::expect_operand_type(program, func, value, &expected_elem_ty)?;
                }
            }
            Instr::MakeArray { elem_ty, items, .. } => {
                for item in items {
                    Self::verify_operand(program, func, item)?;
                    Self::expect_operand_type(program, func, item, elem_ty)?;
                }
            }
            Instr::MakeArrayRepeat { elem_ty, value, .. } => {
                Self::verify_operand(program, func, value)?;
                Self::expect_operand_type(program, func, value, elem_ty)?;
            }
            Instr::VecLen { dst, vec: array } => {
                Self::verify_operand(program, func, array)?;
                Self::expect_temp_type(func, *dst, &IrType::Int)?;
                if !matches!(
                    Self::operand_type(program, func, array),
                    Some(IrType::Vec { .. })
                ) {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::ArrayGet {
                dst,
                elem_ty,
                array,
                index,
            }
            | Instr::VecGet {
                dst,
                elem_ty,
                vec: array,
                index,
            } => {
                Self::verify_operand(program, func, array)?;
                Self::verify_operand(program, func, index)?;
                Self::expect_index_operand_type(program, func, index)?;
                Self::expect_temp_type(func, *dst, elem_ty)?;
                let expected_elem_ty = Self::container_elem_type(program, func, array);
                if !matches!(expected_elem_ty, IrType::Unknown)
                    && !Self::types_compatible(elem_ty, &expected_elem_ty)
                {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::ArraySet {
                array,
                index,
                value,
                ..
            }
            | Instr::VecSet {
                vec: array,
                index,
                value,
                ..
            } => {
                Self::verify_operand(program, func, array)?;
                Self::verify_operand(program, func, index)?;
                Self::verify_operand(program, func, value)?;
                Self::expect_index_operand_type(program, func, index)?;
                let expected_elem_ty = Self::container_elem_type(program, func, array);
                if !matches!(expected_elem_ty, IrType::Unknown) {
                    Self::expect_operand_type(program, func, value, &expected_elem_ty)?;
                }
            }
            Instr::VecDelete {
                vec: array, index, ..
            } => {
                Self::verify_operand(program, func, array)?;
                Self::verify_operand(program, func, index)?;
                Self::expect_index_operand_type(program, func, index)?;
            }
            Instr::MakeStruct {
                struct_id, fields, ..
            } => {
                let Some(strukt) = program
                    .structs
                    .iter()
                    .find(|candidate| candidate.id == *struct_id)
                else {
                    return Err(IrVerifyError::UnknownStruct {
                        function: func.name.clone(),
                    });
                };
                if strukt.fields.len() != fields.len() {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
                for (field, expected) in fields.iter().zip(strukt.fields.iter()) {
                    Self::verify_operand(program, func, field)?;
                    Self::expect_operand_type(program, func, field, &expected.ty)?;
                }
            }
            Instr::StructGet {
                dst,
                ty,
                base,
                field,
            } => {
                Self::verify_operand(program, func, base)?;
                Self::verify_field_ref(program, func, base, field)?;
                Self::expect_temp_type(func, *dst, ty)?;
                if let Some(expected_ty) = Self::field_type(program, func, base, field)
                    && !Self::types_compatible(ty, &expected_ty)
                {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::StructSet {
                base, field, value, ..
            } => {
                Self::verify_operand(program, func, base)?;
                Self::verify_field_ref(program, func, base, field)?;
                Self::verify_operand(program, func, value)?;
                if let Some(expected_ty) = Self::field_type(program, func, base, field) {
                    Self::expect_operand_type(program, func, value, &expected_ty)?;
                }
            }
            Instr::CallDirect {
                dst,
                function,
                args,
                ret_ty,
            } => {
                let Some(target) = program
                    .functions
                    .iter()
                    .find(|candidate| candidate.id == *function)
                else {
                    return Err(IrVerifyError::UnknownFunctionTarget {
                        function: func.name.clone(),
                    });
                };
                if target.params.len() != args.len() {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
                for (arg, param) in args.iter().zip(target.params.iter()) {
                    Self::verify_operand(program, func, arg)?;
                    Self::expect_operand_type(program, func, arg, &param.ty)?;
                }
                Self::expect_call_destination_type(func, *dst, ret_ty, &target.ret_ty)?;
            }
            Instr::CallBuiltin {
                builtin,
                args,
                ret_ty,
                ..
            } => {
                for arg in args {
                    Self::verify_operand(program, func, arg)?;
                }
                if let Some(spec) = find_builtin_spec(&builtin.package, &builtin.name) {
                    if !matches!(ret_ty, IrType::Unknown | IrType::Void) {
                        let expected_ret = IrType::from(&spec.sig.ret);
                        if !Self::types_compatible(ret_ty, &expected_ret) {
                            return Err(IrVerifyError::BadCallSignature {
                                function: func.name.clone(),
                            });
                        }
                    }
                    match spec.sig.kind {
                        BuiltinKind::FixedArity => {
                            if spec.sig.params.len() != args.len() {
                                return Err(IrVerifyError::BadCallSignature {
                                    function: func.name.clone(),
                                });
                            }
                            for (arg, param_ty) in args.iter().zip(spec.sig.params.iter()) {
                                let expected = IrType::from(param_ty);
                                Self::expect_operand_type(program, func, arg, &expected)?;
                            }
                        }
                        BuiltinKind::ArrayOps => {
                            if args.is_empty() {
                                return Err(IrVerifyError::BadCallSignature {
                                    function: func.name.clone(),
                                });
                            }
                            for arg in args {
                                Self::verify_operand(program, func, arg)?;
                            }
                        }
                        BuiltinKind::FormatVariadic => {
                            if args.is_empty() {
                                return Err(IrVerifyError::BadCallSignature {
                                    function: func.name.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Instr::CallIndirect {
                callee,
                args,
                ret_ty,
                ..
            } => {
                Self::verify_operand(program, func, callee)?;
                for arg in args {
                    Self::verify_operand(program, func, arg)?;
                }
                let Some(callee_ty) = Self::operand_type(program, func, callee) else {
                    return Ok(());
                };
                let (params, ret) = match callee_ty {
                    IrType::Fn { params, ret } => (params, ret),
                    IrType::Unknown => return Ok(()),
                    _ => {
                        return Err(IrVerifyError::BadCallSignature {
                            function: func.name.clone(),
                        });
                    }
                };
                if params.len() != args.len() || !Self::types_compatible(ret_ty, &ret) {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
                for (arg, param_ty) in args.iter().zip(params.iter()) {
                    Self::expect_operand_type(program, func, arg, param_ty)?;
                }
            }
            Instr::Const { .. } => {}
            Instr::LoadGlobal { dst, ty, global } => {
                let Some(global_ty) = program
                    .globals
                    .iter()
                    .find(|candidate| candidate.id == *global)
                    .map(|candidate| candidate.ty.clone())
                else {
                    return Err(IrVerifyError::UnknownGlobal);
                };
                Self::expect_temp_type(func, *dst, ty)?;
                if !Self::types_compatible(ty, &global_ty) {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::LoadLocal { dst, ty, local } => {
                let Some(local_ty) = func
                    .locals
                    .iter()
                    .find(|candidate| candidate.id == *local)
                    .map(|candidate| candidate.ty.clone())
                else {
                    return Err(IrVerifyError::UnknownLocal {
                        function: func.name.clone(),
                    });
                };
                Self::expect_temp_type(func, *dst, ty)?;
                if !Self::types_compatible(ty, &local_ty) {
                    return Err(IrVerifyError::OperandTypeMismatch {
                        function: func.name.clone(),
                    });
                }
            }
            Instr::VecNew { .. } => {}
            Instr::MakeClosure { dst, function } => {
                let Some(target) = program
                    .functions
                    .iter()
                    .find(|candidate| candidate.id == *function)
                else {
                    return Err(IrVerifyError::UnknownFunctionTarget {
                        function: func.name.clone(),
                    });
                };
                let Some(actual_dst_ty) = func
                    .temps
                    .iter()
                    .find(|temp| temp.id == *dst)
                    .map(|temp| temp.ty.clone())
                else {
                    return Err(IrVerifyError::UnknownTemp {
                        function: func.name.clone(),
                    });
                };
                let expected_dst_ty = IrType::Fn {
                    params: target.params.iter().map(|param| param.ty.clone()).collect(),
                    ret: Box::new(target.ret_ty.clone()),
                };
                if !Self::types_compatible(&actual_dst_ty, &expected_dst_ty) {
                    return Err(IrVerifyError::BadCallSignature {
                        function: func.name.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}
