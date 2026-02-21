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

#[test]
fn collect_module_symbols_collects_top_level_functions_and_structs() {
    let src = r#"
struct User { id: Int }
let version: Int = 1;
fn add(a: Int, b: Int) -> Int { return a + b; }
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let symbols = collect_module_symbols(&program, "main");
    assert_eq!(symbols.locals.len(), 4);
    assert_eq!(symbols.locals["User"].kind, SymbolKind::Struct);
    assert_eq!(symbols.locals["version"].kind, SymbolKind::GlobalLet);
    assert_eq!(symbols.locals["add"].kind, SymbolKind::Fn);
    assert_eq!(symbols.locals["main"].kind, SymbolKind::Fn);
}

#[test]
fn validate_and_build_export_map_accepts_valid_exports() {
    let src = r#"
struct User { id: Int }
let version: Int = 1;
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add as plus, User, version };
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let symbols = collect_module_symbols(&program, "main");
    let map = validate_and_build_export_map(&program, &symbols, "main", Path::new("main.sk"))
        .expect("valid exports");
    assert_eq!(map.len(), 3);
    assert_eq!(map["plus"].local_name, "add");
    assert_eq!(map["User"].local_name, "User");
    assert_eq!(map["version"].local_name, "version");
}

#[test]
fn validate_and_build_export_map_rejects_unknown_exported_name() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { nope };
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let symbols = collect_module_symbols(&program, "main");
    let errs = validate_and_build_export_map(&program, &symbols, "main", Path::new("main.sk"))
        .expect_err("unknown export should fail");
    assert!(errs
        .iter()
        .any(|e| e.message.contains("Exported name `nope` does not exist")));
}

#[test]
fn validate_and_build_export_map_rejects_duplicate_exported_target_name() {
    let src = r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
fn sub(a: Int, b: Int) -> Int { return a - b; }
export { add as x, sub as x };
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let symbols = collect_module_symbols(&program, "main");
    let errs = validate_and_build_export_map(&program, &symbols, "main", Path::new("main.sk"))
        .expect_err("duplicate export target should fail");
    assert!(errs
        .iter()
        .any(|e| e.message.contains("Duplicate exported target name `x`")));
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
    fs::write(
        folder.join("nested").join("trim.sk"),
        "fn main() -> Int { return 0; }",
    )
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

#[test]
fn resolve_project_builds_multi_hop_graph() {
    let root = make_temp_dir("graph_multihop");
    let main_src = r#"
import a;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
import b;
fn run() -> Int { return 1; }
"#;
    let b_src = r#"
fn util() -> Int { return 2; }
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");
    fs::write(root.join("b.sk"), b_src).expect("write b");

    let graph = resolve_project(&root.join("main.sk")).expect("resolve");
    assert!(graph.modules.contains_key("main"));
    assert!(graph.modules.contains_key("a"));
    assert!(graph.modules.contains_key("b"));
    assert_eq!(graph.modules["main"].imports, vec!["a".to_string()]);
    assert_eq!(graph.modules["a"].imports, vec!["b".to_string()]);
    assert!(graph.modules["b"].imports.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_loads_shared_dependency_once() {
    let root = make_temp_dir("graph_shared");
    let main_src = r#"
import a;
import b;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
import c;
fn fa() -> Int { return 1; }
"#;
    let b_src = r#"
import c;
fn fb() -> Int { return 1; }
"#;
    let c_src = r#"
fn fc() -> Int { return 1; }
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");
    fs::write(root.join("b.sk"), b_src).expect("write b");
    fs::write(root.join("c.sk"), c_src).expect("write c");

    let graph = resolve_project(&root.join("main.sk")).expect("resolve");
    assert!(graph.modules.contains_key("c"));
    assert_eq!(graph.modules.len(), 4);
    let c_count = graph.modules.keys().filter(|id| id.as_str() == "c").count();
    assert_eq!(c_count, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_two_node_cycle_with_chain() {
    let root = make_temp_dir("cycle2");
    let main_src = r#"
import a;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
import b;
fn fa() -> Int { return 1; }
"#;
    let b_src = r#"
import a;
fn fb() -> Int { return 1; }
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");
    fs::write(root.join("b.sk"), b_src).expect("write b");

    let errs = resolve_project(&root.join("main.sk")).expect_err("cycle expected");
    assert!(
        errs.iter()
            .any(|e| { e.kind == ResolveErrorKind::Cycle && e.message.contains("a -> b -> a") })
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_three_node_cycle_with_chain() {
    let root = make_temp_dir("cycle3");
    let main_src = r#"
import a;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
import b;
fn fa() -> Int { return 1; }
"#;
    let b_src = r#"
import c;
fn fb() -> Int { return 1; }
"#;
    let c_src = r#"
import a;
fn fc() -> Int { return 1; }
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");
    fs::write(root.join("b.sk"), b_src).expect("write b");
    fs::write(root.join("c.sk"), c_src).expect("write c");

    let errs = resolve_project(&root.join("main.sk")).expect_err("cycle expected");
    assert!(
        errs.iter().any(|e| {
            e.kind == ResolveErrorKind::Cycle && e.message.contains("a -> b -> c -> a")
        })
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_missing_imported_module() {
    let root = make_temp_dir("missing_dep");
    let main_src = r#"
import missing.dep;
fn main() -> Int { return 0; }
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("missing module expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::MissingModule
            && e.message
                .contains("while resolving import `missing.dep` in module `main`")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_io_error_for_directory_entry_path() {
    let root = make_temp_dir("io_dir_entry");
    let entry_dir = root.join("entry.sk");
    fs::create_dir_all(&entry_dir).expect("create directory");

    let errs = resolve_project(&entry_dir).expect_err("io expected");
    assert!(errs.iter().any(|e| e.kind == ResolveErrorKind::Io));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_duplicate_module_id_collision() {
    let root = make_temp_dir("dup_module_id");
    let main_src = r#"
import a;
fn main() -> Int { return 0; }
"#;
    fs::create_dir_all(root.join("a").join("b")).expect("create nested");
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a").join("b.c.sk"), "fn x() -> Int { return 1; }").expect("write file");
    fs::write(
        root.join("a").join("b").join("c.sk"),
        "fn y() -> Int { return 2; }",
    )
    .expect("write file");

    let errs = resolve_project(&root.join("main.sk")).expect_err("duplicate id expected");
    assert!(
        errs.iter()
            .any(|e| e.kind == ResolveErrorKind::DuplicateModuleId)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_importing_non_exported_symbol() {
    let root = make_temp_dir("import_non_exported");
    let main_src = r#"
from a import hidden;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
fn hidden() -> Int { return 1; }
fn shown() -> Int { return 2; }
export { shown };
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");

    let errs = resolve_project(&root.join("main.sk")).expect_err("non-exported import expected");
    assert!(errs.iter().any(|e| {
        e.message.contains("symbol is not exported") && e.message.contains("hidden")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_duplicate_imported_bindings_in_scope() {
    let root = make_temp_dir("dup_import_bindings");
    let main_src = r#"
from a import foo as x;
from b import bar as x;
fn main() -> Int { return 0; }
"#;
    let a_src = r#"
fn foo() -> Int { return 1; }
export { foo };
"#;
    let b_src = r#"
fn bar() -> Int { return 2; }
export { bar };
"#;
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(root.join("a.sk"), a_src).expect("write a");
    fs::write(root.join("b.sk"), b_src).expect("write b");

    let errs = resolve_project(&root.join("main.sk")).expect_err("duplicate binding expected");
    assert!(errs
        .iter()
        .any(|e| e.message.contains("Duplicate imported binding `x`")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_from_import_against_namespace_root() {
    let root = make_temp_dir("from_namespace_root");
    let main_src = r#"
from string import trim;
fn main() -> Int { return 0; }
"#;
    fs::create_dir_all(root.join("string")).expect("create string folder");
    fs::write(root.join("main.sk"), main_src).expect("write main");
    fs::write(
        root.join("string").join("trim.sk"),
        "fn trim(s: String) -> String { return s; } export { trim };",
    )
    .expect("write trim");
    fs::write(
        root.join("string").join("case.sk"),
        "fn up(s: String) -> String { return s; } export { up };",
    )
    .expect("write case");

    let errs = resolve_project(&root.join("main.sk")).expect_err("namespace-root from-import expected");
    assert!(errs
        .iter()
        .any(|e| e.kind == ResolveErrorKind::AmbiguousModule
            && e.message.contains("resolves to a namespace root")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_supports_re_export_from_module() {
    let root = make_temp_dir("re_export");
    fs::write(
        root.join("a.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
export { add } from a;
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from b import add;
fn main() -> Int { return add(1, 2); }
"#,
    )
    .expect("write main");

    let graph = resolve_project(&root.join("main.sk")).expect("re-export resolve");
    assert!(graph.modules.contains_key("a"));
    assert!(graph.modules.contains_key("b"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_duplicate_bindings_from_wildcard_imports() {
    let root = make_temp_dir("wildcard_conflict");
    fs::write(
        root.join("a.sk"),
        r#"
fn x() -> Int { return 1; }
export { x };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
fn x() -> Int { return 2; }
export { x };
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from a import *;
from b import *;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("wildcard conflict expected");
    assert!(errs
        .iter()
        .any(|e| e.message.contains("Duplicate imported binding `x`")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_detects_circular_re_exports() {
    let root = make_temp_dir("re_export_cycle");
    fs::write(root.join("a.sk"), "export * from b;\n").expect("write a");
    fs::write(root.join("b.sk"), "export * from a;\n").expect("write b");
    fs::write(
        root.join("main.sk"),
        "import a;\nfn main() -> Int { return 0; }\n",
    )
    .expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("re-export cycle expected");
    assert!(errs.iter().any(|e| e.kind == ResolveErrorKind::Cycle));
    let _ = fs::remove_dir_all(root);
}
