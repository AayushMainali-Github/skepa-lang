use crate::ast::{BinaryOp as AstBinaryOp, Expr, UnaryOp as AstUnaryOp};
use crate::ir::{BranchTerminator, ConstValue, Instr, IrType, Operand, Terminator, UnaryOp};

use super::context::{FunctionLowering, IrLowerer};

impl IrLowerer {
    fn expr_to_path_parts(expr: &Expr) -> Option<Vec<String>> {
        match expr {
            Expr::Ident(name) => Some(vec![name.clone()]),
            Expr::Path(parts) => Some(parts.clone()),
            Expr::Field { base, field } => {
                let mut parts = Self::expr_to_path_parts(base)?;
                parts.push(field.clone());
                Some(parts)
            }
            _ => None,
        }
    }

    pub(super) fn compile_expr(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        expr: &Expr,
    ) -> Option<Operand> {
        match expr {
            Expr::IntLit(value) => Some(Operand::Const(ConstValue::Int(*value))),
            Expr::FloatLit(value) => match value.parse::<f64>() {
                Ok(value) => Some(Operand::Const(ConstValue::Float(value))),
                Err(_) => {
                    self.unsupported(format!("invalid float literal `{value}` in IR lowering"));
                    None
                }
            },
            Expr::BoolLit(value) => Some(Operand::Const(ConstValue::Bool(*value))),
            Expr::StringLit(value) => Some(Operand::Const(ConstValue::String(value.clone()))),
            Expr::Ident(name) => lowering
                .locals
                .get(name)
                .copied()
                .map(Operand::Local)
                .or_else(|| {
                    self.imported_global_names
                        .get(name)
                        .and_then(|qualified| self.globals.get(qualified))
                        .map(|(id, _)| Operand::Global(*id))
                })
                .or_else(|| {
                    self.globals
                        .get(name)
                        .or_else(|| self.globals.get(&self.qualify_name(name)))
                        .map(|(id, _)| Operand::Global(*id))
                })
                .or_else(|| self.function_value(func, lowering.current_block, name))
                .or_else(|| {
                    self.unsupported(format!("reference to unresolved identifier `{name}`"));
                    None
                }),
            Expr::Path(parts) => {
                let name = parts.join(".");
                if let Some(qualified) = self.imported_global_names.get(&name)
                    && let Some((id, _)) = self.globals.get(qualified)
                {
                    return Some(Operand::Global(*id));
                }
                if let Some(target_name) = self.namespace_call_targets.get(&name).cloned() {
                    return self.function_value(func, lowering.current_block, &target_name);
                }
                self.globals
                    .get(&name)
                    .or_else(|| self.globals.get(&self.qualify_name(&name)))
                    .map(|(id, _)| Operand::Global(*id))
                    .or_else(|| {
                        self.unsupported(format!(
                            "path `{name}` is not in the initial IR lowering subset"
                        ));
                        None
                    })
            }
            Expr::Field { base, field } => {
                if let Some(parts) = Self::expr_to_path_parts(expr)
                    && parts.len() >= 2
                {
                    let name = parts.join(".");
                    if let Some(qualified) = self.imported_global_names.get(&name)
                        && let Some((id, _)) = self.globals.get(qualified)
                    {
                        return Some(Operand::Global(*id));
                    }
                    if let Some(target_name) = self.namespace_call_targets.get(&name).cloned() {
                        return self.function_value(func, lowering.current_block, &target_name);
                    }
                }
                let base = self.compile_expr(func, lowering, base)?;
                let ty = self.field_type(func, &base, field);
                let field_ref = self.resolve_field_ref(func, &base, field);
                let dst = self.builder.push_temp(func, ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StructGet {
                        dst,
                        ty,
                        base,
                        field: field_ref,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayLit(items) => {
                let mut lowered_items = Vec::with_capacity(items.len());
                for item in items {
                    lowered_items.push(self.compile_expr(func, lowering, item)?);
                }
                let elem_ty = lowered_items
                    .first()
                    .map(|item| self.infer_operand_type(func, item))
                    .unwrap_or(IrType::Unknown);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: lowered_items.len(),
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArray {
                        dst,
                        elem_ty,
                        items: lowered_items,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayRepeat { value, size } => {
                let value = self.compile_expr(func, lowering, value)?;
                let elem_ty = self.infer_operand_type(func, &value);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: *size,
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArrayRepeat {
                        dst,
                        elem_ty,
                        value,
                        size: *size,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::StructLit { name, fields } => {
                let runtime_name = self.resolve_struct_runtime_name(name);
                let Some((struct_id, struct_fields)) = self.structs.get(&runtime_name).cloned()
                else {
                    self.unsupported(format!("unknown struct `{name}` in IR lowering"));
                    return None;
                };
                let mut ordered = Vec::with_capacity(struct_fields.len());
                for declared in &struct_fields {
                    let Some((_, expr)) = fields
                        .iter()
                        .find(|(field_name, _)| field_name == &declared.name)
                    else {
                        self.unsupported(format!(
                            "missing field `{}` in struct literal `{name}`",
                            declared.name
                        ));
                        return None;
                    };
                    ordered.push(self.compile_expr(func, lowering, expr)?);
                }
                let dst = self
                    .builder
                    .push_temp(func, IrType::Named(runtime_name.clone()));
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeStruct {
                        dst,
                        struct_id,
                        fields: ordered,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::FnLit {
                params,
                return_type,
                body,
            } => self.compile_fn_lit(func, lowering.current_block, params, return_type, body),
            Expr::Match { expr, arms } => self.compile_match_expr(func, lowering, expr, arms),
            Expr::Index { base, index } => {
                let array = self.compile_expr(func, lowering, base)?;
                let index = self.compile_expr(func, lowering, index)?;
                let elem_ty = self.array_element_type(func, &array);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::ArrayGet {
                        dst,
                        elem_ty,
                        array,
                        index,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::Group(inner) => self.compile_expr(func, lowering, inner),
            Expr::Unary { op, expr } => {
                let operand = self.compile_expr(func, lowering, expr)?;
                let ty = self.infer_operand_type(func, &operand);
                let dst = self.builder.push_temp(func, ty.clone());
                let op = match op {
                    AstUnaryOp::Neg => UnaryOp::Neg,
                    AstUnaryOp::Not => UnaryOp::Not,
                    AstUnaryOp::BitNot => UnaryOp::BitNot,
                    AstUnaryOp::Pos => {
                        self.unsupported("unary operator is not in the initial IR lowering subset");
                        return None;
                    }
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::Unary {
                        dst,
                        ty,
                        op,
                        operand,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::Binary { left, op, right } => {
                if matches!(op, AstBinaryOp::AndAnd | AstBinaryOp::OrOr) {
                    return self.compile_short_circuit(func, lowering, left, op, right);
                }
                let left = self.compile_expr(func, lowering, left)?;
                let right = self.compile_expr(func, lowering, right)?;
                let ty = self.infer_binary_type(func, &left, op, &right);
                let dst = self.builder.push_temp(func, ty.clone());
                if let Some(op) = self.lower_binary_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Binary {
                            dst,
                            ty,
                            op,
                            left,
                            right,
                        },
                    );
                } else if let Some(op) = self.lower_cmp_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Compare {
                            dst,
                            op,
                            left,
                            right,
                        },
                    );
                } else {
                    self.unsupported("binary operator is not in the initial IR lowering subset");
                    return None;
                }
                Some(Operand::Temp(dst))
            }
            Expr::CustomInfix { .. } => {
                let Expr::CustomInfix {
                    left,
                    operator,
                    right,
                } = expr
                else {
                    unreachable!();
                };
                let left = self.compile_expr(func, lowering, left)?;
                let right = self.compile_expr(func, lowering, right)?;
                let qualified = self.qualify_name(operator);
                let Some(sig) = self.functions.get(&qualified).cloned() else {
                    self.unsupported(format!(
                        "unknown user-defined operator `{operator}` in IR lowering"
                    ));
                    return None;
                };
                let dst = if sig.ret.is_void() {
                    None
                } else {
                    Some(self.builder.push_temp(func, sig.ret.clone()))
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallDirect {
                        dst,
                        ret_ty: sig.ret.clone(),
                        function: sig.id,
                        args: vec![left, right],
                    },
                );
                match dst {
                    Some(dst) => Some(Operand::Temp(dst)),
                    None => Some(Operand::Const(ConstValue::Unit)),
                }
            }
            Expr::Call { callee, args } => self.compile_call(func, lowering, callee, args),
            Expr::Try(inner) => self.compile_try_expr(func, lowering, inner),
        }
    }

    fn compile_try_expr(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        inner: &Expr,
    ) -> Option<Operand> {
        let value = self.compile_expr(func, lowering, inner)?;
        let value_ty = self.infer_operand_type(func, &value);
        let func_ret_ty = func.ret_ty.clone();
        let saved_block = lowering.current_block;
        let value_local = self.builder.push_local(
            func,
            format!("__try{}", lowering.scratch_counter),
            value_ty.clone(),
        );
        lowering.scratch_counter += 1;
        self.builder.push_instr(
            func,
            saved_block,
            Instr::StoreLocal {
                local: value_local,
                ty: value_ty.clone(),
                value,
            },
        );

        match (&value_ty, &func_ret_ty) {
            (IrType::Option { value: inner_ty }, IrType::Option { value: ret_inner }) => {
                let cond_temp = self.builder.push_temp(func, IrType::Bool);
                self.builder.push_instr(
                    func,
                    saved_block,
                    Instr::CallBuiltin {
                        dst: Some(cond_temp),
                        ret_ty: IrType::Bool,
                        builtin: crate::ir::BuiltinCall {
                            package: "option".to_string(),
                            name: "isSome".to_string(),
                        },
                        args: vec![Operand::Local(value_local)],
                    },
                );

                let some_block = self.builder.push_block(func, "try_some");
                let none_block = self.builder.push_block(func, "try_none");
                let join_block = self.builder.push_block(func, "try_join");
                self.builder.set_terminator(
                    func,
                    saved_block,
                    Terminator::Branch(BranchTerminator {
                        cond: Operand::Temp(cond_temp),
                        then_block: some_block,
                        else_block: none_block,
                    }),
                );

                let unwrapped_ty = (**inner_ty).clone();
                let result_local = self.builder.push_local(
                    func,
                    format!("__try_unwrap{}", lowering.scratch_counter),
                    unwrapped_ty.clone(),
                );
                lowering.scratch_counter += 1;
                let unwrap_temp = self.builder.push_temp(func, unwrapped_ty.clone());
                self.builder.push_instr(
                    func,
                    some_block,
                    Instr::CallBuiltin {
                        dst: Some(unwrap_temp),
                        ret_ty: unwrapped_ty.clone(),
                        builtin: crate::ir::BuiltinCall {
                            package: "option".to_string(),
                            name: "unwrapSome".to_string(),
                        },
                        args: vec![Operand::Local(value_local)],
                    },
                );
                self.builder.push_instr(
                    func,
                    some_block,
                    Instr::StoreLocal {
                        local: result_local,
                        ty: unwrapped_ty.clone(),
                        value: Operand::Temp(unwrap_temp),
                    },
                );
                self.builder
                    .set_terminator(func, some_block, Terminator::Jump(join_block));

                let none_temp = self.builder.push_temp(
                    func,
                    IrType::Option {
                        value: Box::new((**ret_inner).clone()),
                    },
                );
                self.builder.push_instr(
                    func,
                    none_block,
                    Instr::CallBuiltin {
                        dst: Some(none_temp),
                        ret_ty: IrType::Option {
                            value: Box::new((**ret_inner).clone()),
                        },
                        builtin: crate::ir::BuiltinCall {
                            package: "option".to_string(),
                            name: "none".to_string(),
                        },
                        args: Vec::new(),
                    },
                );
                self.builder.set_terminator(
                    func,
                    none_block,
                    Terminator::Return(Some(Operand::Temp(none_temp))),
                );

                lowering.current_block = join_block;
                Some(Operand::Local(result_local))
            }
            (
                IrType::Result {
                    ok: ok_ty,
                    err: err_ty,
                },
                IrType::Result {
                    ok: ret_ok,
                    err: ret_err,
                },
            ) => {
                let cond_temp = self.builder.push_temp(func, IrType::Bool);
                self.builder.push_instr(
                    func,
                    saved_block,
                    Instr::CallBuiltin {
                        dst: Some(cond_temp),
                        ret_ty: IrType::Bool,
                        builtin: crate::ir::BuiltinCall {
                            package: "result".to_string(),
                            name: "isOk".to_string(),
                        },
                        args: vec![Operand::Local(value_local)],
                    },
                );

                let ok_block = self.builder.push_block(func, "try_ok");
                let err_block = self.builder.push_block(func, "try_err");
                let join_block = self.builder.push_block(func, "try_join");
                self.builder.set_terminator(
                    func,
                    saved_block,
                    Terminator::Branch(BranchTerminator {
                        cond: Operand::Temp(cond_temp),
                        then_block: ok_block,
                        else_block: err_block,
                    }),
                );

                let unwrapped_ok_ty = (**ok_ty).clone();
                let result_local = self.builder.push_local(
                    func,
                    format!("__try_unwrap{}", lowering.scratch_counter),
                    unwrapped_ok_ty.clone(),
                );
                lowering.scratch_counter += 1;
                let unwrap_ok_temp = self.builder.push_temp(func, unwrapped_ok_ty.clone());
                self.builder.push_instr(
                    func,
                    ok_block,
                    Instr::CallBuiltin {
                        dst: Some(unwrap_ok_temp),
                        ret_ty: unwrapped_ok_ty.clone(),
                        builtin: crate::ir::BuiltinCall {
                            package: "result".to_string(),
                            name: "unwrapOk".to_string(),
                        },
                        args: vec![Operand::Local(value_local)],
                    },
                );
                self.builder.push_instr(
                    func,
                    ok_block,
                    Instr::StoreLocal {
                        local: result_local,
                        ty: unwrapped_ok_ty.clone(),
                        value: Operand::Temp(unwrap_ok_temp),
                    },
                );
                self.builder
                    .set_terminator(func, ok_block, Terminator::Jump(join_block));

                let propagated_err_ty = (**err_ty).clone();
                let unwrap_err_temp = self.builder.push_temp(func, propagated_err_ty.clone());
                self.builder.push_instr(
                    func,
                    err_block,
                    Instr::CallBuiltin {
                        dst: Some(unwrap_err_temp),
                        ret_ty: propagated_err_ty.clone(),
                        builtin: crate::ir::BuiltinCall {
                            package: "result".to_string(),
                            name: "unwrapErr".to_string(),
                        },
                        args: vec![Operand::Local(value_local)],
                    },
                );
                let err_result_ty = IrType::Result {
                    ok: Box::new((**ret_ok).clone()),
                    err: Box::new((**ret_err).clone()),
                };
                let err_result_temp = self.builder.push_temp(func, err_result_ty.clone());
                self.builder.push_instr(
                    func,
                    err_block,
                    Instr::CallBuiltin {
                        dst: Some(err_result_temp),
                        ret_ty: err_result_ty,
                        builtin: crate::ir::BuiltinCall {
                            package: "result".to_string(),
                            name: "err".to_string(),
                        },
                        args: vec![Operand::Temp(unwrap_err_temp)],
                    },
                );
                self.builder.set_terminator(
                    func,
                    err_block,
                    Terminator::Return(Some(Operand::Temp(err_result_temp))),
                );

                lowering.current_block = join_block;
                Some(Operand::Local(result_local))
            }
            _ => {
                self.unsupported("`?` lowering requires Option/Result-compatible function return");
                None
            }
        }
    }

    fn compile_match_expr(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        expr: &Expr,
        arms: &[crate::ast::MatchExprArm],
    ) -> Option<Operand> {
        let target = self.compile_expr(func, lowering, expr)?;
        let target_ty = self.infer_operand_type(func, &target);
        let saved_block = lowering.current_block;
        let match_local = self.builder.push_local(
            func,
            format!("__match_expr{}", lowering.scratch_counter),
            target_ty.clone(),
        );
        lowering.scratch_counter += 1;
        self.builder.push_instr(
            func,
            saved_block,
            Instr::StoreLocal {
                local: match_local,
                ty: target_ty.clone(),
                value: target,
            },
        );

        let result_ty = arms
            .first()
            .map(|arm| self.expr_type(func, lowering, &arm.expr))
            .unwrap_or(IrType::Unknown);
        let result_local = self.builder.push_local(
            func,
            format!("__match_expr_result{}", lowering.scratch_counter),
            result_ty.clone(),
        );
        lowering.scratch_counter += 1;

        let join_block = self.builder.push_block(func, "match_expr_join");
        let fail_block = self.builder.push_block(func, "match_expr_unreachable");
        let mut dispatch_block = saved_block;

        for (index, arm) in arms.iter().enumerate() {
            let body_block = self
                .builder
                .push_block(func, format!("match_expr_arm_{index}"));
            let next_block = if index + 1 == arms.len() {
                fail_block
            } else {
                self.builder
                    .push_block(func, format!("match_expr_next_{index}"))
            };

            if matches!(arm.pattern, crate::ast::MatchPattern::Wildcard) {
                self.builder
                    .set_terminator(func, dispatch_block, Terminator::Jump(body_block));
            } else {
                let cond = self.compile_match_condition(
                    func,
                    dispatch_block,
                    match_local,
                    &target_ty,
                    &arm.pattern,
                )?;
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
                return None;
            }
            let arm_value = self.compile_expr(func, lowering, &arm.expr)?;
            self.builder.push_instr(
                func,
                lowering.current_block,
                Instr::StoreLocal {
                    local: result_local,
                    ty: result_ty.clone(),
                    value: arm_value,
                },
            );
            self.ensure_fallthrough_jump(func, lowering.current_block, join_block);
            lowering.locals = saved_locals;
            dispatch_block = next_block;
        }

        self.builder
            .set_terminator(func, fail_block, Terminator::Unreachable);
        lowering.current_block = join_block;
        Some(Operand::Local(result_local))
    }

    fn expr_type(
        &self,
        func: &crate::ir::IrFunction,
        lowering: &FunctionLowering,
        expr: &Expr,
    ) -> IrType {
        match expr {
            Expr::IntLit(_) => IrType::Int,
            Expr::FloatLit(_) => IrType::Float,
            Expr::BoolLit(_) => IrType::Bool,
            Expr::StringLit(_) => IrType::String,
            Expr::Ident(name) => lowering
                .locals
                .get(name)
                .and_then(|local| func.locals.iter().find(|entry| entry.id == *local))
                .map(|entry| entry.ty.clone())
                .unwrap_or(IrType::Unknown),
            Expr::Group(inner) => self.expr_type(func, lowering, inner),
            _ => IrType::Unknown,
        }
    }

    fn compile_short_circuit(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        left: &Expr,
        op: &AstBinaryOp,
        right: &Expr,
    ) -> Option<Operand> {
        let left_value = self.compile_expr(func, lowering, left)?;
        let result_local = self.builder.push_local(
            func,
            format!("__sc{}", lowering.scratch_counter),
            IrType::Bool,
        );
        lowering.scratch_counter += 1;

        let rhs_block = self.builder.push_block(func, "sc_rhs");
        let short_block = self.builder.push_block(func, "sc_short");
        let join_block = self.builder.push_block(func, "sc_join");

        let (then_block, else_block, short_value) = match op {
            AstBinaryOp::AndAnd => (rhs_block, short_block, false),
            AstBinaryOp::OrOr => (short_block, rhs_block, true),
            _ => return None,
        };

        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: left_value,
                then_block,
                else_block,
            }),
        );

        self.builder.push_instr(
            func,
            short_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: Operand::Const(ConstValue::Bool(short_value)),
            },
        );
        self.builder
            .set_terminator(func, short_block, Terminator::Jump(join_block));

        lowering.current_block = rhs_block;
        let right_value = self.compile_expr(func, lowering, right)?;
        self.builder.push_instr(
            func,
            rhs_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: right_value,
            },
        );
        self.builder
            .set_terminator(func, rhs_block, Terminator::Jump(join_block));

        lowering.current_block = join_block;
        Some(Operand::Local(result_local))
    }
}
