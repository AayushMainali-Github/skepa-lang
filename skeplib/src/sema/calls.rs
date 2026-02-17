use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::BuiltinSig;
use crate::types::TypeInfo;

use super::Checker;
mod arr;
mod datetime;
mod io;
mod str_pkg;

impl Checker {
    fn expr_to_parts(expr: &Expr) -> Option<Vec<String>> {
        match expr {
            Expr::Ident(name) => Some(vec![name.clone()]),
            Expr::Path(parts) => Some(parts.clone()),
            Expr::Field { base, field } => {
                let mut parts = Self::expr_to_parts(base)?;
                parts.push(field.clone());
                Some(parts)
            }
            _ => None,
        }
    }

    pub(super) fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        if let Some(parts) = Self::expr_to_parts(callee)
            && parts.len() == 2
            && (parts[0] == "io"
                || parts[0] == "str"
                || parts[0] == "arr"
                || parts[0] == "datetime")
        {
            return self.check_builtin_call(&parts[0], &parts[1], args, scopes);
        }

        if let Expr::Field { base, field } = callee {
            return self.check_method_call(base, field, args, scopes);
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

    fn check_method_call(
        &mut self,
        base: &Expr,
        method: &str,
        args: &[Expr],
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        let recv_ty = self.check_expr(base, scopes);
        let TypeInfo::Named(struct_name) = recv_ty else {
            if recv_ty != TypeInfo::Unknown {
                self.error(format!(
                    "Method call requires struct receiver, got {:?}",
                    recv_ty
                ));
            }
            for arg in args {
                self.check_expr(arg, scopes);
            }
            return TypeInfo::Unknown;
        };

        let Some(sig) = self.method_sig(&struct_name, method) else {
            self.error(format!(
                "Unknown method `{}` on struct `{}`",
                method, struct_name
            ));
            for arg in args {
                self.check_expr(arg, scopes);
            }
            return TypeInfo::Unknown;
        };

        let mut expected_params = sig.params.clone();
        if let Some(TypeInfo::Named(self_ty)) = expected_params.first()
            && self_ty == &struct_name
        {
            expected_params.remove(0);
        }

        if expected_params.len() != args.len() {
            self.error(format!(
                "Arity mismatch for method `{}.{}`: expected {}, got {}",
                struct_name,
                method,
                expected_params.len(),
                args.len()
            ));
            for arg in args {
                self.check_expr(arg, scopes);
            }
            return sig.ret;
        }

        for (i, arg) in args.iter().enumerate() {
            let got = self.check_expr(arg, scopes);
            let expected = expected_params[i].clone();
            if got != TypeInfo::Unknown && got != expected {
                self.error(format!(
                    "Argument {} for method `{}.{}`: expected {:?}, got {:?}",
                    i + 1,
                    struct_name,
                    method,
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
            "io" => return io::check_io_builtin(self, method, args, scopes, sig),
            "str" => return str_pkg::check_str_builtin(self, method, args, scopes, sig),
            "arr" => return arr::check_arr_builtin(self, method, args, scopes),
            "datetime" => {
                return datetime::check_datetime_builtin(self, method, args, scopes, sig);
            }
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
