use crate::ast::{AssignTarget, Expr, Stmt};
use crate::ir::{BlockId, BranchTerminator, Instr, IrType, Terminator};

use super::context::{FunctionLowering, IrLowerer};

impl IrLowerer {
    pub(super) fn compile_stmt(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        stmt: &Stmt,
    ) -> bool {
        match stmt {
            Stmt::Let { name, ty, value } => {
                if let Some(done) = self.try_compile_vec_new_let(func, lowering, name, ty, value) {
                    return done;
                }
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let ir_ty = ty
                    .as_ref()
                    .map(|ty| self.lower_type_name(ty))
                    .unwrap_or_else(|| self.infer_operand_type(func, &rhs));
                let local = self.builder.push_local(func, name.clone(), ir_ty.clone());
                lowering.locals.insert(name.clone(), local);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty: ir_ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Ident(name),
                value,
            } => {
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let Some(&local) = lowering.locals.get(name) else {
                    self.unsupported(format!("assignment to unknown local `{name}`"));
                    return false;
                };
                let ty = func
                    .locals
                    .iter()
                    .find(|entry| entry.id == local)
                    .map(|entry| entry.ty.clone())
                    .unwrap_or(IrType::Unknown);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Index { base, index },
                value,
            } => {
                let array = match self.compile_expr(func, lowering, base) {
                    Some(value) => value,
                    None => return false,
                };
                let index = match self.compile_expr(func, lowering, index) {
                    Some(value) => value,
                    None => return false,
                };
                let value = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let elem_ty = self.array_element_type(func, &array);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::ArraySet {
                        elem_ty,
                        array,
                        index,
                        value,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Field { base, field },
                value,
            } => {
                let base = match self.compile_expr(func, lowering, base) {
                    Some(value) => value,
                    None => return false,
                };
                let value = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let ty = self.field_type(func, &base, field);
                let field_ref = self.resolve_field_ref(func, &base, field);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StructSet {
                        base,
                        field: field_ref,
                        value,
                        ty,
                    },
                );
                true
            }
            Stmt::Expr(expr) => self.compile_expr(func, lowering, expr).is_some(),
            Stmt::Return(value) => {
                let ret = match value {
                    Some(expr) => match self.compile_expr(func, lowering, expr) {
                        Some(value) => Some(value),
                        None => return false,
                    },
                    None => None,
                };
                self.builder
                    .set_terminator(func, lowering.current_block, Terminator::Return(ret));
                true
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => self.compile_if(func, lowering, cond, then_body, else_body),
            Stmt::While { cond, body } => self.compile_while(func, lowering, cond, body),
            _ => {
                self.unsupported("statement form is not in the initial IR lowering subset");
                false
            }
        }
    }

    fn compile_if(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        then_body: &[Stmt],
        else_body: &[Stmt],
    ) -> bool {
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        let then_block = self.builder.push_block(func, "if_then");
        let else_block = self.builder.push_block(func, "if_else");
        let join_block = self.builder.push_block(func, "if_join");
        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block,
                else_block,
            }),
        );

        lowering.current_block = then_block;
        for stmt in then_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = else_block;
        for stmt in else_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = join_block;
        true
    }

    fn compile_while(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        body: &[Stmt],
    ) -> bool {
        let cond_block = self.builder.push_block(func, "while_cond");
        let body_block = self.builder.push_block(func, "while_body");
        let exit_block = self.builder.push_block(func, "while_exit");

        self.builder
            .set_terminator(func, lowering.current_block, Terminator::Jump(cond_block));

        lowering.current_block = cond_block;
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        self.builder.set_terminator(
            func,
            cond_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block: body_block,
                else_block: exit_block,
            }),
        );

        lowering.current_block = body_block;
        for stmt in body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, cond_block);

        lowering.current_block = exit_block;
        true
    }

    pub(super) fn ensure_fallthrough_jump(
        &self,
        func: &mut crate::ir::IrFunction,
        from: BlockId,
        to: BlockId,
    ) {
        if matches!(
            func.blocks
                .iter()
                .find(|block| block.id == from)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            self.builder
                .set_terminator(func, from, Terminator::Jump(to));
        }
    }
}
