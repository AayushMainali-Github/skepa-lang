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
        "exists" => {
            checker.check_fixed_arity_builtin("fs", method, args, scopes, sig);
            TypeInfo::Result {
                ok: Box::new(TypeInfo::Bool),
                err: Box::new(TypeInfo::String),
            }
        }
        "readText" => {
            checker.check_fixed_arity_builtin("fs", method, args, scopes, sig);
            TypeInfo::Result {
                ok: Box::new(TypeInfo::String),
                err: Box::new(TypeInfo::String),
            }
        }
        "writeText" | "appendText" | "mkdirAll" | "removeFile" | "removeDirAll" => {
            checker.check_fixed_arity_builtin("fs", method, args, scopes, sig);
            TypeInfo::Result {
                ok: Box::new(TypeInfo::Void),
                err: Box::new(TypeInfo::String),
            }
        }
        _ => checker.check_fixed_arity_builtin("fs", method, args, scopes, sig),
    }
}
