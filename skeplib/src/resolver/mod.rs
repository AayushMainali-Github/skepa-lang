use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
