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
    match method {
        "accept" => {
            if args.len() != 1 {
                checker.error(format!(
                    "net.accept expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("net.Listener".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "net.accept argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Opaque("net.Socket".to_string())
        }
        "__testSocket" | "listen" | "connect" => {
            checker.check_fixed_arity_builtin("net", method, args, scopes, sig);
            match method {
                "__testSocket" => TypeInfo::Opaque("net.Socket".to_string()),
                "listen" => TypeInfo::Opaque("net.Listener".to_string()),
                "connect" => TypeInfo::Opaque("net.Socket".to_string()),
                _ => unreachable!(),
            }
        }
        _ => TypeInfo::Unknown,
    }
}
