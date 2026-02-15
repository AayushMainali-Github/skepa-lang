use std::collections::{HashMap, HashSet};

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::builtins::{BuiltinKind, find_builtin_sig};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::types::{FunctionSig, TypeInfo};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemaResult {
    pub has_errors: bool,
}

pub fn analyze_source(source: &str) -> (SemaResult, DiagnosticBag) {
    let (program, mut diags) = Parser::parse_source(source);
    let mut checker = Checker::new(&program);
    checker.check_program(&program);
    for d in checker.diagnostics.into_vec() {
        diags.push(d);
    }
    (
        SemaResult {
            has_errors: !diags.is_empty(),
        },
        diags,
    )
}

struct Checker {
    diagnostics: DiagnosticBag,
    functions: HashMap<String, FunctionSig>,
    imported_modules: HashSet<String>,
    loop_depth: usize,
}

impl Checker {
    fn const_non_negative_int(expr: &Expr) -> Option<usize> {
        match expr {
            Expr::IntLit(v) if *v >= 0 => Some(*v as usize),
            _ => None,
        }
    }

    fn parse_format_specifiers(fmt: &str) -> Result<Vec<char>, String> {
        let mut specs = Vec::new();
        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if chars[i] != '%' {
                i += 1;
                continue;
            }
            if i + 1 >= chars.len() {
                return Err("Format string ends with `%`".to_string());
            }
            let spec = chars[i + 1];
            match spec {
                '%' => {}
                'd' | 'f' | 's' | 'b' => specs.push(spec),
                other => return Err(format!("Unsupported format specifier `%{other}`")),
            }
            i += 2;
        }
        Ok(specs)
    }

    fn new(program: &Program) -> Self {
        let imported_modules = program
            .imports
            .iter()
            .map(|i| i.module.clone())
            .collect::<HashSet<_>>();
        Self {
            diagnostics: DiagnosticBag::new(),
            functions: HashMap::new(),
            imported_modules,
            loop_depth: 0,
        }
    }

    fn check_program(&mut self, program: &Program) {
        for f in &program.functions {
            let params = f
                .params
                .iter()
                .map(|p| TypeInfo::from_ast(&p.ty))
                .collect::<Vec<_>>();
            let ret = f
                .return_type
                .as_ref()
                .map(TypeInfo::from_ast)
                .unwrap_or(TypeInfo::Void);
            self.functions.insert(
                f.name.clone(),
                FunctionSig {
                    name: f.name.clone(),
                    params,
                    ret,
                },
            );
        }

        for f in &program.functions {
            self.check_function(f);
        }
    }

    fn check_function(&mut self, f: &crate::ast::FnDecl) {
        let expected_ret = f
            .return_type
            .as_ref()
            .map(TypeInfo::from_ast)
            .unwrap_or(TypeInfo::Void);
        let mut scopes = vec![HashMap::<String, TypeInfo>::new()];
        for p in &f.params {
            scopes[0].insert(p.name.clone(), TypeInfo::from_ast(&p.ty));
        }

        for stmt in &f.body {
            self.check_stmt(stmt, &mut scopes, &expected_ret);
        }
        if expected_ret != TypeInfo::Void && !Self::block_must_return(&f.body) {
            self.error(format!(
                "Function `{}` may exit without returning {:?}",
                f.name, expected_ret
            ));
        }
    }

    fn block_must_return(stmts: &[Stmt]) -> bool {
        for stmt in stmts {
            if Self::stmt_must_return(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_must_return(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return(_) => true,
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                !else_body.is_empty()
                    && Self::block_must_return(then_body)
                    && Self::block_must_return(else_body)
            }
            _ => false,
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        scopes: &mut Vec<HashMap<String, TypeInfo>>,
        expected_ret: &TypeInfo,
    ) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let expr_ty = self.check_expr(value, scopes);
                let var_ty = match ty {
                    Some(t) => {
                        let declared = TypeInfo::from_ast(t);
                        if expr_ty != TypeInfo::Unknown && declared != expr_ty {
                            self.error(format!(
                                "Type mismatch in let `{name}`: declared {:?}, got {:?}",
                                declared, expr_ty
                            ));
                        }
                        declared
                    }
                    None => expr_ty,
                };
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.clone(), var_ty);
                }
            }
            Stmt::Assign { target, value } => {
                let target_ty = self.lookup_assignment_target(target, scopes);
                let value_ty = self.check_expr(value, scopes);
                if target_ty != TypeInfo::Unknown
                    && value_ty != TypeInfo::Unknown
                    && target_ty != value_ty
                {
                    self.error(format!(
                        "Assignment type mismatch: target {:?}, value {:?}",
                        target_ty, value_ty
                    ));
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, scopes);
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_ty = self.check_expr(cond, scopes);
                if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                    self.error("if condition must be Bool".to_string());
                }

                scopes.push(HashMap::new());
                for s in then_body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();

                scopes.push(HashMap::new());
                for s in else_body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();
            }
            Stmt::While { cond, body } => {
                let cond_ty = self.check_expr(cond, scopes);
                if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                    self.error("while condition must be Bool".to_string());
                }

                self.loop_depth += 1;
                scopes.push(HashMap::new());
                for s in body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();
                self.loop_depth = self.loop_depth.saturating_sub(1);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => {
                scopes.push(HashMap::new());
                if let Some(init) = init {
                    self.check_stmt(init, scopes, expected_ret);
                }

                if let Some(cond) = cond {
                    let cond_ty = self.check_expr(cond, scopes);
                    if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                        self.error("for condition must be Bool".to_string());
                    }
                }

                self.loop_depth += 1;
                for s in body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                if let Some(step) = step {
                    self.check_stmt(step, scopes, expected_ret);
                }
                self.loop_depth = self.loop_depth.saturating_sub(1);
                scopes.pop();
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    self.error("`break` is only allowed inside a loop".to_string());
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    self.error("`continue` is only allowed inside a loop".to_string());
                }
            }
            Stmt::Return(expr_opt) => {
                let ret_ty = match expr_opt {
                    Some(expr) => self.check_expr(expr, scopes),
                    None => TypeInfo::Void,
                };
                if ret_ty != TypeInfo::Unknown && &ret_ty != expected_ret {
                    self.error(format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        expected_ret, ret_ty
                    ));
                }
            }
        }
    }

    fn lookup_assignment_target(
        &mut self,
        target: &AssignTarget,
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        match target {
            AssignTarget::Ident(name) => self.lookup_var(name, scopes),
            AssignTarget::Path(parts) => {
                if parts.len() >= 2 {
                    self.error(
                        "Path assignment semantic typing is not supported yet in v0 checker"
                            .to_string(),
                    );
                }
                TypeInfo::Unknown
            }
            AssignTarget::Index { .. } => {
                if let AssignTarget::Index { base, index } = target {
                    let base_ty = self.check_expr(base, scopes);
                    let idx_ty = self.check_expr(index, scopes);
                    if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                        self.error("Array index must be Int".to_string());
                    }
                    match base_ty {
                        TypeInfo::Array { elem, .. } => *elem,
                        TypeInfo::Unknown => TypeInfo::Unknown,
                        other => {
                            self.error(format!(
                                "Cannot index-assign into non-array type {:?}",
                                other
                            ));
                            TypeInfo::Unknown
                        }
                    }
                } else {
                    TypeInfo::Unknown
                }
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr, scopes: &mut [HashMap<String, TypeInfo>]) -> TypeInfo {
        match expr {
            Expr::IntLit(_) => TypeInfo::Int,
            Expr::FloatLit(_) => TypeInfo::Float,
            Expr::BoolLit(_) => TypeInfo::Bool,
            Expr::StringLit(_) => TypeInfo::String,
            Expr::Ident(name) => self.lookup_var(name, scopes),
            Expr::Path(parts) => {
                if parts.len() == 2 && (parts[0] == "io" || parts[0] == "str" || parts[0] == "arr")
                {
                    return TypeInfo::Unknown;
                }
                self.error(format!("Unknown path `{}`", parts.join(".")));
                TypeInfo::Unknown
            }
            Expr::Group(inner) => self.check_expr(inner, scopes),
            Expr::Unary { op, expr } => {
                let ty = self.check_expr(expr, scopes);
                match op {
                    UnaryOp::Neg => {
                        if ty == TypeInfo::Int || ty == TypeInfo::Float || ty == TypeInfo::Unknown {
                            ty
                        } else {
                            self.error("Unary `-` expects Int or Float".to_string());
                            TypeInfo::Unknown
                        }
                    }
                    UnaryOp::Pos => {
                        if ty == TypeInfo::Int || ty == TypeInfo::Float || ty == TypeInfo::Unknown {
                            ty
                        } else {
                            self.error("Unary `+` expects Int or Float".to_string());
                            TypeInfo::Unknown
                        }
                    }
                    UnaryOp::Not => {
                        if ty == TypeInfo::Bool || ty == TypeInfo::Unknown {
                            TypeInfo::Bool
                        } else {
                            self.error("Unary `!` expects Bool".to_string());
                            TypeInfo::Unknown
                        }
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let lt = self.check_expr(left, scopes);
                let rt = self.check_expr(right, scopes);
                self.check_binary(*op, lt, rt)
            }
            Expr::Call { callee, args } => self.check_call(callee, args, scopes),
            Expr::ArrayLit(items) => {
                if items.is_empty() {
                    self.error("Cannot infer type of empty array literal".to_string());
                    return TypeInfo::Unknown;
                }
                let mut elem_ty = self.check_expr(&items[0], scopes);
                for item in &items[1..] {
                    let t = self.check_expr(item, scopes);
                    if elem_ty == TypeInfo::Unknown {
                        elem_ty = t;
                        continue;
                    }
                    if t != TypeInfo::Unknown && t != elem_ty {
                        self.error(format!(
                            "Array literal element type mismatch: expected {:?}, got {:?}",
                            elem_ty, t
                        ));
                        return TypeInfo::Unknown;
                    }
                }
                TypeInfo::Array {
                    elem: Box::new(elem_ty),
                    size: items.len(),
                }
            }
            Expr::ArrayRepeat { value, size } => {
                let elem_ty = self.check_expr(value, scopes);
                TypeInfo::Array {
                    elem: Box::new(elem_ty),
                    size: *size,
                }
            }
            Expr::Index { base, index } => {
                let base_ty = self.check_expr(base, scopes);
                let idx_ty = self.check_expr(index, scopes);
                if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                    self.error("Array index must be Int".to_string());
                }
                match base_ty {
                    TypeInfo::Array { elem, .. } => *elem,
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.error(format!("Cannot index into non-array type {:?}", other));
                        TypeInfo::Unknown
                    }
                }
            }
        }
    }

    fn check_binary(&mut self, op: BinaryOp, lt: TypeInfo, rt: TypeInfo) -> TypeInfo {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div => {
                if lt == TypeInfo::Int && rt == TypeInfo::Int {
                    TypeInfo::Int
                } else if lt == TypeInfo::Float && rt == TypeInfo::Float {
                    TypeInfo::Float
                } else if op == Add && lt == TypeInfo::String && rt == TypeInfo::String {
                    TypeInfo::String
                } else if op == Add {
                    match (&lt, &rt) {
                        (
                            TypeInfo::Array {
                                elem: l_elem,
                                size: l_size,
                            },
                            TypeInfo::Array {
                                elem: r_elem,
                                size: r_size,
                            },
                        ) if l_elem == r_elem => TypeInfo::Array {
                            elem: l_elem.clone(),
                            size: l_size + r_size,
                        },
                        _ => {
                            if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                                TypeInfo::Unknown
                            } else {
                                self.error(format!(
                                    "Invalid operands for {:?}: left {:?}, right {:?}",
                                    op, lt, rt
                                ));
                                TypeInfo::Unknown
                            }
                        }
                    }
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid operands for {:?}: left {:?}, right {:?}",
                        op, lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            Mod => {
                if lt == TypeInfo::Int && rt == TypeInfo::Int {
                    TypeInfo::Int
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid operands for {:?}: left {:?}, right {:?}",
                        op, lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            EqEq | Neq => {
                if lt == rt || lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Bool
                } else {
                    self.error(format!(
                        "Invalid equality operands: left {:?}, right {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            Lt | Lte | Gt | Gte => {
                if (lt == TypeInfo::Int && rt == TypeInfo::Int)
                    || (lt == TypeInfo::Float && rt == TypeInfo::Float)
                {
                    TypeInfo::Bool
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Invalid comparison operands: left {:?}, right {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
            AndAnd | OrOr => {
                if lt == TypeInfo::Bool && rt == TypeInfo::Bool {
                    TypeInfo::Bool
                } else if lt == TypeInfo::Unknown || rt == TypeInfo::Unknown {
                    TypeInfo::Unknown
                } else {
                    self.error(format!(
                        "Logical operators require Bool operands, got {:?} and {:?}",
                        lt, rt
                    ));
                    TypeInfo::Unknown
                }
            }
        }
    }

    fn check_call(
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

        let Some(sig) = find_builtin_sig(package, method) else {
            self.error(format!("Unknown builtin `{package}.{method}`"));
            return TypeInfo::Unknown;
        };

        match sig.kind {
            BuiltinKind::FixedArity => {
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
            }
            BuiltinKind::FormatVariadic => {
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
            }
            BuiltinKind::ArrayOps => match method {
                "len" | "isEmpty" | "sum" | "first" | "last" | "reverse" | "min" | "max"
                | "sort" => {
                    if args.len() != 1 {
                        self.error(format!(
                            "{package}.{method} expects 1 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let TypeInfo::Array { elem, size } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    match method {
                        "len" => return TypeInfo::Int,
                        "isEmpty" => return TypeInfo::Bool,
                        "reverse" => {
                            return TypeInfo::Array {
                                elem: elem.clone(),
                                size,
                            };
                        }
                        "first" | "last" => return *elem,
                        "sum" => {
                            let sum_ty = *elem;
                            if !matches!(
                                sum_ty,
                                TypeInfo::Int
                                    | TypeInfo::Float
                                    | TypeInfo::String
                                    | TypeInfo::Array { .. }
                                    | TypeInfo::Unknown
                            ) {
                                self.error(format!(
                                    "arr.sum supports Int, Float, String, or Array elements, got {:?}",
                                    sum_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            if let TypeInfo::Array {
                                elem: inner_elem,
                                size: inner_size,
                            } = sum_ty
                            {
                                return TypeInfo::Array {
                                    elem: inner_elem,
                                    size: inner_size.saturating_mul(size),
                                };
                            }
                            return sum_ty;
                        }
                        "min" | "max" => {
                            let elem_ty = *elem;
                            if !matches!(elem_ty, TypeInfo::Int | TypeInfo::Float | TypeInfo::Unknown)
                            {
                                self.error(format!(
                                    "arr.{method} supports Int or Float elements, got {:?}",
                                    elem_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            return elem_ty;
                        }
                        "sort" => {
                            let elem_ty = *elem;
                            if !matches!(
                                elem_ty,
                                TypeInfo::Int
                                    | TypeInfo::Float
                                    | TypeInfo::String
                                    | TypeInfo::Bool
                                    | TypeInfo::Unknown
                            ) {
                                self.error(format!(
                                    "arr.sort supports Int, Float, String, or Bool elements, got {:?}",
                                    elem_ty
                                ));
                                return TypeInfo::Unknown;
                            }
                            return TypeInfo::Array {
                                elem: Box::new(elem_ty),
                                size,
                            };
                        }
                        _ => unreachable!(),
                    }
                }
                "contains" | "indexOf" | "count" => {
                    if args.len() != 2 {
                        self.error(format!(
                            "{package}.{method} expects 2 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let needle_ty = self.check_expr(&args[1], scopes);
                    let TypeInfo::Array { elem, .. } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    let elem_ty = *elem;
                    if needle_ty != TypeInfo::Unknown
                        && elem_ty != TypeInfo::Unknown
                        && needle_ty != elem_ty
                    {
                        self.error(format!(
                            "{package}.{method} argument 2 expects {:?}, got {:?}",
                            elem_ty, needle_ty
                        ));
                    }
                    return match method {
                        "contains" => TypeInfo::Bool,
                        "indexOf" | "count" => TypeInfo::Int,
                        _ => unreachable!(),
                    };
                }
                "join" => {
                    if args.len() != 2 {
                        self.error(format!(
                            "{package}.{method} expects 2 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let sep_ty = self.check_expr(&args[1], scopes);
                    if sep_ty != TypeInfo::String && sep_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 2 expects String, got {:?}",
                            sep_ty
                        ));
                    }
                    let TypeInfo::Array { elem, .. } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    if *elem != TypeInfo::String && *elem != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 1 expects Array[String], got {:?}",
                            TypeInfo::Array { elem, size: 0 }
                        ));
                        return TypeInfo::Unknown;
                    }
                    return TypeInfo::String;
                }
                "slice" => {
                    if args.len() != 3 {
                        self.error(format!(
                            "{package}.{method} expects 3 argument(s), got {}",
                            args.len()
                        ));
                        return TypeInfo::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let start_ty = self.check_expr(&args[1], scopes);
                    let end_ty = self.check_expr(&args[2], scopes);
                    if start_ty != TypeInfo::Int && start_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 2 expects Int, got {:?}",
                            start_ty
                        ));
                    }
                    if end_ty != TypeInfo::Int && end_ty != TypeInfo::Unknown {
                        self.error(format!(
                            "{package}.{method} argument 3 expects Int, got {:?}",
                            end_ty
                        ));
                    }
                    let TypeInfo::Array { elem, size } = arr_ty else {
                        if arr_ty != TypeInfo::Unknown {
                            self.error(format!(
                                "{package}.{method} argument 1 expects Array, got {:?}",
                                arr_ty
                            ));
                        }
                        return TypeInfo::Unknown;
                    };
                    let Some(start) = Self::const_non_negative_int(&args[1]) else {
                        self.error(
                            "arr.slice argument 2 must be a non-negative Int literal for static arrays"
                                .to_string(),
                        );
                        return TypeInfo::Unknown;
                    };
                    let Some(end) = Self::const_non_negative_int(&args[2]) else {
                        self.error(
                            "arr.slice argument 3 must be a non-negative Int literal for static arrays"
                                .to_string(),
                        );
                        return TypeInfo::Unknown;
                    };
                    if start > end || end > size {
                        self.error(format!(
                            "arr.slice bounds out of range at compile time: start={start}, end={end}, len={size}"
                        ));
                        return TypeInfo::Unknown;
                    }
                    return TypeInfo::Array {
                        elem,
                        size: end - start,
                    };
                }
                _ => {
                    self.error(format!("Unsupported array builtin `{package}.{method}`"));
                    return TypeInfo::Unknown;
                }
            },
        }

        sig.ret.clone()
    }

    fn lookup_var(&mut self, name: &str, scopes: &mut [HashMap<String, TypeInfo>]) -> TypeInfo {
        for scope in scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return t.clone();
            }
        }
        self.error(format!("Unknown variable `{name}`"));
        TypeInfo::Unknown
    }

    fn error(&mut self, message: String) {
        self.diagnostics.error(message, Span::default());
    }
}
