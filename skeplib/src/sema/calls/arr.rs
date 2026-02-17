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
        "len" | "isEmpty" | "first" | "last" => {
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
                "first" | "last" => *elem,
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
        _ => {
            checker.error(format!("Unsupported array builtin `arr.{method}`"));
            TypeInfo::Unknown
        }
    }
}
