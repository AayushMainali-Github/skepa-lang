mod common;

use skeplib::resolver::{ResolveErrorKind, resolve_project};

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
fn rejects_namespace_reexports_until_first_class_support_exists() {
    let project = common::TempProject::new("namespace_reexport_rejected");
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

    let errs = resolve_project(&entry).expect_err("namespace re-export expected");
    assert!(errs.iter().any(|e| {
        e.kind == ResolveErrorKind::ExportUnknown
            && e.message.contains("Cannot export module namespace `tools`")
    }));
}
