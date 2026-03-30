use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;

pub(super) fn check_ffi_builtin(
    checker: &mut Checker,
    method: &str,
    args: &[Expr],
    scopes: &mut [HashMap<String, TypeInfo>],
    sig: &BuiltinSig,
) -> TypeInfo {
    match method {
        "bind" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.bind expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let library_ty = checker.check_expr(&args[0], scopes);
            let expected_library = TypeInfo::Opaque("ffi.Library".to_string());
            if library_ty != TypeInfo::Unknown && library_ty != expected_library {
                checker.error(format!(
                    "ffi.bind argument 1 expects {:?}, got {:?}",
                    expected_library, library_ty
                ));
            }
            let name_ty = checker.check_expr(&args[1], scopes);
            if name_ty != TypeInfo::Unknown && name_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.bind argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    name_ty
                ));
            }
            TypeInfo::Opaque("ffi.Symbol".to_string())
        }
        "closeLibrary" => {
            if args.len() != 1 {
                checker.error(format!(
                    "ffi.closeLibrary expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("ffi.Library".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "ffi.closeLibrary argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Void
        }
        "closeSymbol" => {
            if args.len() != 1 {
                checker.error(format!(
                    "ffi.closeSymbol expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "ffi.closeSymbol argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Void
        }
        "open" => {
            checker.check_fixed_arity_builtin("ffi", method, args, scopes, sig);
            TypeInfo::Opaque("ffi.Library".to_string())
        }
        _ => TypeInfo::Unknown,
    }
}
