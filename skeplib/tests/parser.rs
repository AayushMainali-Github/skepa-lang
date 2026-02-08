use skeplib::ast::{Expr, Stmt, TypeName};
use skeplib::parser::Parser;

#[test]
fn parses_import_and_main_return_zero() {
    let src = r#"
import io;

fn main() -> Int {
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].module, "io");
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
    assert_eq!(program.functions[0].params.len(), 0);
    assert_eq!(program.functions[0].body.len(), 1);
    assert!(matches!(program.functions[0].body[0], Stmt::Return(_)));
}

#[test]
fn reports_missing_semicolon_after_return() {
    let src = r#"
fn main() -> Int {
  return 0
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Expected `;` after return statement")));
}

#[test]
fn parses_typed_function_parameters() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
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
fn reports_missing_colon_in_parameter() {
    let src = r#"
fn add(a Int) -> Int {
  return 0;
}
"#;
    let (_program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Expected `:` after parameter name")));
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
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
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
        Stmt::Assign { name, value } => {
            assert_eq!(name, "y");
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
    let (_program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(diags
        .as_slice()
        .iter()
        .any(|d| d.message.contains("Expected `=` in let declaration")));
}
