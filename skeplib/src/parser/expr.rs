use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::token::TokenKind;

use super::Parser;

impl Parser {
    pub(super) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Option<Expr> {
        let mut expr = self.parse_logical_and()?;
        while self.at(TokenKind::OrOr) {
            self.bump();
            let rhs = self.parse_logical_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::OrOr,
                right: Box::new(rhs),
            };
        }
        Some(expr)
    }

    fn parse_logical_and(&mut self) -> Option<Expr> {
        let mut expr = self.parse_equality()?;
        while self.at(TokenKind::AndAnd) {
            self.bump();
            let rhs = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::AndAnd,
                right: Box::new(rhs),
            };
        }
        Some(expr)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.at(TokenKind::EqEq) {
                Some(BinaryOp::EqEq)
            } else if self.at(TokenKind::Neq) {
                Some(BinaryOp::Neq)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(rhs),
            };
        }
        Some(expr)
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match self.current().kind {
                TokenKind::Lt => Some(BinaryOp::Lt),
                TokenKind::Lte => Some(BinaryOp::Lte),
                TokenKind::Gt => Some(BinaryOp::Gt),
                TokenKind::Gte => Some(BinaryOp::Gte),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(rhs),
            };
        }
        Some(expr)
    }

    fn parse_term(&mut self) -> Option<Expr> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = if self.at(TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.at(TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(rhs),
            };
        }
        Some(expr)
    }

    fn parse_factor(&mut self) -> Option<Expr> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.at(TokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.at(TokenKind::Slash) {
                Some(BinaryOp::Div)
            } else if self.at(TokenKind::Percent) {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(rhs),
            };
        }
        Some(expr)
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
