use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::{
    TypeInfo, task_channel_type, task_channel_value_type, task_task_type, task_task_value_type,
};

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
                        } else if value_ty == TypeInfo::Unknown {
                            checker.require_shareable_task_boundary(&value_ty, "task.send payload");
                        } else {
                            checker.require_shareable_task_boundary(&expected, "task.send payload");
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
        "spawn" => {
            let Some(function_expr) = args.first() else {
                checker.error(format!(
                    "task.spawn expects a function argument, got {} argument(s)",
                    args.len()
                ));
                return TypeInfo::Unknown;
            };
            let function_ty = checker.check_expr(function_expr, scopes);
            let values = args[1..]
                .iter()
                .map(|arg| checker.check_expr(arg, scopes))
                .collect::<Vec<_>>();
            match function_ty {
                TypeInfo::Fn { params, ret } => {
                    if values.len() != params.len() {
                        checker.error(format!(
                            "task.spawn function expects {} argument(s), got {}",
                            params.len(),
                            values.len()
                        ));
                        TypeInfo::Unknown
                    } else {
                        for (index, (actual, expected)) in
                            values.iter().zip(params.iter()).enumerate()
                        {
                            if !Checker::types_compatible(actual, expected) {
                                checker.error(format!(
                                    "task.spawn argument {} expects {:?}, got {:?}",
                                    index + 1,
                                    expected,
                                    actual
                                ));
                            }
                            checker.require_shareable_task_boundary(
                                actual,
                                &format!("task.spawn argument {}", index + 1),
                            );
                        }
                        checker.require_shareable_task_boundary(&ret, "task.spawn result");
                        TypeInfo::Result {
                            ok: Box::new(task_task_type(&ret)),
                            err: Box::new(TypeInfo::String),
                        }
                    }
                }
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!(
                        "task.spawn argument 1 expects Fn(...) -> T, got {:?}",
                        got
                    ));
                    TypeInfo::Unknown
                }
            }
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
                TypeInfo::Opaque(name) => match task_channel_value_type(&name) {
                    Some(value) => {
                        checker.require_shareable_task_boundary(&value, "task.recv payload");
                        value
                    }
                    None => {
                        checker.error(format!(
                            "task.recv argument 1 expects Channel with a known payload type, got {:?}",
                            TypeInfo::Opaque(name)
                        ));
                        TypeInfo::Unknown
                    }
                },
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
                TypeInfo::Opaque(name) => match task_task_value_type(&name) {
                    Some(value) => {
                        checker.require_shareable_task_boundary(&value, "task.join result");
                        value
                    }
                    None => {
                        checker.error(format!(
                            "task.join argument 1 expects Task with a known result type, got {:?}",
                            TypeInfo::Opaque(name)
                        ));
                        TypeInfo::Unknown
                    }
                },
                TypeInfo::Unknown => TypeInfo::Unknown,
                got => {
                    checker.error(format!("task.join argument 1 expects Task, got {:?}", got));
                    TypeInfo::Unknown
                }
            }
        }
        "close" => {
            if args.len() != 1 {
                checker.error(format!(
                    "task.close expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Void;
            }
            match checker.check_expr(&args[0], scopes) {
                TypeInfo::Opaque(name)
                    if task_task_value_type(&name).is_some()
                        || task_channel_value_type(&name).is_some() => {}
                TypeInfo::Opaque(name) => checker.error(format!(
                    "task.close argument 1 expects Task or Channel, got {:?}",
                    TypeInfo::Opaque(name)
                )),
                TypeInfo::Unknown => {}
                got => checker.error(format!(
                    "task.close argument 1 expects Task or Channel, got {:?}",
                    got
                )),
            }
            TypeInfo::Void
        }
        _ => {
            checker.check_fixed_arity_builtin("task", method, args, scopes, sig);
            TypeInfo::Unknown
        }
    }
}
