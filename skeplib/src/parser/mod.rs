use crate::ast::{FnDecl, ImportDecl, Param, Program};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::lexer::lex;
use crate::token::{Token, TokenKind};

mod expr;
mod stmt;
mod types;

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
                    if self.at(TokenKind::RParen) {
                        break;
                    }
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

    fn expect_ident(&mut self, message: &str) -> Option<Token> {
        if self.at(TokenKind::Ident) {
            return Some(self.bump());
        }
        self.error_here_expected(message);
        None
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
                | TokenKind::KwFor
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
            .error_expected_found(message, &found, self.current().span);
    }
}
