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
    if matches!(
        method,
        "call0Int"
            | "call1Int"
            | "call1IntVoid"
            | "call1StringInt"
            | "call1StringVoid"
            | "call2StringInt"
            | "call2StringIntInt"
            | "call1BytesInt"
    ) {
        checker.error(format!(
            "`ffi.{method}` is a low-level internal helper; use `extern(\"...\") fn ...;` declarations instead"
        ));
        return TypeInfo::Unknown;
    }
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
        "call0Int" => {
            if args.len() != 1 {
                checker.error(format!(
                    "ffi.call0Int expects 1 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let got = checker.check_expr(&args[0], scopes);
            let expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if got != TypeInfo::Unknown && got != expected {
                checker.error(format!(
                    "ffi.call0Int argument 1 expects {:?}, got {:?}",
                    expected, got
                ));
            }
            TypeInfo::Int
        }
        "call1Int" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.call1Int expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call1Int argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let value_ty = checker.check_expr(&args[1], scopes);
            if value_ty != TypeInfo::Unknown && value_ty != TypeInfo::Int {
                checker.error(format!(
                    "ffi.call1Int argument 2 expects {:?}, got {:?}",
                    TypeInfo::Int,
                    value_ty
                ));
            }
            TypeInfo::Int
        }
        "call1IntVoid" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.call1IntVoid expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call1IntVoid argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let value_ty = checker.check_expr(&args[1], scopes);
            if value_ty != TypeInfo::Unknown && value_ty != TypeInfo::Int {
                checker.error(format!(
                    "ffi.call1IntVoid argument 2 expects {:?}, got {:?}",
                    TypeInfo::Int,
                    value_ty
                ));
            }
            TypeInfo::Void
        }
        "call1StringInt" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.call1StringInt expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call1StringInt argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let value_ty = checker.check_expr(&args[1], scopes);
            if value_ty != TypeInfo::Unknown && value_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.call1StringInt argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    value_ty
                ));
            }
            TypeInfo::Int
        }
        "call1StringVoid" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.call1StringVoid expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call1StringVoid argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let value_ty = checker.check_expr(&args[1], scopes);
            if value_ty != TypeInfo::Unknown && value_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.call1StringVoid argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    value_ty
                ));
            }
            TypeInfo::Void
        }
        "call1BytesInt" => {
            if args.len() != 2 {
                checker.error(format!(
                    "ffi.call1BytesInt expects 2 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call1BytesInt argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let value_ty = checker.check_expr(&args[1], scopes);
            if value_ty != TypeInfo::Unknown && value_ty != TypeInfo::Bytes {
                checker.error(format!(
                    "ffi.call1BytesInt argument 2 expects {:?}, got {:?}",
                    TypeInfo::Bytes,
                    value_ty
                ));
            }
            TypeInfo::Int
        }
        "call2StringInt" => {
            if args.len() != 3 {
                checker.error(format!(
                    "ffi.call2StringInt expects 3 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call2StringInt argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let left_ty = checker.check_expr(&args[1], scopes);
            if left_ty != TypeInfo::Unknown && left_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.call2StringInt argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    left_ty
                ));
            }
            let right_ty = checker.check_expr(&args[2], scopes);
            if right_ty != TypeInfo::Unknown && right_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.call2StringInt argument 3 expects {:?}, got {:?}",
                    TypeInfo::String,
                    right_ty
                ));
            }
            TypeInfo::Int
        }
        "call2StringIntInt" => {
            if args.len() != 3 {
                checker.error(format!(
                    "ffi.call2StringIntInt expects 3 argument(s), got {}",
                    args.len()
                ));
                return TypeInfo::Unknown;
            }
            let symbol_ty = checker.check_expr(&args[0], scopes);
            let symbol_expected = TypeInfo::Opaque("ffi.Symbol".to_string());
            if symbol_ty != TypeInfo::Unknown && symbol_ty != symbol_expected {
                checker.error(format!(
                    "ffi.call2StringIntInt argument 1 expects {:?}, got {:?}",
                    symbol_expected, symbol_ty
                ));
            }
            let left_ty = checker.check_expr(&args[1], scopes);
            if left_ty != TypeInfo::Unknown && left_ty != TypeInfo::String {
                checker.error(format!(
                    "ffi.call2StringIntInt argument 2 expects {:?}, got {:?}",
                    TypeInfo::String,
                    left_ty
                ));
            }
            let right_ty = checker.check_expr(&args[2], scopes);
            if right_ty != TypeInfo::Unknown && right_ty != TypeInfo::Int {
                checker.error(format!(
                    "ffi.call2StringIntInt argument 3 expects {:?}, got {:?}",
                    TypeInfo::Int,
                    right_ty
                ));
            }
            TypeInfo::Int
        }
        "open" => {
            checker.check_fixed_arity_builtin("ffi", method, args, scopes, sig);
            TypeInfo::Opaque("ffi.Library".to_string())
        }
        _ => TypeInfo::Unknown,
    }
}
