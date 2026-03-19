use super::*;

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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);

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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
fn parses_chained_index_field_and_call_in_complex_order() {
    let src = r#"
fn main() -> Int {
  let x = makeUsers()[0].build(1).items[2];
  return 0;
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Let {
            value: Expr::Index { base, index },
            ..
        } => {
            assert!(matches!(**index, Expr::IntLit(2)));
            assert!(matches!(**base, Expr::Field { .. }));
        }
        other => panic!("expected complex chained index expression, got {other:?}"),
    }
}
