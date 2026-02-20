use skeplib::resolver::{
    module_id_from_relative_path, resolve_project, ModuleGraph, ModuleUnit, ResolveErrorKind,
};
use std::collections::HashMap;
use std::path::Path;

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
