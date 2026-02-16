use std::collections::HashMap;

use crate::ast::Expr;
use crate::types::TypeInfo;

use super::Checker;

impl Checker {
    pub(super) fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if let Expr::Path(parts) = callee
            && parts.len() == 2
        {
            return self.check_builtin_call(&parts[0], &parts[1], args, scopes);
        }

        let fn_name = match callee {
            Expr::Ident(name) => name.clone(),
            Expr::Path(parts) => parts.join("."),
            _ => {
                self.error("Invalid call target".to_string());
                return TypeInfo::Unknown;
            }
        };

        let Some(sig) = self.functions.get(&fn_name).cloned() else {
            self.error(format!("Unknown function `{fn_name}`"));
            return TypeInfo::Unknown;
        };

        if sig.params.len() != args.len() {
            self.error(format!(
                "Arity mismatch for `{}`: expected {}, got {}",
                sig.name,
                sig.params.len(),
                args.len()
            ));
            return sig.ret.clone();
        }

        for (i, arg) in args.iter().enumerate() {
            let got = self.check_expr(arg, scopes);
            let expected = sig.params[i].clone();
            if got != TypeInfo::Unknown && got != expected {
                self.error(format!(
                    "Argument {} for `{}`: expected {:?}, got {:?}",
                    i + 1,
                    sig.name,
                    expected,
                    got
                ));
            }
        }

        sig.ret
    }

    fn check_builtin_call(
        &mut self,
        package: &str,
        method: &str,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if !self.imported_modules.contains(package) {
            self.error(format!("`{package}.*` used without `import {package};`"));
            return TypeInfo::Unknown;
        }

        let Some(sig) = crate::builtins::find_builtin_sig(package, method) else {
            self.error(format!("Unknown builtin `{package}.{method}`"));
            return TypeInfo::Unknown;
        };

        match sig.kind {
            crate::builtins::BuiltinKind::FixedArity => {
                if sig.params.len() != args.len() {
                    self.error(format!(
                        "{package}.{method} expects {} argument(s), got {}",
                        sig.params.len(),
                        args.len()
                    ));
                    return sig.ret.clone();
                }

                for (idx, arg) in args.iter().enumerate() {
                    let got = self.check_expr(arg, scopes);
                    let expected = sig.params[idx].clone();
                    if got != TypeInfo::Unknown && got != expected {
                        self.error(format!(
                            "{package}.{method} argument {} expects {:?}, got {:?}",
                            idx + 1,
                            expected,
                            got
                        ));
                    }
                }
            }
            crate::builtins::BuiltinKind::FormatVariadic => {
                if args.is_empty() {
                    self.error(format!("{package}.{method} expects at least 1 argument"));
                    return sig.ret.clone();
                }
                let fmt_ty = self.check_expr(&args[0], scopes);
                if fmt_ty != TypeInfo::String && fmt_ty != TypeInfo::Unknown {
                    self.error(format!(
                        "{package}.{method} argument 1 expects {:?}, got {:?}",
                        TypeInfo::String,
                        fmt_ty
                    ));
                }

                if let Expr::StringLit(fmt) = &args[0] {
                    match Self::parse_format_specifiers(fmt) {
                        Ok(specs) => {
                            let expected_args = specs.len();
                            let got_args = args.len().saturating_sub(1);
                            if expected_args != got_args {
                                self.error(format!(
                                    "{package}.{method} format expects {} value argument(s), got {}",
                                    expected_args, got_args
                                ));
                            }
                            for (idx, arg) in args.iter().skip(1).enumerate() {
                                let got = self.check_expr(arg, scopes);
                                if idx >= specs.len() {
                                    continue;
                                }
                                let expected = match specs[idx] {
                                    'd' => TypeInfo::Int,
                                    'f' => TypeInfo::Float,
                                    's' => TypeInfo::String,
                                    'b' => TypeInfo::Bool,
                                    _ => TypeInfo::Unknown,
                                };
                                if got != TypeInfo::Unknown && got != expected {
                                    self.error(format!(
                                        "{package}.{method} argument {} expects {:?} for `%{}`, got {:?}",
                                        idx + 2,
                                        expected,
                                        specs[idx],
                                        got
                                    ));
                                }
                            }
                        }
                        Err(msg) => self.error(format!("{package}.{method} format error: {msg}")),
                    }
                } else {
                    for arg in args.iter().skip(1) {
                        self.check_expr(arg, scopes);
                    }
                }
            }
            crate::builtins::BuiltinKind::ArrayOps => match method {
                "len" | "isEmpty" | "sum" | "first" | "last" | "reverse" | "min" | "max"
                | "sort" => {
                    if args.len() != 1 {
                        self.error(format!(
                            "{package}.{method} expects 1 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let TypeInfo::Array { elem, size } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    match method {
                        "len" => return TypeInfo::Int,
                        "isEmpty" => return TypeInfo::Bool,
                        "reverse" => {
                            return TypeInfo::Array {
                                elem: elem.clone(),
                                size,
                            };
                        }
                        "first" | "last" => return *elem,
                        "sum" => {
                            let sum_ty = *elem;
                            if !matches!(
                                sum_ty,
                                TypeInfo::Int
                                    | TypeInfo::Float
                                    | TypeInfo::String
                                    | TypeInfo::Array { .. }
                                    | TypeInfo::Unknown
                            ) {
                                self.error(format!(
                                    "arr.sum supports Int, Float, String, or Array elements, got {:?}",
                                    sum_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            if let TypeInfo::Array {
                                elem: inner_elem,
                                size: inner_size,
                            } = sum_ty
                            {
                                return TypeInfo::Array {
                                    elem: inner_elem,
                                    size: inner_size.saturating_mul(size),
                                };
                            }
                            return sum_ty;
                        }
                        "min" | "max" => {
                            let elem_ty = *elem;
                            if !matches!(
                                elem_ty,
                                TypeInfo::Int | TypeInfo::Float | TypeInfo::Unknown
                            ) {
                                self.error(format!(
                                    "arr.{method} supports Int or Float elements, got {:?}",
                                    elem_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            return elem_ty;
                        }
                        "sort" => {
                            let elem_ty = *elem;
                            if !matches!(
                                elem_ty,
                                TypeInfo::Int
                                    | TypeInfo::Float
                                    | TypeInfo::String
                                    | TypeInfo::Bool
                                    | TypeInfo::Unknown
                            ) {
                                self.error(format!(
                                    "arr.sort supports Int, Float, String, or Bool elements, got {:?}",
                                    elem_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            return TypeInfo::Array {
                                elem: Box::new(elem_ty),
                                size,
                            };
                        }
                        _ => unreachable!(),
                    }
                }
                "contains" | "indexOf" | "count" => {
                    if args.len() != 2 {
                        self.error(format!(
                            "{package}.{method} expects 2 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let needle_ty = self.check_expr(&args[1], scopes);
                    let TypeInfo::Array { elem, .. } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    let elem_ty = *elem;
                    if needle_ty != TypeInfo::Unknown
                        && elem_ty != TypeInfo::Unknown
                        && needle_ty != elem_ty
                    {
                        self.error(format!(
                            "{package}.{method} argument 2 expects {:?}, got {:?}",
                            elem_ty, needle_ty
                        ));
                    }
                    return match method {
                        "contains" => TypeInfo::Bool,
                        "indexOf" | "count" => TypeInfo::Int,
                        _ => unreachable!(),
                    };
                }
                "join" => {
                    if args.len() != 2 {
                        self.error(format!(
                            "{package}.{method} expects 2 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let sep_ty = self.check_expr(&args[1], scopes);
                    if sep_ty != TypeInfo::String && sep_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 2 expects String, got {:?}",
                            sep_ty
                        ));
                    }
                    let TypeInfo::Array { elem, .. } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    if *elem != TypeInfo::String && *elem != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 1 expects Array[String], got {:?}",
                            TypeInfo::Array { elem, size: 0 }
                        ));
                        return TypeInfo::Unknown;
                    }
                    return TypeInfo::String;
                }
                "slice" => {
                    if args.len() != 3 {
                        self.error(format!(
                            "{package}.{method} expects 3 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let start_ty = self.check_expr(&args[1], scopes);
                    let end_ty = self.check_expr(&args[2], scopes);
                    if start_ty != TypeInfo::Int && start_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 2 expects Int, got {:?}",
                            start_ty
                        ));
                    }
                    if end_ty != TypeInfo::Int && end_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 3 expects Int, got {:?}",
                            end_ty
                        ));
                    }
                    let TypeInfo::Array { elem, size } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    let Some(start) = Self::const_non_negative_int(&args[1]) else {
                        self.error(
                            "arr.slice argument 2 must be a non-negative Int literal for static arrays"
                                .to_string(),
                        );
                        return TypeInfo::Unknown;
                    };
                    let Some(end) = Self::const_non_negative_int(&args[2]) else {
                        self.error(
                            "arr.slice argument 3 must be a non-negative Int literal for static arrays"
                                .to_string(),
                        );
                        return TypeInfo::Unknown;
                    };
                    if start > end || end > size {
                        self.error(format!(
                            "arr.slice bounds out of range at compile time: start={start}, end={end}, len={size}"
                        ));
                        return TypeInfo::Unknown;
                    }
                    return TypeInfo::Array {
                        elem,
                        size: end - start,
                    };
                }
                _ => {
                    self.error(format!("Unsupported array builtin `{package}.{method}`"));
                    return TypeInfo::Unknown;
                }
            },
        }

        sig.ret.clone()
    }
}
