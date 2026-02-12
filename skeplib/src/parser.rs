use crate::ast::{
    AssignTarget, BinaryOp, Expr, FnDecl, ImportDecl, Param, Program, Stmt, TypeName, UnaryOp,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::lexer::lex;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    idx: usize,
    diagnostics: DiagnosticBag,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            tokens: vec![Token::new(TokenKind::Eof, "", Span::default())],
            idx: 0,
            diagnostics: DiagnosticBag::new(),
        }
    }
}

impl Parser {
    pub fn parse_source(source: &str) -> (Program, DiagnosticBag) {
        let (tokens, mut diagnostics) = lex(source);
        let mut parser = Parser {
            tokens,
            idx: 0,
            diagnostics: DiagnosticBag::new(),
        };
        let program = parser.parse_program();
        for d in parser.diagnostics.into_vec() {
            diagnostics.push(d);
        }
        (program, diagnostics)
    }

    fn parse_program(&mut self) -> Program {
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::KwImport) {
                if let Some(i) = self.parse_import() {
                    imports.push(i);
                }
                continue;
            }

            if self.at(TokenKind::KwFn) {
                if let Some(f) = self.parse_function() {
                    functions.push(f);
                }
                continue;
            }

            self.error_here_expected("Expected top-level declaration (`import` or `fn`)");
            self.synchronize_toplevel();
        }

        Program { imports, functions }
    }

    fn parse_import(&mut self) -> Option<ImportDecl> {
        self.expect(TokenKind::KwImport, "Expected `import`")?;
        let module = self.expect_ident("Expected module name after `import`")?;
        self.expect(TokenKind::Semi, "Expected `;` after import")?;
        Some(ImportDecl {
            module: module.lexeme,
        })
    }

    fn parse_function(&mut self) -> Option<FnDecl> {
        self.expect(TokenKind::KwFn, "Expected `fn`")?;
        let name = self.expect_ident("Expected function name after `fn`")?;
        self.expect(TokenKind::LParen, "Expected `(` after function name")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let param_name = self.expect_ident("Expected parameter name")?;
                self.expect(TokenKind::Colon, "Expected `:` after parameter name")?;
                let param_ty = self.expect_type_name("Expected parameter type after `:`")?;
                params.push(Param {
                    name: param_name.lexeme,
                    ty: param_ty,
                });

                if self.at(TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RParen, "Expected `)` after parameters")?;

        let mut return_type = None;
        if self.at(TokenKind::Arrow) {
            self.bump();
            return_type = Some(self.expect_type_name("Expected return type after `->`")?);
        }

        self.expect(TokenKind::LBrace, "Expected `{` before function body")?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        self.expect(TokenKind::RBrace, "Expected `}` after function body")?;

        Some(FnDecl {
            name: name.lexeme,
            params,
            return_type,
            body,
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
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

    fn parse_block(&mut self, open_err: &str) -> Option<Vec<Stmt>> {
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

    fn parse_expr(&mut self) -> Option<Expr> {
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
        self.parse_call()
    }

    fn parse_call(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if !self.at(TokenKind::LParen) {
                break;
            }
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
                            self.error_here_expected(
                                "Trailing comma is not allowed in call arguments",
                            );
                            return None;
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
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
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
            let mut parts = vec![self.bump().lexeme];
            while self.at(TokenKind::Dot) {
                self.bump();
                let part = self.expect_ident("Expected identifier after `.`")?;
                parts.push(part.lexeme);
            }
            if parts.len() == 1 {
                return Some(Expr::Ident(parts.remove(0)));
            }
            return Some(Expr::Path(parts));
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
            return false;
        }
        false
    }

    fn parse_assignment_target(&mut self) -> Option<AssignTarget> {
        let mut parts = vec![self.expect_ident("Expected assignment target")?.lexeme];
        while self.at(TokenKind::Dot) {
            self.bump();
            let part = self.expect_ident("Expected identifier after `.` in assignment target")?;
            parts.push(part.lexeme);
        }
        if parts.len() == 1 {
            Some(AssignTarget::Ident(parts.remove(0)))
        } else {
            Some(AssignTarget::Path(parts))
        }
    }

    fn expect_ident(&mut self, message: &str) -> Option<Token> {
        if self.at(TokenKind::Ident) {
            return Some(self.bump());
        }
        self.error_here_expected(message);
        None
    }

    fn expect_type_name(&mut self, message: &str) -> Option<TypeName> {
        let ty = match self.current().kind {
            TokenKind::TyInt => TypeName::Int,
            TokenKind::TyFloat => TypeName::Float,
            TokenKind::TyBool => TypeName::Bool,
            TokenKind::TyString => TypeName::String,
            TokenKind::TyVoid => TypeName::Void,
            _ => {
                self.error_here_expected(message);
                return None;
            }
        };
        let _ = self.bump();
        Some(ty)
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.at(kind) {
            return Some(self.bump());
        }
        self.error_here_expected(message);
        None
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn current(&self) -> &Token {
        let last = self.tokens.len().saturating_sub(1);
        &self.tokens[self.idx.min(last)]
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if self.idx < self.tokens.len() {
            self.idx += 1;
        }
        token
    }

    fn synchronize_stmt(&mut self) {
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Semi) {
                self.bump();
                return;
            }
            if self.at(TokenKind::RBrace) {
                return;
            }
            match self.current().kind {
                TokenKind::KwLet
                | TokenKind::KwIf
                | TokenKind::KwWhile
                | TokenKind::KwBreak
                | TokenKind::KwContinue
                | TokenKind::KwReturn
                | TokenKind::Ident => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn synchronize_toplevel(&mut self) {
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::KwImport) || self.at(TokenKind::KwFn) {
                return;
            }
            self.bump();
        }
    }

    fn token_label(token: &Token) -> String {
        if token.kind == TokenKind::Eof {
            return "EOF".to_string();
        }
        if !token.lexeme.is_empty() {
            return format!("`{}`", token.lexeme);
        }
        format!("{:?}", token.kind)
    }

    fn error_here_expected(&mut self, message: &str) {
        let found = Self::token_label(self.current());
        self.diagnostics
            .error(format!("{message}; found {found}"), self.current().span);
    }

    fn decode_string_escapes(&mut self, raw: &str, span: Span) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                out.push(ch);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    self.diagnostics.error(
                        format!("Invalid escape sequence `\\{other}` in string literal"),
                        span,
                    );
                    out.push(other);
                }
                None => {
                    self.diagnostics
                        .error("String ends with trailing escape `\\`", span);
                }
            }
        }
        out
    }
}
