use std::collections::{HashMap, HashSet};

use crate::ast::{Program, Stmt, TypeName};
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
    methods: HashMap<String, HashMap<String, FunctionSig>>,
    imported_modules: HashSet<String>,
    struct_names: HashSet<String>,
    struct_fields: HashMap<String, HashMap<String, TypeInfo>>,
    loop_depth: usize,
}

impl Checker {
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
            methods: HashMap::new(),
            imported_modules,
            struct_names: HashSet::new(),
            struct_fields: HashMap::new(),
            loop_depth: 0,
        }
    }

    fn check_program(&mut self, program: &Program) {
        self.check_struct_declarations(program);
        self.check_impl_declarations(program);
        self.collect_method_signatures(program);

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

        for imp in &program.impls {
            for method in &imp.methods {
                self.check_method(imp.target.as_str(), method);
            }
        }
    }

    fn check_struct_declarations(&mut self, program: &Program) {
        for s in &program.structs {
            if !self.struct_names.insert(s.name.clone()) {
                self.error(format!("Duplicate struct declaration `{}`", s.name));
            }
        }

        for s in &program.structs {
            let mut seen_fields = HashSet::new();
            let mut field_types = HashMap::new();
            for field in &s.fields {
                if !seen_fields.insert(field.name.clone()) {
                    self.error(format!(
                        "Duplicate field `{}` in struct `{}`",
                        field.name, s.name
                    ));
                }
                self.check_decl_type_exists(
                    &field.ty,
                    format!("Unknown type in struct `{}` field `{}`", s.name, field.name),
                );
                field_types.insert(field.name.clone(), TypeInfo::from_ast(&field.ty));
            }
            self.struct_fields.insert(s.name.clone(), field_types);
        }
    }

    fn check_impl_declarations(&mut self, program: &Program) {
        let mut global_seen_methods: HashMap<String, HashSet<String>> = HashMap::new();
        for imp in &program.impls {
            if !self.struct_names.contains(&imp.target) {
                self.error(format!("Unknown impl target struct `{}`", imp.target));
            }

            let seen_methods = global_seen_methods.entry(imp.target.clone()).or_default();
            for method in &imp.methods {
                if !seen_methods.insert(method.name.clone()) {
                    self.error(format!(
                        "Duplicate method `{}` in impl `{}`",
                        method.name, imp.target
                    ));
                }

                if method.params.is_empty() {
                    self.error(format!(
                        "Method `{}.{}` must declare `self` as first parameter",
                        imp.target, method.name
                    ));
                } else {
                    let first = &method.params[0];
                    let expected_self_ty = TypeInfo::Named(imp.target.clone());
                    let actual_self_ty = TypeInfo::from_ast(&first.ty);
                    if first.name != "self" || actual_self_ty != expected_self_ty {
                        self.error(format!(
                            "Method `{}.{}` must declare `self: {}` as first parameter",
                            imp.target, method.name, imp.target
                        ));
                    }
                }

                for param in &method.params {
                    self.check_decl_type_exists(
                        &param.ty,
                        format!(
                            "Unknown type in method `{}` parameter `{}`",
                            method.name, param.name
                        ),
                    );
                }
                if let Some(ret) = &method.return_type {
                    self.check_decl_type_exists(
                        ret,
                        format!("Unknown return type in method `{}`", method.name),
                    );
                }
            }
        }
    }

    fn collect_method_signatures(&mut self, program: &Program) {
        for imp in &program.impls {
            let methods = self.methods.entry(imp.target.clone()).or_default();
            for method in &imp.methods {
                let params = method
                    .params
                    .iter()
                    .map(|p| TypeInfo::from_ast(&p.ty))
                    .collect::<Vec<_>>();
                let ret = method
                    .return_type
                    .as_ref()
                    .map(TypeInfo::from_ast)
                    .unwrap_or(TypeInfo::Void);
                methods.entry(method.name.clone()).or_insert(FunctionSig {
                    name: method.name.clone(),
                    params,
                    ret,
                });
            }
        }
    }

    fn check_decl_type_exists(&mut self, ty: &TypeName, err_prefix: String) {
        match ty {
            TypeName::Int | TypeName::Float | TypeName::Bool | TypeName::String | TypeName::Void => {}
            TypeName::Array { elem, .. } => self.check_decl_type_exists(elem, err_prefix),
            TypeName::Named(name) => {
                if !self.struct_names.contains(name) {
                    self.error(format!("{err_prefix}: `{name}`"));
                }
            }
        }
    }

    pub(super) fn field_type(&self, struct_name: &str, field: &str) -> Option<TypeInfo> {
        self.struct_fields
            .get(struct_name)
            .and_then(|f| f.get(field))
            .cloned()
    }

    pub(super) fn method_sig(&self, struct_name: &str, method: &str) -> Option<FunctionSig> {
        self.methods
            .get(struct_name)
            .and_then(|m| m.get(method))
            .cloned()
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

    fn check_method(&mut self, target: &str, m: &crate::ast::MethodDecl) {
        let expected_ret = m
            .return_type
            .as_ref()
            .map(TypeInfo::from_ast)
            .unwrap_or(TypeInfo::Void);
        let mut scopes = vec![HashMap::<String, TypeInfo>::new()];
        for p in &m.params {
            scopes[0].insert(p.name.clone(), TypeInfo::from_ast(&p.ty));
        }
        if !scopes[0].contains_key("self") {
            scopes[0].insert("self".to_string(), TypeInfo::Named(target.to_string()));
        }

        for stmt in &m.body {
            self.check_stmt(stmt, &mut scopes, &expected_ret);
        }
        if expected_ret != TypeInfo::Void && !Self::block_must_return(&m.body) {
            self.error(format!(
                "Method `{}.{}` may exit without returning {:?}",
                target, m.name, expected_ret
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
