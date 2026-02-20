use skeplib::resolver::{
    collect_import_module_paths, module_id_from_relative_path, module_path_from_import,
    resolve_import_target, resolve_project, scan_folder_modules, ImportTarget, ModuleGraph,
    ModuleUnit, ResolveErrorKind,
};
use skeplib::parser::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn resolver_graph_types_construct_cleanly() {
    let mut modules = HashMap::new();
    modules.insert(
        "main".to_string(),
        ModuleUnit {
            id: "main".to_string(),
            path: Path::new("main.sk").to_path_buf(),
            source: "fn main() -> Int { return 0; }".to_string(),
            imports: vec!["io".to_string()],
        },
    );
    let graph = ModuleGraph { modules };
    assert_eq!(graph.modules.len(), 1);
    assert!(graph.modules.contains_key("main"));
}

#[test]
fn resolve_project_reports_missing_entry() {
    let missing = Path::new("skeplib/tests/fixtures/resolver/does_not_exist.sk");
    let err = resolve_project(missing).expect_err("missing entry should error");
    assert_eq!(err[0].kind, ResolveErrorKind::MissingModule);
}

#[test]
fn module_id_from_relative_path_uses_dot_notation() {
    let id = module_id_from_relative_path(Path::new("main.sk")).expect("module id");
    assert_eq!(id, "main");

    let nested =
        module_id_from_relative_path(Path::new("utils/math.sk")).expect("nested module id");
    assert_eq!(nested, "utils.math");
}

#[test]
fn module_id_from_relative_path_rejects_non_sk_extension() {
    let err = module_id_from_relative_path(Path::new("utils/math.txt")).expect_err("must fail");
    assert_eq!(err.kind, ResolveErrorKind::MissingModule);
}

#[test]
fn module_path_from_import_maps_dotted_path_to_sk_file() {
    let root = Path::new("project");
    let import_path = vec!["utils".to_string(), "math".to_string()];
    let got = module_path_from_import(root, &import_path);
    assert_eq!(got, Path::new("project").join("utils").join("math.sk"));
}

#[test]
fn collect_import_module_paths_includes_import_and_from_import() {
    let src = r#"
import alpha.beta;
from gamma.delta import x as y;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let paths = collect_import_module_paths(&program);
    assert_eq!(
        paths,
        vec![
            vec!["alpha".to_string(), "beta".to_string()],
            vec!["gamma".to_string(), "delta".to_string()]
        ]
    );
}

fn make_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skepa_resolver_{label}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn resolve_import_target_prefers_file_when_only_file_exists() {
    let root = make_temp_dir("file");
    fs::write(root.join("a.sk"), "fn main() -> Int { return 0; }").expect("write file");
    let target =
        resolve_import_target(&root, &[String::from("a")]).expect("file target should resolve");
    assert!(matches!(target, ImportTarget::File(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_import_target_returns_folder_when_only_folder_exists() {
    let root = make_temp_dir("folder");
    fs::create_dir_all(root.join("a")).expect("create folder");
    let target =
        resolve_import_target(&root, &[String::from("a")]).expect("folder target should resolve");
    assert!(matches!(target, ImportTarget::Folder(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_import_target_reports_ambiguity_when_file_and_folder_exist() {
    let root = make_temp_dir("ambig");
    fs::write(root.join("a.sk"), "fn main() -> Int { return 0; }").expect("write file");
    fs::create_dir_all(root.join("a")).expect("create folder");
    let err = resolve_import_target(&root, &[String::from("a")]).expect_err("must be ambiguous");
    assert_eq!(err.kind, ResolveErrorKind::AmbiguousModule);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scan_folder_modules_recursively_collects_sk_files_with_prefixed_ids() {
    let root = make_temp_dir("scan_recursive");
    let folder = root.join("string");
    fs::create_dir_all(folder.join("nested")).expect("create nested folder");
    fs::write(folder.join("case.sk"), "fn main() -> Int { return 0; }").expect("write case");
    fs::write(folder.join("nested").join("trim.sk"), "fn main() -> Int { return 0; }")
        .expect("write trim");
    fs::write(folder.join("README.md"), "ignore").expect("write ignored file");

    let entries = scan_folder_modules(&folder, &[String::from("string")]).expect("scan");
    let mut ids = entries.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    ids.sort();
    assert_eq!(
        ids,
        vec!["string.case".to_string(), "string.nested.trim".to_string()]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scan_folder_modules_ignores_non_sk_files() {
    let root = make_temp_dir("scan_filter");
    let folder = root.join("pkg");
    fs::create_dir_all(&folder).expect("create folder");
    fs::write(folder.join("a.txt"), "ignore").expect("write txt");
    fs::write(folder.join("b.sk"), "fn main() -> Int { return 0; }").expect("write sk");

    let entries = scan_folder_modules(&folder, &[String::from("pkg")]).expect("scan");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "pkg.b");
    let _ = fs::remove_dir_all(root);
}
