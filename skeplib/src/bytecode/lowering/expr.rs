use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::vm::default_builtin_id;

use super::context::{Compiler, FnCtx, InlinableMethod, PrimitiveType};
use super::{Instr, Value};

impl Compiler {
    pub(super) fn compile_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ctx: &mut FnCtx,
        code: &mut Vec<Instr>,
    ) {
        match callee {
            Expr::Ident(name) => {
                if ctx.lookup(name).is_some() {
                    self.compile_expr(callee, ctx, code);
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::CallValue { argc: args.len() });
                } else if let Some(target) = self.direct_import_calls.get(name).cloned() {
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::Call {
                        name: target,
                        argc: args.len(),
                    });
                } else if let Some(target) = self.local_fn_qualified.get(name).cloned() {
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::Call {
                        name: target,
                        argc: args.len(),
                    });
                } else {
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::Call {
                        name: name.clone(),
                        argc: args.len(),
                    });
                }
            }
            Expr::Field { base, field } => {
                if let Some(parts) = Self::expr_to_parts(callee)
                    && parts.len() == 2
                    && matches!(base.as_ref(), Expr::Ident(pkg) if ctx.lookup(pkg).is_none())
                {
                    if self.specialized_builtin_call(&parts, args, ctx, code) {
                        return;
                    }
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    if let Some(id) = default_builtin_id(&parts[0], &parts[1]) {
                        code.push(Instr::CallBuiltinId {
                            id,
                            argc: args.len(),
                        });
                    } else {
                        code.push(Instr::CallBuiltin {
                            package: parts[0].clone(),
                            name: parts[1].clone(),
                            argc: args.len(),
                        });
                    }
                    return;
                }
                if let Some(parts) = Self::expr_to_parts(callee)
                    && let Some(target) = self.resolve_qualified_import_call(&parts)
                {
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::Call {
                        name: target,
                        argc: args.len(),
                    });
                    return;
                }
                if let Some(parts) = Self::expr_to_parts(callee)
                    && parts.len() > 2
                {
                    self.error("Only `package.function(...)` builtins are supported".to_string());
                    return;
                }

                if let Some(base_ty) = Self::infer_expr_named_type(base, ctx) {
                    let target_name = self.resolve_struct_runtime_name(&base_ty);
                    let mangled = Self::mangle_method_name(&target_name, field);
                    if self.function_names.contains(&mangled) {
                        if self.try_inline_method_call(base, &mangled, args, ctx, code) {
                            return;
                        }
                        self.compile_expr(base, ctx, code);
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::Call {
                            name: mangled,
                            argc: args.len() + 1,
                        });
                    } else {
                        self.compile_expr(base, ctx, code);
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::CallMethodId {
                            id: self.intern_method_name(field),
                            argc: args.len(),
                        });
                    }
                } else {
                    self.compile_expr(base, ctx, code);
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::CallMethodId {
                        id: self.intern_method_name(field),
                        argc: args.len(),
                    });
                }
            }
            _ => {
                self.compile_expr(callee, ctx, code);
                for arg in args {
                    self.compile_expr(arg, ctx, code);
                }
                code.push(Instr::CallValue { argc: args.len() });
            }
        }
    }

    fn specialized_builtin_call(
        &mut self,
        parts: &[String],
        args: &[Expr],
        ctx: &mut FnCtx,
        code: &mut Vec<Instr>,
    ) -> bool {
        if parts.len() != 2 || parts[0] != "str" {
            return false;
        }
        match (parts[1].as_str(), args) {
            ("len", [Expr::Ident(name)]) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::StrLenLocal(slot));
                    true
                } else {
                    self.compile_expr(&args[0], ctx, code);
                    code.push(Instr::StrLen);
                    true
                }
            }
            ("len", [arg]) => {
                self.compile_expr(arg, ctx, code);
                code.push(Instr::StrLen);
                true
            }
            ("indexOf", [Expr::Ident(name), Expr::StringLit(needle)]) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::StrIndexOfLocalConst {
                        slot,
                        needle: Rc::<str>::from(needle.clone()),
                    });
                    true
                } else {
                    self.compile_expr(&args[0], ctx, code);
                    code.push(Instr::StrIndexOfConst(Rc::<str>::from(needle.clone())));
                    true
                }
            }
            ("indexOf", [arg, Expr::StringLit(needle)]) => {
                self.compile_expr(arg, ctx, code);
                code.push(Instr::StrIndexOfConst(Rc::<str>::from(needle.clone())));
                true
            }
            ("slice", [Expr::Ident(name), Expr::IntLit(start), Expr::IntLit(end)]) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::StrSliceLocalConst {
                        slot,
                        start: *start,
                        end: *end,
                    });
                    true
                } else {
                    self.compile_expr(&args[0], ctx, code);
                    code.push(Instr::StrSliceConst {
                        start: *start,
                        end: *end,
                    });
                    true
                }
            }
            ("slice", [arg, Expr::IntLit(start), Expr::IntLit(end)]) => {
                self.compile_expr(arg, ctx, code);
                code.push(Instr::StrSliceConst {
                    start: *start,
                    end: *end,
                });
                true
            }
            ("contains", [Expr::Ident(name), Expr::StringLit(needle)]) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::StrContainsLocalConst {
                        slot,
                        needle: Rc::<str>::from(needle.clone()),
                    });
                    true
                } else {
                    self.compile_expr(&args[0], ctx, code);
                    code.push(Instr::StrContainsConst(Rc::<str>::from(needle.clone())));
                    true
                }
            }
            ("contains", [arg, Expr::StringLit(needle)]) => {
                self.compile_expr(arg, ctx, code);
                code.push(Instr::StrContainsConst(Rc::<str>::from(needle.clone())));
                true
            }
            _ => false,
        }
    }

    pub(super) fn infer_expr_named_type(expr: &Expr, ctx: &FnCtx) -> Option<String> {
        match expr {
            Expr::Ident(name) => ctx.named_type(name),
            Expr::StructLit { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    pub(super) fn infer_expr_primitive_type(expr: &Expr, ctx: &FnCtx) -> Option<PrimitiveType> {
        match expr {
            Expr::IntLit(_) => Some(PrimitiveType::Int),
            Expr::FloatLit(_) => Some(PrimitiveType::Float),
            Expr::BoolLit(_) => Some(PrimitiveType::Bool),
            Expr::StringLit(_) => Some(PrimitiveType::String),
            Expr::Ident(name) => ctx.primitive_type(name),
            Expr::Group(inner) => Self::infer_expr_primitive_type(inner, ctx),
            Expr::Unary { op, expr } => match (op, Self::infer_expr_primitive_type(expr, ctx)) {
                (UnaryOp::Neg | UnaryOp::Pos, Some(PrimitiveType::Int)) => Some(PrimitiveType::Int),
                (UnaryOp::Not, Some(PrimitiveType::Bool)) => Some(PrimitiveType::Bool),
                _ => None,
            },
            Expr::Binary { left, op, right } => {
                let left_ty = Self::infer_expr_primitive_type(left, ctx);
                let right_ty = Self::infer_expr_primitive_type(right, ctx);
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        if left_ty == Some(PrimitiveType::Int)
                            && right_ty == Some(PrimitiveType::Int)
                        {
                            Some(PrimitiveType::Int)
                        } else {
                            None
                        }
                    }
                    BinaryOp::EqEq
                    | BinaryOp::Neq
                    | BinaryOp::Lt
                    | BinaryOp::Lte
                    | BinaryOp::Gt
                    | BinaryOp::Gte
                    | BinaryOp::AndAnd
                    | BinaryOp::OrOr => Some(PrimitiveType::Bool),
                }
            }
            _ => None,
        }
    }

    fn resolve_qualified_import_call(&self, parts: &[String]) -> Option<String> {
        let key = parts.join(".");
        if let Some(target) = self.namespace_call_targets.get(&key) {
            return Some(target.clone());
        }
        let prefix = self.module_namespaces.get(parts.first()?)?.clone();
        let mut full = prefix;
        full.extend_from_slice(&parts[1..]);
        Some(full.join("."))
    }

    fn try_inline_method_call(
        &mut self,
        base: &Expr,
        mangled: &str,
        args: &[Expr],
        ctx: &mut FnCtx,
        code: &mut Vec<Instr>,
    ) -> bool {
        let Expr::Ident(base_name) = base else {
            return false;
        };
        let Some(base_slot) = ctx.lookup(base_name) else {
            return false;
        };
        let Some(pattern) = self.inlinable_methods.get(mangled).copied() else {
            return false;
        };
        match pattern {
            InlinableMethod::StructFieldAdd { field_slot } => {
                if args.len() != 1 {
                    return false;
                }
                code.push(Instr::StructGetLocalSlot {
                    slot: base_slot,
                    field_slot,
                });
                self.compile_expr(&args[0], ctx, code);
                code.push(Instr::Add);
                true
            }
            InlinableMethod::StructFieldAddMulFieldMod {
                lhs_field_slot,
                rhs_field_slot,
                mul,
                modulo,
            } => {
                if args.len() != 1 {
                    return false;
                }
                code.push(Instr::StructGetLocalSlot {
                    slot: base_slot,
                    field_slot: lhs_field_slot,
                });
                self.compile_expr(&args[0], ctx, code);
                code.push(Instr::Add);
                code.push(Instr::LoadConst(Value::Int(mul)));
                code.push(Instr::MulInt);
                code.push(Instr::StructGetLocalSlot {
                    slot: base_slot,
                    field_slot: rhs_field_slot,
                });
                code.push(Instr::Add);
                code.push(Instr::LoadConst(Value::Int(modulo)));
                code.push(Instr::ModInt);
                true
            }
        }
    }
}
