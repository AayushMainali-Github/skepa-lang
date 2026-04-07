use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_os_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    if method == "envGet" {
        if args.len() != 1 {
            checker.error(format!(
                "os.envGet expects 1 argument(s), got {}",
                args.len()
            ));
            return TypeInfo::Unknown;
        }
        let got = checker.check_expr(&args[0], scopes);
        if got != TypeInfo::String && got != TypeInfo::Unknown {
            checker.error(format!("os.envGet argument 1 expects String"));
        }
        return TypeInfo::Option {
            value: Box::new(TypeInfo::String),
        };
    }

    if matches!(method, "exec" | "execOut") {
        if args.len() != 2 {
            checker.error(format!(
                "os.{method} expects 2 argument(s), got {}",
                args.len()
            ));
            return TypeInfo::Unknown;
        }

        let program_ty = checker.check_expr(&args[0], scopes);
        if program_ty != TypeInfo::String {
            checker.error(format!("os.{method} argument 1 expects String"));
        }

        let argv_ty = checker.check_expr(&args[1], scopes);
        let expected_argv_ty = TypeInfo::Vec {
            elem: Box::new(TypeInfo::String),
        };
        if argv_ty != expected_argv_ty {
            checker.error(format!("os.{method} argument 2 expects Vec[String]"));
        }

        return match method {
            "exec" => TypeInfo::Result {
                ok: Box::new(TypeInfo::Int),
                err: Box::new(TypeInfo::String),
            },
            "execOut" => TypeInfo::Result {
                ok: Box::new(TypeInfo::String),
                err: Box::new(TypeInfo::String),
            },
            _ => sig.ret.clone(),
        };
    }

    checker.check_fixed_arity_builtin("os", method, args, scopes, sig)
}
