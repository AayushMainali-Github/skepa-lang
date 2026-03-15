use std::rc::Rc;

use crate::ast::{AssignTarget, MatchLiteral, MatchPattern, Stmt, TypeName};

use super::context::{Compiler, FnCtx, LoopCtx, PrimitiveType};
use super::{Instr, Value};

impl Compiler {
    pub(super) fn compile_stmt(
        &mut self,
        stmt: &Stmt,
        ctx: &mut FnCtx,
        loops: &mut Vec<LoopCtx>,
        code: &mut Vec<Instr>,
    ) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let explicit_named = match ty {
                    Some(TypeName::Named(type_name)) => {
                        Some(self.resolve_struct_runtime_name(type_name))
                    }
                    _ => None,
                };
                let explicit_primitive = match ty {
                    Some(TypeName::Int) => Some(PrimitiveType::Int),
                    Some(TypeName::Float) => Some(PrimitiveType::Float),
                    Some(TypeName::Bool) => Some(PrimitiveType::Bool),
                    Some(TypeName::String) => Some(PrimitiveType::String),
                    Some(TypeName::Void) => Some(PrimitiveType::Void),
                    _ => None,
                };
                let inferred_named = Self::infer_expr_named_type(value, ctx);
                let inferred_primitive = Self::infer_expr_primitive_type(value, ctx);
                let slot = if let Some(type_name) = explicit_named.or(inferred_named) {
                    ctx.alloc_local_with_named_type(name.clone(), type_name)
                } else if let Some(ty) = explicit_primitive.or(inferred_primitive) {
                    ctx.alloc_local_with_primitive_type(name.clone(), ty)
                } else {
                    ctx.alloc_local(name.clone())
                };
                if let Some(instr) = self.specialized_local_value_to_local(value, ctx, slot) {
                    code.push(instr);
                } else {
                    self.compile_expr(value, ctx, code);
                    code.push(Instr::StoreLocal(slot));
                }
            }
            Stmt::Assign { target, value } => match target {
                AssignTarget::Ident(name) => {
                    if let Some(slot) = ctx.lookup(name) {
                        if let Some(instr) = Self::specialized_local_assign(value, ctx, slot) {
                            code.push(instr);
                        } else if let Some((rhs, instr)) =
                            Self::specialized_local_stack_assign(value, ctx, name, slot)
                        {
                            self.compile_expr(rhs, ctx, code);
                            code.push(instr);
                        } else if let Some(instr) =
                            self.specialized_local_value_to_local(value, ctx, slot)
                        {
                            code.push(instr);
                        } else {
                            self.compile_expr(value, ctx, code);
                            code.push(Instr::StoreLocal(slot));
                        }
                    } else if let Some(slot) = self.global_slots.get(name).copied() {
                        self.compile_expr(value, ctx, code);
                        code.push(Instr::StoreGlobal(slot));
                    } else {
                        self.error(format!("Unknown local `{name}` in assignment"));
                    }
                }
                AssignTarget::Path(parts) => {
                    self.compile_expr(value, ctx, code);
                    self.error(format!(
                        "Path assignment not supported in bytecode v0: {}",
                        parts.join(".")
                    ));
                }
                AssignTarget::Index { base, index } => {
                    if let Some((root, indices)) = Self::flatten_index_target(base, index) {
                        if let Some(slot) = ctx.lookup(&root) {
                            if indices.len() == 1 {
                                if let Some(instr) =
                                    Self::specialized_local_array_assign(&root, indices[0], value)
                                {
                                    self.compile_expr(indices[0], ctx, code);
                                    code.push(instr.with_slot(slot));
                                } else {
                                    self.compile_expr(indices[0], ctx, code);
                                    self.compile_expr(value, ctx, code);
                                    code.push(Instr::ArraySetLocal(slot));
                                }
                            } else {
                                code.push(Instr::LoadLocal(slot));
                                for idx in &indices {
                                    self.compile_expr(idx, ctx, code);
                                }
                                self.compile_expr(value, ctx, code);
                                code.push(Instr::ArraySetChain(indices.len()));
                                code.push(Instr::StoreLocal(slot));
                            }
                        } else {
                            self.error(format!("Unknown local `{root}` in index assignment"));
                        }
                    } else {
                        self.error("Unsupported index assignment target".to_string());
                    }
                }
                AssignTarget::Field { .. } => {
                    if let Some((root, fields)) = Self::flatten_field_target(target) {
                        if let Some(slot) = ctx.lookup(&root) {
                            code.push(Instr::LoadLocal(slot));
                            self.compile_expr(value, ctx, code);
                            if let Some(root_ty) = ctx.named_type(&root)
                                && let Some(slots) = self.resolve_field_slots(&root_ty, &fields)
                            {
                                code.push(Instr::StructSetPathSlots(slots));
                            } else {
                                code.push(Instr::StructSetPath(fields));
                            }
                            code.push(Instr::StoreLocal(slot));
                        } else {
                            self.error("Path assignment not supported in bytecode v0".to_string());
                        }
                    } else {
                        self.compile_expr(value, ctx, code);
                        self.error(
                            "Unsupported field assignment target in bytecode compiler".to_string(),
                        );
                    }
                }
            },
            Stmt::Expr(expr) => {
                self.compile_expr(expr, ctx, code);
                code.push(Instr::Pop);
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr, ctx, code);
                } else {
                    code.push(Instr::LoadConst(Value::Unit));
                }
                code.push(Instr::Return);
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                let jmp_false_at = self.compile_cond_jump_false(cond, ctx, code);

                for s in then_body {
                    self.compile_stmt(s, ctx, loops, code);
                }

                if else_body.is_empty() {
                    let after_then = code.len();
                    Self::patch_jump_false_target(code, jmp_false_at, after_then);
                } else {
                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));

                    let else_start = code.len();
                    Self::patch_jump_false_target(code, jmp_false_at, else_start);

                    for s in else_body {
                        self.compile_stmt(s, ctx, loops, code);
                    }

                    let end = code.len();
                    code[jmp_end_at] = Instr::Jump(end);
                }
            }
            Stmt::While { cond, body } => {
                let loop_start = code.len();
                loops.push(LoopCtx {
                    continue_target: loop_start,
                    break_jumps: Vec::new(),
                });
                let jmp_false_at = self.compile_cond_jump_false(cond, ctx, code);

                for s in body {
                    self.compile_stmt(s, ctx, loops, code);
                }

                code.push(Instr::Jump(loop_start));
                let loop_end = code.len();
                Self::patch_jump_false_target(code, jmp_false_at, loop_end);
                if let Some(lp) = loops.pop() {
                    for at in lp.break_jumps {
                        code[at] = Instr::Jump(loop_end);
                    }
                }
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => {
                if let Some(init) = init {
                    self.compile_stmt(init, ctx, loops, code);
                }

                let cond_start = code.len();
                if let Some(cond) = cond {
                    self.compile_expr(cond, ctx, code);
                } else {
                    code.push(Instr::LoadConst(Value::Bool(true)));
                }
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                let jmp_body_at = code.len();
                code.push(Instr::Jump(usize::MAX));
                let step_start = code.len();
                loops.push(LoopCtx {
                    continue_target: step_start,
                    break_jumps: Vec::new(),
                });

                if let Some(step) = step {
                    self.compile_stmt(step, ctx, loops, code);
                }
                code.push(Instr::Jump(cond_start));
                let body_start = code.len();
                code[jmp_body_at] = Instr::Jump(body_start);

                for s in body {
                    self.compile_stmt(s, ctx, loops, code);
                }
                code.push(Instr::Jump(step_start));

                let loop_end = code.len();
                Self::patch_jump_false_target(code, jmp_false_at, loop_end);
                if let Some(lp) = loops.pop() {
                    for at in lp.break_jumps {
                        code[at] = Instr::Jump(loop_end);
                    }
                }
            }
            Stmt::Break => {
                if let Some(lp) = loops.last_mut() {
                    let at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    lp.break_jumps.push(at);
                } else {
                    self.error("`break` used outside a loop".to_string());
                }
            }
            Stmt::Continue => {
                if let Some(lp) = loops.last() {
                    code.push(Instr::Jump(lp.continue_target));
                } else {
                    self.error("`continue` used outside a loop".to_string());
                }
            }
            Stmt::Match { expr, arms } => {
                self.compile_expr(expr, ctx, code);
                let match_slot = ctx.alloc_anonymous_local();
                code.push(Instr::StoreLocal(match_slot));

                let mut end_jumps = Vec::new();
                for arm in arms {
                    self.compile_match_pattern_condition(&arm.pattern, match_slot, code);
                    let jmp_false_at = code.len();
                    code.push(Instr::JumpIfFalse(usize::MAX));

                    for s in &arm.body {
                        self.compile_stmt(s, ctx, loops, code);
                    }

                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    end_jumps.push(jmp_end_at);

                    let next_arm = code.len();
                    code[jmp_false_at] = Instr::JumpIfFalse(next_arm);
                }

                let end = code.len();
                for at in end_jumps {
                    code[at] = Instr::Jump(end);
                }
            }
        }
    }

    fn compile_match_pattern_condition(
        &mut self,
        pattern: &MatchPattern,
        match_slot: usize,
        code: &mut Vec<Instr>,
    ) {
        match pattern {
            MatchPattern::Wildcard => code.push(Instr::LoadConst(Value::Bool(true))),
            MatchPattern::Literal(lit) => {
                code.push(Instr::LoadLocal(match_slot));
                self.compile_match_literal(lit, code);
                code.push(Instr::Eq);
            }
            MatchPattern::Or(parts) => {
                let mut iter = parts.iter();
                if let Some(first) = iter.next() {
                    self.compile_match_pattern_condition(first, match_slot, code);
                    for part in iter {
                        self.compile_match_pattern_condition(part, match_slot, code);
                        code.push(Instr::OrBool);
                    }
                } else {
                    code.push(Instr::LoadConst(Value::Bool(false)));
                }
            }
        }
    }

    fn compile_match_literal(&mut self, lit: &MatchLiteral, code: &mut Vec<Instr>) {
        match lit {
            MatchLiteral::Int(v) => code.push(Instr::LoadConst(Value::Int(*v))),
            MatchLiteral::Bool(v) => code.push(Instr::LoadConst(Value::Bool(*v))),
            MatchLiteral::String(v) => {
                code.push(Instr::LoadConst(Value::String(Rc::<str>::from(v.clone()))))
            }
            MatchLiteral::Float(v) => match v.parse::<f64>() {
                Ok(n) => code.push(Instr::LoadConst(Value::Float(n))),
                Err(_) => {
                    self.error(format!("Invalid float literal in match pattern `{v}`"));
                    code.push(Instr::LoadConst(Value::Float(0.0)));
                }
            },
        }
    }
}
