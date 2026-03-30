use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_task_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    checker.check_fixed_arity_builtin("task", method, args, scopes, sig);
    match method {
        "__testTask" => TypeInfo::Opaque("task.Task".to_string()),
        "__testChannel" => TypeInfo::Opaque("task.Channel".to_string()),
        _ => TypeInfo::Unknown,
    }
}
