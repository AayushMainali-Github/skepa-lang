use crate::ast::{AssignTarget, Expr, Stmt};
use crate::token::TokenKind;

use super::Parser;

impl Parser {
    pub(super) fn parse_stmt(&mut self) -> Option<Stmt> {
        if self.at(TokenKind::KwIf) {
            self.bump();
            self.expect(TokenKind::LParen, "Expected `(` after `if`")?;
            let cond = self.parse_expr()?;
            self.expect(TokenKind::RParen, "Expected `)` after if condition")?;
            let then_body = self.parse_block("Expected `{` before if body")?;
            let else_body = if self.at(TokenKind::KwElse) {
                self.bump();
                if self.at(TokenKind::KwIf) {
                    let nested_if = self.parse_stmt()?;
                    vec![nested_if]
                } else {
                    self.parse_block("Expected `{` before else body")?
                }
            } else {
                Vec::new()
            };
            return Some(Stmt::If {
                cond,
                then_body,
                else_body,
            });
        }

        if self.at(TokenKind::KwWhile) {
            self.bump();
            self.expect(TokenKind::LParen, "Expected `(` after `while`")?;
            let cond = self.parse_expr()?;
            self.expect(TokenKind::RParen, "Expected `)` after while condition")?;
            let body = self.parse_block("Expected `{` before while body")?;
            return Some(Stmt::While { cond, body });
        }
        if self.at(TokenKind::KwFor) {
            self.bump();
            self.expect(TokenKind::LParen, "Expected `(` after `for`")?;

            let init = if self.at(TokenKind::Semi) {
                self.bump();
                None
            } else {
                let stmt = self.parse_for_clause_stmt()?;
                self.expect(TokenKind::Semi, "Expected `;` after for init clause")?;
                Some(Box::new(stmt))
            };

            let cond = if self.at(TokenKind::Semi) {
                self.bump();
                None
            } else {
                let expr = self.parse_expr()?;
                self.expect(TokenKind::Semi, "Expected `;` after for condition")?;
                Some(expr)
            };

            let step = if self.at(TokenKind::RParen) {
                None
            } else {
                let stmt = self.parse_for_clause_stmt()?;
                Some(Box::new(stmt))
            };

            self.expect(TokenKind::RParen, "Expected `)` after for clauses")?;
            let body = self.parse_block("Expected `{` before for body")?;
            return Some(Stmt::For {
                init,
                cond,
                step,
                body,
            });
        }
        if self.at(TokenKind::KwBreak) {
            self.bump();
            self.expect(TokenKind::Semi, "Expected `;` after `break`")?;
            return Some(Stmt::Break);
        }
        if self.at(TokenKind::KwContinue) {
            self.bump();
            self.expect(TokenKind::Semi, "Expected `;` after `continue`")?;
            return Some(Stmt::Continue);
        }

        if self.at(TokenKind::KwLet) {
            self.bump();
            let name = self.expect_ident("Expected variable name after `let`")?;
            let mut ty = None;
            if self.at(TokenKind::Colon) {
                self.bump();
                ty = Some(self.expect_type_name("Expected type after `:`")?);
            }
            self.expect(TokenKind::Assign, "Expected `=` in let declaration")?;
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semi, "Expected `;` after let declaration")?;
            return Some(Stmt::Let {
                name: name.lexeme,
                ty,
                value,
            });
        }

        if self.at(TokenKind::Ident) && self.can_start_assignment_target() {
            let target = self.parse_assignment_target()?;
            self.expect(TokenKind::Assign, "Expected `=` after assignment target")?;
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semi, "Expected `;` after assignment")?;
            return Some(Stmt::Assign { target, value });
        }

        if self.at(TokenKind::KwReturn) {
            self.bump();
            if self.at(TokenKind::Semi) {
                self.bump();
                return Some(Stmt::Return(None));
            }

            let expr = self.parse_expr()?;
            self.expect(TokenKind::Semi, "Expected `;` after return statement")?;
            return Some(Stmt::Return(Some(expr)));
        }

        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semi, "Expected `;` after expression statement")?;
        Some(Stmt::Expr(expr))
    }

    pub(super) fn parse_block(&mut self, open_err: &str) -> Option<Vec<Stmt>> {
        self.expect(TokenKind::LBrace, open_err)?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        self.expect(TokenKind::RBrace, "Expected `}` after block")?;
        Some(body)
    }

    fn parse_for_clause_stmt(&mut self) -> Option<Stmt> {
        if self.at(TokenKind::KwLet) {
            self.bump();
            let name = self.expect_ident("Expected variable name after `let` in for clause")?;
            let mut ty = None;
            if self.at(TokenKind::Colon) {
                self.bump();
                ty = Some(self.expect_type_name("Expected type after `:` in for clause")?);
            }
            self.expect(TokenKind::Assign, "Expected `=` in for let clause")?;
            let value = self.parse_expr()?;
            return Some(Stmt::Let {
                name: name.lexeme,
                ty,
                value,
            });
        }

        if self.at(TokenKind::Ident) && self.can_start_assignment_target() {
            let target = self.parse_assignment_target()?;
            self.expect(TokenKind::Assign, "Expected `=` after assignment target")?;
            let value = self.parse_expr()?;
            return Some(Stmt::Assign { target, value });
        }

        let expr = self.parse_expr()?;
        Some(Stmt::Expr(expr))
    }

    fn can_start_assignment_target(&self) -> bool {
        if !self.at(TokenKind::Ident) {
            return false;
        }
        let mut i = self.idx + 1;
        let last = self.tokens.len().saturating_sub(1);
        while i <= last {
            let k = self.tokens[i].kind;
            if k == TokenKind::Assign {
                return true;
            }
            if k == TokenKind::Dot {
                i += 1;
                if i <= last && self.tokens[i].kind == TokenKind::Ident {
                    i += 1;
                    continue;
                }
            }
            if k == TokenKind::LBracket {
                let mut depth = 1usize;
                i += 1;
                while i <= last && depth > 0 {
                    let cur = self.tokens[i].kind;
                    if cur == TokenKind::LBracket {
                        depth += 1;
                    } else if cur == TokenKind::RBracket {
                        depth -= 1;
                    }
                    i += 1;
                }
                if depth == 0 {
                    continue;
                }
            }
            return false;
        }
        false
    }

    fn parse_assignment_target(&mut self) -> Option<AssignTarget> {
        let mut base = Expr::Ident(self.expect_ident("Expected assignment target")?.lexeme);

        while self.at(TokenKind::Dot) {
            self.bump();
            let part = self.expect_ident("Expected identifier after `.` in assignment target")?;
            base = Expr::Field {
                base: Box::new(base),
                field: part.lexeme,
            };
        }

        let mut index_target = None;
        while self.at(TokenKind::LBracket) {
            self.bump();
            let index = self.parse_expr()?;
            self.expect(TokenKind::RBracket, "Expected `]` after assignment index")?;
            index_target = Some((base.clone(), index.clone()));
            base = Expr::Index {
                base: Box::new(base),
                index: Box::new(index),
            };
        }

        if let Some((b, i)) = index_target {
            Some(AssignTarget::Index {
                base: Box::new(b),
                index: i,
            })
        } else {
            match base {
                Expr::Ident(n) => Some(AssignTarget::Ident(n)),
                Expr::Field { base, field } => Some(AssignTarget::Field { base, field }),
                _ => None,
            }
        }
    }
}
