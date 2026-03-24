use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::types::TypeInfo;

use super::Checker;

impl Checker {
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

    fn check_path_expr(
        &mut self,
        parts: &[String],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if parts.len() == 2
            && matches!(
                parts[0].as_str(),
                "io" | "str" | "arr" | "datetime" | "random" | "os" | "fs" | "vec"
            )
        {
            self.error(format!(
                "Builtin path `{}` is not a value; call it as a function",
                parts.join(".")
            ));
            return TypeInfo::Unknown;
        }

        let joined = parts.join(".");
        let mut lookup_keys = vec![joined.clone()];
        if let Some(prefix) = self.module_namespaces.get(&parts[0]) {
            let mut expanded = prefix.clone();
            expanded.extend_from_slice(&parts[1..]);
            let expanded_joined = expanded.join(".");
            if expanded_joined != joined {
                lookup_keys.push(expanded_joined);
            }
        }

        for key in &lookup_keys {
            if let Some(sig) = self.functions.get(key) {
                return TypeInfo::Fn {
                    params: sig.params.clone(),
                    ret: Box::new(sig.ret.clone()),
                };
            }
            if let Some(ty) = self.globals.get(key) {
                return ty.clone();
            }
        }

        if let Some(sig) = self.functions.get(&joined) {
            return TypeInfo::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
            };
        }
        if self.resolve_named_type_name(&joined).is_some() {
            self.error(format!("Type path `{joined}` is not a value expression"));
            return TypeInfo::Unknown;
        }

        let _ = scopes;
        self.error(format!("Unknown path `{joined}`"));
        TypeInfo::Unknown
    }

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
            Expr::Path(parts) => self.check_path_expr(parts, scopes),
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
                    UnaryOp::BitNot => {
                        if ty == TypeInfo::Int || ty == TypeInfo::Unknown {
                            TypeInfo::Int
                        } else {
                            self.error("Unary `~` expects Int".to_string());
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
            Expr::CustomInfix {
                left,
                operator,
                right,
            } => {
                let _ = self.check_expr(left, scopes);
                let _ = self.check_expr(right, scopes);
                self.error(format!(
                    "User-defined operator expressions are parsed but not semantically enabled yet: `{}`",
                    operator
                ));
                TypeInfo::Unknown
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
                    TypeInfo::Vec { elem } => *elem,
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.error(format!("Cannot index into non-indexable type {:?}", other));
                        TypeInfo::Unknown
                    }
                }
            }
            Expr::Field { base, field } => {
                if let Some(parts) = Self::expr_to_path_parts(expr)
                    && parts.len() >= 2
                    && (self.module_namespaces.contains_key(&parts[0])
                        || matches!(
                            parts[0].as_str(),
                            "io" | "str" | "arr" | "datetime" | "random" | "os" | "fs" | "vec"
                        ))
                {
                    return self.check_path_expr(&parts, scopes);
                }
                let base_ty = self.check_expr(base, scopes);
                match base_ty {
                    TypeInfo::Named(struct_name) => {
                        if let Some(field_ty) = self.field_type(&struct_name, field) {
                            field_ty
                        } else {
                            self.error(format!(
                                "Unknown field `{}` on struct `{}`",
                                field, struct_name
                            ));
                            TypeInfo::Unknown
                        }
                    }
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.error(format!(
                            "Field access requires struct value, got {:?}",
                            other
                        ));
                        TypeInfo::Unknown
                    }
                }
            }
            Expr::StructLit { name, fields } => {
                let Some(expected_fields) = self.struct_fields.get(name).cloned() else {
                    self.error(format!("Unknown struct `{name}`"));
                    for (_, expr) in fields {
                        self.check_expr(expr, scopes);
                    }
                    return TypeInfo::Unknown;
                };

                let mut seen = HashMap::new();
                for (field_name, expr) in fields {
                    let value_ty = self.check_expr(expr, scopes);
                    let Some(expected_ty) = expected_fields.get(field_name).cloned() else {
                        self.error(format!(
                            "Unknown field `{field_name}` in struct `{name}` literal"
                        ));
                        continue;
                    };
                    if seen.insert(field_name.clone(), ()).is_some() {
                        self.error(format!(
                            "Duplicate field `{field_name}` in struct `{name}` literal"
                        ));
                    }
                    if value_ty != TypeInfo::Unknown && value_ty != expected_ty {
                        self.error(format!(
                            "Type mismatch for field `{field_name}` in struct `{name}` literal: expected {:?}, got {:?}",
                            expected_ty, value_ty
                        ));
                    }
                }

                for expected_name in expected_fields.keys() {
                    if !seen.contains_key(expected_name) {
                        self.error(format!(
                            "Missing field `{expected_name}` in struct `{name}` literal"
                        ));
                    }
                }

                TypeInfo::Named(name.clone())
            }
            Expr::FnLit {
                params,
                return_type,
                body,
            } => {
                for p in params {
                    self.check_decl_type_exists(
                        &p.ty,
                        format!("Unknown type in function literal parameter `{}`", p.name),
                    );
                }
                self.check_decl_type_exists(
                    return_type,
                    "Unknown function literal return type".to_string(),
                );

                let expected_ret = TypeInfo::from_ast(return_type);
                let mut inner_scopes = scopes.to_vec();
                let outer_scope_len = inner_scopes.len();
                inner_scopes.push(HashMap::<String, TypeInfo>::new());
                for p in params {
                    inner_scopes[outer_scope_len].insert(p.name.clone(), TypeInfo::from_ast(&p.ty));
                }
                self.fn_lit_scope_floors.push(outer_scope_len);
                let saved_loop_depth = self.loop_depth;
                self.loop_depth = 0;
                for stmt in body {
                    self.check_stmt(stmt, &mut inner_scopes, &expected_ret);
                }
                self.loop_depth = saved_loop_depth;
                self.fn_lit_scope_floors.pop();
                if expected_ret != TypeInfo::Void && !Self::block_must_return(body) {
                    self.error(format!(
                        "Function literal may exit without returning {:?}",
                        expected_ret
                    ));
                }

                TypeInfo::Fn {
                    params: params.iter().map(|p| TypeInfo::from_ast(&p.ty)).collect(),
                    ret: Box::new(expected_ret),
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
            BitAnd | BitOr | BitXor | Shl | Shr => {
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
                if matches!(lt, TypeInfo::Fn { .. }) || matches!(rt, TypeInfo::Fn { .. }) {
                    self.error("Function values cannot be compared with `==` or `!=`".to_string());
                    return TypeInfo::Unknown;
                }
                if matches!(lt, TypeInfo::Vec { .. }) || matches!(rt, TypeInfo::Vec { .. }) {
                    self.error("Vector values cannot be compared with `==` or `!=`".to_string());
                    return TypeInfo::Unknown;
                }
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
