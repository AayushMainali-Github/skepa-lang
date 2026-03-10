mod exports;
mod fs_scan;
mod support;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::Parser;

use self::exports::validate_import_bindings;
use self::support::with_importer_context;

pub use self::exports::{build_export_maps, collect_module_symbols, validate_and_build_export_map};
pub use self::fs_scan::{
    collect_import_module_paths, module_id_from_relative_path, module_path_from_import,
    resolve_import_target, scan_folder_modules,
};

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
    Codegen,
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
        ResolveErrorKind::Codegen => "E-CODEGEN",
        ResolveErrorKind::Io
        | ResolveErrorKind::NonUtf8Path
        | ResolveErrorKind::DuplicateModuleId => "E-MOD-NOT-FOUND",
    }
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
                    "io" | "str" | "arr" | "datetime" | "random" | "os" | "fs" | "vec"
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    File(PathBuf),
    Folder(PathBuf),
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
