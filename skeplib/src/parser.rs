use crate::ast::{Expr, FnDecl, ImportDecl, Param, Program, Stmt, TypeName};
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

            self.error_here("Expected top-level declaration (`import` or `fn`)");
            self.bump();
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
                None => {
                    let _ = self.bump();
                }
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

        if self.at(TokenKind::Ident) && self.peek_kind() == TokenKind::Assign {
            let name = self.bump().lexeme;
            self.bump();
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semi, "Expected `;` after assignment")?;
            return Some(Stmt::Assign { name, value });
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

        self.error_here("Only `return` statements are supported in this parser step");
        None
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        if self.at(TokenKind::IntLit) {
            let tok = self.bump();
            let value = tok.lexeme.parse::<i64>().ok()?;
            return Some(Expr::IntLit(value));
        }
        if self.at(TokenKind::Ident) {
            let tok = self.bump();
            return Some(Expr::Ident(tok.lexeme));
        }
        self.error_here("Expected expression");
        None
    }

    fn expect_ident(&mut self, message: &str) -> Option<Token> {
        if self.at(TokenKind::Ident) {
            return Some(self.bump());
        }
        self.error_here(message);
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
                self.error_here(message);
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
        self.error_here(message);
        None
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn current(&self) -> &Token {
        let last = self.tokens.len().saturating_sub(1);
        &self.tokens[self.idx.min(last)]
    }

    fn peek_kind(&self) -> TokenKind {
        let i = (self.idx + 1).min(self.tokens.len().saturating_sub(1));
        self.tokens[i].kind
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if self.idx < self.tokens.len() {
            self.idx += 1;
        }
        token
    }

    fn error_here(&mut self, message: &str) {
        self.diagnostics
            .error(message, self.current().span);
    }
}
