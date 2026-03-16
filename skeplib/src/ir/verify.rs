use crate::builtins::{BuiltinKind, find_builtin_sig};
use crate::ir::{IrFunction, IrProgram, IrType, Operand, Terminator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrVerifyError {
    MissingEntryBlock { function: String },
    DuplicateBlockId { function: String },
    DuplicateParamId { function: String },
    DuplicateLocalId { function: String },
    DuplicateTempId { function: String },
    MissingTerminator { function: String, block: String },
    UnknownBlockTarget { function: String, block: String },
    UnknownTemp { function: String },
    UnknownLocal { function: String },
    UnknownGlobal,
    UnknownFunctionTarget { function: String },
    UnknownStruct { function: String },
    UnknownField { function: String, field: String },
    BadCallSignature { function: String },
    ReturnTypeMismatch { function: String },
    OperandTypeMismatch { function: String },
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
        let mut param_ids = std::collections::HashSet::new();
        for param in &func.params {
            if !param_ids.insert(param.id) {
                return Err(IrVerifyError::DuplicateParamId {
                    function: func.name.clone(),
                });
            }
        }
        let mut local_ids = std::collections::HashSet::new();
        for local in &func.locals {
            if !local_ids.insert(local.id) {
                return Err(IrVerifyError::DuplicateLocalId {
                    function: func.name.clone(),
                });
            }
        }
        let mut temp_ids = std::collections::HashSet::new();
        for temp in &func.temps {
            if !temp_ids.insert(temp.id) {
                return Err(IrVerifyError::DuplicateTempId {
                    function: func.name.clone(),
                });
            }
        }
        let mut block_ids = std::collections::HashSet::new();
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                return Err(IrVerifyError::DuplicateBlockId {
                    function: func.name.clone(),
                });
            }
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
                    crate::ir::Instr::StoreGlobal { global, value, .. } => {
                        if !program
                            .globals
                            .iter()
                            .any(|candidate| candidate.id == *global)
                        {
                            return Err(IrVerifyError::UnknownGlobal);
                        }
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::StoreLocal { local, value, .. } => {
                        if !func.locals.iter().any(|candidate| candidate.id == *local) {
                            return Err(IrVerifyError::UnknownLocal {
                                function: func.name.clone(),
                            });
                        }
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
                    crate::ir::Instr::VecLen { vec: array, .. } => {
                        Self::verify_operand(program, func, array)?;
                        Self::expect_operand_type(
                            program,
                            func,
                            array,
                            &IrType::Vec {
                                elem: Box::new(IrType::Unknown),
                            },
                        )?;
                    }
                    crate::ir::Instr::ArrayGet { array, index, .. }
                    | crate::ir::Instr::VecGet {
                        vec: array, index, ..
                    } => {
                        Self::verify_operand(program, func, array)?;
                        Self::verify_operand(program, func, index)?;
                        Self::expect_index_operand_type(program, func, index)?;
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
                        Self::expect_index_operand_type(program, func, index)?;
                    }
                    crate::ir::Instr::VecDelete {
                        vec: array, index, ..
                    } => {
                        Self::verify_operand(program, func, array)?;
                        Self::verify_operand(program, func, index)?;
                        Self::expect_index_operand_type(program, func, index)?;
                    }
                    crate::ir::Instr::MakeStruct {
                        struct_id, fields, ..
                    } => {
                        if !program
                            .structs
                            .iter()
                            .any(|candidate| candidate.id == *struct_id)
                        {
                            return Err(IrVerifyError::UnknownStruct {
                                function: func.name.clone(),
                            });
                        }
                        for field in fields {
                            Self::verify_operand(program, func, field)?;
                        }
                    }
                    crate::ir::Instr::StructGet { base, field, .. } => {
                        Self::verify_operand(program, func, base)?;
                        Self::verify_field_ref(program, func, base, field)?;
                    }
                    crate::ir::Instr::StructSet {
                        base, field, value, ..
                    } => {
                        Self::verify_operand(program, func, base)?;
                        Self::verify_field_ref(program, func, base, field)?;
                        Self::verify_operand(program, func, value)?;
                    }
                    crate::ir::Instr::CallDirect { function, args, .. } => {
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
                    }
                    crate::ir::Instr::CallBuiltin {
                        builtin,
                        args,
                        ret_ty,
                        ..
                    } => {
                        for arg in args {
                            Self::verify_operand(program, func, arg)?;
                        }
                        if let Some(sig) = find_builtin_sig(&builtin.package, &builtin.name) {
                            if !matches!(ret_ty, IrType::Unknown | IrType::Void) {
                                let expected_ret = IrType::from(&sig.ret);
                                if !Self::types_compatible(ret_ty, &expected_ret) {
                                    return Err(IrVerifyError::BadCallSignature {
                                        function: func.name.clone(),
                                    });
                                }
                            }
                            match sig.kind {
                                BuiltinKind::FixedArity => {
                                    if sig.params.len() != args.len() {
                                        return Err(IrVerifyError::BadCallSignature {
                                            function: func.name.clone(),
                                        });
                                    }
                                    for (arg, param_ty) in args.iter().zip(sig.params.iter()) {
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
                    crate::ir::Instr::CallIndirect {
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
                            continue;
                        };
                        let (params, ret) = match callee_ty {
                            IrType::Fn { params, ret } => (params, ret),
                            IrType::Unknown => continue,
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
                    crate::ir::Instr::Const { .. } => {}
                    crate::ir::Instr::LoadGlobal { global, .. } => {
                        if !program
                            .globals
                            .iter()
                            .any(|candidate| candidate.id == *global)
                        {
                            return Err(IrVerifyError::UnknownGlobal);
                        }
                    }
                    crate::ir::Instr::LoadLocal { local, .. } => {
                        if !func.locals.iter().any(|candidate| candidate.id == *local) {
                            return Err(IrVerifyError::UnknownLocal {
                                function: func.name.clone(),
                            });
                        }
                    }
                    crate::ir::Instr::VecNew { .. } => {}
                    crate::ir::Instr::MakeClosure { function, .. } => {
                        if !program
                            .functions
                            .iter()
                            .any(|candidate| candidate.id == *function)
                        {
                            return Err(IrVerifyError::UnknownFunctionTarget {
                                function: func.name.clone(),
                            });
                        }
                    }
                }
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

    fn verify_block_target(
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

    fn expect_index_operand_type(
        program: &IrProgram,
        func: &IrFunction,
        operand: &Operand,
    ) -> Result<(), IrVerifyError> {
        Self::expect_operand_type(program, func, operand, &IrType::Int)
    }

    fn expect_operand_type(
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

    fn types_compatible(actual: &IrType, expected: &IrType) -> bool {
        if actual == expected {
            return true;
        }
        matches!(
            (actual, expected),
            (_, IrType::Unknown) | (IrType::Unknown, _) | (IrType::Vec { .. }, IrType::Vec { .. })
        )
    }

    fn verify_field_ref(
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

    fn operand_type(
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
