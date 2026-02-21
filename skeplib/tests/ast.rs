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
        imports: vec![ImportDecl::ImportModule {
            path: vec!["io".to_string()],
            alias: None,
        }],
        exports: Vec::new(),
        globals: Vec::new(),
        structs: Vec::new(),
        impls: Vec::new(),
        functions: vec![FnDecl {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: Some(TypeName::Int),
            body: Vec::new(),
        }],
    };

    assert_eq!(program.imports.len(), 1);
    assert_eq!(
        program.imports[0],
        ImportDecl::ImportModule {
            path: vec!["io".to_string()],
            alias: None
        }
    );
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
    assert_eq!(program.functions[0].return_type, Some(TypeName::Int));
}

#[test]
fn struct_and_impl_ast_nodes_roundtrip_data() {
    let s = skeplib::ast::StructDecl {
        name: "User".to_string(),
        fields: vec![
            skeplib::ast::FieldDecl {
                name: "id".to_string(),
                ty: TypeName::Int,
            },
            skeplib::ast::FieldDecl {
                name: "name".to_string(),
                ty: TypeName::String,
            },
        ],
    };
    let m = skeplib::ast::MethodDecl {
        name: "label".to_string(),
        params: vec![Param {
            name: "prefix".to_string(),
            ty: TypeName::String,
        }],
        return_type: Some(TypeName::String),
        body: vec![Stmt::Return(Some(Expr::StringLit("x".to_string())))],
    };
    let i = skeplib::ast::ImplDecl {
        target: "User".to_string(),
        methods: vec![m],
    };
    let p = Program {
        imports: Vec::new(),
        exports: Vec::new(),
        globals: Vec::new(),
        structs: vec![s],
        impls: vec![i],
        functions: Vec::new(),
    };
    assert_eq!(p.structs[0].name, "User");
    assert_eq!(p.structs[0].fields.len(), 2);
    assert_eq!(p.impls[0].target, "User");
    assert_eq!(p.impls[0].methods[0].name, "label");
}

#[test]
fn struct_expression_and_field_assignment_nodes_are_supported() {
    let lit = Expr::StructLit {
        name: "User".to_string(),
        fields: vec![
            ("id".to_string(), Expr::IntLit(1)),
            ("name".to_string(), Expr::StringLit("sam".to_string())),
        ],
    };
    let get = Expr::Field {
        base: Box::new(Expr::Ident("u".to_string())),
        field: "name".to_string(),
    };
    let set = skeplib::ast::AssignTarget::Field {
        base: Box::new(Expr::Ident("u".to_string())),
        field: "name".to_string(),
    };
    assert!(matches!(lit, Expr::StructLit { .. }));
    assert!(matches!(get, Expr::Field { .. }));
    assert!(matches!(set, skeplib::ast::AssignTarget::Field { .. }));
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
