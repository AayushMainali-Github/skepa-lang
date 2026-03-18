#[path = "common.rs"]
mod common;

use skeplib::parser::Parser;
use skeplib::resolver::{
    ImportTarget, ModuleGraph, ModuleUnit, ResolveErrorKind, SymbolKind,
    collect_import_module_paths, collect_module_symbols, module_id_from_relative_path,
    module_path_from_import, resolve_import_target, resolve_project, scan_folder_modules,
    validate_and_build_export_map,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn make_temp_dir(label: &str) -> std::path::PathBuf {
    common::make_temp_dir(&format!("skepa_resolver_{label}"))
}

mod resolver_cases {
    use super::*;

    mod basics;
    mod fs_graph;
    mod project_rules;
}
