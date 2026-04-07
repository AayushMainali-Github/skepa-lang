use super::*;

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
    assert_no_diags(&diags);
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
    assert_has_diag(&diags, "Expected match pattern (`_`, literal, or variant)");
}

#[test]
fn parses_match_statement_with_option_and_result_variant_patterns() {
    let src = r#"
fn main() -> Int {
  match (some(1)) {
    Some(x) => { return x; }
    None => { return 0; }
  }
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                MatchPattern::Variant {
                    name: "Some".to_string(),
                    binding: Some("x".to_string()),
                }
            );
            assert_eq!(
                arms[1].pattern,
                MatchPattern::Variant {
                    name: "None".to_string(),
                    binding: None,
                }
            );
        }
        _ => panic!("expected match statement"),
    }
}

#[test]
fn parses_match_expression_with_variant_arms() {
    let src = r#"
fn main() -> Int {
  let x: Int = match (some(1)) {
    Some(v) => v,
    None => 0,
  };
  return x;
}

#[test]
fn parses_variant_pattern_with_payload_wildcard() {
    let src = r#"
fn main() -> Int {
  match (some(1)) {
    Some(_) => { return 1; }
    None => { return 0; }
  }
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                MatchPattern::Variant {
                    name: "Some".to_string(),
                    binding: None,
                }
            );
        }
        _ => panic!("expected match statement"),
    }
}

#[test]
fn reports_empty_variant_pattern_payload() {
    let src = r#"
fn main() -> Int {
  match (some(1)) {
    Some() => { return 1; }
    None => { return 0; }
  }
}
"#;
    let diags = parse_err(src);
    assert_has_diag(
        &diags,
        "Expected identifier or `_` inside variant match pattern",
    );
}

#[test]
fn reports_empty_none_variant_pattern_payload() {
    let src = r#"
fn main() -> Int {
  match (none()) {
    None() => { return 1; }
    _ => { return 0; }
  }
}
"#;
    let diags = parse_err(src);
    assert_has_diag(
        &diags,
        "Expected identifier or `_` inside variant match pattern",
    );
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => match value {
            Expr::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                assert_eq!(
                    arms[0].pattern,
                    MatchPattern::Variant {
                        name: "Some".to_string(),
                        binding: Some("v".to_string())
                    }
                );
                assert_eq!(arms[1].pattern, MatchPattern::Variant {
                    name: "None".to_string(),
                    binding: None
                });
            }
            _ => panic!("expected match expression"),
        },
        _ => panic!("expected let statement"),
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
}

#[test]
fn accepts_trailing_comma_in_function_params() {
    let src = r#"
fn add(a: Int, b: Int,) -> Int {
  return a + b;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert_no_diags(&diags);
    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn accepts_top_level_global_let_declaration() {
    let src = r#"
let x = 1;
fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
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
    assert_no_diags(&diags);
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => match value {
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
            } => match &**expr {
                Expr::Group(inner) => match &**inner {
                    Expr::Binary {
                        left,
                        op: BinaryOp::EqEq,
                        right,
                    } => {
                        assert!(matches!(&**left, Expr::Group(_)));
                        assert!(matches!(&**right, Expr::IntLit(3)));
                    }
                    _ => panic!("expected grouped equality"),
                },
                _ => panic!("expected grouped unary operand"),
            },
            _ => panic!("expected unary not"),
        },
        _ => panic!("expected let"),
    }
}

#[test]
fn parses_deeply_nested_grouped_expression_stress() {
    let mut expr = "1".to_string();
    for _ in 0..96 {
        expr = format!("({expr})");
    }
    let src = format!(
        r#"
fn main() -> Int {{
  return {expr};
}}
"#
    );
    let (_program, diags) = Parser::parse_source(&src);
    assert_no_diags(&diags);
}

#[test]
fn parses_match_for_and_function_literal_combination() {
    let src = r#"
fn main() -> Int {
  let bump: Fn(Int) -> Int = fn(x: Int) -> Int { return x + 1; };
  match (1) {
    1 => {
      for (let i = 0; i < 2; i = i + 1) {
        let y = bump(i);
      }
      return bump(4);
    }
    _ => {
      return 0;
    }
  }
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[1] {
        Stmt::Match { arms, .. } => {
            assert_eq!(arms.len(), 2);
            assert!(matches!(arms[0].body[0], Stmt::For { .. }));
            assert!(matches!(arms[0].body[1], Stmt::Return(_)));
        }
        _ => panic!("expected match statement"),
    }
}
