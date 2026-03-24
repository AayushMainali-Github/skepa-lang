use super::*;
use crate::common::assert_no_diags;
use skeplib::resolver::ResolveError;

#[test]
fn resolver_graph_types_construct_cleanly() {
    let mut modules = HashMap::new();
    modules.insert(
        "main".to_string(),
        ModuleUnit {
            id: "main".to_string(),
            path: Path::new("main.sk").to_path_buf(),
            source: "fn main() -> Int { return 0; }".to_string(),
            program: Program {
                functions: vec![],
                imports: vec![],
                globals: vec![],
                exports: vec![],
                structs: vec![],
                impls: vec![],
                operators: vec![],
            },
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
    assert_eq!(err[0].code, "E-MOD-NOT-FOUND");
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
    assert_eq!(err.kind, ResolveErrorKind::InvalidModulePath);
    assert_eq!(err.code, "E-MOD-PATH");
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
    let symbols = collect_module_symbols(&program, "main");
    let errs = validate_and_build_export_map(&program, &symbols, "main", Path::new("main.sk"))
        .expect_err("unknown export should fail");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Exported name `nope` does not exist"))
    );
    assert!(errs.iter().any(|e| e.code == "E-EXPORT-UNKNOWN"));
    assert!(
        errs.iter()
            .any(|e| e.kind == ResolveErrorKind::ExportUnknown)
    );
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
    assert_no_diags(&diags);
    let symbols = collect_module_symbols(&program, "main");
    let errs = validate_and_build_export_map(&program, &symbols, "main", Path::new("main.sk"))
        .expect_err("duplicate export target should fail");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate exported target name `x`"))
    );
    assert!(
        errs.iter()
            .any(|e| e.kind == ResolveErrorKind::ImportConflict)
    );
}

#[test]
fn resolve_project_reports_parse_errors_and_does_not_mask_them_as_missing_module() {
    let root = make_temp_dir("parse_failure");
    fs::write(
        root.join("main.sk"),
        r#"
fn main() -> Int {
  let x = ;
  return 0;
}
"#,
    )
    .expect("write malformed entry");

    let errs = resolve_project(&root.join("main.sk")).expect_err("parse should fail resolution");
    assert!(errs.iter().any(|e| e.kind == ResolveErrorKind::Parse));
    assert!(errs.iter().any(|e| e.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Expected expression"))
    );
}

#[test]
fn resolve_error_codes_distinguish_duplicate_io_and_path_failures() {
    let io_err = ResolveError::new(ResolveErrorKind::Io, "io", None);
    assert_eq!(io_err.code, "E-MOD-IO");

    let path_err = ResolveError::new(ResolveErrorKind::NonUtf8Path, "path", None);
    assert_eq!(path_err.code, "E-MOD-PATH");

    let invalid_path_err = ResolveError::new(ResolveErrorKind::InvalidModulePath, "bad", None);
    assert_eq!(invalid_path_err.code, "E-MOD-PATH");

    let dup_err = ResolveError::new(ResolveErrorKind::DuplicateModuleId, "dup", None);
    assert_eq!(dup_err.code, "E-MOD-DUPLICATE");

    let conflict_err = ResolveError::new(ResolveErrorKind::ImportConflict, "conflict", None);
    assert_eq!(conflict_err.code, "E-IMPORT-CONFLICT");

    let not_exported_err = ResolveError::new(ResolveErrorKind::NotExported, "missing", None);
    assert_eq!(not_exported_err.code, "E-IMPORT-NOT-EXPORTED");

    let export_unknown_err = ResolveError::new(ResolveErrorKind::ExportUnknown, "unknown", None);
    assert_eq!(export_unknown_err.code, "E-EXPORT-UNKNOWN");
}
