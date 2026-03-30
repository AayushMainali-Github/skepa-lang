use std::collections::HashMap;

use crate::ast::Expr;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_map_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
) -> TypeInfo {
    match method {
        "new" => {
            if !args.is_empty() {
                checker.error(format!("map.new expects 0 argument(s), got {}", args.len()));
            }
            TypeInfo::Unknown
        }
        "len" => {
            if args.len() != 1 {
                checker.error(format!("map.len expects 1 argument(s), got {}", args.len()));
                return TypeInfo::Unknown;
            }
            match checker.check_expr(&args[0], scopes) {
                TypeInfo::Map { .. } | TypeInfo::Unknown => {}
                got => checker.error(format!("map.len argument 1 expects Map, got {:?}", got)),
            }
            TypeInfo::Int
        }
        "has" => {
            if args.len() != 2 {
                checker.error(format!("map.has expects 2 argument(s), got {}", args.len()));
                return TypeInfo::Unknown;
            }
            let map_ty = checker.check_expr(&args[0], scopes);
            let key_ty = checker.check_expr(&args[1], scopes);
            if key_ty != TypeInfo::Unknown && key_ty != TypeInfo::String {
                checker.error(format!(
                    "map.has argument 2 expects String, got {:?}",
                    key_ty
                ));
            }
            match map_ty {
                TypeInfo::Map { .. } | TypeInfo::Unknown => {}
                got => checker.error(format!("map.has argument 1 expects Map, got {:?}", got)),
            }
            TypeInfo::Bool
        }
        "get" => {
            if args.len() != 2 {
                checker.error(format!("map.get expects 2 argument(s), got {}", args.len()));
                return TypeInfo::Unknown;
            }
            let map_ty = checker.check_expr(&args[0], scopes);
            let key_ty = checker.check_expr(&args[1], scopes);
            if key_ty != TypeInfo::Unknown && key_ty != TypeInfo::String {
                checker.error(format!(
                    "map.get argument 2 expects String, got {:?}",
                    key_ty
                ));
            }
            match map_ty {
                TypeInfo::Map { value } => *value,
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("map.get argument 1 expects Map, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        "insert" => {
            if args.len() != 3 {
                checker.error(format!(
                    "map.insert expects 3 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let map_ty = checker.check_expr(&args[0], scopes);
            let key_ty = checker.check_expr(&args[1], scopes);
            let value_ty = checker.check_expr(&args[2], scopes);
            if key_ty != TypeInfo::Unknown && key_ty != TypeInfo::String {
                checker.error(format!(
                    "map.insert argument 2 expects String, got {:?}",
                    key_ty
                ));
            }
            match map_ty {
                TypeInfo::Map { value } => {
                    let expected = *value;
                    if value_ty != TypeInfo::Unknown && value_ty != expected {
                        checker.error(format!(
                            "map.insert argument 3 expects {:?}, got {:?}",
                            expected, value_ty
                        ));
                    }
                }
                TypeInfo::Unknown => {}
                got => checker.error(format!("map.insert argument 1 expects Map, got {:?}", got)),
            }
            TypeInfo::Void
        }
        "remove" => {
            if args.len() != 2 {
                checker.error(format!(
                    "map.remove expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let map_ty = checker.check_expr(&args[0], scopes);
            let key_ty = checker.check_expr(&args[1], scopes);
            if key_ty != TypeInfo::Unknown && key_ty != TypeInfo::String {
                checker.error(format!(
                    "map.remove argument 2 expects String, got {:?}",
                    key_ty
                ));
            }
            match map_ty {
                TypeInfo::Map { value } => *value,
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("map.remove argument 1 expects Map, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        _ => {
            checker.error(format!("Unknown builtin `map.{method}`"));
            TypeInfo::Unknown
        }
    }
}
