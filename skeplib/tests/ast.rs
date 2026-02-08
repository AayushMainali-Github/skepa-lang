use skeplib::ast::{Expr, FnDecl, ImportDecl, Param, Program, Stmt, TypeName};
use skeplib::parser::Parser;

#[test]
fn create_empty_program() {
    let program = Program::default();
    assert!(program.imports.is_empty());
    assert!(program.functions.is_empty());
}

#[test]
fn create_program_with_one_import_and_one_function() {
    let program = Program {
        imports: vec![ImportDecl {
            module: "io".to_string(),
        }],
        functions: vec![FnDecl {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: Some(TypeName::Int),
            body: Vec::new(),
        }],
    };

    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].module, "io");
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
    assert_eq!(program.functions[0].return_type, Some(TypeName::Int));
}

#[test]
fn function_can_store_return_zero_stmt() {
    let function = FnDecl {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: Some(TypeName::Int),
        body: vec![Stmt::Return(Some(Expr::IntLit(0)))],
    };

    assert_eq!(function.body.len(), 1);
    assert_eq!(function.body[0], Stmt::Return(Some(Expr::IntLit(0))));
}

#[test]
fn int_literal_value_is_preserved() {
    let expr = Expr::IntLit(42);
    match expr {
        Expr::IntLit(v) => assert_eq!(v, 42),
        _ => unreachable!("expected int literal"),
    }
}

#[test]
fn function_can_store_params_and_return_type() {
    let function = FnDecl {
        name: "add".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                ty: TypeName::Int,
            },
            Param {
                name: "b".to_string(),
                ty: TypeName::Int,
            },
        ],
        return_type: Some(TypeName::Int),
        body: vec![Stmt::Return(Some(Expr::IntLit(0)))],
    };
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].name, "a");
    assert_eq!(function.params[0].ty, TypeName::Int);
    assert_eq!(function.return_type, Some(TypeName::Int));
}

#[test]
fn program_pretty_print_is_stable() {
    let src = r#"
import io;
fn main() -> Int {
  let x: Int = 1;
  io.println("ok");
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());

    let pretty = program.pretty();
    assert!(pretty.contains("import io"));
    assert!(pretty.contains("fn main() -> Int"));
    assert!(pretty.contains("let x: Int = 1"));
    assert!(pretty.contains("expr io.println(\"ok\")"));
    assert!(pretty.contains("return 0"));
}
