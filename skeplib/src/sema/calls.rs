use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;
mod arr;
mod io;
mod str_pkg;

impl Checker {
    pub(super) fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if let Expr::Path(parts) = callee
            && parts.len() == 2
        {
            return self.check_builtin_call(&parts[0], &parts[1], args, scopes);
        }

        let fn_name = match callee {
            Expr::Ident(name) => name.clone(),
            Expr::Path(parts) => parts.join("."),
            _ => {
                self.error("Invalid call target".to_string());
                return TypeInfo::Unknown;
            }
        };

        let Some(sig) = self.functions.get(&fn_name).cloned() else {
            self.error(format!("Unknown function `{fn_name}`"));
            return TypeInfo::Unknown;
        };

        if sig.params.len() != args.len() {
            self.error(format!(
                "Arity mismatch for `{}`: expected {}, got {}",
                sig.name,
                sig.params.len(),
                args.len()
            ));
            return sig.ret.clone();
        }

        for (i, arg) in args.iter().enumerate() {
            let got = self.check_expr(arg, scopes);
            let expected = sig.params[i].clone();
            if got != TypeInfo::Unknown && got != expected {
                self.error(format!(
                    "Argument {} for `{}`: expected {:?}, got {:?}",
                    i + 1,
                    sig.name,
                    expected,
                    got
                ));
            }
        }

        sig.ret
    }

    fn check_builtin_call(
        &mut self,
        package: &str,
        method: &str,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if !self.imported_modules.contains(package) {
            self.error(format!("`{package}.*` used without `import {package};`"));
            return TypeInfo::Unknown;
        }

        let Some(sig) = crate::builtins::find_builtin_sig(package, method) else {
            self.error(format!("Unknown builtin `{package}.{method}`"));
            return TypeInfo::Unknown;
        };

        match package {
            "io" => return io::check_io_builtin(self, method, args, scopes, &sig),
            "str" => return str_pkg::check_str_builtin(self, method, args, scopes, &sig),
            "arr" => return arr::check_arr_builtin(self, method, args, scopes),
            _ => {}
        }

        sig.ret.clone()
    }

    pub(super) fn check_fixed_arity_builtin(
        &mut self,
        package: &str,
        method: &str,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
        sig: &BuiltinSig,
    ) -> TypeInfo {
        if sig.params.len() != args.len() {
            self.error(format!(
                "{package}.{method} expects {} argument(s), got {}",
                sig.params.len(),
                args.len()
            ));
            return sig.ret.clone();
        }

        for (idx, arg) in args.iter().enumerate() {
            let got = self.check_expr(arg, scopes);
            let expected = sig.params[idx].clone();
            if got != TypeInfo::Unknown && got != expected {
                self.error(format!(
                    "{package}.{method} argument {} expects {:?}, got {:?}",
                    idx + 1,
                    expected,
                    got
                ));
            }
        }
        sig.ret.clone()
    }

    pub(super) fn check_format_variadic_builtin(
        &mut self,
        package: &str,
        method: &str,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
        sig: &BuiltinSig,
    ) -> TypeInfo {
        if args.is_empty() {
            self.error(format!("{package}.{method} expects at least 1 argument"));
            return sig.ret.clone();
        }
        let fmt_ty = self.check_expr(&args[0], scopes);
        if fmt_ty != TypeInfo::String && fmt_ty != TypeInfo::Unknown {
            self.error(format!(
                "{package}.{method} argument 1 expects {:?}, got {:?}",
                TypeInfo::String,
                fmt_ty
            ));
        }

        if let Expr::StringLit(fmt) = &args[0] {
            match Self::parse_format_specifiers(fmt) {
                Ok(specs) => {
                    let expected_args = specs.len();
                    let got_args = args.len().saturating_sub(1);
                    if expected_args != got_args {
                        self.error(format!(
                            "{package}.{method} format expects {} value argument(s), got {}",
                            expected_args, got_args
                        ));
                    }
                    for (idx, arg) in args.iter().skip(1).enumerate() {
                        let got = self.check_expr(arg, scopes);
                        if idx >= specs.len() {
                            continue;
                        }
                        let expected = match specs[idx] {
                            'd' => TypeInfo::Int,
                            'f' => TypeInfo::Float,
                            's' => TypeInfo::String,
                            'b' => TypeInfo::Bool,
                            _ => TypeInfo::Unknown,
                        };
                        if got != TypeInfo::Unknown && got != expected {
                            self.error(format!(
                                "{package}.{method} argument {} expects {:?} for `%{}`, got {:?}",
                                idx + 2,
                                expected,
                                specs[idx],
                                got
                            ));
                        }
                    }
                }
                Err(msg) => self.error(format!("{package}.{method} format error: {msg}")),
            }
        } else {
            for arg in args.iter().skip(1) {
                self.check_expr(arg, scopes);
            }
        }
        sig.ret.clone()
    }
}
