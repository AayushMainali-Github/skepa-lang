use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::{TypeInfo, task_channel_type, task_channel_value_type, task_task_value_type};

use super::Checker;

pub(super) fn check_task_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    match method {
        "channel" => {
            checker.check_fixed_arity_builtin("task", method, args, scopes, sig);
            TypeInfo::Unknown
        }
        "send" => {
            if args.len() != 2 {
                checker.error(format!(
                    "task.send expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let channel_ty = checker.check_expr(&args[0], scopes);
            let value_ty = checker.check_expr(&args[1], scopes);
            match channel_ty {
                TypeInfo::Opaque(name) => match task_channel_value_type(&name) {
                    Some(expected) => {
                        if value_ty != TypeInfo::Unknown && value_ty != expected {
                            checker.error(format!(
                                "task.send argument 2 expects {:?}, got {:?}",
                                expected, value_ty
                            ));
                        }
                    }
                    None => checker.error(format!(
                        "task.send argument 1 expects {:?}, got {:?}",
                        task_channel_type(&TypeInfo::Unknown),
                        TypeInfo::Opaque(name)
                    )),
                },
                TypeInfo::Unknown => {}
                got => checker.error(format!(
                    "task.send argument 1 expects Channel, got {:?}",
                    got
                )),
            }
            TypeInfo::Void
        }
        "recv" => {
            if args.len() != 1 {
                checker.error(format!(
                    "task.recv expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            match checker.check_expr(&args[0], scopes) {
                TypeInfo::Opaque(name) => task_channel_value_type(&name).unwrap_or_else(|| {
                    checker.error(format!(
                        "task.recv argument 1 expects Channel, got {:?}",
                        TypeInfo::Opaque(name)
                    ));
                    TypeInfo::Unknown
                }),
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!(
                        "task.recv argument 1 expects Channel, got {:?}",
                        got
                    ));
                    TypeInfo::Unknown
                }
            }
        }
        "__testTask" => {
            if args.len() != 1 {
                checker.error(format!(
                    "task.__testTask expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Opaque("task.Task".to_string());
            }
            let value_ty = checker.check_expr(&args[0], scopes);
            if matches!(value_ty, TypeInfo::Unknown) {
                TypeInfo::Opaque("task.Task".to_string())
            } else {
                crate::types::task_task_type(&value_ty)
            }
        }
        "__testChannel" => TypeInfo::Opaque("task.Channel".to_string()),
        "join" => {
            if args.len() != 1 {
                checker.error(format!(
                    "task.join expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            match checker.check_expr(&args[0], scopes) {
                TypeInfo::Opaque(name) => task_task_value_type(&name).unwrap_or_else(|| {
                    checker.error(format!(
                        "task.join argument 1 expects Task, got {:?}",
                        TypeInfo::Opaque(name)
                    ));
                    TypeInfo::Unknown
                }),
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("task.join argument 1 expects Task, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        _ => {
            checker.check_fixed_arity_builtin("task", method, args, scopes, sig);
            TypeInfo::Unknown
        }
    }
}
