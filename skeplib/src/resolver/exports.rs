use std::collections::HashMap;
use std::path::Path;

use crate::ast::{ImportDecl, Program};

use super::support::suggest_name;
use super::{
    ExportMap, ModuleGraph, ModuleId, ModuleSymbols, ResolveError, ResolveErrorKind, SymbolKind,
    SymbolRef,
};

pub fn build_export_maps(
    graph: &ModuleGraph,
) -> Result<HashMap<ModuleId, ExportMap>, Vec<ResolveError>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Visiting,
        Done,
    }
    let mut out = HashMap::<ModuleId, ExportMap>::new();
    let mut marks = HashMap::<ModuleId, Mark>::new();
    let mut stack = Vec::<ModuleId>::new();
    let mut errors = Vec::<ResolveError>::new();

    fn visit(
        id: &str,
        graph: &ModuleGraph,
        out: &mut HashMap<ModuleId, ExportMap>,
        marks: &mut HashMap<ModuleId, Mark>,
        stack: &mut Vec<ModuleId>,
        errors: &mut Vec<ResolveError>,
    ) {
        if matches!(marks.get(id), Some(Mark::Done)) {
            return;
        }
        if matches!(marks.get(id), Some(Mark::Visiting)) {
            let mut cycle = stack.clone();
            cycle.push(id.to_string());
            errors.push(
                ResolveError::new(
                    ResolveErrorKind::Cycle,
                    format!("Circular re-export detected: {}", cycle.join(" -> ")),
                    graph.modules.get(id).map(|u| u.path.clone()),
                )
                .with_code("E-MOD-CYCLE"),
            );
            return;
        }
        let Some(unit) = graph.modules.get(id) else {
            return;
        };
        marks.insert(id.to_string(), Mark::Visiting);
        stack.push(id.to_string());
        let program = &unit.program;
        let symbols = collect_module_symbols(program, id);
        let mut map = match validate_and_build_export_map(program, &symbols, id, &unit.path) {
            Ok(m) => m,
            Err(mut e) => {
                errors.append(&mut e);
                HashMap::new()
            }
        };

        for ex in &program.exports {
            match ex {
                crate::ast::ExportDecl::From { path, items } => {
                    let deps = resolve_import_module_targets(graph, path);
                    if deps.len() != 1 {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::AmbiguousModule,
                            format!(
                                "re-export source `{}` in module `{}` ({}) must resolve to a single module",
                                path.join("."),
                                id,
                                unit.path.display()
                            ),
                            Some(unit.path.clone()),
                        ));
                        continue;
                    }
                    let dep = deps[0].clone();
                    visit(&dep, graph, out, marks, stack, errors);
                    let Some(dep_map) = out.get(&dep) else {
                        continue;
                    };
                    for item in items {
                        let export_name = item.alias.clone().unwrap_or_else(|| item.name.clone());
                        let Some(sym) = dep_map.get(&item.name).cloned() else {
                            let suggestion =
                                suggest_name(&item.name, dep_map.keys().map(|k| k.as_str()));
                            let msg = if let Some(s) = suggestion {
                                format!(
                                    "Cannot re-export `{}` from `{}` in module `{}` ({}): symbol is not exported; did you mean `{}`?",
                                    item.name,
                                    path.join("."),
                                    id,
                                    unit.path.display(),
                                    s
                                )
                            } else {
                                format!(
                                    "Cannot re-export `{}` from `{}` in module `{}` ({}): symbol is not exported",
                                    item.name,
                                    path.join("."),
                                    id,
                                    unit.path.display()
                                )
                            };
                            errors.push(ResolveError::new(
                                ResolveErrorKind::NotExported,
                                msg,
                                Some(unit.path.clone()),
                            ));
                            continue;
                        };
                        if map.insert(export_name.clone(), sym).is_some() {
                            errors.push(ResolveError::new(
                                ResolveErrorKind::ImportConflict,
                                format!(
                                    "Duplicate exported target name `{}` in module `{}` ({})",
                                    export_name,
                                    id,
                                    unit.path.display()
                                ),
                                Some(unit.path.clone()),
                            ));
                        }
                    }
                }
                crate::ast::ExportDecl::FromAll { path } => {
                    let deps = resolve_import_module_targets(graph, path);
                    if deps.len() != 1 {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::AmbiguousModule,
                            format!(
                                "re-export source `{}` in module `{}` ({}) must resolve to a single module",
                                path.join("."),
                                id,
                                unit.path.display()
                            ),
                            Some(unit.path.clone()),
                        ));
                        continue;
                    }
                    let dep = deps[0].clone();
                    visit(&dep, graph, out, marks, stack, errors);
                    let Some(dep_map) = out.get(&dep) else {
                        continue;
                    };
                    for (name, sym) in dep_map {
                        if map.insert(name.clone(), sym.clone()).is_some() {
                            errors.push(ResolveError::new(
                                ResolveErrorKind::ImportConflict,
                                format!(
                                    "Duplicate exported target name `{}` in module `{}` ({})",
                                    name,
                                    id,
                                    unit.path.display()
                                ),
                                Some(unit.path.clone()),
                            ));
                        }
                    }
                }
                crate::ast::ExportDecl::Local { .. } => {}
            }
        }

        out.insert(id.to_string(), map);
        stack.pop();
        marks.insert(id.to_string(), Mark::Done);
    }

    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        visit(&id, graph, &mut out, &mut marks, &mut stack, &mut errors);
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

pub(crate) fn resolve_import_module_targets(
    graph: &ModuleGraph,
    import_path: &[String],
) -> Vec<ModuleId> {
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

pub(super) fn validate_import_bindings(
    graph: &ModuleGraph,
    export_maps: &HashMap<ModuleId, ExportMap>,
) -> Vec<ResolveError> {
    let mut errors = Vec::new();
    for (id, unit) in &graph.modules {
        let program = &unit.program;
        let mut bound_names = HashMap::<String, String>::new();

        for import in &program.imports {
            match import {
                ImportDecl::ImportModule { path, alias } => {
                    if let Some(a) = alias
                        && let Some(prev) =
                            bound_names.insert(a.clone(), "module alias".to_string())
                    {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::ImportConflict,
                            format!(
                                "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                a, id, unit.path.display(), prev
                            ),
                            Some(unit.path.clone()),
                        ));
                    }
                    let _ = resolve_import_module_targets(graph, path);
                }
                ImportDecl::ImportFrom {
                    path,
                    wildcard,
                    items,
                } => {
                    let targets = resolve_import_module_targets(graph, path);
                    if targets.is_empty() {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::MissingModule,
                            format!(
                                "Cannot resolve from-import source `{}` in module `{}` ({})",
                                path.join("."),
                                id,
                                unit.path.display()
                            ),
                            Some(unit.path.clone()),
                        ));
                        continue;
                    }
                    if targets.len() != 1 {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::AmbiguousModule,
                            format!(
                                "from-import source `{}` in module `{}` ({}) resolves to a namespace root; import a concrete file module instead",
                                path.join("."),
                                id,
                                unit.path.display()
                            ),
                            Some(unit.path.clone()),
                        ));
                        continue;
                    }
                    let target = &targets[0];
                    let exports = match export_maps.get(target) {
                        Some(m) => m,
                        None => continue,
                    };

                    if *wildcard {
                        let mut names = exports.keys().cloned().collect::<Vec<_>>();
                        names.sort();
                        for local in names {
                            if let Some(prev) = bound_names
                                .insert(local.clone(), "from-import wildcard".to_string())
                            {
                                errors.push(ResolveError::new(
                                    ResolveErrorKind::ImportConflict,
                                    format!(
                                        "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                        local, id, unit.path.display(), prev
                                    ),
                                    Some(unit.path.clone()),
                                ));
                            }
                        }
                    } else {
                        for item in items {
                            if !exports.contains_key(&item.name) {
                                let suggestion =
                                    suggest_name(&item.name, exports.keys().map(|k| k.as_str()));
                                let target_path = graph
                                    .modules
                                    .get(target)
                                    .map(|u| u.path.display().to_string())
                                    .unwrap_or_else(|| "<unknown>".to_string());
                                let msg = if let Some(s) = suggestion {
                                    format!(
                                        "Cannot import `{}` from `{}` in module `{}` ({}) -> target `{}` ({}): symbol is not exported; did you mean `{}`?",
                                        item.name,
                                        path.join("."),
                                        id,
                                        unit.path.display(),
                                        target,
                                        target_path,
                                        s
                                    )
                                } else {
                                    format!(
                                        "Cannot import `{}` from `{}` in module `{}` ({}) -> target `{}` ({}): symbol is not exported",
                                        item.name,
                                        path.join("."),
                                        id,
                                        unit.path.display(),
                                        target,
                                        target_path
                                    )
                                };
                                errors.push(ResolveError::new(
                                    ResolveErrorKind::NotExported,
                                    msg,
                                    Some(unit.path.clone()),
                                ));
                                continue;
                            }
                            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            if let Some(prev) =
                                bound_names.insert(local.clone(), "from-import".to_string())
                            {
                                errors.push(ResolveError::new(
                                    ResolveErrorKind::ImportConflict,
                                    format!(
                                        "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                        local, id, unit.path.display(), prev
                                    ),
                                    Some(unit.path.clone()),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    errors
}

pub fn collect_module_symbols(program: &Program, module_id: &str) -> ModuleSymbols {
    let mut locals = HashMap::new();
    for f in &program.functions {
        locals.insert(
            f.name.clone(),
            SymbolRef {
                module_id: module_id.to_string(),
                local_name: f.name.clone(),
                kind: SymbolKind::Fn,
            },
        );
    }
    for operator in &program.operators {
        locals.insert(
            operator.name.clone(),
            SymbolRef {
                module_id: module_id.to_string(),
                local_name: operator.name.clone(),
                kind: SymbolKind::Fn,
            },
        );
    }
    for s in &program.structs {
        locals.insert(
            s.name.clone(),
            SymbolRef {
                module_id: module_id.to_string(),
                local_name: s.name.clone(),
                kind: SymbolKind::Struct,
            },
        );
    }
    for g in &program.globals {
        locals.insert(
            g.name.clone(),
            SymbolRef {
                module_id: module_id.to_string(),
                local_name: g.name.clone(),
                kind: SymbolKind::GlobalLet,
            },
        );
    }
    ModuleSymbols { locals }
}

pub fn validate_and_build_export_map(
    program: &Program,
    symbols: &ModuleSymbols,
    module_id: &str,
    module_path: &Path,
) -> Result<ExportMap, Vec<ResolveError>> {
    let mut export_map = HashMap::new();
    let mut errors = Vec::new();

    if program.exports.is_empty() {
        return Ok(export_map);
    }

    for export_decl in &program.exports {
        if let crate::ast::ExportDecl::Local { items } = export_decl {
            for item in items {
                let export_name = item.alias.as_ref().unwrap_or(&item.name).clone();
                let sym = if let Some(sym) = symbols.locals.get(&item.name).cloned() {
                    Some(sym)
                } else if let Some(crate::ast::ImportDecl::ImportModule { path, .. }) = program
                    .imports
                    .iter()
                    .find(|i| matches!(i, crate::ast::ImportDecl::ImportModule { alias, path } if alias.as_deref() == Some(item.name.as_str()) || path.first().is_some_and(|p| p == &item.name)))
                {
                    Some(SymbolRef {
                        module_id: module_id.to_string(),
                        local_name: path.join("."),
                        kind: SymbolKind::Namespace,
                    })
                } else {
                    None
                };
                let Some(sym) = sym else {
                    errors.push(ResolveError::new(
                        ResolveErrorKind::ExportUnknown,
                        format!(
                            "Exported name `{}` does not exist in module `{}` ({})",
                            item.name,
                            module_id,
                            module_path.display()
                        ),
                        Some(module_path.to_path_buf()),
                    ));
                    continue;
                };

                if export_map.insert(export_name.clone(), sym).is_some() {
                    errors.push(ResolveError::new(
                        ResolveErrorKind::ImportConflict,
                        format!(
                            "Duplicate exported target name `{}` in module `{}` ({})",
                            export_name,
                            module_id,
                            module_path.display()
                        ),
                        Some(module_path.to_path_buf()),
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(export_map)
    } else {
        Err(errors)
    }
}
