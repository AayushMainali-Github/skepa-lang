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
    _sig: &BuiltinSig,
) -> TypeInfo {
    if matches!(
        method,
        "call"
            | "call0Int"
            | "call0Void"
            | "call0Bool"
            | "call1Int"
            | "call1IntBool"
            | "call1IntVoid"
            | "call1StringInt"
            | "call1StringVoid"
            | "call2StringInt"
            | "call2StringIntInt"
            | "call1BytesInt"
            | "call2IntInt"
            | "call2BytesIntInt"
    ) {
        checker.error(format!(
            "`ffi.{method}` is a low-level internal helper; use `extern(\"...\") fn ...;` declarations instead"
        ));
        return TypeInfo::Unknown;
    }
    match method {
        "open" => {
            if args.len() != 1 {
                checker.error(format!(
                    "ffi.open expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let path_ty = checker.check_expr(&args[0], scopes);
            if path_ty != TypeInfo::Unknown && path_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.open argument 1 expects {:?}, got {:?}",
                    TypeInfo::String,
                    path_ty
                ));
            }
            TypeInfo::Result {
                ok: Box::new(TypeInfo::Opaque("ffi.Library".to_string())),
                err: Box::new(TypeInfo::String),
            }
        }
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
            TypeInfo::Result {
                ok: Box::new(TypeInfo::Opaque("ffi.Symbol".to_string())),
                err: Box::new(TypeInfo::String),
            }
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
        _ => TypeInfo::Unknown,
    }
}
