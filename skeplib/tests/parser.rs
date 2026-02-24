mod common;

use common::{assert_has_diag, parse_err, parse_ok};
use skeplib::ast::{
    AssignTarget, BinaryOp, Expr, MatchLiteral, MatchPattern, Stmt, TypeName, UnaryOp,
};
use skeplib::parser::Parser;

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
fn reports_missing_semicolon_after_return() {
    let src = r#"
fn main() -> Int {
  return 0
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `;` after return statement");
}

#[test]
fn parses_typed_function_parameters() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return 0;
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.functions.len(), 1);
    let f = &program.functions[0];
    assert_eq!(f.name, "add");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].name, "a");
    assert_eq!(f.params[0].ty, TypeName::Int);
    assert_eq!(f.params[1].name, "b");
    assert_eq!(f.params[1].ty, TypeName::Int);
}

#[test]
fn parses_static_array_type_annotations() {
    let src = r#"
fn sum_row(row: [Int; 4]) -> [Int; 4] {
  return row;
}
"#;
    let program = parse_ok(src);
    let f = &program.functions[0];
    assert_eq!(
        f.params[0].ty,
        TypeName::Array {
            elem: Box::new(TypeName::Int),
            size: 4
        }
    );
    assert_eq!(
        f.return_type,
        Some(TypeName::Array {
            elem: Box::new(TypeName::Int),
            size: 4
        })
    );
}

#[test]
fn parses_nested_static_array_type_annotations() {
    let src = r#"
fn mat(m: [[Int; 3]; 2]) -> [[Int; 3]; 2] {
  return m;
}
"#;
    let program = parse_ok(src);
    let want = TypeName::Array {
        elem: Box::new(TypeName::Array {
            elem: Box::new(TypeName::Int),
            size: 3,
        }),
        size: 2,
    };
    assert_eq!(program.functions[0].params[0].ty, want.clone());
    assert_eq!(program.functions[0].return_type, Some(want));
}

#[test]
fn parses_function_type_annotations_in_params_and_return() {
    let src = r#"
fn apply(f: Fn(Int, Int) -> Int) -> Fn(Int, Int) -> Int {
  return f;
}
"#;
    let program = parse_ok(src);
    let f = &program.functions[0];
    assert_eq!(
        f.params[0].ty,
        TypeName::Fn {
            params: vec![TypeName::Int, TypeName::Int],
            ret: Box::new(TypeName::Int),
        }
    );
    assert_eq!(
        f.return_type,
        Some(TypeName::Fn {
            params: vec![TypeName::Int, TypeName::Int],
            ret: Box::new(TypeName::Int),
        })
    );
}

#[test]
fn reports_missing_arrow_in_function_type() {
    let src = r#"
fn bad(f: Fn(Int, Int) Int) -> Int {
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `->` after function type parameters");
}

#[test]
fn parses_function_literal_expression() {
    let src = r#"
fn main() -> Int {
  let f: Fn(Int) -> Int = fn(x: Int) -> Int {
    return x + 1;
  };
  return f(2);
}
"#;
    let program = parse_ok(src);
    let body = &program.functions[0].body;
    match &body[0] {
        Stmt::Let { value, .. } => match value {
            Expr::FnLit {
                params,
                return_type,
                body,
            } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");
                assert_eq!(params[0].ty, TypeName::Int);
                assert_eq!(*return_type, TypeName::Int);
                assert!(matches!(body[0], Stmt::Return(_)));
            }
            _ => panic!("expected fn literal in let value"),
        },
        _ => panic!("expected let statement"),
    }
}

#[test]
fn parses_immediate_function_literal_call() {
    let src = r#"
fn main() -> Int {
  return (fn(x: Int) -> Int { return x + 1; })(2);
}
"#;
    let program = parse_ok(src);
    let body = &program.functions[0].body;
    match &body[0] {
        Stmt::Return(Some(Expr::Call { callee, args })) => {
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], Expr::IntLit(2)));
            match callee.as_ref() {
                Expr::Group(inner) => assert!(matches!(inner.as_ref(), Expr::FnLit { .. })),
                _ => panic!("expected grouped fn literal callee"),
            }
        }
        _ => panic!("expected return call expression"),
    }
}

#[test]
fn parses_function_returning_function_literal_and_chained_call() {
    let src = r#"
fn makeInc() -> Fn(Int) -> Int {
  return fn(x: Int) -> Int { return x + 1; };
}

fn main() -> Int {
  return makeInc()(2);
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.functions.len(), 2);
    match &program.functions[0].body[0] {
        Stmt::Return(Some(Expr::FnLit { .. })) => {}
        _ => panic!("expected function literal return in makeInc"),
    }
    match &program.functions[1].body[0] {
        Stmt::Return(Some(Expr::Call { callee, args })) => {
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], Expr::IntLit(2)));
            assert!(matches!(callee.as_ref(), Expr::Call { .. }));
        }
        _ => panic!("expected chained call in main"),
    }
}

#[test]
fn parses_struct_field_with_function_type() {
    let src = r#"
struct Op {
  apply: Fn(Int, Int) -> Int
}

fn add(a: Int, b: Int) -> Int { return a + b; }

fn main() -> Int {
  let op: Op = Op { apply: add };
  return (op.apply)(2, 3);
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.structs.len(), 1);
    let s = &program.structs[0];
    assert_eq!(s.fields.len(), 1);
    assert_eq!(s.fields[0].name, "apply");
    assert_eq!(
        s.fields[0].ty,
        TypeName::Fn {
            params: vec![TypeName::Int, TypeName::Int],
            ret: Box::new(TypeName::Int),
        }
    );
}

#[test]
fn reports_missing_colon_in_parameter() {
    let src = r#"
fn add(a Int) -> Int {
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `:` after parameter name");
}

#[test]
fn parses_let_and_assignment_statements() {
    let src = r#"
fn main() -> Int {
  let x: Int = 1;
  let y = x;
  y = 2;
  return 0;
}
"#;
    let program = parse_ok(src);
    let body = &program.functions[0].body;
    assert_eq!(body.len(), 4);

    match &body[0] {
        Stmt::Let { name, ty, value } => {
            assert_eq!(name, "x");
            assert_eq!(*ty, Some(TypeName::Int));
            assert_eq!(*value, Expr::IntLit(1));
        }
        _ => panic!("expected let"),
    }

    match &body[1] {
        Stmt::Let { name, ty, value } => {
            assert_eq!(name, "y");
            assert_eq!(*ty, None);
            assert_eq!(*value, Expr::Ident("x".to_string()));
        }
        _ => panic!("expected let"),
    }

    match &body[2] {
        Stmt::Assign { target, value } => {
            assert_eq!(*target, AssignTarget::Ident("y".to_string()));
            assert_eq!(*value, Expr::IntLit(2));
        }
        _ => panic!("expected assignment"),
    }
}

#[test]
fn reports_missing_equals_in_let_declaration() {
    let src = r#"
fn main() -> Int {
  let x: Int 1;
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `=` in let declaration");
}

#[test]
fn parses_void_return_statement() {
    let src = r#"
fn log() -> Void {
  return;
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.functions.len(), 1);
    assert!(matches!(program.functions[0].body[0], Stmt::Return(None)));
}

#[test]
fn reports_missing_parameter_type() {
    let src = r#"
fn add(a:) -> Int {
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected parameter type after `:`");
}

#[test]
fn reports_missing_semicolon_after_assignment() {
    let src = r#"
fn main() -> Int {
  let x = 1;
  x = 2
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `;` after assignment"))
    );
}

#[test]
fn parses_array_literals_and_repeat_literals() {
    let src = r#"
fn main() -> Int {
  let a = [1, 2, 3];
  let b = [0; 8];
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => {
            assert!(matches!(value, Expr::ArrayLit(items) if items.len() == 3))
        }
        _ => panic!("expected let"),
    }
    match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => {
            assert!(matches!(value, Expr::ArrayRepeat { size, .. } if *size == 8))
        }
        _ => panic!("expected let"),
    }
}

#[test]
fn parses_path_assignment_target() {
    let src = r#"
fn main() -> Int {
  obj.field = 2;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Assign { target, value } => {
            assert!(matches!(target, AssignTarget::Field { .. }));
            assert_eq!(*value, Expr::IntLit(2));
        }
        _ => panic!("expected assignment"),
    }
}

#[test]
fn parses_index_expression_and_index_assignment_target() {
    let src = r#"
fn main() -> Int {
  let a = [1, 2, 3];
  let x = a[1];
  a[2] = x;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => assert!(matches!(value, Expr::Index { .. })),
        _ => panic!("expected index let"),
    }
    match &program.functions[0].body[2] {
        Stmt::Assign { target, .. } => assert!(matches!(target, AssignTarget::Index { .. })),
        _ => panic!("expected index assignment"),
    }
}

#[test]
fn parses_expression_statement() {
    let src = r#"
fn main() -> Int {
  ping;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert!(matches!(
        program.functions[0].body[0],
        Stmt::Expr(Expr::Ident(_))
    ));
}

#[test]
fn parses_call_expressions_for_ident_and_path() {
    let src = r#"
fn main() -> Int {
  hello(1, 2);
  io.println("ok");
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Expr(Expr::Call { callee, args }) => {
            assert!(matches!(&**callee, Expr::Ident(name) if name == "hello"));
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected call"),
    }
    match &program.functions[0].body[1] {
        Stmt::Expr(Expr::Call { callee, args }) => {
            assert!(matches!(&**callee, Expr::Field { .. }));
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected path call"),
    }
}

#[test]
fn reports_malformed_call_missing_right_paren() {
    let src = r#"
fn main() -> Int {
  hello(1, 2;
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `)` after call arguments"))
    );
}

#[test]
fn parses_unary_and_binary_with_precedence() {
    let src = r#"
fn main() -> Int {
  let x = -1 + 2 * 3 == 5 && !false || true;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());

    let expr = match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => value,
        _ => panic!("expected let"),
    };

    match expr {
        Expr::Binary {
            left,
            op: BinaryOp::OrOr,
            right,
        } => {
            assert!(matches!(**right, Expr::BoolLit(true)));
            match &**left {
                Expr::Binary {
                    op: BinaryOp::AndAnd,
                    ..
                } => {}
                _ => panic!("expected && on left of ||"),
            }
        }
        _ => panic!("expected top-level ||"),
    }
}

#[test]
fn parses_float_literal_expression() {
    let src = r#"
fn main() -> Float {
  return 3.14;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Return(Some(Expr::FloatLit(v))) => assert_eq!(v, "3.14"),
        other => panic!("expected float return, got {other:?}"),
    }
}

#[test]
fn parses_grouped_expression_shape() {
    let src = r#"
fn main() -> Int {
  let v = (1 + 2) * 3;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let expr = match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => value,
        _ => panic!("expected let"),
    };
    match expr {
        Expr::Binary {
            left,
            op: BinaryOp::Mul,
            right,
        } => {
            assert!(matches!(**right, Expr::IntLit(3)));
            match &**left {
                Expr::Group(inner) => assert!(matches!(
                    **inner,
                    Expr::Binary {
                        op: BinaryOp::Add,
                        ..
                    }
                )),
                _ => panic!("expected grouped left operand"),
            }
        }
        _ => panic!("expected multiply"),
    }
}

#[test]
fn parses_modulo_operator() {
    let src = r#"
fn main() -> Int {
  let x = 7 % 3;
  return x;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Let {
            value: Expr::Binary {
                op: BinaryOp::Mod, ..
            },
            ..
        } => {}
        _ => panic!("expected modulo expression"),
    }
}

#[test]
fn parses_unary_neg_and_not() {
    let src = r#"
fn main() -> Int {
  let a = -1;
  let p = +2;
  let b = !false;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => assert!(matches!(
            value,
            Expr::Unary {
                op: UnaryOp::Neg,
                ..
            }
        )),
        _ => panic!("expected let"),
    }
    match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => assert!(matches!(
            value,
            Expr::Unary {
                op: UnaryOp::Pos,
                ..
            }
        )),
        _ => panic!("expected let"),
    }
    match &program.functions[0].body[2] {
        Stmt::Let { value, .. } => assert!(matches!(
            value,
            Expr::Unary {
                op: UnaryOp::Not,
                ..
            }
        )),
        _ => panic!("expected let"),
    }
}

#[test]
fn parses_if_else_statement() {
    let src = r#"
fn main() -> Int {
  if (true) {
    return 1;
  } else {
    return 0;
  }
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            assert_eq!(*cond, Expr::BoolLit(true));
            assert_eq!(then_body.len(), 1);
            assert_eq!(else_body.len(), 1);
        }
        _ => panic!("expected if"),
    }
}

#[test]
fn parses_match_statement_with_literals_and_wildcard() {
    let src = r#"
fn main() -> Int {
  match (1) {
    0 => { return 10; }
    1 => { return 20; }
    _ => { return 30; }
  }
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Match { expr, arms } => {
            assert_eq!(*expr, Expr::IntLit(1));
            assert_eq!(arms.len(), 3);
            assert_eq!(arms[0].pattern, MatchPattern::Literal(MatchLiteral::Int(0)));
            assert_eq!(arms[1].pattern, MatchPattern::Literal(MatchLiteral::Int(1)));
            assert_eq!(arms[2].pattern, MatchPattern::Wildcard);
            assert!(matches!(arms[0].body[0], Stmt::Return(_)));
        }
        _ => panic!("expected match statement"),
    }
}

#[test]
fn parses_match_or_pattern_and_string_pattern() {
    let src = r#"
fn main() -> Int {
  match ("y") {
    "y" | "Y" => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Match { arms, .. } => match &arms[0].pattern {
            MatchPattern::Or(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(
                    parts[0],
                    MatchPattern::Literal(MatchLiteral::String("y".to_string()))
                );
                assert_eq!(
                    parts[1],
                    MatchPattern::Literal(MatchLiteral::String("Y".to_string()))
                );
            }
            _ => panic!("expected or-pattern"),
        },
        _ => panic!("expected match statement"),
    }
}

#[test]
fn reports_match_missing_fat_arrow() {
    let src = r#"
fn main() -> Int {
  match (1) {
    1 { return 1; }
    _ => { return 0; }
  }
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `=>` after match pattern");
}

#[test]
fn reports_empty_match_body() {
    let src = r#"
fn main() -> Int {
  match (1) {
  }
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected at least one match arm");
}

#[test]
fn reports_invalid_match_pattern_identifier() {
    let src = r#"
fn main() -> Int {
  match (1) {
    x => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected match pattern (`_` or literal)");
}

#[test]
fn parses_while_statement() {
    let src = r#"
fn main() -> Int {
  while (true) {
    return 0;
  }
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::While { cond, body } => {
            assert_eq!(*cond, Expr::BoolLit(true));
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected while"),
    }
}

#[test]
fn parses_break_and_continue_in_while() {
    let src = r#"
fn main() -> Int {
  while (true) {
    continue;
    break;
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::While { body, .. } => {
            assert!(matches!(body[0], Stmt::Continue));
            assert!(matches!(body[1], Stmt::Break));
        }
        _ => panic!("expected while"),
    }
}

#[test]
fn parses_for_statement_with_all_clauses() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 10; i = i + 1) {
    ping(i);
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            assert!(init.is_some());
            assert!(cond.is_some());
            assert!(step.is_some());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn parses_for_with_no_clauses() {
    let src = r#"
fn main() -> Int {
  for (;;) {
    break;
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            assert!(init.is_none());
            assert!(cond.is_none());
            assert!(step.is_none());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn parses_for_with_only_init_clause() {
    let src = r#"
fn main() -> Int {
  for (let i = 0;;) {
    break;
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            assert!(init.is_some());
            assert!(cond.is_none());
            assert!(step.is_none());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn parses_for_with_only_condition_clause() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  for (; i < 3;) {
    break;
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[1] {
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            assert!(init.is_none());
            assert!(cond.is_some());
            assert!(step.is_none());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn parses_for_with_only_step_clause() {
    let src = r#"
fn main() -> Int {
  let i = 0;
  for (;; i = i + 1) {
    break;
  }
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[1] {
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            assert!(init.is_none());
            assert!(cond.is_none());
            assert!(step.is_some());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected for"),
    }
}

#[test]
fn parses_nested_blocks_in_if_and_while() {
    let src = r#"
fn main() -> Int {
  if (true) {
    while (false) {
      ping();
    }
  } else {
    return 0;
  }
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::If { then_body, .. } => match &then_body[0] {
            Stmt::While { body, .. } => {
                assert!(matches!(body[0], Stmt::Expr(_)));
            }
            _ => panic!("expected nested while"),
        },
        _ => panic!("expected outer if"),
    }
}

#[test]
fn reports_missing_paren_after_if_condition() {
    let src = r#"
fn main() -> Int {
  if (true {
    return 0;
  }
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `)` after if condition"))
    );
}

#[test]
fn reports_missing_block_after_while() {
    let src = r#"
fn main() -> Int {
  while (true)
    return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `{` before while body"))
    );
}

#[test]
fn reports_missing_first_semicolon_in_for_header() {
    let src = r#"
fn main() -> Int {
  for (let i = 0 i < 3; i = i + 1) {
    ping(i);
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `;` after for init clause"))
    );
}

#[test]
fn reports_missing_second_semicolon_in_for_header() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 3 i = i + 1) {
    ping(i);
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `;` after for condition"))
    );
}

#[test]
fn reports_missing_right_paren_in_for_header() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 3; i = i + 1 {
    ping(i);
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `)` after for clauses"))
    );
}

#[test]
fn reports_invalid_return_in_for_init_clause() {
    let src = r#"
fn main() -> Int {
  for (return 1; true; ) {
    return 0;
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected expression"))
    );
}

#[test]
fn reports_invalid_break_in_for_step_clause() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 3; break) {
    return 0;
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected expression"))
    );
}

#[test]
fn reports_invalid_assignment_target_in_for_step_clause() {
    let src = r#"
fn main() -> Int {
  for (let i = 0; i < 3; (i + 1) = 2) {
    return 0;
  }
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `)` after for clauses"))
    );
}

#[test]
fn parser_recovers_and_parses_next_statement_after_error() {
    let src = r#"
fn main() -> Int {
  let x = ;
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(
        program.functions[0]
            .body
            .iter()
            .any(|s| matches!(s, Stmt::Return(Some(Expr::IntLit(0)))))
    );
}

#[test]
fn diagnostics_include_found_token_context() {
    let src = r#"
fn main() -> Int {
  let x Int = 1;
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("found `Int`"))
    );
}

#[test]
fn parses_else_if_chain() {
    let src = r#"
fn main() -> Int {
  if (false) {
    return 1;
  } else if (true) {
    return 2;
  } else {
    return 3;
  }
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::If { else_body, .. } => {
            assert_eq!(else_body.len(), 1);
            assert!(matches!(else_body[0], Stmt::If { .. }));
        }
        _ => panic!("expected if"),
    }
}

#[test]
fn parses_escaped_string_literals() {
    let src = r#"
fn main() -> Int {
  io.println("line1\nline2\t\"ok\"\\");
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Expr(Expr::Call { args, .. }) => {
            assert_eq!(args.len(), 1);
            match &args[0] {
                Expr::StringLit(s) => {
                    assert!(s.contains('\n'));
                    assert!(s.contains('\t'));
                    assert!(s.contains("\"ok\""));
                    assert!(s.ends_with('\\'));
                }
                _ => panic!("expected string arg"),
            }
        }
        _ => panic!("expected call expression statement"),
    }
}

#[test]
fn reports_invalid_escape_sequence_in_string() {
    let src = r#"
fn main() -> Int {
  io.println("bad\q");
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Invalid escape sequence"))
    );
}

#[test]
fn accepts_trailing_comma_in_call_arguments() {
    let src = r#"
fn main() -> Int {
  hello(1,);
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
}

#[test]
fn accepts_trailing_comma_in_function_params() {
    let src = r#"
fn add(a: Int, b: Int,) -> Int {
  return a + b;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn accepts_top_level_global_let_declaration() {
    let src = r#"
let x = 1;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(program.globals.len(), 1);
    assert_eq!(program.globals[0].name, "x");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn recovers_after_top_level_error_and_parses_following_items() {
    let src = r#"
?? nonsense
import io;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn reports_missing_comma_between_call_arguments() {
    let src = r#"
fn main() -> Int {
  hello(1 2);
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected `)` after call arguments"))
    );
}

#[test]
fn reports_leading_comma_in_call_arguments() {
    let src = r#"
fn main() -> Int {
  hello(,1);
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected expression before `,` in call"))
    );
}

#[test]
fn parser_collects_multiple_errors_in_one_function() {
    let src = r#"
fn main() -> Int {
  let x = ;
  hello(1,);
  return 0
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(
        diags.len() >= 2,
        "expected multiple diagnostics, got {:?}",
        diags.as_slice()
    );
}

#[test]
fn parses_chained_call_on_call_expression() {
    let src = r#"
fn main() -> Int {
  make()(1);
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Expr(Expr::Call { callee, args }) => {
            assert_eq!(args.len(), 1);
            assert!(matches!(&**callee, Expr::Call { .. }));
        }
        _ => panic!("expected chained call"),
    }
}

#[test]
fn parses_nested_group_and_unary_expression() {
    let src = r#"
fn main() -> Int {
  let x = !((1 + 2) == 3);
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => match value {
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
            } => {
                assert!(matches!(&**expr, Expr::Group(_)));
            }
            _ => panic!("expected unary not"),
        },
        _ => panic!("expected let"),
    }
}

#[test]
fn parses_struct_declaration_with_typed_fields() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  return 0;
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.structs.len(), 1);
    let s = &program.structs[0];
    assert_eq!(s.name, "User");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "id");
    assert_eq!(s.fields[1].name, "name");
}

#[test]
fn parses_impl_methods_with_self_and_params() {
    let src = r#"
struct User { id: Int, name: String }

impl User {
  fn greet(self) -> String {
    return self.name;
  }

  fn label(self, prefix: String) -> String {
    return prefix + self.name;
  }
}

fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.impls.len(), 1);
    let imp = &program.impls[0];
    assert_eq!(imp.target, "User");
    assert_eq!(imp.methods.len(), 2);
    assert_eq!(imp.methods[0].params[0].name, "self");
    assert_eq!(
        imp.methods[0].params[0].ty,
        TypeName::Named("User".to_string())
    );
    assert_eq!(imp.methods[1].params.len(), 2);
}

#[test]
fn reports_invalid_struct_field_missing_colon() {
    let src = r#"
struct User {
  id Int,
}

fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `:` after field name");
}

#[test]
fn parses_struct_literal_field_access_and_field_assignment_target() {
    let src = r#"
fn main() -> Int {
  let u = User { id: 1, name: "sam" };
  let n = u.name;
  u.name = "max";
  return 0;
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => assert!(matches!(value, Expr::StructLit { .. })),
        _ => panic!("expected struct literal"),
    }
    match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => assert!(matches!(value, Expr::Field { .. })),
        _ => panic!("expected field access"),
    }
    match &program.functions[0].body[2] {
        Stmt::Assign { target, .. } => assert!(matches!(target, AssignTarget::Field { .. })),
        _ => panic!("expected field assignment target"),
    }
}

#[test]
fn parses_vec_type_annotations() {
    let src = r#"
fn take(xs: Vec[Int]) -> Vec[String] {
  let ys: Vec[String] = vec.new();
  return ys;
}
"#;
    let program = parse_ok(src);
    let f = &program.functions[0];
    assert_eq!(f.params[0].ty.as_str(), "Vec[Int]");
    assert_eq!(f.return_type.as_ref().expect("ret").as_str(), "Vec[String]");
    match &f.body[0] {
        Stmt::Let { ty: Some(ty), .. } => assert_eq!(ty.as_str(), "Vec[String]"),
        _ => panic!("expected typed let"),
    }
}

#[test]
fn reports_malformed_vec_type_syntax() {
    let src = r#"
fn main() -> Int {
  let xs: Vec[] = 0;
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected vector element type");
}
