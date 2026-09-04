mod common;

use skeplib::resolver::{ResolveErrorKind, build_export_maps, resolve_project};

#[cfg(unix)]
#[test]
fn rejects_symlinked_module_aliases() {
    use std::os::unix::fs::symlink;

    let project = common::TempProject::new("symlinked_module_alias");
    project.file(
        "b.sk",
        "fn value() -> Int { return 1; }\nexport { value };\n",
    );
    symlink(project.root().join("b.sk"), project.root().join("link.sk"))
        .expect("create module symlink");
    let entry = project.file(
        "main.sk",
        r#"
from b import value;
from link import value as linked;
fn main() -> Int { return value() + linked(); }
"#,
    );

    let errs = resolve_project(&entry).expect_err("symlink alias expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::DuplicateModuleId
            && e.message.contains("Duplicate module path")
            && e.message.contains("b.sk")
            && e.message.contains("link.sk")
    }));
}

#[test]
fn rejects_unaliased_module_namespace_conflict_with_direct_import() {
    let project = common::TempProject::new("module_namespace_direct_conflict");
    project.file(
        "a.sk",
        "fn local() -> Int { return 1; }\nexport { local };\n",
    );
    project.file("b.sk", "fn a() -> Int { return 2; }\nexport { a };\n");
    let entry = project.file(
        "main.sk",
        r#"
import a;
from b import a;
fn main() -> Int { return 0; }
"#,
    );

    let errs = resolve_project(&entry).expect_err("namespace/import conflict expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::ImportConflict
            && e.message.contains("Duplicate imported binding `a`")
    }));
}

#[test]
fn rejects_unaliased_module_namespace_conflict_with_wildcard_import() {
    let project = common::TempProject::new("module_namespace_wildcard_conflict");
    project.file(
        "a.sk",
        "fn local() -> Int { return 1; }\nexport { local };\n",
    );
    project.file("b.sk", "fn a() -> Int { return 2; }\nexport { a };\n");
    let entry = project.file(
        "main.sk",
        r#"
import a;
from b import *;
fn main() -> Int { return 0; }
"#,
    );

    let errs = resolve_project(&entry).expect_err("namespace/wildcard conflict expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::ImportConflict
            && e.message.contains("Duplicate imported binding `a`")
    }));
}

#[test]
fn reports_missing_imported_operator_as_resolver_error_before_parse() {
    let project = common::TempProject::new("missing_imported_operator_preparse");
    project.file(
        "a.sk",
        "fn value() -> Int { return 1; }\nexport { value };\n",
    );
    let entry = project.file(
        "main.sk",
        r#"
from a import xoxo;
fn main() -> Int { return 1 `xoxo` 2; }
"#,
    );

    let errs = resolve_project(&entry).expect_err("missing operator export expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::NotExported
            && e.message.contains("Cannot import operator `xoxo`")
    }));
    assert!(!errs.iter().any(|e| e.kind == ResolveErrorKind::Parse));
}

#[test]
fn accepts_operator_export_declared_before_opr() {
    let project = common::TempProject::new("export_before_opr_precedence");
    project.file(
        "ops.sk",
        r#"
export { add };
opr add(a: Int, b: Int) -> Int precedence 5 { return a + b; }
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from ops import add;
fn main() -> Int { return 2 `add` 3; }
"#,
    );

    resolve_project(&entry).expect("export before opr should still export operator precedence");
}

#[test]
fn rejects_duplicate_imported_operator_precedence_before_parse() {
    let project = common::TempProject::new("duplicate_imported_operator_precedence");
    project.file(
        "a.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 3 { return lhs + rhs; }
export { xoxo };
"#,
    );
    project.file(
        "b.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 { return lhs + rhs; }
export { xoxo };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from a import xoxo;
from b import xoxo;
fn main() -> Int { return 1 `xoxo` 2; }
"#,
    );

    let errs = resolve_project(&entry).expect_err("duplicate operator precedence expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::ImportConflict
            && e.message
                .contains("Duplicate imported operator precedence `xoxo`")
    }));
    assert!(!errs.iter().any(|e| e.kind == ResolveErrorKind::Parse));
}

#[test]
fn rejects_duplicate_operator_precedence_reexport_collision() {
    let project = common::TempProject::new("duplicate_operator_precedence_reexport");
    project.file(
        "a.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 3 { return lhs + rhs; }
export { xoxo };
"#,
    );
    project.file(
        "b.sk",
        r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 { return lhs + rhs; }
export { xoxo };
"#,
    );
    project.file(
        "c.sk",
        r#"
export * from a;
export * from b;
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from c import xoxo;
fn main() -> Int { return 1 `xoxo` 2; }
"#,
    );

    let errs = resolve_project(&entry).expect_err("duplicate re-export precedence expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::ImportConflict
            && e.message
                .contains("Duplicate exported operator precedence `xoxo`")
    }));
    assert!(!errs.iter().any(|e| e.kind == ResolveErrorKind::Parse));
}

#[test]
fn supports_namespace_reexports_with_aliases() {
    let project = common::TempProject::new("namespace_reexport");
    project.file(
        "tools.sk",
        "fn value() -> Int { return 1; }\nexport { value };\n",
    );
    project.file(
        "mod.sk",
        r#"
import tools;
export { tools as toolset };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from mod import toolset;
fn main() -> Int { return 0; }
"#,
    );

    let graph = resolve_project(&entry).expect("namespace re-export should resolve");
    let exports = build_export_maps(&graph).expect("export map should resolve");
    let symbol = exports
        .get("mod")
        .and_then(|map| map.get("toolset"))
        .expect("aliased namespace export");
    assert_eq!(symbol.kind, skeplib::resolver::SymbolKind::Namespace);
    assert_eq!(symbol.local_name, "tools");
}
