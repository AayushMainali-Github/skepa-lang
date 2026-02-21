use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::{ImportDecl, Program, Stmt, TypeName};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::resolver::{ModuleGraph, ModuleId, ResolveError, build_export_maps, resolve_project};
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

#[derive(Debug, Clone, Default)]
struct ModuleApi {
    functions: HashMap<String, FunctionSig>,
    structs: HashMap<String, HashMap<String, TypeInfo>>,
    methods: HashMap<String, HashMap<String, FunctionSig>>,
    globals: HashMap<String, TypeInfo>,
}

#[derive(Debug, Clone, Default)]
struct ModuleExternalContext {
    imported_functions: HashMap<String, FunctionSig>,
    imported_structs: HashMap<String, HashMap<String, TypeInfo>>,
    imported_methods: HashMap<String, HashMap<String, FunctionSig>>,
    imported_globals: HashMap<String, TypeInfo>,
    direct_import_targets: HashMap<String, String>,
}

pub fn analyze_project_entry(
    entry: &Path,
) -> Result<(SemaResult, DiagnosticBag), Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    Ok(analyze_project_graph(&graph))
}

fn analyze_project_graph(graph: &ModuleGraph) -> (SemaResult, DiagnosticBag) {
    let mut programs = HashMap::<ModuleId, Program>::new();
    let mut diags = DiagnosticBag::new();
    let mut module_paths = HashMap::<ModuleId, String>::new();
    for (id, unit) in &graph.modules {
        module_paths.insert(id.clone(), unit.path.display().to_string());
        let (program, parse_diags) = Parser::parse_source(&unit.source);
        for mut d in parse_diags.into_vec() {
            d.message = format!("{}: {}", unit.path.display(), d.message);
            diags.push(d);
        }
        programs.insert(id.clone(), program);
    }

    let mut module_apis = HashMap::<ModuleId, ModuleApi>::new();
    for (id, program) in &programs {
        module_apis.insert(id.clone(), build_module_api(program));
    }
    let export_maps = match build_export_maps(graph) {
        Ok(maps) => maps,
        Err(errs) => {
            for e in errs {
                let mut msg = format!("resolver error: {}", e.message);
                if let Some(path) = e.path.as_ref() {
                    msg = format!("resolver error [{}]: {}", path.display(), e.message);
                }
                diags.error(msg, Span::default());
            }
            HashMap::new()
        }
    };

    for (id, program) in &programs {
        let ctx = build_external_context(id, program, graph, &module_apis, &export_maps);
        let mut checker = Checker::new(program);
        checker.apply_external_context(ctx);
        checker.check_program(program);
        let pfx = module_paths.get(id).cloned().unwrap_or_else(|| id.clone());
        for mut d in checker.diagnostics.into_vec() {
            d.message = format!("{pfx}: {}", d.message);
            diags.push(d);
        }
    }

    (
        SemaResult {
            has_errors: !diags.is_empty(),
        },
        diags,
    )
}

fn resolve_import_module_targets(graph: &ModuleGraph, import_path: &[String]) -> Vec<ModuleId> {
    let import_id = import_path.join(".");
    if graph.modules.contains_key(&import_id) {
        return vec![import_id];
    }
    let prefix = format!("{import_id}.");
    let mut matches = graph
        .modules
        .keys()
        .filter(|id| id.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    matches.sort();
    matches
}

fn build_module_api(program: &Program) -> ModuleApi {
    let mut api = ModuleApi::default();
    for f in &program.functions {
        api.functions.insert(
            f.name.clone(),
            FunctionSig {
                name: f.name.clone(),
                params: f.params.iter().map(|p| TypeInfo::from_ast(&p.ty)).collect(),
                ret: f
                    .return_type
                    .as_ref()
                    .map(TypeInfo::from_ast)
                    .unwrap_or(TypeInfo::Void),
            },
        );
    }
    for s in &program.structs {
        let mut fields = HashMap::new();
        for fld in &s.fields {
            fields.insert(fld.name.clone(), TypeInfo::from_ast(&fld.ty));
        }
        api.structs.insert(s.name.clone(), fields);
    }
    for i in &program.impls {
        let methods = api.methods.entry(i.target.clone()).or_default();
        for m in &i.methods {
            methods.insert(
                m.name.clone(),
                FunctionSig {
                    name: m.name.clone(),
                    params: m.params.iter().map(|p| TypeInfo::from_ast(&p.ty)).collect(),
                    ret: m
                        .return_type
                        .as_ref()
                        .map(TypeInfo::from_ast)
                        .unwrap_or(TypeInfo::Void),
                },
            );
        }
    }
    for g in &program.globals {
        api.globals.insert(
            g.name.clone(),
            g.ty.as_ref()
                .map(TypeInfo::from_ast)
                .unwrap_or(TypeInfo::Unknown),
        );
    }
    api
}

fn build_external_context(
    _module_id: &str,
    program: &Program,
    graph: &ModuleGraph,
    apis: &HashMap<ModuleId, ModuleApi>,
    export_maps: &HashMap<ModuleId, HashMap<String, crate::resolver::SymbolRef>>,
) -> ModuleExternalContext {
    let mut ctx = ModuleExternalContext::default();

    for imp in &program.imports {
        match imp {
            ImportDecl::ImportFrom {
                path,
                wildcard,
                items,
            } => {
                let targets = resolve_import_module_targets(graph, path);
                if targets.len() != 1 {
                    continue;
                }
                let target = &targets[0];
                let Some(exports) = export_maps.get(target) else {
                    continue;
                };
                if *wildcard {
                    let mut names = exports.keys().cloned().collect::<Vec<_>>();
                    names.sort();
                    for name in names {
                        let Some(sym) = exports.get(&name) else {
                            continue;
                        };
                        let Some(api) = apis.get(&sym.module_id) else {
                            continue;
                        };
                        match sym.kind {
                            crate::resolver::SymbolKind::Fn => {
                                if let Some(sig) = api.functions.get(&sym.local_name).cloned() {
                                    ctx.imported_functions.insert(name.clone(), sig);
                                    ctx.direct_import_targets.insert(
                                        name.clone(),
                                        format!("{}.{}", sym.module_id, name),
                                    );
                                }
                            }
                            crate::resolver::SymbolKind::Struct => {
                                if let Some(fields) = api.structs.get(&sym.local_name).cloned() {
                                    ctx.imported_structs.insert(name.clone(), fields);
                                }
                                if let Some(methods) = api.methods.get(&sym.local_name).cloned() {
                                    ctx.imported_methods.insert(
                                        name.clone(),
                                        rebind_methods_self_type(methods, &sym.local_name, &name),
                                    );
                                }
                            }
                            crate::resolver::SymbolKind::GlobalLet => {
                                if let Some(ty) = api.globals.get(&sym.local_name).cloned() {
                                    ctx.imported_globals.insert(name.clone(), ty);
                                }
                            }
                            crate::resolver::SymbolKind::Namespace => {}
                        }
                    }
                } else {
                    for item in items {
                        let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                        let Some(sym) = exports.get(&item.name) else {
                            continue;
                        };
                        let Some(api) = apis.get(&sym.module_id) else {
                            continue;
                        };
                        match sym.kind {
                            crate::resolver::SymbolKind::Fn => {
                                if let Some(sig) = api.functions.get(&sym.local_name).cloned() {
                                    ctx.imported_functions.insert(local.clone(), sig);
                                    ctx.direct_import_targets
                                        .insert(local, format!("{}.{}", sym.module_id, item.name));
                                }
                            }
                            crate::resolver::SymbolKind::Struct => {
                                if let Some(fields) = api.structs.get(&sym.local_name).cloned() {
                                    ctx.imported_structs.insert(local.clone(), fields);
                                }
                                if let Some(methods) = api.methods.get(&sym.local_name).cloned() {
                                    ctx.imported_methods.insert(
                                        local.clone(),
                                        rebind_methods_self_type(methods, &sym.local_name, &local),
                                    );
                                }
                            }
                            crate::resolver::SymbolKind::GlobalLet => {
                                if let Some(ty) = api.globals.get(&sym.local_name).cloned() {
                                    ctx.imported_globals.insert(local, ty);
                                }
                            }
                            crate::resolver::SymbolKind::Namespace => {}
                        }
                    }
                }
            }
            ImportDecl::ImportModule { path, .. } => {
                let targets = resolve_import_module_targets(graph, path);
                for target in targets {
                    let Some(exports) = export_maps.get(&target) else {
                        continue;
                    };
                    for (exported_name, sym) in exports {
                        let Some(api) = apis.get(&sym.module_id) else {
                            continue;
                        };
                        let q = format!("{target}.{exported_name}");
                        match sym.kind {
                            crate::resolver::SymbolKind::Fn => {
                                if let Some(sig) = api.functions.get(&sym.local_name).cloned() {
                                    ctx.imported_functions.insert(q, sig);
                                }
                            }
                            crate::resolver::SymbolKind::Struct => {
                                if let Some(fields) = api.structs.get(&sym.local_name).cloned() {
                                    ctx.imported_structs.insert(q.clone(), fields);
                                }
                                if let Some(methods) = api.methods.get(&sym.local_name).cloned() {
                                    ctx.imported_methods.insert(
                                        q.clone(),
                                        rebind_methods_self_type(methods, &sym.local_name, &q),
                                    );
                                }
                            }
                            crate::resolver::SymbolKind::GlobalLet => {
                                if let Some(ty) = api.globals.get(&sym.local_name).cloned() {
                                    ctx.imported_globals.insert(q, ty);
                                }
                            }
                            crate::resolver::SymbolKind::Namespace => {}
                        }
                    }
                }
            }
        }
    }
    ctx
}

fn rebind_methods_self_type(
    methods: HashMap<String, FunctionSig>,
    from_struct_name: &str,
    to_struct_name: &str,
) -> HashMap<String, FunctionSig> {
    let mut out = HashMap::new();
    for (name, mut sig) in methods {
        if let Some(first) = sig.params.first_mut()
            && *first == TypeInfo::Named(from_struct_name.to_string())
        {
            *first = TypeInfo::Named(to_struct_name.to_string());
        }
        out.insert(name, sig);
    }
    out
}

struct Checker {
    diagnostics: DiagnosticBag,
    functions: HashMap<String, FunctionSig>,
    methods: HashMap<String, HashMap<String, FunctionSig>>,
    imported_modules: HashSet<String>,
    direct_imports: HashMap<String, String>,
    module_namespaces: HashMap<String, Vec<String>>,
    struct_names: HashSet<String>,
    struct_fields: HashMap<String, HashMap<String, TypeInfo>>,
    globals: HashMap<String, TypeInfo>,
    loop_depth: usize,
    fn_lit_scope_floors: Vec<usize>,
}

impl Checker {
    fn resolve_named_type_name(&self, name: &str) -> Option<String> {
        if self.struct_names.contains(name) {
            return Some(name.to_string());
        }
        if !name.contains('.') {
            return None;
        }
        let mut parts = name.split('.').map(ToString::to_string).collect::<Vec<_>>();
        if parts.is_empty() {
            return None;
        }
        let root = parts.remove(0);
        if let Some(prefix) = self.module_namespaces.get(&root) {
            let mut fq = prefix.clone();
            fq.extend(parts);
            let joined = fq.join(".");
            if self.struct_names.contains(&joined) {
                return Some(joined);
            }
        }
        if self.struct_names.contains(name) {
            return Some(name.to_string());
        }
        None
    }

    fn apply_external_context(&mut self, ctx: ModuleExternalContext) {
        for (name, sig) in ctx.imported_functions {
            self.functions.entry(name.clone()).or_insert(sig);
        }
        for (name, fields) in ctx.imported_structs {
            self.struct_names.insert(name.clone());
            self.struct_fields.entry(name).or_insert(fields);
        }
        for (name, methods) in ctx.imported_methods {
            let slot = self.methods.entry(name).or_default();
            for (m, sig) in methods {
                slot.entry(m).or_insert(sig);
            }
        }
        for (name, ty) in ctx.imported_globals {
            self.globals.entry(name).or_insert(ty);
        }
        for (local, target) in ctx.direct_import_targets {
            self.direct_imports.insert(local, target);
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
        let mut imported_modules = HashSet::new();
        let mut direct_imports = HashMap::new();
        let mut module_namespaces = HashMap::new();
        for imp in &program.imports {
            match imp {
                crate::ast::ImportDecl::ImportModule { path, alias } => {
                    if path.len() == 1
                        && matches!(
                            path[0].as_str(),
                            "io" | "str" | "arr" | "datetime" | "random"
                        )
                    {
                        imported_modules.insert(path[0].clone());
                    }
                    let ns = alias
                        .clone()
                        .unwrap_or_else(|| path.first().cloned().unwrap_or_default());
                    if !ns.is_empty() {
                        let mapped = if alias.is_some() {
                            path.clone()
                        } else {
                            vec![path.first().cloned().unwrap_or_default()]
                        };
                        module_namespaces.insert(ns, mapped);
                    }
                }
                crate::ast::ImportDecl::ImportFrom {
                    path,
                    wildcard: _,
                    items,
                } => {
                    let prefix = path.join(".");
                    for item in items {
                        let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                        direct_imports.insert(local, format!("{prefix}.{}", item.name));
                    }
                }
            }
        }
        Self {
            diagnostics: DiagnosticBag::new(),
            functions: HashMap::new(),
            methods: HashMap::new(),
            imported_modules,
            direct_imports,
            module_namespaces,
            struct_names: HashSet::new(),
            struct_fields: HashMap::new(),
            globals: HashMap::new(),
            loop_depth: 0,
            fn_lit_scope_floors: Vec::new(),
        }
    }

    fn check_program(&mut self, program: &Program) {
        self.check_struct_declarations(program);
        self.check_impl_declarations(program);
        self.collect_method_signatures(program);

        for f in &program.functions {
            for p in &f.params {
                self.check_decl_type_exists(
                    &p.ty,
                    format!(
                        "Unknown type in function `{}` parameter `{}`",
                        f.name, p.name
                    ),
                );
            }
            if let Some(ret) = &f.return_type {
                self.check_decl_type_exists(
                    ret,
                    format!("Unknown return type in function `{}`", f.name),
                );
            }
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

        self.check_global_declarations(program);
        self.check_export_declarations(program);

        for f in &program.functions {
            self.check_function(f);
        }

        for imp in &program.impls {
            for method in &imp.methods {
                self.check_method(imp.target.as_str(), method);
            }
        }
    }

    fn check_global_declarations(&mut self, program: &Program) {
        let mut scope = HashMap::<String, TypeInfo>::new();
        let mut scopes = vec![HashMap::<String, TypeInfo>::new()];
        for g in &program.globals {
            if scope.contains_key(&g.name) {
                self.error(format!(
                    "Duplicate global variable declaration `{}`",
                    g.name
                ));
                continue;
            }
            if let Some(t) = &g.ty {
                self.check_decl_type_exists(
                    t,
                    format!("Unknown type in global variable `{}`", g.name),
                );
            }
            let expr_ty = self.check_expr(&g.value, &mut scopes);
            let declared_ty = g.ty.as_ref().map(TypeInfo::from_ast);
            let final_ty = match declared_ty {
                Some(declared) => {
                    if expr_ty != TypeInfo::Unknown && expr_ty != declared {
                        self.error(format!(
                            "Type mismatch in global let `{}`: declared {:?}, got {:?}",
                            g.name, declared, expr_ty
                        ));
                    }
                    declared
                }
                None => expr_ty,
            };
            scope.insert(g.name.clone(), final_ty.clone());
            self.globals.insert(g.name.clone(), final_ty.clone());
            scopes[0].insert(g.name.clone(), final_ty);
        }
    }

    fn check_export_declarations(&mut self, program: &Program) {
        let mut local_exportables = HashSet::new();
        for f in &program.functions {
            local_exportables.insert(f.name.as_str());
        }
        for s in &program.structs {
            local_exportables.insert(s.name.as_str());
        }
        for g in &program.globals {
            local_exportables.insert(g.name.as_str());
        }

        let mut seen_targets = HashSet::new();
        for export_decl in &program.exports {
            match export_decl {
                crate::ast::ExportDecl::Local { items }
                | crate::ast::ExportDecl::From { items, .. } => {
                    for item in items {
                        if matches!(export_decl, crate::ast::ExportDecl::Local { .. })
                            && !local_exportables.contains(item.name.as_str())
                        {
                            self.error(format!(
                                "Exported name `{}` does not exist in this module",
                                item.name
                            ));
                        }
                        let target = item.alias.as_deref().unwrap_or(item.name.as_str());
                        if !seen_targets.insert(target.to_string()) {
                            self.error(format!("Duplicate exported target name `{target}`"));
                        }
                    }
                }
                crate::ast::ExportDecl::FromAll { .. } => {}
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
            TypeName::Int
            | TypeName::Float
            | TypeName::Bool
            | TypeName::String
            | TypeName::Void => {}
            TypeName::Array { elem, .. } => self.check_decl_type_exists(elem, err_prefix),
            TypeName::Fn { params, ret } => {
                for p in params {
                    self.check_decl_type_exists(p, err_prefix.clone());
                }
                self.check_decl_type_exists(ret, err_prefix);
            }
            TypeName::Named(name) => {
                if self.resolve_named_type_name(name).is_none() {
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
        let floor = self.fn_lit_scope_floors.last().copied().unwrap_or(0);
        for (idx, scope) in scopes.iter().enumerate().rev() {
            if idx < floor {
                continue;
            }
            if let Some(t) = scope.get(name) {
                return t.clone();
            }
        }
        if let Some(sig) = self.functions.get(name) {
            return TypeInfo::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
            };
        }
        if let Some(ty) = self.globals.get(name) {
            return ty.clone();
        }
        if !self.fn_lit_scope_floors.is_empty() {
            self.error(format!(
                "Function literals cannot capture outer variable `{name}`"
            ));
            return TypeInfo::Unknown;
        }
        self.error(format!("Unknown variable `{name}`"));
        TypeInfo::Unknown
    }

    fn error(&mut self, message: String) {
        self.diagnostics.error(message, Span::default());
    }
}
