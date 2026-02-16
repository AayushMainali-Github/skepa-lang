mod common;

use common::{assert_has_diag, parse_err, parse_ok};
use skeplib::ast::{AssignTarget, BinaryOp, Expr, Stmt, TypeName, UnaryOp};
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
            assert_eq!(
                *target,
                AssignTarget::Path(vec!["obj".to_string(), "field".to_string()])
            );
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
            assert!(
                matches!(&**callee, Expr::Path(parts) if parts == &vec!["io".to_string(), "println".to_string()])
            );
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
fn enforces_top_level_declarations_only() {
    let src = r#"
let x = 1;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty());
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Expected top-level declaration"))
    );
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
