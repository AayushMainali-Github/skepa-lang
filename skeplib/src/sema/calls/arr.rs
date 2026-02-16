use std::collections::HashMap;

use crate::ast::Expr;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_arr_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
) -> TypeInfo {
    match method {
        "len" | "isEmpty" | "sum" | "first" | "last" | "reverse" | "min" | "max" | "sort" => {
            if args.len() != 1 {
                checker.error(format!(
                    "arr.{method} expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let arr_ty = checker.check_expr(&args[0], scopes);
            let TypeInfo::Array { elem, size } = arr_ty else {
                if arr_ty != TypeInfo::Unknown {
                    checker.error(format!(
                        "arr.{method} argument 1 expects Array, got {:?}",
                        arr_ty
                    ));
                }
                return TypeInfo::Unknown;
            };
            match method {
                "len" => TypeInfo::Int,
                "isEmpty" => TypeInfo::Bool,
                "reverse" => TypeInfo::Array {
                    elem: elem.clone(),
                    size,
                },
                "first" | "last" => *elem,
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
                        checker.error(format!(
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
                    sum_ty
                }
                "min" | "max" => {
                    let elem_ty = *elem;
                    if !matches!(elem_ty, TypeInfo::Int | TypeInfo::Float | TypeInfo::Unknown) {
                        checker.error(format!(
                            "arr.{method} supports Int or Float elements, got {:?}",
                            elem_ty
                        ));
                        return TypeInfo::Unknown;
                    }
                    elem_ty
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
                        checker.error(format!(
                            "arr.sort supports Int, Float, String, or Bool elements, got {:?}",
                            elem_ty
                        ));
                        return TypeInfo::Unknown;
                    }
                    TypeInfo::Array {
                        elem: Box::new(elem_ty),
                        size,
                    }
                }
                _ => unreachable!(),
            }
        }
        "contains" | "indexOf" | "count" => {
            if args.len() != 2 {
                checker.error(format!(
                    "arr.{method} expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let arr_ty = checker.check_expr(&args[0], scopes);
            let needle_ty = checker.check_expr(&args[1], scopes);
            let TypeInfo::Array { elem, .. } = arr_ty else {
                if arr_ty != TypeInfo::Unknown {
                    checker.error(format!(
                        "arr.{method} argument 1 expects Array, got {:?}",
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
                checker.error(format!(
                    "arr.{method} argument 2 expects {:?}, got {:?}",
                    elem_ty, needle_ty
                ));
            }
            match method {
                "contains" => TypeInfo::Bool,
                "indexOf" | "count" => TypeInfo::Int,
                _ => unreachable!(),
            }
        }
        "join" => {
            if args.len() != 2 {
                checker.error(format!(
                    "arr.{method} expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let arr_ty = checker.check_expr(&args[0], scopes);
            let sep_ty = checker.check_expr(&args[1], scopes);
            if sep_ty != TypeInfo::String && sep_ty != TypeInfo::Unknown {
                checker.error(format!(
                    "arr.{method} argument 2 expects String, got {:?}",
                    sep_ty
                ));
            }
            let TypeInfo::Array { elem, .. } = arr_ty else {
                if arr_ty != TypeInfo::Unknown {
                    checker.error(format!(
                        "arr.{method} argument 1 expects Array, got {:?}",
                        arr_ty
                    ));
                }
                return TypeInfo::Unknown;
            };
            if *elem != TypeInfo::String && *elem != TypeInfo::Unknown {
                checker.error(format!(
                    "arr.{method} argument 1 expects Array[String], got {:?}",
                    TypeInfo::Array { elem, size: 0 }
                ));
                return TypeInfo::Unknown;
            }
            TypeInfo::String
        }
        "slice" => {
            if args.len() != 3 {
                checker.error(format!(
                    "arr.{method} expects 3 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let arr_ty = checker.check_expr(&args[0], scopes);
            let start_ty = checker.check_expr(&args[1], scopes);
            let end_ty = checker.check_expr(&args[2], scopes);
            if start_ty != TypeInfo::Int && start_ty != TypeInfo::Unknown {
                checker.error(format!(
                    "arr.{method} argument 2 expects Int, got {:?}",
                    start_ty
                ));
            }
            if end_ty != TypeInfo::Int && end_ty != TypeInfo::Unknown {
                checker.error(format!(
                    "arr.{method} argument 3 expects Int, got {:?}",
                    end_ty
                ));
            }
            let TypeInfo::Array { elem, size } = arr_ty else {
                if arr_ty != TypeInfo::Unknown {
                    checker.error(format!(
                        "arr.{method} argument 1 expects Array, got {:?}",
                        arr_ty
                    ));
                }
                return TypeInfo::Unknown;
            };
            let Some(start) = Checker::const_non_negative_int(&args[1]) else {
                checker.error(
                    "arr.slice argument 2 must be a non-negative Int literal for static arrays"
                        .to_string(),
                );
                return TypeInfo::Unknown;
            };
            let Some(end) = Checker::const_non_negative_int(&args[2]) else {
                checker.error(
                    "arr.slice argument 3 must be a non-negative Int literal for static arrays"
                        .to_string(),
                );
                return TypeInfo::Unknown;
            };
            if start > end || end > size {
                checker.error(format!(
                    "arr.slice bounds out of range at compile time: start={start}, end={end}, len={size}"
                ));
                return TypeInfo::Unknown;
            }
            TypeInfo::Array {
                elem,
                size: end - start,
            }
        }
        _ => {
            checker.error(format!("Unsupported array builtin `arr.{method}`"));
            TypeInfo::Unknown
        }
    }
}
