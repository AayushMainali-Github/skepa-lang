use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, Program, Stmt};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::types::{FunctionSig, TypeInfo};

mod calls;
mod expr;
mod stmt;

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
