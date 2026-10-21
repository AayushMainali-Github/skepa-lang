use std::collections::{HashMap, HashSet};

use crate::builtins::{BuiltinKind, find_builtin_spec_any};
use crate::ir::{
    ConstValue, FunctionId, Instr, IrFunction, IrProgram, IrType, Operand, Terminator,
};
use crate::types::{
    TypeInfo, display_type, shareability_failure, task_channel_value_type, task_task_type,
    task_task_value_type,
};

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
            Instr::VecGet {
                dst,
                elem_ty,
                vec: array,
                index,
            } => {
                Self::verify_operand(program, func, array)?;
                Self::verify_operand(program, func, index)?;
                Self::expect_index_operand_type(program, func, index)?;
                Self::expect_temp_type(
                    func,
                    *dst,
                    &IrType::Option {
                        value: Box::new(elem_ty.clone()),
                    },
                )?;
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
                if builtin.package == "ffi" && builtin.name == "call" {
                    Self::verify_generic_ffi_call(program, func, args, ret_ty)?;
                    return Ok(());
                }
                if builtin.package == "task" {
                    Self::verify_task_boundary(program, func, &builtin.name, args, ret_ty)?;
                }
                let task_spawn = builtin.package == "task" && builtin.name == "spawn";
                if !task_spawn
                    && let Some(spec) = find_builtin_spec_any(&builtin.package, &builtin.name)
                {
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
                        BuiltinKind::FunctionVariadic => {
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

    fn verify_task_boundary(
        program: &IrProgram,
        func: &IrFunction,
        name: &str,
        args: &[crate::ir::Operand],
        ret_ty: &IrType,
    ) -> Result<(), IrVerifyError> {
        let bad_signature = || IrVerifyError::BadCallSignature {
            function: func.name.clone(),
        };
        let struct_fields = program
            .structs
            .iter()
            .map(|strukt| {
                (
                    strukt.name.clone(),
                    strukt
                        .fields
                        .iter()
                        .map(|field| (field.name.clone(), TypeInfo::from(&field.ty)))
                        .collect::<HashMap<_, _>>(),
                )
            })
            .collect::<HashMap<_, _>>();

        match name {
            "send" => {
                if args.len() != 2 {
                    return Err(bad_signature());
                }
                let Some(channel_ty) = Self::operand_type(program, func, &args[0]) else {
                    return Err(bad_signature());
                };
                let TypeInfo::Opaque(channel_name) = TypeInfo::from(&channel_ty) else {
                    return Err(bad_signature());
                };
                let Some(payload_ty) = task_channel_value_type(&channel_name) else {
                    return Err(bad_signature());
                };
                let Some(value_ir_ty) = Self::operand_type(program, func, &args[1]) else {
                    return Err(bad_signature());
                };
                if !Self::types_compatible(&value_ir_ty, &IrType::from(&payload_ty)) {
                    return Err(bad_signature());
                }
                Self::verify_shareable_task_type(
                    func,
                    "task.send payload",
                    &payload_ty,
                    &struct_fields,
                )?;
                Self::verify_shareable_task_type(
                    func,
                    "task.send payload",
                    &TypeInfo::from(&value_ir_ty),
                    &struct_fields,
                )?;
            }
            "recv" | "join" => {
                if args.len() != 1 {
                    return Err(bad_signature());
                }
                let Some(handle_ty) = Self::operand_type(program, func, &args[0]) else {
                    return Err(bad_signature());
                };
                let TypeInfo::Opaque(handle_name) = TypeInfo::from(&handle_ty) else {
                    return Err(bad_signature());
                };
                let payload_ty = if name == "recv" {
                    task_channel_value_type(&handle_name)
                } else {
                    task_task_value_type(&handle_name)
                }
                .ok_or_else(bad_signature)?;
                if !Self::types_compatible(ret_ty, &IrType::from(&payload_ty)) {
                    return Err(bad_signature());
                }
                let boundary = if name == "recv" {
                    "task.recv payload"
                } else {
                    "task.join result"
                };
                Self::verify_shareable_task_type(func, boundary, &payload_ty, &struct_fields)?;
                Self::verify_shareable_task_type(
                    func,
                    boundary,
                    &TypeInfo::from(ret_ty),
                    &struct_fields,
                )?;
            }
            "spawn" => {
                if args.is_empty() {
                    return Err(bad_signature());
                }
                let Some(code_ty) = Self::operand_type(program, func, &args[0]) else {
                    return Err(bad_signature());
                };
                match TypeInfo::from(&code_ty) {
                    TypeInfo::Fn { params, ret } => {
                        if params.len() + 1 != args.len() {
                            return Err(bad_signature());
                        }
                        for (index, param) in params.iter().enumerate() {
                            let Some(actual_ty) =
                                Self::operand_type(program, func, &args[index + 1])
                            else {
                                return Err(bad_signature());
                            };
                            if !Self::types_compatible(&actual_ty, &IrType::from(param)) {
                                return Err(bad_signature());
                            }
                            Self::verify_shareable_task_type(
                                func,
                                &format!("task.spawn argument {}", index + 1),
                                param,
                                &struct_fields,
                            )?;
                            Self::verify_shareable_task_type(
                                func,
                                &format!("task.spawn argument {}", index + 1),
                                &TypeInfo::from(&actual_ty),
                                &struct_fields,
                            )?;
                        }
                        let expected_task_ty = IrType::Result {
                            ok: Box::new(IrType::from(&task_task_type(&ret))),
                            err: Box::new(IrType::String),
                        };
                        if matches!(ret_ty, IrType::Unknown)
                            || !Self::types_compatible(ret_ty, &expected_task_ty)
                        {
                            return Err(bad_signature());
                        }
                        Self::verify_shareable_task_type(
                            func,
                            "task.spawn result",
                            &ret,
                            &struct_fields,
                        )?;
                        Self::verify_spawned_code_effects(program, func, &args[0], &struct_fields)?;
                    }
                    TypeInfo::Unknown => {
                        Self::verify_shareable_task_type(
                            func,
                            "task.spawn code entry",
                            &TypeInfo::Unknown,
                            &struct_fields,
                        )?;
                    }
                    _ => return Err(bad_signature()),
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn verify_spawned_code_effects(
        program: &IrProgram,
        caller: &IrFunction,
        code_entry: &Operand,
        struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
    ) -> Result<(), IrVerifyError> {
        let Some(target) = Self::resolve_function_value(caller, code_entry) else {
            return Err(IrVerifyError::UnverifiableTaskCodeEntry {
                function: caller.name.clone(),
            });
        };
        Self::verify_task_function_effects(program, target, struct_fields, &mut HashSet::new())
    }

    fn verify_task_function_effects(
        program: &IrProgram,
        function_id: FunctionId,
        struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
        visiting: &mut HashSet<FunctionId>,
    ) -> Result<(), IrVerifyError> {
        if !visiting.insert(function_id) {
            return Ok(());
        }
        let Some(function) = program
            .functions
            .iter()
            .find(|candidate| candidate.id == function_id)
        else {
            return Err(IrVerifyError::UnknownFunctionTarget {
                function: format!("task function {:?}", function_id),
            });
        };

        for block in &function.blocks {
            for instruction in &block.instrs {
                if let Some(global_id) = Self::mutated_global(instruction) {
                    let Some(global) = program.globals.iter().find(|g| g.id == global_id) else {
                        return Err(IrVerifyError::UnknownGlobal);
                    };
                    return Err(IrVerifyError::UnsafeTaskGlobalMutation {
                        function: function.name.clone(),
                        global: global.name.clone(),
                    });
                }

                for global_id in
                    Self::referenced_globals(instruction)
                        .into_iter()
                        .chain(match instruction {
                            Instr::LoadGlobal { global, .. } => Some(*global),
                            _ => None,
                        })
                {
                    let Some(global) = program.globals.iter().find(|g| g.id == global_id) else {
                        return Err(IrVerifyError::UnknownGlobal);
                    };
                    Self::verify_shareable_task_type(
                        function,
                        &format!("task.spawn global `{}`", global.name),
                        &TypeInfo::from(&global.ty),
                        struct_fields,
                    )?;
                }

                match instruction {
                    Instr::CallDirect {
                        function: callee_id,
                        ..
                    } => Self::verify_task_function_effects(
                        program,
                        *callee_id,
                        struct_fields,
                        visiting,
                    )?,
                    Instr::CallIndirect { callee, .. } => {
                        let Some(target) = Self::resolve_function_value(function, callee) else {
                            return Err(IrVerifyError::UnverifiableTaskCallTarget {
                                function: function.name.clone(),
                            });
                        };
                        Self::verify_task_function_effects(
                            program,
                            target,
                            struct_fields,
                            visiting,
                        )?;
                    }
                    _ => {}
                }
            }
            let terminator_operands = match &block.terminator {
                Terminator::Branch(branch) => vec![&branch.cond],
                Terminator::Return(Some(value)) => vec![value],
                _ => Vec::new(),
            };
            for operand in terminator_operands {
                if let Operand::Global(global_id) = operand {
                    let Some(global) = program.globals.iter().find(|g| g.id == *global_id) else {
                        return Err(IrVerifyError::UnknownGlobal);
                    };
                    Self::verify_shareable_task_type(
                        function,
                        &format!("task.spawn global `{}`", global.name),
                        &TypeInfo::from(&global.ty),
                        struct_fields,
                    )?;
                }
            }
        }
        visiting.remove(&function_id);
        Ok(())
    }

    fn resolve_function_value(function: &IrFunction, operand: &Operand) -> Option<FunctionId> {
        fn resolve(
            ir_function: &IrFunction,
            operand: &Operand,
            visiting: &mut HashSet<(bool, usize)>,
        ) -> Option<FunctionId> {
            match operand {
                Operand::Temp(temp) => {
                    if !visiting.insert((false, temp.0)) {
                        return None;
                    }
                    for instruction in ir_function.blocks.iter().flat_map(|block| &block.instrs) {
                        match instruction {
                            Instr::MakeClosure {
                                dst,
                                function: target,
                            } if dst == temp => {
                                return Some(*target);
                            }
                            Instr::Copy { dst, src, .. } if dst == temp => {
                                return resolve(ir_function, src, visiting);
                            }
                            Instr::LoadLocal { dst, local, .. } if dst == temp => {
                                return resolve_local(ir_function, *local, visiting);
                            }
                            _ => {}
                        }
                    }
                    None
                }
                Operand::Local(local) => resolve_local(ir_function, *local, visiting),
                _ => None,
            }
        }

        fn resolve_local(
            function: &IrFunction,
            local: crate::ir::LocalId,
            visiting: &mut HashSet<(bool, usize)>,
        ) -> Option<FunctionId> {
            if !visiting.insert((true, local.0)) {
                return None;
            }
            let values = function
                .blocks
                .iter()
                .flat_map(|block| &block.instrs)
                .filter_map(|instruction| match instruction {
                    Instr::StoreLocal {
                        local: stored_local,
                        value,
                        ..
                    } if *stored_local == local => Some(value),
                    _ => None,
                });
            let mut target = None;
            for value in values {
                let mut branch_visiting = visiting.clone();
                let resolved = resolve(function, value, &mut branch_visiting)?;
                if target.is_some_and(|existing| existing != resolved) {
                    return None;
                }
                target = Some(resolved);
            }
            target
        }

        resolve(function, operand, &mut HashSet::new())
    }

    fn referenced_globals(instruction: &Instr) -> Vec<crate::ir::GlobalId> {
        fn add_operand(globals: &mut Vec<crate::ir::GlobalId>, operand: &Operand) {
            if let Operand::Global(global) = operand {
                globals.push(*global);
            }
        }

        let mut globals = Vec::new();
        match instruction {
            Instr::Const { .. }
            | Instr::LoadGlobal { .. }
            | Instr::LoadLocal { .. }
            | Instr::VecNew { .. }
            | Instr::MakeClosure { .. } => {}
            Instr::Copy { src, .. }
            | Instr::Unary { operand: src, .. }
            | Instr::StoreLocal { value: src, .. }
            | Instr::MakeArrayRepeat { value: src, .. } => add_operand(&mut globals, src),
            Instr::StoreGlobal { value, .. } => add_operand(&mut globals, value),
            Instr::Binary { left, right, .. }
            | Instr::Compare { left, right, .. }
            | Instr::Logic { left, right, .. } => {
                add_operand(&mut globals, left);
                add_operand(&mut globals, right);
            }
            Instr::MakeArray { items, .. } | Instr::MakeStruct { fields: items, .. } => {
                for item in items {
                    add_operand(&mut globals, item);
                }
            }
            Instr::ArrayGet { array, index, .. }
            | Instr::VecGet {
                vec: array, index, ..
            }
            | Instr::VecDelete {
                vec: array, index, ..
            } => {
                add_operand(&mut globals, array);
                add_operand(&mut globals, index);
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
                add_operand(&mut globals, array);
                add_operand(&mut globals, index);
                add_operand(&mut globals, value);
            }
            Instr::VecLen { vec, .. } => add_operand(&mut globals, vec),
            Instr::VecPush { vec, value } => {
                add_operand(&mut globals, vec);
                add_operand(&mut globals, value);
            }
            Instr::StructGet { base, .. } => add_operand(&mut globals, base),
            Instr::StructSet { base, value, .. } => {
                add_operand(&mut globals, base);
                add_operand(&mut globals, value);
            }
            Instr::CallDirect { args, .. } | Instr::CallBuiltin { args, .. } => {
                for arg in args {
                    add_operand(&mut globals, arg);
                }
            }
            Instr::CallIndirect { callee, args, .. } => {
                add_operand(&mut globals, callee);
                for arg in args {
                    add_operand(&mut globals, arg);
                }
            }
        }
        globals
    }

    fn mutated_global(instruction: &Instr) -> Option<crate::ir::GlobalId> {
        match instruction {
            Instr::StoreGlobal { global, .. } => Some(*global),
            Instr::ArraySet {
                array: Operand::Global(global),
                ..
            }
            | Instr::VecPush {
                vec: Operand::Global(global),
                ..
            }
            | Instr::VecSet {
                vec: Operand::Global(global),
                ..
            }
            | Instr::VecDelete {
                vec: Operand::Global(global),
                ..
            }
            | Instr::StructSet {
                base: Operand::Global(global),
                ..
            } => Some(*global),
            Instr::CallBuiltin { builtin, args, .. }
                if ((builtin.package == "vec"
                    && matches!(builtin.name.as_str(), "push" | "set" | "delete"))
                    || (builtin.package == "map"
                        && matches!(builtin.name.as_str(), "insert" | "remove")))
                    && matches!(args.first(), Some(Operand::Global(_))) =>
            {
                match args.first() {
                    Some(Operand::Global(global)) => Some(*global),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn verify_shareable_task_type(
        func: &IrFunction,
        boundary: &str,
        ty: &TypeInfo,
        struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
    ) -> Result<(), IrVerifyError> {
        let failure = shareability_failure(ty, struct_fields);
        let Some(failure) = failure else {
            return Ok(());
        };
        Err(IrVerifyError::NonShareableConcurrencyValue {
            function: func.name.clone(),
            boundary: boundary.to_string(),
            capability: failure.capability,
            path: failure.path,
            ty: display_type(&failure.ty),
        })
    }

    fn verify_generic_ffi_call(
        program: &IrProgram,
        func: &IrFunction,
        args: &[crate::ir::Operand],
        ret_ty: &IrType,
    ) -> Result<(), IrVerifyError> {
        if args.len() < 2 {
            return Err(IrVerifyError::BadCallSignature {
                function: func.name.clone(),
            });
        }
        Self::expect_operand_type(
            program,
            func,
            &args[0],
            &IrType::Opaque("ffi.Symbol".to_string()),
        )?;
        Self::expect_operand_type(program, func, &args[1], &IrType::String)?;
        let Some(signature) = Self::const_string_value(&args[1]) else {
            return Err(IrVerifyError::BadCallSignature {
                function: func.name.clone(),
            });
        };
        let Some((expected_args, expected_ret)) = Self::ffi_signature_shape(signature) else {
            return Err(IrVerifyError::BadCallSignature {
                function: func.name.clone(),
            });
        };
        if args.len() != expected_args.len() + 2 || !Self::types_compatible(ret_ty, &expected_ret) {
            return Err(IrVerifyError::BadCallSignature {
                function: func.name.clone(),
            });
        }
        for (arg, expected) in args[2..].iter().zip(expected_args.iter()) {
            Self::expect_operand_type(program, func, arg, expected)?;
        }
        Ok(())
    }

    fn const_string_value(operand: &crate::ir::Operand) -> Option<&str> {
        match operand {
            crate::ir::Operand::Const(ConstValue::String(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    fn ffi_signature_shape(signature: &str) -> Option<(Vec<IrType>, IrType)> {
        let shape = match signature {
            "->i64" => (vec![], IrType::Int),
            "->void" => (vec![], IrType::Void),
            "->_Bool" => (vec![], IrType::Bool),
            "->i32bool" | "system:->BOOL" => (vec![], IrType::Bool),
            "i64->i64" => (vec![IrType::Int], IrType::Int),
            "i64->_Bool" => (vec![IrType::Int], IrType::Bool),
            "i64->i32bool" | "system:i64->BOOL" => (vec![IrType::Int], IrType::Bool),
            "i64->void" => (vec![IrType::Int], IrType::Void),
            "cstr->usize" | "system:cstr->i32" => (vec![IrType::String], IrType::Int),
            "cstr->void" | "system:cstr->void" => (vec![IrType::String], IrType::Void),
            "cstr,cstr->i32" | "system:cstr,cstr->i32" => {
                (vec![IrType::String, IrType::String], IrType::Int)
            }
            "cstr,usize->usize" => (vec![IrType::String, IrType::Int], IrType::Int),
            "i64,i64->i64" => (vec![IrType::Int, IrType::Int], IrType::Int),
            "bytes->usize" => (vec![IrType::Bytes], IrType::Int),
            "bytes,usize->usize" => (vec![IrType::Bytes, IrType::Int], IrType::Int),
            _ => return None,
        };
        Some(shape)
    }
}
