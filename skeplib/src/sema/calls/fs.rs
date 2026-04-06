use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_fs_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    match method {
        "readText" => {
            checker.check_fixed_arity_builtin("fs", method, args, scopes, sig);
            TypeInfo::Result {
                ok: Box::new(TypeInfo::String),
                err: Box::new(TypeInfo::String),
            }
        }
        _ => checker.check_fixed_arity_builtin("fs", method, args, scopes, sig),
    }
}
