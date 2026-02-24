use std::collections::HashMap;

use crate::ast::Expr;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_vec_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
) -> TypeInfo {
    match method {
        "new" => {
            if !args.is_empty() {
                checker.error(format!("vec.new expects 0 argument(s), got {}", args.len()));
            }
            TypeInfo::Unknown
        }
        "len" => {
            if args.len() != 1 {
                checker.error(format!("vec.len expects 1 argument(s), got {}", args.len()));
                return TypeInfo::Int;
            }
            match checker.check_expr(&args[0], scopes) {
                TypeInfo::Vec { .. } | TypeInfo::Unknown => {}
                got => checker.error(format!("vec.len argument 1 expects Vec, got {:?}", got)),
            }
            TypeInfo::Int
        }
        "push" => {
            if args.len() != 2 {
                checker.error(format!("vec.push expects 2 argument(s), got {}", args.len()));
                return TypeInfo::Void;
            }
            let vec_ty = checker.check_expr(&args[0], scopes);
            let val_ty = checker.check_expr(&args[1], scopes);
            match vec_ty {
                TypeInfo::Vec { elem } => {
                    let expected = *elem;
                    if val_ty != TypeInfo::Unknown && val_ty != expected {
                        checker.error(format!(
                            "vec.push argument 2 expects {:?}, got {:?}",
                            expected, val_ty
                        ));
                    }
                }
                TypeInfo::Unknown => {}
                got => checker.error(format!("vec.push argument 1 expects Vec, got {:?}", got)),
            }
            TypeInfo::Void
        }
        "get" => {
            if args.len() != 2 {
                checker.error(format!("vec.get expects 2 argument(s), got {}", args.len()));
                return TypeInfo::Unknown;
            }
            let vec_ty = checker.check_expr(&args[0], scopes);
            let idx_ty = checker.check_expr(&args[1], scopes);
            if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                checker.error(format!("vec.get argument 2 expects Int, got {:?}", idx_ty));
            }
            match vec_ty {
                TypeInfo::Vec { elem } => *elem,
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("vec.get argument 1 expects Vec, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        "set" => {
            if args.len() != 3 {
                checker.error(format!("vec.set expects 3 argument(s), got {}", args.len()));
                return TypeInfo::Void;
            }
            let vec_ty = checker.check_expr(&args[0], scopes);
            let idx_ty = checker.check_expr(&args[1], scopes);
            let val_ty = checker.check_expr(&args[2], scopes);
            if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                checker.error(format!("vec.set argument 2 expects Int, got {:?}", idx_ty));
            }
            match vec_ty {
                TypeInfo::Vec { elem } => {
                    let expected = *elem;
                    if val_ty != TypeInfo::Unknown && val_ty != expected {
                        checker.error(format!(
                            "vec.set argument 3 expects {:?}, got {:?}",
                            expected, val_ty
                        ));
                    }
                }
                TypeInfo::Unknown => {}
                got => checker.error(format!("vec.set argument 1 expects Vec, got {:?}", got)),
            }
            TypeInfo::Void
        }
        "delete" => {
            if args.len() != 2 {
                checker.error(format!(
                    "vec.delete expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let vec_ty = checker.check_expr(&args[0], scopes);
            let idx_ty = checker.check_expr(&args[1], scopes);
            if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                checker.error(format!(
                    "vec.delete argument 2 expects Int, got {:?}",
                    idx_ty
                ));
            }
            match vec_ty {
                TypeInfo::Vec { elem } => *elem,
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("vec.delete argument 1 expects Vec, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        _ => {
            checker.error(format!("Unknown builtin `vec.{method}`"));
            TypeInfo::Unknown
        }
    }
}

