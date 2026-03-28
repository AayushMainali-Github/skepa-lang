use super::*;

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

#[test]
fn parses_bytes_type_annotations() {
    let src = r#"
fn take(data: Bytes) -> Bytes {
  return data;
}
"#;
    let program = parse_ok(src);
    let f = &program.functions[0];
    assert_eq!(f.params[0].ty, TypeName::Bytes);
    assert_eq!(f.return_type, Some(TypeName::Bytes));
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
fn parses_grouped_function_literal_vs_grouped_callee_ambiguity_shapes() {
    let src = r#"
fn makeId() -> Fn(Int) -> Int {
  return fn(x: Int) -> Int { return x; };
}

fn main() -> Int {
  let a = ((fn(x: Int) -> Int { return x + 1; }))(2);
  let b = (makeId())(3);
  return a + b;
}
"#;
    let program = parse_ok(src);
    match &program.functions[1].body[0] {
        Stmt::Let {
            value: Expr::Call { callee, args },
            ..
        } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(callee.as_ref(), Expr::Group(_)));
        }
        other => panic!("expected grouped function literal call, got {other:?}"),
    }
    match &program.functions[1].body[1] {
        Stmt::Let {
            value: Expr::Call { callee, args },
            ..
        } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(callee.as_ref(), Expr::Group(_)));
        }
        other => panic!("expected grouped callee call, got {other:?}"),
    }
}
