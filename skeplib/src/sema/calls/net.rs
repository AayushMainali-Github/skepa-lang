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
        "read" => {
            if args.len() != 1 {
                checker.error(format!(
                    "net.read expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("net.Socket".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "net.read argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::String
        }
        "readBytes" => {
            if args.len() != 1 {
                checker.error(format!(
                    "net.readBytes expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("net.Socket".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "net.readBytes argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Bytes
        }
        "write" => {
            if args.len() != 2 {
                checker.error(format!(
                    "net.write expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let socket_ty = checker.check_expr(&args[0], scopes);
            let socket_expected = TypeInfo::Opaque("net.Socket".to_string());
            if socket_ty != TypeInfo::Unknown && socket_ty != socket_expected {
                checker.error(format!(
                    "net.write argument 1 expects {:?}, got {:?}",
                    socket_expected, socket_ty
                ));
            }
            let data_ty = checker.check_expr(&args[1], scopes);
            if data_ty != TypeInfo::Unknown && data_ty != TypeInfo::String {
                checker.error(format!(
                    "net.write argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    data_ty
                ));
            }
            TypeInfo::Void
        }
        "writeBytes" => {
            if args.len() != 2 {
                checker.error(format!(
                    "net.writeBytes expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let socket_ty = checker.check_expr(&args[0], scopes);
            let socket_expected = TypeInfo::Opaque("net.Socket".to_string());
            if socket_ty != TypeInfo::Unknown && socket_ty != socket_expected {
                checker.error(format!(
                    "net.writeBytes argument 1 expects {:?}, got {:?}",
                    socket_expected, socket_ty
                ));
            }
            let data_ty = checker.check_expr(&args[1], scopes);
            if data_ty != TypeInfo::Unknown && data_ty != TypeInfo::Bytes {
                checker.error(format!(
                    "net.writeBytes argument 2 expects {:?}, got {:?}",
                    TypeInfo::Bytes,
                    data_ty
                ));
            }
            TypeInfo::Void
        }
        "close" => {
            if args.len() != 1 {
                checker.error(format!(
                    "net.close expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("net.Socket".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "net.close argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Void
        }
        "closeListener" => {
            if args.len() != 1 {
                checker.error(format!(
                    "net.closeListener expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("net.Listener".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "net.closeListener argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Void
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
