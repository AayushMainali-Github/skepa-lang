use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::{BuiltinKind, BuiltinSig};
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_datetime_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    match sig.kind {
        BuiltinKind::FixedArity => {
            checker.check_fixed_arity_builtin("datetime", method, args, scopes, sig);
            if method == "parseUnix" {
                TypeInfo::Result {
                    ok: Box::new(TypeInfo::Int),
                    err: Box::new(TypeInfo::String),
                }
            } else {
                sig.ret.clone()
            }
        }
        BuiltinKind::FormatVariadic | BuiltinKind::ArrayOps => sig.ret.clone(),
    }
}
