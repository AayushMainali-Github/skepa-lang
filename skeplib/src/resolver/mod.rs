use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{ImportDecl, Program};
use crate::parser::Parser;

pub type ModuleId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleUnit {
    pub id: ModuleId,
    pub path: PathBuf,
    pub source: String,
    pub imports: Vec<ModuleId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleGraph {
    pub modules: HashMap<ModuleId, ModuleUnit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Fn,
    Struct,
    GlobalLet,
    Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    pub module_id: ModuleId,
    pub local_name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleSymbols {
    pub locals: HashMap<String, SymbolRef>,
}

pub type ExportMap = HashMap<String, SymbolRef>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveErrorKind {
    MissingModule,
    AmbiguousModule,
    Io,
    NonUtf8Path,
    DuplicateModuleId,
    Cycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub kind: ResolveErrorKind,
    pub code: &'static str,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

impl ResolveError {
    pub fn new(kind: ResolveErrorKind, message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            kind,
            code: code_for_kind(kind),
            message: message.into(),
            path,
            line: None,
            col: None,
        }
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = code;
        self
    }

    pub fn with_line_col(mut self, line: usize, col: usize) -> Self {
        self.line = Some(line);
        self.col = Some(col);
        self
    }
}

fn code_for_kind(kind: ResolveErrorKind) -> &'static str {
    match kind {
        ResolveErrorKind::MissingModule => "E-MOD-NOT-FOUND",
        ResolveErrorKind::Cycle => "E-MOD-CYCLE",
        ResolveErrorKind::AmbiguousModule => "E-MOD-AMBIG",
        ResolveErrorKind::Io
        | ResolveErrorKind::NonUtf8Path
        | ResolveErrorKind::DuplicateModuleId => "E-MOD-NOT-FOUND",
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.chars().count();
    }
    if b.is_empty() {
        return a.chars().count();
    }
    let b_chars = b.chars().collect::<Vec<_>>();
    let mut prev = (0..=b_chars.len()).collect::<Vec<_>>();
    let mut cur = vec![0usize; b_chars.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b_chars.len()]
}

fn suggest_name<'a>(needle: &str, haystack: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for cand in haystack {
        let d = levenshtein(needle, cand);
        if d <= 2 {
            match best {
                Some((_, bd)) if d >= bd => {}
                _ => best = Some((cand, d)),
            }
        }
    }
    best.map(|(s, _)| s.to_string())
}

pub fn resolve_project(entry: &Path) -> Result<ModuleGraph, Vec<ResolveError>> {
    if !entry.exists() {
        return Err(vec![ResolveError::new(
            ResolveErrorKind::MissingModule,
            format!("Entry module not found: {}", entry.display()),
            Some(entry.to_path_buf()),
        )]);
    }
    let root = entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut graph = ModuleGraph::default();
    let mut errors = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(entry.to_path_buf());

    while let Some(path) = queue.pop_front() {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => path.clone(),
        };
        let id = match module_id_from_relative_path(&rel) {
            Ok(id) => id,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        if let Some(existing) = graph.modules.get(&id) {
            if existing.path != path {
                errors.push(ResolveError::new(
                    ResolveErrorKind::DuplicateModuleId,
                    format!(
                        "Duplicate module id `{}` from {} and {}",
                        id,
                        existing.path.display(),
                        path.display()
                    ),
                    Some(path.clone()),
                ));
            }
            continue;
        }

        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(ResolveError::new(
                    ResolveErrorKind::Io,
                    format!("Failed to read {}: {}", path.display(), e),
                    Some(path.clone()),
                ));
                continue;
            }
        };
        let (program, _diags) = Parser::parse_source(&source);
        let import_paths = collect_import_module_paths(&program);
        let mut imports = Vec::new();

        for import_path in import_paths {
            if import_path.len() == 1
                && matches!(
                    import_path[0].as_str(),
                    "io" | "str" | "arr" | "datetime" | "random"
                )
            {
                continue;
            }
            let import_text = import_path.join(".");
            match resolve_import_target(&root, &import_path) {
                Ok(ImportTarget::File(target_file)) => {
                    let target_rel = match target_file.strip_prefix(&root) {
                        Ok(r) => r.to_path_buf(),
                        Err(_) => target_file.clone(),
                    };
                    match module_id_from_relative_path(&target_rel) {
                        Ok(dep_id) => imports.push(dep_id),
                        Err(e) => errors.push(e),
                    }
                    queue.push_back(target_file);
                }
                Ok(ImportTarget::Folder(target_folder)) => {
                    match scan_folder_modules(&target_folder, &import_path) {
                        Ok(entries) => {
                            for (dep_id, dep_path) in entries {
                                imports.push(dep_id);
                                queue.push_back(dep_path);
                            }
                        }
                        Err(e) => {
                            errors.push(with_importer_context(e, &id, &path, &import_text, &source))
                        }
                    }
                }
                Err(e) => errors.push(with_importer_context(e, &id, &path, &import_text, &source)),
            }
        }

        graph.modules.insert(
            id.clone(),
            ModuleUnit {
                id,
                path,
                source,
                imports,
            },
        );
    }

    if errors.is_empty() {
        errors.extend(detect_cycles(&graph));
    }
    if errors.is_empty() {
        match build_export_maps(&graph) {
            Ok(export_maps) => errors.extend(validate_import_bindings(&graph, &export_maps)),
            Err(mut e) => errors.append(&mut e),
        }
    }
    if errors.is_empty() {
        Ok(graph)
    } else {
        Err(errors)
    }
}

fn with_importer_context(
    mut err: ResolveError,
    importer_id: &str,
    importer_path: &Path,
    import_text: &str,
    importer_source: &str,
) -> ResolveError {
    if let Some((line, col)) = find_import_line_col(importer_source, import_text) {
        err = err.with_line_col(line, col);
    }
    err.message = format!(
        "{} (while resolving import `{}` in module `{}` at {})",
        err.message,
        import_text,
        importer_id,
        importer_path.display()
    );
    err
}

fn find_import_line_col(source: &str, import_text: &str) -> Option<(usize, usize)> {
    let pat_import = format!("import {import_text}");
    let pat_from = format!("from {import_text} import");
    for (idx, line) in source.lines().enumerate() {
        if let Some(col) = line
            .find(&pat_import)
            .or_else(|| line.find(&pat_from))
            .map(|v| v + 1)
        {
            return Some((idx + 1, col));
        }
    }
    None
}

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
        let (program, _diags) = Parser::parse_source(&unit.source);
        let symbols = collect_module_symbols(&program, id);
        let mut map = match validate_and_build_export_map(&program, &symbols, id, &unit.path) {
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
                            errors.push(
                                ResolveError::new(
                                    ResolveErrorKind::MissingModule,
                                    msg,
                                    Some(unit.path.clone()),
                                )
                                .with_code("E-IMPORT-NOT-EXPORTED"),
                            );
                            continue;
                        };
                        if map.insert(export_name.clone(), sym).is_some() {
                            errors.push(
                                ResolveError::new(
                                    ResolveErrorKind::DuplicateModuleId,
                                    format!(
                                        "Duplicate exported target name `{}` in module `{}` ({})",
                                        export_name,
                                        id,
                                        unit.path.display()
                                    ),
                                    Some(unit.path.clone()),
                                )
                                .with_code("E-IMPORT-CONFLICT"),
                            );
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
                            errors.push(
                                ResolveError::new(
                                    ResolveErrorKind::DuplicateModuleId,
                                    format!(
                                        "Duplicate exported target name `{}` in module `{}` ({})",
                                        name,
                                        id,
                                        unit.path.display()
                                    ),
                                    Some(unit.path.clone()),
                                )
                                .with_code("E-IMPORT-CONFLICT"),
                            );
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

fn validate_import_bindings(
    graph: &ModuleGraph,
    export_maps: &HashMap<ModuleId, ExportMap>,
) -> Vec<ResolveError> {
    let mut errors = Vec::new();
    for (id, unit) in &graph.modules {
        let (program, _diags) = Parser::parse_source(&unit.source);
        let mut bound_names = HashMap::<String, String>::new();

        for import in &program.imports {
            match import {
                ImportDecl::ImportModule { path, alias } => {
                    if let Some(a) = alias
                        && let Some(prev) =
                            bound_names.insert(a.clone(), "module alias".to_string())
                    {
                        errors.push(ResolveError::new(
                            ResolveErrorKind::DuplicateModuleId,
                            format!(
                                "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                a, id, unit.path.display(), prev
                            ),
                            Some(unit.path.clone()),
                        ).with_code("E-IMPORT-CONFLICT"));
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
                                    ResolveErrorKind::DuplicateModuleId,
                                    format!(
                                        "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                        local, id, unit.path.display(), prev
                                    ),
                                    Some(unit.path.clone()),
                                ).with_code("E-IMPORT-CONFLICT"));
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
                                errors.push(
                                    ResolveError::new(
                                        ResolveErrorKind::MissingModule,
                                        msg,
                                        Some(unit.path.clone()),
                                    )
                                    .with_code("E-IMPORT-NOT-EXPORTED"),
                                );
                                continue;
                            }
                            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            if let Some(prev) =
                                bound_names.insert(local.clone(), "from-import".to_string())
                            {
                                errors.push(ResolveError::new(
                                    ResolveErrorKind::DuplicateModuleId,
                                    format!(
                                        "Duplicate imported binding `{}` in module `{}` ({}) (conflicts with {})",
                                        local, id, unit.path.display(), prev
                                    ),
                                    Some(unit.path.clone()),
                                ).with_code("E-IMPORT-CONFLICT"));
                            }
                        }
                    }
                }
            }
        }
    }
    errors
}

pub fn module_id_from_relative_path(path: &Path) -> Result<ModuleId, ResolveError> {
    if path.extension().and_then(|e| e.to_str()) != Some("sk") {
        return Err(ResolveError::new(
            ResolveErrorKind::MissingModule,
            format!("Expected .sk module path, got {}", path.display()),
            Some(path.to_path_buf()),
        ));
    }

    let no_ext = path.with_extension("");
    let mut parts = Vec::new();
    for comp in no_ext.components() {
        let s = comp.as_os_str().to_str().ok_or_else(|| {
            ResolveError::new(
                ResolveErrorKind::NonUtf8Path,
                format!("Non-UTF8 path component in {}", path.display()),
                Some(path.to_path_buf()),
            )
        })?;
        if s.is_empty() || s == "." {
            continue;
        }
        parts.push(s.to_string());
    }

    if parts.is_empty() {
        return Err(ResolveError::new(
            ResolveErrorKind::MissingModule,
            format!("Cannot derive module id from path {}", path.display()),
            Some(path.to_path_buf()),
        ));
    }
    Ok(parts.join("."))
}

pub fn module_path_from_import(root: &Path, import_path: &[String]) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in import_path {
        path.push(part);
    }
    path.set_extension("sk");
    path
}

pub fn collect_import_module_paths(program: &Program) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for import in &program.imports {
        match import {
            ImportDecl::ImportModule { path, .. } => out.push(path.clone()),
            ImportDecl::ImportFrom { path, .. } => out.push(path.clone()),
        }
    }
    for export in &program.exports {
        match export {
            crate::ast::ExportDecl::From { path, .. }
            | crate::ast::ExportDecl::FromAll { path } => out.push(path.clone()),
            crate::ast::ExportDecl::Local { .. } => {}
        }
    }
    out
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
                    errors.push(
                        ResolveError::new(
                            ResolveErrorKind::MissingModule,
                            format!(
                                "Exported name `{}` does not exist in module `{}` ({})",
                                item.name,
                                module_id,
                                module_path.display()
                            ),
                            Some(module_path.to_path_buf()),
                        )
                        .with_code("E-EXPORT-UNKNOWN"),
                    );
                    continue;
                };

                if export_map.insert(export_name.clone(), sym).is_some() {
                    errors.push(
                        ResolveError::new(
                            ResolveErrorKind::DuplicateModuleId,
                            format!(
                                "Duplicate exported target name `{}` in module `{}` ({})",
                                export_name,
                                module_id,
                                module_path.display()
                            ),
                            Some(module_path.to_path_buf()),
                        )
                        .with_code("E-IMPORT-CONFLICT"),
                    );
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    File(PathBuf),
    Folder(PathBuf),
}

pub fn resolve_import_target(
    root: &Path,
    import_path: &[String],
) -> Result<ImportTarget, ResolveError> {
    let file_path = module_path_from_import(root, import_path);
    let mut folder_path = root.to_path_buf();
    for part in import_path {
        folder_path.push(part);
    }

    let file_exists = match fs::metadata(&file_path) {
        Ok(meta) => meta.is_file(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(ResolveError::new(
                ResolveErrorKind::Io,
                format!("Failed to read metadata for {}: {}", file_path.display(), e),
                Some(file_path),
            ));
        }
    };
    let folder_exists = match fs::metadata(&folder_path) {
        Ok(meta) => meta.is_dir(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(ResolveError::new(
                ResolveErrorKind::Io,
                format!(
                    "Failed to read metadata for {}: {}",
                    folder_path.display(),
                    e
                ),
                Some(folder_path),
            ));
        }
    };

    match (file_exists, folder_exists) {
        (true, true) => Err(ResolveError::new(
            ResolveErrorKind::AmbiguousModule,
            format!(
                "Ambiguous import `{}`: both {} and {} exist",
                import_path.join("."),
                file_path.display(),
                folder_path.display()
            ),
            Some(root.to_path_buf()),
        )),
        (true, false) => Ok(ImportTarget::File(file_path)),
        (false, true) => Ok(ImportTarget::Folder(folder_path)),
        (false, false) => Err(ResolveError::new(
            ResolveErrorKind::MissingModule,
            format!("Module not found for import `{}`", import_path.join(".")),
            Some(root.to_path_buf()),
        )),
    }
}

pub fn scan_folder_modules(
    folder_root: &Path,
    import_prefix: &[String],
) -> Result<Vec<(ModuleId, PathBuf)>, ResolveError> {
    let mut out = Vec::new();
    scan_folder_modules_inner(folder_root, folder_root, import_prefix, &mut out)?;
    Ok(out)
}

fn scan_folder_modules_inner(
    folder_root: &Path,
    dir: &Path,
    import_prefix: &[String],
    out: &mut Vec<(ModuleId, PathBuf)>,
) -> Result<(), ResolveError> {
    let entries = fs::read_dir(dir).map_err(|e| {
        ResolveError::new(
            ResolveErrorKind::Io,
            format!("Failed to read directory {}: {}", dir.display(), e),
            Some(dir.to_path_buf()),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            ResolveError::new(
                ResolveErrorKind::Io,
                format!("Failed to read directory entry in {}: {}", dir.display(), e),
                Some(dir.to_path_buf()),
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            ResolveError::new(
                ResolveErrorKind::Io,
                format!("Failed to read file type for {}: {}", path.display(), e),
                Some(path.clone()),
            )
        })?;
        if file_type.is_dir() {
            scan_folder_modules_inner(folder_root, &path, import_prefix, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("sk") {
            continue;
        }
        let rel = path.strip_prefix(folder_root).map_err(|_| {
            ResolveError::new(
                ResolveErrorKind::Io,
                format!(
                    "Failed to strip folder prefix {} from {}",
                    folder_root.display(),
                    path.display()
                ),
                Some(path.clone()),
            )
        })?;
        let rel_no_ext = rel.with_extension("");
        let mut parts: Vec<String> = import_prefix.to_vec();
        for comp in rel_no_ext.components() {
            let s = comp.as_os_str().to_str().ok_or_else(|| {
                ResolveError::new(
                    ResolveErrorKind::NonUtf8Path,
                    format!("Non-UTF8 path component in {}", path.display()),
                    Some(path.clone()),
                )
            })?;
            if s.is_empty() || s == "." {
                continue;
            }
            parts.push(s.to_string());
        }
        if parts.is_empty() {
            continue;
        }
        out.push((parts.join("."), path));
    }
    Ok(())
}

pub fn detect_cycles(graph: &ModuleGraph) -> Vec<ResolveError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    fn dfs(
        node: &str,
        graph: &ModuleGraph,
        colors: &mut HashMap<String, Color>,
        stack: &mut Vec<String>,
        errors: &mut Vec<ResolveError>,
    ) {
        colors.insert(node.to_string(), Color::Gray);
        stack.push(node.to_string());

        let imports = graph
            .modules
            .get(node)
            .map(|m| m.imports.clone())
            .unwrap_or_default();
        for dep in imports {
            if !graph.modules.contains_key(&dep) {
                continue;
            }
            match colors.get(dep.as_str()).copied().unwrap_or(Color::White) {
                Color::White => dfs(&dep, graph, colors, stack, errors),
                Color::Gray => {
                    if let Some(pos) = stack.iter().position(|s| s == &dep) {
                        let mut cycle = stack[pos..].to_vec();
                        cycle.push(dep.clone());
                        let chain = cycle.join(" -> ");
                        errors.push(ResolveError::new(
                            ResolveErrorKind::Cycle,
                            format!("Import cycle detected: {chain}"),
                            None,
                        ));
                    }
                }
                Color::Black => {}
            }
        }

        stack.pop();
        colors.insert(node.to_string(), Color::Black);
    }

    let mut colors = HashMap::<String, Color>::new();
    for id in graph.modules.keys() {
        colors.insert(id.clone(), Color::White);
    }
    let mut stack = Vec::<String>::new();
    let mut errors = Vec::<ResolveError>::new();
    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        if colors.get(id.as_str()).copied() == Some(Color::White) {
            dfs(&id, graph, &mut colors, &mut stack, &mut errors);
        }
    }
    errors
}
