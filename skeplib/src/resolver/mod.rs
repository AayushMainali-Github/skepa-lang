use std::collections::HashMap;
use std::fs;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    File(PathBuf),
    Folder(PathBuf),
}

pub fn resolve_import_target(root: &Path, import_path: &[String]) -> Result<ImportTarget, ResolveError> {
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
            ))
        }
    };
    let folder_exists = match fs::metadata(&folder_path) {
        Ok(meta) => meta.is_dir(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(ResolveError::new(
                ResolveErrorKind::Io,
                format!("Failed to read metadata for {}: {}", folder_path.display(), e),
                Some(folder_path),
            ))
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
