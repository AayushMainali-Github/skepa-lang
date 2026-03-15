use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Stmt};

use super::context::{Compiler, InlinableFunction, InlinableMethod, StructLayout};

impl Compiler {
    pub(super) fn detect_inlinable_function(
        func: &crate::ast::FnDecl,
    ) -> Option<InlinableFunction> {
        if func.params.len() != 1 {
            return None;
        }
        let param_name = &func.params[0].name;
        let [Stmt::Return(Some(expr))] = func.body.as_slice() else {
            return None;
        };
        let expr = Self::strip_groups(expr);
        let Expr::Binary { left, op, right } = expr else {
            return None;
        };
        if *op != BinaryOp::Add {
            return None;
        }
        match (Self::strip_groups(left), Self::strip_groups(right)) {
            (Expr::Ident(name), Expr::IntLit(rhs)) if name == param_name => {
                Some(InlinableFunction::AddConst(*rhs))
            }
            (Expr::IntLit(rhs), Expr::Ident(name)) if name == param_name => {
                Some(InlinableFunction::AddConst(*rhs))
            }
            _ => None,
        }
    }

    pub(super) fn detect_inlinable_method(
        method: &crate::ast::MethodDecl,
        target_name: &str,
        layouts: &HashMap<String, StructLayout>,
    ) -> Option<InlinableMethod> {
        let layout = layouts.get(target_name)?;
        if method.params.len() != 2 {
            return None;
        }
        let arg_name = &method.params[1].name;
        let [Stmt::Return(Some(expr))] = method.body.as_slice() else {
            return None;
        };
        if let Some(field_name) = Self::match_self_field_plus_arg(expr, arg_name) {
            return Some(InlinableMethod::StructFieldAdd {
                field_slot: *layout.field_slots.get(field_name)?,
            });
        }
        let (lhs_field, rhs_field, mul, modulo) =
            Self::match_self_field_add_mul_field_mod(expr, arg_name)?;
        Some(InlinableMethod::StructFieldAddMulFieldMod {
            lhs_field_slot: *layout.field_slots.get(lhs_field)?,
            rhs_field_slot: *layout.field_slots.get(rhs_field)?,
            mul,
            modulo,
        })
    }

    fn match_self_field_plus_arg<'a>(expr: &'a Expr, arg_name: &str) -> Option<&'a str> {
        let expr = Self::strip_groups(expr);
        let Expr::Binary { left, op, right } = expr else {
            return None;
        };
        if *op != BinaryOp::Add {
            return None;
        }
        match (Self::strip_groups(left), Self::strip_groups(right)) {
            (Expr::Field { base, field }, Expr::Ident(arg)) if arg == arg_name => {
                let Expr::Ident(self_name) = Self::strip_groups(base) else {
                    return None;
                };
                if self_name == "self" {
                    Some(field.as_str())
                } else {
                    None
                }
            }
            (Expr::Ident(arg), Expr::Field { base, field }) if arg == arg_name => {
                let Expr::Ident(self_name) = Self::strip_groups(base) else {
                    return None;
                };
                if self_name == "self" {
                    Some(field.as_str())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn match_self_field_add_mul_field_mod<'a>(
        expr: &'a Expr,
        arg_name: &str,
    ) -> Option<(&'a str, &'a str, i64, i64)> {
        let expr = Self::strip_groups(expr);
        let Expr::Binary {
            left: mod_left,
            op: mod_op,
            right: mod_right,
        } = expr
        else {
            return None;
        };
        if *mod_op != BinaryOp::Mod {
            return None;
        }
        let Expr::IntLit(modulo) = Self::strip_groups(mod_right) else {
            return None;
        };
        let mod_left = Self::strip_groups(mod_left);
        let Expr::Binary {
            left: add_left,
            op: add_op,
            right: add_right,
        } = mod_left
        else {
            return None;
        };
        if *add_op != BinaryOp::Add {
            return None;
        }
        let Expr::Field {
            base: rhs_base,
            field: rhs_field,
        } = Self::strip_groups(add_right)
        else {
            return None;
        };
        let Expr::Ident(rhs_self) = Self::strip_groups(rhs_base) else {
            return None;
        };
        if rhs_self != "self" {
            return None;
        }
        let add_left = Self::strip_groups(add_left);
        let Expr::Binary {
            left: mul_left,
            op: mul_op,
            right: mul_right,
        } = add_left
        else {
            return None;
        };
        if *mul_op != BinaryOp::Mul {
            return None;
        }
        let Expr::IntLit(mul) = Self::strip_groups(mul_right) else {
            return None;
        };
        let lhs_field = Self::match_self_field_plus_arg(mul_left, arg_name)?;
        Some((lhs_field, rhs_field.as_str(), *mul, *modulo))
    }

    pub(super) fn strip_groups(mut expr: &Expr) -> &Expr {
        while let Expr::Group(inner) = expr {
            expr = inner;
        }
        expr
    }
}
