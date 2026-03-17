use super::*;

#[test]
fn parses_import_and_main_return_zero() {
    let src = r#"
import io;

fn main() -> Int {
  return 0;
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.imports.len(), 1);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportModule {
            path: vec!["io".to_string()],
            alias: None
        }
    );
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
    assert_eq!(program.functions[0].params.len(), 0);
    assert_eq!(program.functions[0].body.len(), 1);
    assert!(matches!(program.functions[0].body[0], Stmt::Return(_)));
}

#[test]
fn parses_import_module_dotted_path() {
    let src = r#"
import utils.math;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportModule {
            path: vec!["utils".to_string(), "math".to_string()],
            alias: None,
        }
    );
}

#[test]
fn parses_import_module_with_alias() {
    let src = r#"
import utils.math as m;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportModule {
            path: vec!["utils".to_string(), "math".to_string()],
            alias: Some("m".to_string()),
        }
    );
}

#[test]
fn parses_from_import_single_item() {
    let src = r#"
from utils.math import add;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportFrom {
            path: vec!["utils".to_string(), "math".to_string()],
            wildcard: false,
            items: vec![skeplib::ast::ImportItem {
                name: "add".to_string(),
                alias: None,
            }],
        }
    );
}

#[test]
fn parses_from_import_multiple_items_with_aliases() {
    let src = r#"
from utils.math import add, sub as minus;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportFrom {
            path: vec!["utils".to_string(), "math".to_string()],
            wildcard: false,
            items: vec![
                skeplib::ast::ImportItem {
                    name: "add".to_string(),
                    alias: None,
                },
                skeplib::ast::ImportItem {
                    name: "sub".to_string(),
                    alias: Some("minus".to_string()),
                }
            ],
        }
    );
}

#[test]
fn parses_from_import_wildcard() {
    let src = r#"
from utils.math import *;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(
        program.imports[0],
        skeplib::ast::ImportDecl::ImportFrom {
            path: vec!["utils".to_string(), "math".to_string()],
            wildcard: true,
            items: vec![],
        }
    );
}

#[test]
fn reports_duplicate_alias_in_same_from_import_clause() {
    let src = r#"
from utils.math import add as x, sub as x;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Duplicate import alias `x` in from-import clause");
}

#[test]
fn reports_duplicate_name_in_same_from_import_clause() {
    let src = r#"
from utils.math import add, add;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(
        &diags,
        "Duplicate imported symbol `add` in from-import clause",
    );
}

#[test]
fn parses_export_clause_basic() {
    let src = r#"
export { add, User, version };
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.exports.len(), 1);
    assert_eq!(
        match &program.exports[0] {
            skeplib::ast::ExportDecl::Local { items } => items.clone(),
            _ => panic!("expected local export"),
        },
        vec![
            skeplib::ast::ExportItem {
                name: "add".to_string(),
                alias: None,
            },
            skeplib::ast::ExportItem {
                name: "User".to_string(),
                alias: None,
            },
            skeplib::ast::ExportItem {
                name: "version".to_string(),
                alias: None,
            },
        ]
    );
}

#[test]
fn parses_export_clause_with_aliases() {
    let src = r#"
export { add as plus, sub };
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.exports.len(), 1);
    assert_eq!(
        match &program.exports[0] {
            skeplib::ast::ExportDecl::Local { items } => items.clone(),
            _ => panic!("expected local export"),
        },
        vec![
            skeplib::ast::ExportItem {
                name: "add".to_string(),
                alias: Some("plus".to_string()),
            },
            skeplib::ast::ExportItem {
                name: "sub".to_string(),
                alias: None,
            },
        ]
    );
}

#[test]
fn parses_export_from_clause() {
    let src = r#"
export { add as plus } from utils.math;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    match &program.exports[0] {
        skeplib::ast::ExportDecl::From { path, items } => {
            assert_eq!(path, &vec!["utils".to_string(), "math".to_string()]);
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "add");
            assert_eq!(items[0].alias.as_deref(), Some("plus"));
        }
        _ => panic!("expected export-from"),
    }
}

#[test]
fn parses_export_all_from_clause() {
    let src = r#"
export * from utils.math;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    match &program.exports[0] {
        skeplib::ast::ExportDecl::FromAll { path } => {
            assert_eq!(path, &vec!["utils".to_string(), "math".to_string()]);
        }
        _ => panic!("expected export-all-from"),
    }
}

#[test]
fn reports_empty_export_clause() {
    let src = r#"
export { };
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected at least one export item");
}

#[test]
fn reports_export_missing_brace_or_semicolon() {
    let src = r#"
export { add
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `}` after export list");
}

#[test]
fn accepts_multiple_export_blocks_in_one_file() {
    let src = r#"
export { a };
export { b };
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.exports.len(), 2);
}

#[test]
fn reports_export_inside_function_body() {
    let src = r#"
fn main() -> Int {
  export { add };
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "`export` is only allowed at top-level");
}

#[test]
fn reports_export_inside_if_block() {
    let src = r#"
fn main() -> Int {
  if (true) {
    export { add };
  }
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "`export` is only allowed at top-level");
}

#[test]
fn reports_from_import_leading_comma() {
    let src = r#"
from utils.math import , add;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(
        &diags,
        "Expected imported symbol name before `,` in from-import",
    );
}

#[test]
fn reports_from_import_trailing_comma() {
    let src = r#"
from utils.math import add,;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Trailing `,` is not allowed in from-import");
}

#[test]
fn reports_export_leading_comma() {
    let src = r#"
export { , add };
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected export symbol name before `,`");
}

#[test]
fn reports_export_trailing_comma() {
    let src = r#"
export { add, };
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Trailing `,` is not allowed in export list");
}

#[test]
fn reports_malformed_dotted_import_path() {
    let src = r#"
import a..b;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected identifier after `.` in module path");
}

#[test]
fn reports_import_path_starting_with_dot() {
    let src = r#"
import .a;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected module path after `import`");
}

#[test]
fn reports_import_path_ending_with_dot() {
    let src = r#"
import a.;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected identifier after `.` in module path");
}

#[test]
fn reports_malformed_dotted_from_import_path() {
    let src = r#"
from a..b import x;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected identifier after `.` in module path");
}

#[test]
fn reports_from_import_missing_item() {
    let src = r#"
from a.b import;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected imported symbol name after `import`");
}

#[test]
fn reports_import_alias_missing_identifier() {
    let src = r#"
import a.b as;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected alias name after `as`");
}

#[test]
fn reports_export_alias_missing_identifier() {
    let src = r#"
export { a as };
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected alias name after `as`");
}

#[test]
fn reports_from_import_wildcard_with_extra_items() {
    let src = r#"
from a.b import *, x;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `;` after from-import");
}

#[test]
fn reports_export_star_missing_from_clause() {
    let src = r#"
export *;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `from` after `export *`");
}

#[test]
fn parses_mixed_multiple_export_blocks() {
    let src = r#"
export { localA };
export { ext as extAlias } from pkg.mod;
export * from shared.core;
fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.exports.len(), 3);
    assert!(matches!(
        program.exports[0],
        skeplib::ast::ExportDecl::Local { .. }
    ));
    assert!(matches!(
        program.exports[1],
        skeplib::ast::ExportDecl::From { .. }
    ));
    assert!(matches!(
        program.exports[2],
        skeplib::ast::ExportDecl::FromAll { .. }
    ));
}

#[test]
fn reports_export_star_missing_module_path() {
    let src = r#"
export * from ;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected module path after `from`");
}

#[test]
fn reports_export_from_missing_symbol_item() {
    let src = r#"
export { } from a.b;
fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected at least one export item");
}

#[test]
fn malformed_import_export_combinations_recover_to_following_function() {
    let src = r#"
from a..b import x;
export * from ;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(
        diags.len() >= 2,
        "expected multiple diagnostics, got {:?}",
        diags.as_slice()
    );
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
}
