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
  let c = ~7;
  return 0;
}

#[test]
fn parses_user_defined_operator_used_before_declaration_in_same_module() {
    let src = r#"
fn main() -> Int {
  return 1 `xoxo` 2;
}

opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs + rhs;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert_no_diags(&diags);
    match &program.functions[0].body[0] {
        Stmt::Return(Some(Expr::CustomInfix {
            left,
            operator,
            right,
        })) => {
            assert_eq!(operator, "xoxo");
            assert!(matches!(&**left, Expr::IntLit(1)));
            assert!(matches!(&**right, Expr::IntLit(2)));
        }
        other => panic!("expected custom infix return expression, got {other:?}"),
    }
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
    match &program.functions[0].body[3] {
        Stmt::Let { value, .. } => assert!(matches!(
            value,
            Expr::Unary {
                op: UnaryOp::BitNot,
                ..
            }
        )),
        _ => panic!("expected let"),
    }
}

#[test]
fn parses_bitwise_and_shift_precedence() {
    let src = r#"
fn main() -> Int {
  let x = 1 | 2 ^ 3 & 4 << 1;
  return x;
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
            op: BinaryOp::BitOr,
            left: _,
            right,
        } => match &**right {
            Expr::Binary {
                op: BinaryOp::BitXor,
                right,
                ..
            } => match &**right {
                Expr::Binary {
                    op: BinaryOp::BitAnd,
                    right,
                    ..
                } => match &**right {
                    Expr::Binary {
                        op: BinaryOp::Shl,
                        ..
                    } => {}
                    _ => panic!("expected shift on right of bitand"),
                },
                _ => panic!("expected bitand on right of xor"),
            },
            _ => panic!("expected xor on right of bitor"),
        },
        _ => panic!("expected top-level bitwise or"),
    }
}

#[test]
fn parses_operator_declaration_and_custom_backtick_infix_expr() {
    let src = r#"
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 2 {
  return lhs + rhs;
}

fn main() -> Int {
  let x = 5 `xoxo` 4 + 3;
  return x;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert_no_diags(&diags);
    assert_eq!(program.operators.len(), 1);
    assert_eq!(program.operators[0].name, "xoxo");
    assert_eq!(program.operators[0].precedence, 2);
    assert_eq!(program.operators[0].params.len(), 2);
    match &program.functions[0].body[0] {
        Stmt::Let {
            value:
                Expr::CustomInfix {
                    left,
                    operator,
                    right,
                },
            ..
        } => {
            assert_eq!(operator, "xoxo");
            assert!(matches!(&**left, Expr::IntLit(5)));
            assert!(matches!(
                &**right,
                Expr::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
        }
        other => panic!("expected custom infix expression, got {other:?}"),
    }
}

#[test]
fn custom_operator_precedence_interacts_with_builtin_binary_ops() {
    let src = r#"
opr low(lhs: Int, rhs: Int) -> Int precedence 1 {
  return lhs + rhs;
}

opr high(lhs: Int, rhs: Int) -> Int precedence 10 {
  return lhs + rhs;
}

fn main() -> Int {
  let a = 1 + 2 `low` 3 * 4;
  let b = 1 + 2 `high` 3 * 4;
  return a + b;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert_no_diags(&diags);

    let a = match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => value,
        _ => panic!("expected first let"),
    };
    match a {
        Expr::CustomInfix {
            left,
            operator,
            right,
        } => {
            assert_eq!(operator, "low");
            assert!(matches!(
                &**left,
                Expr::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
            assert!(matches!(
                &**right,
                Expr::Binary {
                    op: BinaryOp::Mul,
                    ..
                }
            ));
        }
        other => panic!("expected low-precedence custom infix, got {other:?}"),
    }

    let b = match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => value,
        _ => panic!("expected second let"),
    };
    match b {
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right: _,
        } => match &**left {
            Expr::CustomInfix {
                left,
                operator,
                right,
            } => {
                assert_eq!(operator, "high");
                assert!(matches!(&**left, Expr::IntLit(1)));
                assert!(matches!(
                    &**right,
                    Expr::Binary {
                        op: BinaryOp::Mul,
                        ..
                    }
                ));
            }
            other => panic!("expected custom infix under add, got {other:?}"),
        },
        other => panic!("expected add around high-precedence custom infix, got {other:?}"),
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
