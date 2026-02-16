use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::types::TypeInfo;

use super::Checker;

impl Checker {
    pub(super) fn check_expr(
        &mut self,
        expr: &Expr,
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        match expr {
            Expr::IntLit(_) => TypeInfo::Int,
            Expr::FloatLit(_) => TypeInfo::Float,
            Expr::BoolLit(_) => TypeInfo::Bool,
            Expr::StringLit(_) => TypeInfo::String,
            Expr::Ident(name) => self.lookup_var(name, scopes),
            Expr::Path(parts) => {
                if parts.len() == 2 && (parts[0] == "io" || parts[0] == "str" || parts[0] == "arr")
                {
                    return TypeInfo::Unknown;
                }
                self.error(format!("Unknown path `{}`", parts.join(".")));
                TypeInfo::Unknown
            }
            Expr::Group(inner) => self.check_expr(inner, scopes),
            Expr::Unary { op, expr } => {
                let ty = self.check_expr(expr, scopes);
                match op {
                    UnaryOp::Neg => {
                        if ty == TypeInfo::Int || ty == TypeInfo::Float || ty == TypeInfo::Unknown {
                            ty
                        } else {
                            self.error("Unary `-` expects Int or Float".to_string());
                            TypeInfo::Unknown
                        }
                    }
                    UnaryOp::Pos => {
                        if ty == TypeInfo::Int || ty == TypeInfo::Float || ty == TypeInfo::Unknown {
                            ty
                        } else {
                            self.error("Unary `+` expects Int or Float".to_string());
                            TypeInfo::Unknown
                        }
                    }
                    UnaryOp::Not => {
                        if ty == TypeInfo::Bool || ty == TypeInfo::Unknown {
                            TypeInfo::Bool
                        } else {
                            self.error("Unary `!` expects Bool".to_string());
                            TypeInfo::Unknown
                        }
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let lt = self.check_expr(left, scopes);
                let rt = self.check_expr(right, scopes);
                self.check_binary(*op, lt, rt)
            }
            Expr::Call { callee, args } => self.check_call(callee, args, scopes),
            Expr::ArrayLit(items) => {
                if items.is_empty() {
                    self.error("Cannot infer type of empty array literal".to_string());
                    return TypeInfo::Unknown;
                }
                let mut elem_ty = self.check_expr(&items[0], scopes);
                for item in &items[1..] {
                    let t = self.check_expr(item, scopes);
                    if elem_ty == TypeInfo::Unknown {
                        elem_ty = t;
                        continue;
                    }
                    if t != TypeInfo::Unknown && t != elem_ty {
                        self.error(format!(
                            "Array literal element type mismatch: expected {:?}, got {:?}",
                            elem_ty, t
                        ));
                        return TypeInfo::Unknown;
                    }
                }
                TypeInfo::Array {
                    elem: Box::new(elem_ty),
                    size: items.len(),
                }
            }
            Expr::ArrayRepeat { value, size } => {
                let elem_ty = self.check_expr(value, scopes);
                TypeInfo::Array {
                    elem: Box::new(elem_ty),
                    size: *size,
                }
            }
            Expr::Index { base, index } => {
                let base_ty = self.check_expr(base, scopes);
                let idx_ty = self.check_expr(index, scopes);
                if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                    self.error("Array index must be Int".to_string());
                }
                match base_ty {
                    TypeInfo::Array { elem, .. } => *elem,
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.error(format!("Cannot index into non-array type {:?}", other));
                        TypeInfo::Unknown
                    }
                }
            }
        }
    }

    fn check_binary(&mut self, op: BinaryOp, lt: TypeInfo, rt: TypeInfo) -> TypeInfo {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div => {
                if lt == TypeInfo::Int && rt == TypeInfo::Int {
                    TypeInfo::Int
                } else if lt == TypeInfo::Float && rt == TypeInfo::Float {
                    TypeInfo::Float
                } else if op == Add && lt == TypeInfo::String && rt == TypeInfo::String {
                    TypeInfo::String
                } else if op == Add {
                    match (&lt, &rt) {
                        (
                            TypeInfo::Array {
                                elem: l_elem,
                                size: l_size,
                            },
                            TypeInfo::Array {
                                elem: r_elem,
                                size: r_size,
                            },
                        ) if l_elem == r_elem => TypeInfo::Array {
                            elem: l_elem.clone(),
                            size: l_size + r_size,
                        },
                        _ => {
                            if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                                TypeInfo::Unknown
                            } else {
                                self.error(format!(
                                    "Invalid operands for {:?}: left {:?}, right {:?}",
                                    op, lt, rt
                                ));
                                TypeInfo::Unknown
                            }
                        }
                    }
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid operands for {:?}: left {:?}, right {:?}",
                        op, lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            Mod => {
                if lt == TypeInfo::Int && rt == TypeInfo::Int {
                    TypeInfo::Int
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid operands for {:?}: left {:?}, right {:?}",
                        op, lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            EqEq | Neq => {
                if lt == rt || lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Bool
                } else {
                    self.error(format!(
                        "Invalid equality operands: left {:?}, right {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            Lt | Lte | Gt | Gte => {
                if (lt == TypeInfo::Int && rt == TypeInfo::Int)
                    || (lt == TypeInfo::Float && rt == TypeInfo::Float)
                {
                    TypeInfo::Bool
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid comparison operands: left {:?}, right {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            AndAnd | OrOr => {
                if lt == TypeInfo::Bool && rt == TypeInfo::Bool {
                    TypeInfo::Bool
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Logical operators require Bool operands, got {:?} and {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
        }
    }
}
