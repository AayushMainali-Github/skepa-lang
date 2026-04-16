use super::*;

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
    assert!(
        errs.iter().any(|e| {
            e.message.contains("symbol is not exported") && e.message.contains("hidden")
        })
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-NOT-EXPORTED"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_missing_imported_operator_before_parse_error() {
    let root = make_temp_dir("missing_imported_operator_ordering");
    fs::write(
        root.join("a.sk"),
        r#"
fn shown() -> Int { return 1; }
export { shown };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("main.sk"),
        r#"
from a import xoxo;
fn main() -> Int { return 1 `xoxo` 2; }
"#,
    )
    .expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("missing operator export expected");
    assert!(errs.iter().any(|e| {
        e.code == "E-IMPORT-NOT-EXPORTED"
            && e.message.contains("Cannot import operator `xoxo`")
    }));
    assert!(!errs.iter().any(|e| e.kind == ResolveErrorKind::Parse));
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
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate imported binding `x`"))
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-CONFLICT"));
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

    let errs =
        resolve_project(&root.join("main.sk")).expect_err("namespace-root from-import expected");
    assert!(
        errs.iter()
            .any(|e| e.kind == ResolveErrorKind::AmbiguousModule
                && e.message.contains("resolves to a namespace root"))
    );
    assert!(errs.iter().any(|e| e.code == "E-MOD-AMBIG"));
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
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate imported binding `x`"))
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-CONFLICT"));
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
    assert!(errs.iter().any(|e| e.code == "E-MOD-CYCLE"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_re_export_of_non_exported_symbol() {
    let root = make_temp_dir("re_export_non_exported_symbol");
    fs::write(
        root.join("a.sk"),
        r#"
fn hidden() -> Int { return 1; }
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
export { hidden } from a;
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
import b;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = resolve_project(&root.join("main.sk")).expect_err("re-export unknown should fail");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Cannot re-export `hidden`"))
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-NOT-EXPORTED"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_duplicate_export_name_between_local_and_export_all() {
    let root = make_temp_dir("dup_local_and_export_all");
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
export * from a;
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
import b;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs =
        resolve_project(&root.join("main.sk")).expect_err("duplicate export target expected");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate exported target name `x`"))
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-CONFLICT"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_wildcard_and_explicit_import_binding_conflict() {
    let root = make_temp_dir("wildcard_and_explicit_conflict");
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
fn y() -> Int { return 2; }
export { y };
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
from a import *;
from b import y as x;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs =
        resolve_project(&root.join("main.sk")).expect_err("import binding conflict expected");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate imported binding `x`"))
    );
    assert!(errs.iter().any(|e| e.code == "E-IMPORT-CONFLICT"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_supports_namespace_export_alias_and_import() {
    let root = make_temp_dir("namespace_export_alias");
    fs::create_dir_all(root.join("string")).expect("create string dir");
    fs::write(
        root.join("string").join("case.sk"),
        r#"
fn up(s: String) -> String { return s; }
export { up };
"#,
    )
    .expect("write case");
    fs::write(
        root.join("mod.sk"),
        r#"
import string.case as case;
export { case as caseTools };
"#,
    )
    .expect("write mod");
    fs::write(
        root.join("main.sk"),
        r#"
from mod import caseTools;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let graph = resolve_project(&root.join("main.sk")).expect("namespace export should resolve");
    assert!(graph.modules.contains_key("mod"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_suggests_close_symbol_name_for_import() {
    let root = make_temp_dir("import_did_you_mean");
    fs::write(
        root.join("a.sk"),
        r#"
fn shown() -> Int { return 1; }
export { shown };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("main.sk"),
        r#"
from a import shwon;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = resolve_project(&root.join("main.sk")).expect_err("did-you-mean expected");
    assert!(
        errs.iter().any(
            |e| e.code == "E-IMPORT-NOT-EXPORTED" && e.message.contains("did you mean `shown`")
        )
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_suggests_close_symbol_name_for_re_export() {
    let root = make_temp_dir("reexport_did_you_mean");
    fs::write(
        root.join("a.sk"),
        r#"
fn shown() -> Int { return 1; }
export { shown };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
export { shwon } from a;
"#,
    )
    .expect("write b");
    fs::write(
        root.join("main.sk"),
        r#"
import b;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs = resolve_project(&root.join("main.sk")).expect_err("did-you-mean re-export expected");
    assert!(errs.iter().any(|e| {
        e.code == "E-IMPORT-NOT-EXPORTED" && e.message.contains("did you mean `shown`")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_duplicate_import_binding_from_wildcard_and_explicit_same_module() {
    let root = make_temp_dir("wildcard_plus_explicit_same_module_conflict");
    fs::write(
        root.join("a.sk"),
        r#"
fn x() -> Int { return 1; }
export { x };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("main.sk"),
        r#"
from a import *;
from a import x;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs =
        resolve_project(&root.join("main.sk")).expect_err("duplicate imported binding expected");
    assert!(
        errs.iter().any(|e| e.code == "E-IMPORT-CONFLICT"
            && e.message.contains("Duplicate imported binding `x`"))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_rejects_re_export_name_collision_between_from_blocks() {
    let root = make_temp_dir("reexport_name_collision_between_from_blocks");
    fs::write(
        root.join("a.sk"),
        r#"
fn f() -> Int { return 1; }
export { f };
"#,
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        r#"
fn g() -> Int { return 2; }
export { g };
"#,
    )
    .expect("write b");
    fs::write(
        root.join("m.sk"),
        r#"
export { f as z } from a;
export { g as z } from b;
"#,
    )
    .expect("write m");
    fs::write(
        root.join("main.sk"),
        r#"
import m;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    let errs =
        resolve_project(&root.join("main.sk")).expect_err("duplicate export target expected");
    assert!(errs.iter().any(|e| {
        e.code == "E-IMPORT-CONFLICT" && e.message.contains("Duplicate exported target name `z`")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_supports_deep_re_export_chain() {
    let root = make_temp_dir("deep_reexport_chain");
    fs::write(
        root.join("a.sk"),
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    )
    .expect("write a");
    fs::write(root.join("b.sk"), "export { add } from a;\n").expect("write b");
    fs::write(root.join("c.sk"), "export { add } from b;\n").expect("write c");
    fs::write(
        root.join("main.sk"),
        r#"
from c import add;
fn main() -> Int { return add(1, 2); }
"#,
    )
    .expect("write main");

    let graph = resolve_project(&root.join("main.sk")).expect("deep re-export should resolve");
    assert!(graph.modules.contains_key("a"));
    assert!(graph.modules.contains_key("b"));
    assert!(graph.modules.contains_key("c"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_allows_same_leaf_module_name_in_different_folders() {
    let root = make_temp_dir("same_leaf_module_names");
    fs::create_dir_all(root.join("a")).expect("create a");
    fs::create_dir_all(root.join("b")).expect("create b");
    fs::write(
        root.join("a").join("common.sk"),
        "fn left() -> Int { return 1; } export { left };",
    )
    .expect("write a/common");
    fs::write(
        root.join("b").join("common.sk"),
        "fn right() -> Int { return 2; } export { right };",
    )
    .expect("write b/common");
    fs::write(
        root.join("main.sk"),
        r#"
from a.common import left;
from b.common import right;
fn main() -> Int { return left() + right(); }
"#,
    )
    .expect("write main");

    let graph = resolve_project(&root.join("main.sk")).expect("nested same-name modules");
    assert!(graph.modules.contains_key("a.common"));
    assert!(graph.modules.contains_key("b.common"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_builds_larger_shared_dependency_graph() {
    let root = make_temp_dir("large_project_graph");
    fs::create_dir_all(root.join("pkg")).expect("create pkg");
    fs::write(
        root.join("main.sk"),
        r#"
import a;
import d;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");
    fs::write(root.join("a.sk"), "import b;\nimport c;\n").expect("write a");
    fs::write(root.join("b.sk"), "import pkg.shared;\n").expect("write b");
    fs::write(root.join("c.sk"), "import pkg.shared;\n").expect("write c");
    fs::write(root.join("d.sk"), "import e;\n").expect("write d");
    fs::write(root.join("e.sk"), "import pkg.shared;\n").expect("write e");
    fs::write(
        root.join("pkg").join("shared.sk"),
        "fn util() -> Int { return 1; }\n",
    )
    .expect("write shared");

    let graph = resolve_project(&root.join("main.sk")).expect("large graph should resolve");
    assert_eq!(graph.modules.len(), 7);
    assert_eq!(graph.modules["main"].imports.len(), 2);
    assert_eq!(graph.modules["pkg.shared"].imports.len(), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_shadowing_across_several_modules() {
    let root = make_temp_dir("shadowing_across_modules");
    fs::write(
        root.join("a.sk"),
        "fn one() -> Int { return 1; } export { one };",
    )
    .expect("write a");
    fs::write(
        root.join("b.sk"),
        "fn two() -> Int { return 2; } export { two };",
    )
    .expect("write b");
    fs::write(
        root.join("c.sk"),
        "fn three() -> Int { return 3; } export { three };",
    )
    .expect("write c");
    fs::write(
        root.join("main.sk"),
        r#"
from a import one as shared;
from b import two as shared;
from c import three as shared;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("shadowing conflict expected");
    let dups = errs
        .iter()
        .filter(|e| {
            e.code == "E-IMPORT-CONFLICT"
                && e.message.contains("Duplicate imported binding `shared`")
        })
        .count();
    assert!(dups >= 2, "errors: {errs:?}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_reports_bad_import_chain_with_intermediate_module_context() {
    let root = make_temp_dir("bad_import_chain");
    fs::write(
        root.join("main.sk"),
        "import a;\nfn main() -> Int { return 0; }\n",
    )
    .expect("write main");
    fs::write(root.join("a.sk"), "import b;\n").expect("write a");
    fs::write(root.join("b.sk"), "import missing.deep;\n").expect("write b");

    let errs = resolve_project(&root.join("main.sk")).expect_err("missing deep import expected");
    assert!(errs.iter().any(|e| {
        e.code == "E-MOD-NOT-FOUND"
            && e.message.contains("missing.deep")
            && e.message.contains("module `b`")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_project_respects_case_sensitivity_where_relevant() {
    if cfg!(windows) {
        return;
    }
    let root = make_temp_dir("case_sensitive_imports");
    fs::write(root.join("util.sk"), "fn value() -> Int { return 1; }\n").expect("write util");
    fs::write(
        root.join("main.sk"),
        r#"
import Util;
fn main() -> Int { return 0; }
"#,
    )
    .expect("write main");

    let errs = resolve_project(&root.join("main.sk")).expect_err("case mismatch should fail");
    assert!(
        errs.iter()
            .any(|e| e.code == "E-MOD-NOT-FOUND" && e.message.contains("Util"))
    );
    let _ = fs::remove_dir_all(root);
}
