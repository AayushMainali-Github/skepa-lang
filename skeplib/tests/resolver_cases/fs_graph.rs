use super::*;

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
    assert_eq!(err.code, "E-MOD-AMBIG");
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
    assert!(errs.iter().any(|e| e.code == "E-MOD-CYCLE"));
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
    assert!(errs.iter().any(|e| e.code == "E-MOD-CYCLE"));
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
    assert!(errs.iter().any(|e| e.code == "E-MOD-NOT-FOUND"));
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
