use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{ImportDecl, Program};

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
    pub message: String,
    pub path: Option<PathBuf>,
}

impl ResolveError {
    pub fn new(kind: ResolveErrorKind, message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            kind,
            message: message.into(),
            path,
        }
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
    Ok(ModuleGraph::default())
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
    out
}
