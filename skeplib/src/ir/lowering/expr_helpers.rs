use std::collections::HashMap;

use crate::ast::{BinaryOp as AstBinaryOp, Stmt};
use crate::diagnostic::Span;
use crate::ir::{BinaryOp, BlockId, ConstValue, Instr, IrType, Operand, Terminator};
use crate::types::TypeInfo;

use super::context::{FunctionLowering, IrLowerer};

impl IrLowerer {
    pub(super) fn infer_operand_type(
        &self,
        func: &crate::ir::IrFunction,
        operand: &Operand,
    ) -> IrType {
        match operand {
            Operand::Const(ConstValue::Int(_)) => IrType::Int,
            Operand::Const(ConstValue::Float(_)) => IrType::Float,
            Operand::Const(ConstValue::Bool(_)) => IrType::Bool,
            Operand::Const(ConstValue::String(_)) => IrType::String,
            Operand::Const(ConstValue::Unit) => IrType::Void,
            Operand::Temp(id) => func
                .temps
                .iter()
                .find(|temp| temp.id == *id)
                .map(|temp| temp.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Local(id) => func
                .locals
                .iter()
                .find(|local| local.id == *id)
                .map(|local| local.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Global(id) => self
                .globals
                .values()
                .find(|(global_id, _)| global_id == id)
                .map(|(_, ty)| ty.clone())
                .unwrap_or(IrType::Unknown),
        }
    }

    pub(super) fn array_element_type(
        &self,
        func: &crate::ir::IrFunction,
        operand: &Operand,
    ) -> IrType {
        match self.infer_operand_type(func, operand) {
            IrType::Array { elem, .. } => *elem,
            IrType::Vec { elem } => *elem,
            _ => IrType::Unknown,
        }
    }

    pub(super) fn field_type(
        &self,
        func: &crate::ir::IrFunction,
        base: &Operand,
        field: &str,
    ) -> IrType {
        let IrType::Named(struct_name) = self.infer_operand_type(func, base) else {
            return IrType::Unknown;
        };
        self.structs
            .get(&struct_name)
            .and_then(|(_, fields)| fields.iter().find(|entry| entry.name == field))
            .map(|entry| entry.ty.clone())
            .unwrap_or(IrType::Unknown)
    }

    pub(super) fn resolve_field_ref(
        &self,
        func: &crate::ir::IrFunction,
        base: &Operand,
        field: &str,
    ) -> crate::ir::FieldRef {
        let index = match self.infer_operand_type(func, base) {
            IrType::Named(struct_name) => self
                .structs
                .get(&struct_name)
                .and_then(|(_, fields)| fields.iter().position(|entry| entry.name == field))
                .unwrap_or(usize::MAX),
            _ => usize::MAX,
        };
        crate::ir::FieldRef {
            index,
            name: field.to_string(),
        }
    }

    pub(super) fn infer_binary_type(
        &self,
        func: &crate::ir::IrFunction,
        left: &Operand,
        op: &AstBinaryOp,
        right: &Operand,
    ) -> IrType {
        if self.lower_cmp_op(op).is_some() {
            IrType::Bool
        } else {
            let left_ty = self.infer_operand_type(func, left);
            let right_ty = self.infer_operand_type(func, right);
            if left_ty == right_ty {
                left_ty
            } else {
                IrType::Unknown
            }
        }
    }

    pub(super) fn lower_binary_op(&self, op: &AstBinaryOp) -> Option<BinaryOp> {
        match op {
            AstBinaryOp::Add => Some(BinaryOp::Add),
            AstBinaryOp::Sub => Some(BinaryOp::Sub),
            AstBinaryOp::Mul => Some(BinaryOp::Mul),
            AstBinaryOp::Div => Some(BinaryOp::Div),
            AstBinaryOp::Mod => Some(BinaryOp::Mod),
            AstBinaryOp::BitAnd => Some(BinaryOp::BitAnd),
            AstBinaryOp::BitOr => Some(BinaryOp::BitOr),
            AstBinaryOp::BitXor => Some(BinaryOp::BitXor),
            AstBinaryOp::Shl => Some(BinaryOp::Shl),
            AstBinaryOp::Shr => Some(BinaryOp::Shr),
            AstBinaryOp::AndAnd | AstBinaryOp::OrOr => None,
            _ => None,
        }
    }

    pub(super) fn lower_cmp_op(&self, op: &AstBinaryOp) -> Option<crate::ir::CmpOp> {
        match op {
            AstBinaryOp::EqEq => Some(crate::ir::CmpOp::Eq),
            AstBinaryOp::Neq => Some(crate::ir::CmpOp::Ne),
            AstBinaryOp::Lt => Some(crate::ir::CmpOp::Lt),
            AstBinaryOp::Lte => Some(crate::ir::CmpOp::Le),
            AstBinaryOp::Gt => Some(crate::ir::CmpOp::Gt),
            AstBinaryOp::Gte => Some(crate::ir::CmpOp::Ge),
            _ => None,
        }
    }

    pub(super) fn function_value(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        name: &str,
    ) -> Option<Operand> {
        let target_name = self
            .direct_import_calls
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.qualify_name(name));
        let sig = self
            .functions
            .get(name)
            .or_else(|| self.functions.get(&target_name))
            .cloned()?;
        let dst = self.builder.push_temp(
            func,
            IrType::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
            },
        );
        self.builder.push_instr(
            func,
            block,
            Instr::MakeClosure {
                dst,
                function: sig.id,
            },
        );
        Some(Operand::Temp(dst))
    }

    pub(super) fn compile_fn_lit(
        &mut self,
        outer_func: &mut crate::ir::IrFunction,
        block: BlockId,
        params: &[crate::ast::Param],
        return_type: &crate::ast::TypeName,
        body: &[Stmt],
    ) -> Option<Operand> {
        self.fn_lit_counter += 1;
        let name = format!("__fn_lit_{}", self.fn_lit_counter);
        let ret_ty = IrType::from(&TypeInfo::from_ast(return_type));
        let function_id = crate::ir::FunctionId(self.functions.len() + self.lifted_functions.len());
        self.functions.insert(
            name.clone(),
            super::context::FunctionSig {
                id: function_id,
                params: params
                    .iter()
                    .map(|param| IrType::from(&TypeInfo::from_ast(&param.ty)))
                    .collect(),
                ret: ret_ty.clone(),
            },
        );

        let mut lifted = self.builder.begin_function(name, ret_ty.clone());
        lifted.id = function_id;
        let mut lowering = FunctionLowering {
            current_block: lifted.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
            loops: Vec::new(),
        };
        let mut param_types = Vec::with_capacity(params.len());
        for param in params {
            let ty_info = TypeInfo::from_ast(&param.ty);
            let ir_ty = IrType::from(&ty_info);
            param_types.push(ir_ty.clone());
            self.builder
                .push_param(&mut lifted, param.name.clone(), ir_ty.clone());
            let local = self
                .builder
                .push_local(&mut lifted, param.name.clone(), ir_ty);
            lowering.locals.insert(param.name.clone(), local);
        }
        for stmt in body {
            if !self.compile_stmt(&mut lifted, &mut lowering, stmt) {
                return None;
            }
        }
        if matches!(
            lifted
                .blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if ret_ty.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    "IR lowering currently requires explicit return in non-void function literal",
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut lifted, lowering.current_block, terminator);
        }
        self.lifted_functions.push(lifted);

        let dst = self.builder.push_temp(
            outer_func,
            IrType::Fn {
                params: param_types,
                ret: Box::new(ret_ty),
            },
        );
        self.builder.push_instr(
            outer_func,
            block,
            Instr::MakeClosure {
                dst,
                function: function_id,
            },
        );
        Some(Operand::Temp(dst))
    }
}
