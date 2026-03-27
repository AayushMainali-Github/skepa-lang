use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_net_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    checker.check_fixed_arity_builtin("net", method, args, scopes, sig);
    match method {
        "__testSocket" => TypeInfo::Opaque("net.Socket".to_string()),
        _ => TypeInfo::Unknown,
    }
}
