use crate::ast::{Expr, FnDecl, ImportDecl, Program, Stmt};
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
        self.expect(TokenKind::RParen, "Expected `)` after parameters")?;

        if self.at(TokenKind::Arrow) {
            self.bump();
            self.expect_type_name("Expected return type after `->`")?;
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
            body,
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
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

    fn expect_type_name(&mut self, message: &str) -> Option<Token> {
        let kind = self.current().kind;
        if matches!(
            kind,
            TokenKind::TyInt
                | TokenKind::TyFloat
                | TokenKind::TyBool
                | TokenKind::TyString
                | TokenKind::TyVoid
        ) {
            return Some(self.bump());
        }
        self.error_here(message);
        None
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
