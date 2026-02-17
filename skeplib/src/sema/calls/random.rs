use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::{BuiltinKind, BuiltinSig};
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_random_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    match sig.kind {
        BuiltinKind::FixedArity => {
            checker.check_fixed_arity_builtin("random", method, args, scopes, sig)
        }
        BuiltinKind::FormatVariadic | BuiltinKind::ArrayOps => sig.ret.clone(),
    }
}

