use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_bytes_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    let ty = checker.check_fixed_arity_builtin("bytes", method, args, scopes, sig);
    if method == "get" {
        TypeInfo::Option {
            value: Box::new(TypeInfo::Int),
        }
    } else {
        ty
    }
}
