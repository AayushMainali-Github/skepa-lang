use std::collections::HashMap;
use std::path::Path;

use crate::ast::{ImportDecl, Program};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::resolver::{ModuleGraph, ModuleId, ResolveError, build_export_maps, resolve_project};
use crate::types::{FunctionSig, TypeInfo};

use super::{Checker, SemaResult};

#[derive(Debug, Clone, Default)]
pub(super) struct ModuleApi {
    pub functions: HashMap<String, FunctionSig>,
    pub structs: HashMap<String, HashMap<String, TypeInfo>>,
    pub methods: HashMap<String, HashMap<String, FunctionSig>>,
    pub globals: HashMap<String, TypeInfo>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ModuleExternalContext {
    pub imported_functions: HashMap<String, FunctionSig>,
    pub imported_structs: HashMap<String, HashMap<String, TypeInfo>>,
    pub imported_methods: HashMap<String, HashMap<String, FunctionSig>>,
    pub imported_globals: HashMap<String, TypeInfo>,
    pub direct_import_targets: HashMap<String, String>,
}

pub fn analyze_project_entry(
    entry: &Path,
) -> Result<(SemaResult, DiagnosticBag), Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    Ok(analyze_project_graph(&graph))
}

pub fn analyze_project_entry_phased(
    entry: &Path,
) -> Result<(SemaResult, DiagnosticBag, DiagnosticBag), Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    Ok(analyze_project_graph_phased(&graph))
}

pub fn analyze_project_graph(graph: &ModuleGraph) -> (SemaResult, DiagnosticBag) {
    analyze_project_graph_impl(graph)
}

pub fn analyze_project_graph_phased(
    graph: &ModuleGraph,
) -> (SemaResult, DiagnosticBag, DiagnosticBag) {
    analyze_project_graph_phased_impl(graph)
}

fn analyze_project_graph_impl(graph: &ModuleGraph) -> (SemaResult, DiagnosticBag) {
    let (result, parse_diags, sema_diags) = analyze_project_graph_phased_impl(graph);
    let mut all = DiagnosticBag::new();
    for d in parse_diags.into_vec() {
        all.push(d);
    }
    for d in sema_diags.into_vec() {
        all.push(d);
    }
    (result, all)
}

fn analyze_project_graph_phased_impl(
    graph: &ModuleGraph,
) -> (SemaResult, DiagnosticBag, DiagnosticBag) {
    let mut programs = HashMap::<ModuleId, Program>::new();
    let mut parse_diags_all = DiagnosticBag::new();
    let mut sema_diags_all = DiagnosticBag::new();
    let mut module_paths = HashMap::<ModuleId, String>::new();
    for (id, unit) in &graph.modules {
        module_paths.insert(id.clone(), unit.path.display().to_string());
        let (program, parse_diags) = Parser::parse_source(&unit.source);
        for mut d in parse_diags.into_vec() {
            d.message = format!("{}: {}", unit.path.display(), d.message);
            parse_diags_all.push(d);
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
                sema_diags_all.error(msg, Span::default());
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
            sema_diags_all.push(d);
        }
    }

    (
        SemaResult {
            has_errors: !parse_diags_all.is_empty() || !sema_diags_all.is_empty(),
        },
        parse_diags_all,
        sema_diags_all,
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
