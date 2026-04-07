use crate::ast::{AssignTarget, Expr, MatchLiteral, MatchPattern, Stmt};
use crate::ir::{BlockId, BranchTerminator, Instr, IrType, Operand, Terminator};

use super::context::{FunctionLowering, IrLowerer, LoopLowering};

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
                if let Some(done) = self.try_compile_map_new_let(func, lowering, name, ty, value) {
                    return done;
                }
                if let Some(done) =
                    self.try_compile_task_channel_let(func, lowering, name, ty, value)
                {
                    return done;
                }
                if let Some(done) =
                    self.try_compile_typed_empty_array_let(func, lowering, name, ty, value)
                {
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
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => self.compile_for(
                func,
                lowering,
                init.as_deref(),
                cond.as_ref(),
                step.as_deref(),
                body,
            ),
            Stmt::Break => self.compile_break(func, lowering),
            Stmt::Continue => self.compile_continue(func, lowering),
            Stmt::Match { expr, arms } => self.compile_match(func, lowering, expr, arms),
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

        lowering.loops.push(LoopLowering {
            continue_block: cond_block,
            break_block: exit_block,
        });
        lowering.current_block = body_block;
        for stmt in body {
            if !self.compile_stmt(func, lowering, stmt) {
                lowering.loops.pop();
                return false;
            }
        }
        lowering.loops.pop();
        self.ensure_fallthrough_jump(func, lowering.current_block, cond_block);

        lowering.current_block = exit_block;
        true
    }

    fn compile_for(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        init: Option<&Stmt>,
        cond: Option<&Expr>,
        step: Option<&Stmt>,
        body: &[Stmt],
    ) -> bool {
        if let Some(init) = init
            && !self.compile_stmt(func, lowering, init)
        {
            return false;
        }

        let cond_block = self.builder.push_block(func, "for_cond");
        let body_block = self.builder.push_block(func, "for_body");
        let step_block = self.builder.push_block(func, "for_step");
        let exit_block = self.builder.push_block(func, "for_exit");

        self.builder
            .set_terminator(func, lowering.current_block, Terminator::Jump(cond_block));

        lowering.current_block = cond_block;
        let cond_value = match cond {
            Some(cond) => match self.compile_expr(func, lowering, cond) {
                Some(value) => value,
                None => return false,
            },
            None => crate::ir::Operand::Const(crate::ir::ConstValue::Bool(true)),
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

        lowering.loops.push(LoopLowering {
            continue_block: step_block,
            break_block: exit_block,
        });
        lowering.current_block = body_block;
        for stmt in body {
            if !self.compile_stmt(func, lowering, stmt) {
                lowering.loops.pop();
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, step_block);

        lowering.current_block = step_block;
        if let Some(step) = step
            && !self.compile_stmt(func, lowering, step)
        {
            lowering.loops.pop();
            return false;
        }
        lowering.loops.pop();
        self.ensure_fallthrough_jump(func, lowering.current_block, cond_block);

        lowering.current_block = exit_block;
        true
    }

    fn compile_break(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
    ) -> bool {
        let Some(loop_ctx) = lowering.loops.last() else {
            self.unsupported("`break` is not valid outside a loop in IR lowering");
            return false;
        };
        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Jump(loop_ctx.break_block),
        );
        true
    }

    fn compile_continue(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
    ) -> bool {
        let Some(loop_ctx) = lowering.loops.last() else {
            self.unsupported("`continue` is not valid outside a loop in IR lowering");
            return false;
        };
        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Jump(loop_ctx.continue_block),
        );
        true
    }

    fn compile_match(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        expr: &Expr,
        arms: &[crate::ast::MatchArm],
    ) -> bool {
        let target = match self.compile_expr(func, lowering, expr) {
            Some(value) => value,
            None => return false,
        };
        let target_ty = self.infer_operand_type(func, &target);
        let match_local = self.builder.push_local(
            func,
            format!("__match{}", lowering.scratch_counter),
            target_ty.clone(),
        );
        lowering.scratch_counter += 1;
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local: match_local,
                ty: target_ty.clone(),
                value: target,
            },
        );

        let join_block = self.builder.push_block(func, "match_join");
        let mut dispatch_block = lowering.current_block;

        for (index, arm) in arms.iter().enumerate() {
            let body_block = self.builder.push_block(func, format!("match_arm_{index}"));
            let next_block = if index + 1 == arms.len() {
                join_block
            } else {
                self.builder.push_block(func, format!("match_next_{index}"))
            };

            if matches!(arm.pattern, MatchPattern::Wildcard) {
                self.builder
                    .set_terminator(func, dispatch_block, Terminator::Jump(body_block));
            } else {
                let cond = match self.compile_match_condition(
                    func,
                    dispatch_block,
                    match_local,
                    &target_ty,
                    &arm.pattern,
                ) {
                    Some(cond) => cond,
                    None => return false,
                };
                self.builder.set_terminator(
                    func,
                    dispatch_block,
                    Terminator::Branch(BranchTerminator {
                        cond,
                        then_block: body_block,
                        else_block: next_block,
                    }),
                );
            }

            lowering.current_block = body_block;
            let saved_locals = lowering.locals.clone();
            if !self.bind_match_pattern(func, lowering, match_local, &target_ty, &arm.pattern) {
                return false;
            }
            for stmt in &arm.body {
                if !self.compile_stmt(func, lowering, stmt) {
                    return false;
                }
            }
            self.ensure_fallthrough_jump(func, lowering.current_block, join_block);
            lowering.locals = saved_locals;
            dispatch_block = next_block;
        }

        if matches!(
            func.blocks
                .iter()
                .find(|block| block.id == dispatch_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            self.builder
                .set_terminator(func, dispatch_block, Terminator::Jump(join_block));
        }

        lowering.current_block = join_block;
        true
    }

    pub(super) fn compile_match_condition(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        match_local: crate::ir::LocalId,
        _target_ty: &IrType,
        pattern: &MatchPattern,
    ) -> Option<Operand> {
        match pattern {
            MatchPattern::Wildcard => Some(Operand::Const(crate::ir::ConstValue::Bool(true))),
            MatchPattern::Literal(lit) => {
                let rhs = match lit {
                    MatchLiteral::Int(v) => Operand::Const(crate::ir::ConstValue::Int(*v)),
                    MatchLiteral::Bool(v) => Operand::Const(crate::ir::ConstValue::Bool(*v)),
                    MatchLiteral::String(v) => {
                        Operand::Const(crate::ir::ConstValue::String(v.clone()))
                    }
                    MatchLiteral::Float(v) => {
                        let parsed = v.parse::<f64>().ok()?;
                        Operand::Const(crate::ir::ConstValue::Float(parsed))
                    }
                };
                let dst = self.builder.push_temp(func, IrType::Bool);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::Compare {
                        dst,
                        op: crate::ir::CmpOp::Eq,
                        left: Operand::Local(match_local),
                        right: rhs,
                    },
                );
                Some(Operand::Temp(dst))
            }
            MatchPattern::Variant { name, .. } => {
                let builtin = match name.as_str() {
                    "Some" => ("option", "isSome"),
                    "None" => ("option", "isNone"),
                    "Ok" => ("result", "isOk"),
                    "Err" => ("result", "isErr"),
                    other => {
                        self.unsupported(format!(
                            "unsupported match variant `{other}` in lowering"
                        ));
                        return None;
                    }
                };
                let dst = self.builder.push_temp(func, IrType::Bool);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::CallBuiltin {
                        dst: Some(dst),
                        ret_ty: IrType::Bool,
                        builtin: crate::ir::BuiltinCall {
                            package: builtin.0.to_string(),
                            name: builtin.1.to_string(),
                        },
                        args: vec![Operand::Local(match_local)],
                    },
                );
                Some(Operand::Temp(dst))
            }
            MatchPattern::Or(parts) => {
                let mut parts = parts.iter();
                let first = self.compile_match_condition(
                    func,
                    block,
                    match_local,
                    _target_ty,
                    parts.next()?,
                )?;
                let mut acc = first;
                for part in parts {
                    let rhs =
                        self.compile_match_condition(func, block, match_local, _target_ty, part)?;
                    let dst = self.builder.push_temp(func, IrType::Bool);
                    self.builder.push_instr(
                        func,
                        block,
                        Instr::Logic {
                            dst,
                            op: crate::ir::LogicOp::Or,
                            left: acc,
                            right: rhs,
                        },
                    );
                    acc = Operand::Temp(dst);
                }
                Some(acc)
            }
        }
    }

    pub(super) fn bind_match_pattern(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        match_local: crate::ir::LocalId,
        target_ty: &IrType,
        pattern: &MatchPattern,
    ) -> bool {
        let MatchPattern::Variant {
            name,
            binding: Some(binding),
        } = pattern
        else {
            return true;
        };

        let (builtin_pkg, builtin_name, value_ty) = match (name.as_str(), target_ty) {
            ("Some", IrType::Option { value }) => ("option", "unwrapSome", (**value).clone()),
            ("Ok", IrType::Result { ok, .. }) => ("result", "unwrapOk", (**ok).clone()),
            ("Err", IrType::Result { err, .. }) => ("result", "unwrapErr", (**err).clone()),
            _ => return true,
        };

        let dst = self.builder.push_temp(func, value_ty.clone());
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallBuiltin {
                dst: Some(dst),
                ret_ty: value_ty.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: builtin_pkg.to_string(),
                    name: builtin_name.to_string(),
                },
                args: vec![Operand::Local(match_local)],
            },
        );
        let local = self
            .builder
            .push_local(func, binding.clone(), value_ty.clone());
        lowering.locals.insert(binding.clone(), local);
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: value_ty,
                value: Operand::Temp(dst),
            },
        );
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

    fn try_compile_typed_empty_array_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Array { elem, size }) = ty else {
            return None;
        };
        let Expr::ArrayLit(items) = value else {
            return None;
        };
        if !items.is_empty() {
            return None;
        }

        let elem_ty = self.lower_type_name(elem);
        let array_ty = IrType::Array {
            elem: Box::new(elem_ty.clone()),
            size: *size,
        };
        let local = self
            .builder
            .push_local(func, name.to_string(), array_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, array_ty.clone());
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::MakeArray {
                dst,
                elem_ty,
                items: Vec::new(),
            },
        );
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: array_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }
}
