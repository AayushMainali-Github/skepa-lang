use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::token::TokenKind;

use super::Parser;

#[derive(Debug, Clone)]
enum InfixOp {
    Builtin(BinaryOp),
    Custom(String),
}

impl Parser {
    pub(super) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_binary_expr(0)
    }

    fn parse_binary_expr(&mut self, min_precedence: i64) -> Option<Expr> {
        let mut expr = self.parse_unary()?;

        loop {
            let Some((op, precedence)) = self.peek_infix_operator() else {
                break;
            };
            if precedence < min_precedence {
                break;
            }

            self.consume_infix_operator(&op)?;
            let rhs = self.parse_binary_expr(precedence + 1)?;
            expr = match op {
                InfixOp::Builtin(op) => Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(rhs),
                },
                InfixOp::Custom(operator) => Expr::CustomInfix {
                    left: Box::new(expr),
                    operator,
                    right: Box::new(rhs),
                },
            };
        }

        Some(expr)
    }

    fn peek_infix_operator(&mut self) -> Option<(InfixOp, i64)> {
        match self.current().kind {
            TokenKind::OrOr => Some((InfixOp::Builtin(BinaryOp::OrOr), 1)),
            TokenKind::AndAnd => Some((InfixOp::Builtin(BinaryOp::AndAnd), 2)),
            TokenKind::EqEq => Some((InfixOp::Builtin(BinaryOp::EqEq), 3)),
            TokenKind::Neq => Some((InfixOp::Builtin(BinaryOp::Neq), 3)),
            TokenKind::Lt => Some((InfixOp::Builtin(BinaryOp::Lt), 4)),
            TokenKind::Lte => Some((InfixOp::Builtin(BinaryOp::Lte), 4)),
            TokenKind::Gt => Some((InfixOp::Builtin(BinaryOp::Gt), 4)),
            TokenKind::Gte => Some((InfixOp::Builtin(BinaryOp::Gte), 4)),
            TokenKind::Pipe => Some((InfixOp::Builtin(BinaryOp::BitOr), 5)),
            TokenKind::Caret => Some((InfixOp::Builtin(BinaryOp::BitXor), 6)),
            TokenKind::Amp => Some((InfixOp::Builtin(BinaryOp::BitAnd), 7)),
            TokenKind::Shl => Some((InfixOp::Builtin(BinaryOp::Shl), 8)),
            TokenKind::Shr => Some((InfixOp::Builtin(BinaryOp::Shr), 8)),
            TokenKind::Plus => Some((InfixOp::Builtin(BinaryOp::Add), 9)),
            TokenKind::Minus => Some((InfixOp::Builtin(BinaryOp::Sub), 9)),
            TokenKind::Star => Some((InfixOp::Builtin(BinaryOp::Mul), 10)),
            TokenKind::Slash => Some((InfixOp::Builtin(BinaryOp::Div), 10)),
            TokenKind::Percent => Some((InfixOp::Builtin(BinaryOp::Mod), 10)),
            TokenKind::Backtick => {
                let operator = self.tokens.get(self.idx + 1)?;
                let closing = self.tokens.get(self.idx + 2)?;
                if operator.kind != TokenKind::Ident || closing.kind != TokenKind::Backtick {
                    self.error_here_expected("Expected backtick operator in the form `` `name` ``");
                    return None;
                }
                let precedence = match self.custom_operator_precedences.get(&operator.lexeme) {
                    Some(precedence) => *precedence,
                    None => {
                        self.diagnostics.error(
                            format!(
                                "Unknown operator `{}`; declare it locally or import it with `from ... import ...` so its precedence is known during parsing",
                                operator.lexeme
                            ),
                            operator.span,
                        );
                        0
                    }
                };
                Some((InfixOp::Custom(operator.lexeme.clone()), precedence))
            }
            _ => None,
        }
    }

    fn consume_infix_operator(&mut self, op: &InfixOp) -> Option<()> {
        match op {
            InfixOp::Builtin(_) => {
                self.bump();
            }
            InfixOp::Custom(_) => {
                self.expect(
                    TokenKind::Backtick,
                    "Expected opening backtick for custom operator",
                )?;
                self.expect_ident("Expected operator name after backtick")?;
                self.expect(
                    TokenKind::Backtick,
                    "Expected closing backtick after custom operator name",
                )?;
            }
        }
        Some(())
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.at(TokenKind::Bang) {
            self.bump();
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.at(TokenKind::Tilde) {
            self.bump();
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(expr),
            });
        }
        if self.at(TokenKind::Minus) {
            self.bump();
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        if self.at(TokenKind::Plus) {
            self.bump();
            let expr = self.parse_unary()?;
            return Some(Expr::Unary {
                op: UnaryOp::Pos,
                expr: Box::new(expr),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.at(TokenKind::LParen) {
                self.bump();
                let mut args = Vec::new();
                if !self.at(TokenKind::RParen) {
                    loop {
                        if self.at(TokenKind::Comma) {
                            self.error_here_expected("Expected expression before `,` in call");
                            return None;
                        }
                        args.push(self.parse_expr()?);
                        if self.at(TokenKind::Comma) {
                            self.bump();
                            if self.at(TokenKind::RParen) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "Expected `)` after call arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            if self.at(TokenKind::LBracket) {
                self.bump();
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket, "Expected `]` after index expression")?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                };
                continue;
            }

            if self.at(TokenKind::Dot) {
                self.bump();
                let field = self.expect_ident("Expected identifier after `.`")?;
                expr = Expr::Field {
                    base: Box::new(expr),
                    field: field.lexeme,
                };
                continue;
            }

            break;
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        if self.at(TokenKind::KwFn) {
            self.bump();
            self.expect(
                TokenKind::LParen,
                "Expected `(` after `fn` in function literal",
            )?;
            let mut params = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    let name = self.expect_ident("Expected parameter name in function literal")?;
                    self.expect(TokenKind::Colon, "Expected `:` after parameter name")?;
                    let ty = self.expect_type_name("Expected parameter type")?;
                    params.push(crate::ast::Param {
                        name: name.lexeme,
                        ty,
                    });
                    if self.at(TokenKind::Comma) {
                        self.bump();
                        if self.at(TokenKind::RParen) {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            self.expect(
                TokenKind::RParen,
                "Expected `)` after function literal parameters",
            )?;
            self.expect(
                TokenKind::Arrow,
                "Expected `->` after function literal parameters",
            )?;
            let return_type = self.expect_type_name("Expected function literal return type")?;
            let body = self.parse_block("Expected `{` before function literal body")?;
            return Some(Expr::FnLit {
                params,
                return_type,
                body,
            });
        }
        if self.at(TokenKind::IntLit) {
            let tok = self.bump();
            let value = tok.lexeme.parse::<i64>().ok()?;
            return Some(Expr::IntLit(value));
        }
        if self.at(TokenKind::FloatLit) {
            let tok = self.bump();
            return Some(Expr::FloatLit(tok.lexeme));
        }
        if self.at(TokenKind::KwTrue) {
            self.bump();
            return Some(Expr::BoolLit(true));
        }
        if self.at(TokenKind::KwFalse) {
            self.bump();
            return Some(Expr::BoolLit(false));
        }
        if self.at(TokenKind::StringLit) {
            let tok = self.bump();
            let s = tok
                .lexeme
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(&tok.lexeme)
                .to_string();
            let s = self.decode_string_escapes(&s, tok.span);
            return Some(Expr::StringLit(s));
        }
        if self.at(TokenKind::Ident) {
            let name = self.bump().lexeme;
            if self.at(TokenKind::LBrace) {
                self.bump();
                let mut fields = Vec::new();
                if !self.at(TokenKind::RBrace) {
                    loop {
                        let field = self.expect_ident("Expected field name in struct literal")?;
                        self.expect(TokenKind::Colon, "Expected `:` after field name")?;
                        let value = self.parse_expr()?;
                        fields.push((field.lexeme, value));
                        if self.at(TokenKind::Comma) {
                            self.bump();
                            if self.at(TokenKind::RBrace) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(TokenKind::RBrace, "Expected `}` after struct literal")?;
                return Some(Expr::StructLit { name, fields });
            }
            return Some(Expr::Ident(name));
        }
        if self.at(TokenKind::LBracket) {
            self.bump();
            if self.at(TokenKind::RBracket) {
                self.bump();
                return Some(Expr::ArrayLit(Vec::new()));
            }
            let first = self.parse_expr()?;
            if self.at(TokenKind::Semi) {
                self.bump();
                let sz = self.expect(TokenKind::IntLit, "Expected integer size in array repeat")?;
                let size = match sz.lexeme.parse::<usize>() {
                    Ok(v) => v,
                    Err(_) => {
                        self.error_here_expected("Expected valid array repeat size");
                        return None;
                    }
                };
                self.expect(
                    TokenKind::RBracket,
                    "Expected `]` after array repeat literal",
                )?;
                return Some(Expr::ArrayRepeat {
                    value: Box::new(first),
                    size,
                });
            }
            let mut items = vec![first];
            while self.at(TokenKind::Comma) {
                self.bump();
                if self.at(TokenKind::RBracket) {
                    break;
                }
                items.push(self.parse_expr()?);
            }
            self.expect(TokenKind::RBracket, "Expected `]` after array literal")?;
            return Some(Expr::ArrayLit(items));
        }
        if self.at(TokenKind::LParen) {
            self.bump();
            let expr = self.parse_expr()?;
            self.expect(TokenKind::RParen, "Expected `)` after grouped expression")?;
            return Some(Expr::Group(Box::new(expr)));
        }

        self.error_here_expected("Expected expression");
        None
    }
}
