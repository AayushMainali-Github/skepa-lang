use crate::ast::{
    ExportDecl, ExportItem, FieldDecl, FnDecl, ImplDecl, ImportDecl, ImportItem, MethodDecl, Param,
    Program, StructDecl, TypeName,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::lexer::lex;
use crate::token::{Token, TokenKind};
use std::collections::HashSet;

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
        let mut exports = Vec::new();
        let mut seen_export_decl = false;
        let mut structs = Vec::new();
        let mut impls = Vec::new();
        let mut functions = Vec::new();

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::KwImport) {
                if let Some(i) = self.parse_import() {
                    imports.push(i);
                }
                continue;
            }
            if self.at(TokenKind::KwFrom) {
                if let Some(i) = self.parse_from_import() {
                    imports.push(i);
                }
                continue;
            }
            if self.at(TokenKind::KwExport) {
                if let Some(e) = self.parse_export_decl() {
                    if seen_export_decl {
                        self.diagnostics.error(
                            "Only one `export { ... };` block is allowed per file",
                            self.current().span,
                        );
                    } else {
                        exports.push(e);
                        seen_export_decl = true;
                    }
                }
                continue;
            }

            if self.at(TokenKind::KwFn) {
                if let Some(f) = self.parse_function() {
                    functions.push(f);
                }
                continue;
            }
            if self.at(TokenKind::KwStruct) {
                if let Some(s) = self.parse_struct_decl() {
                    structs.push(s);
                }
                continue;
            }
            if self.at(TokenKind::KwImpl) {
                if let Some(i) = self.parse_impl_decl() {
                    impls.push(i);
                }
                continue;
            }

            self.error_here_expected(
                "Expected top-level declaration (`import`, `from`, `export`, `struct`, `impl`, or `fn`)",
            );
            self.synchronize_toplevel();
        }

        Program {
            imports,
            exports,
            structs,
            impls,
            functions,
        }
    }

    fn parse_import(&mut self) -> Option<ImportDecl> {
        self.expect(TokenKind::KwImport, "Expected `import`")?;
        let path = self.parse_dotted_path("Expected module path after `import`")?;
        let alias = if self.at(TokenKind::KwAs) {
            self.bump();
            Some(self.expect_ident("Expected alias name after `as`")?.lexeme)
        } else {
            None
        };
        self.expect(TokenKind::Semi, "Expected `;` after import")?;
        Some(ImportDecl::ImportModule { path, alias })
    }

    fn parse_from_import(&mut self) -> Option<ImportDecl> {
        self.expect(TokenKind::KwFrom, "Expected `from`")?;
        let path = self.parse_dotted_path("Expected module path after `from`")?;
        self.expect(
            TokenKind::KwImport,
            "Expected `import` after module path in from-import",
        )?;
        let mut items = Vec::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        let mut seen_aliases: HashSet<String> = HashSet::new();
        if self.at(TokenKind::Comma) {
            self.error_here_expected("Expected imported symbol name before `,` in from-import");
            return None;
        }
        loop {
            let name = self
                .expect_ident("Expected imported symbol name after `import`")?
                .lexeme;
            let alias = if self.at(TokenKind::KwAs) {
                self.bump();
                Some(self.expect_ident("Expected alias name after `as`")?.lexeme)
            } else {
                None
            };
            if !seen_names.insert(name.clone()) {
                self.diagnostics.error(
                    format!("Duplicate imported symbol `{name}` in from-import clause"),
                    self.current().span,
                );
            }
            if let Some(a) = &alias
                && !seen_aliases.insert(a.clone())
            {
                self.diagnostics.error(
                    format!("Duplicate import alias `{a}` in from-import clause"),
                    self.current().span,
                );
            }
            items.push(ImportItem { name, alias });
            if self.at(TokenKind::Comma) {
                self.bump();
                if self.at(TokenKind::Semi) {
                    self.error_here_expected("Trailing `,` is not allowed in from-import");
                    return None;
                }
                if self.at(TokenKind::Comma) {
                    self.error_here_expected(
                        "Expected imported symbol name before `,` in from-import",
                    );
                    return None;
                }
                continue;
            }
            break;
        }
        self.expect(TokenKind::Semi, "Expected `;` after from-import")?;
        Some(ImportDecl::ImportFrom { path, items })
    }

    fn parse_dotted_path(&mut self, first_err: &str) -> Option<Vec<String>> {
        let mut path = vec![self.expect_ident(first_err)?.lexeme];
        while self.at(TokenKind::Dot) {
            self.bump();
            let next = self.expect_ident("Expected identifier after `.` in module path")?;
            path.push(next.lexeme);
        }
        Some(path)
    }

    fn parse_export_decl(&mut self) -> Option<ExportDecl> {
        self.expect(TokenKind::KwExport, "Expected `export`")?;
        self.expect(TokenKind::LBrace, "Expected `{` after `export`")?;
        if self.at(TokenKind::RBrace) {
            self.error_here_expected("Expected at least one export item");
            return None;
        }
        if self.at(TokenKind::Comma) {
            self.error_here_expected("Expected export symbol name before `,`");
            return None;
        }
        let mut items = Vec::new();
        loop {
            let name = self.expect_ident("Expected export symbol name")?.lexeme;
            let alias = if self.at(TokenKind::KwAs) {
                self.bump();
                Some(self.expect_ident("Expected alias name after `as`")?.lexeme)
            } else {
                None
            };
            items.push(ExportItem { name, alias });
            if self.at(TokenKind::Comma) {
                self.bump();
                if self.at(TokenKind::RBrace) {
                    self.error_here_expected("Trailing `,` is not allowed in export list");
                    return None;
                }
                if self.at(TokenKind::Comma) {
                    self.error_here_expected("Expected export symbol name before `,`");
                    return None;
                }
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBrace, "Expected `}` after export list")?;
        self.expect(TokenKind::Semi, "Expected `;` after export declaration")?;
        Some(ExportDecl { items })
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

    fn parse_struct_decl(&mut self) -> Option<StructDecl> {
        self.expect(TokenKind::KwStruct, "Expected `struct`")?;
        let name = self.expect_ident("Expected struct name after `struct`")?;
        self.expect(TokenKind::LBrace, "Expected `{` after struct name")?;
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let field_name = self.expect_ident("Expected field name in struct")?;
            self.expect(TokenKind::Colon, "Expected `:` after field name")?;
            let field_ty = self.expect_type_name("Expected field type after `:`")?;
            fields.push(FieldDecl {
                name: field_name.lexeme,
                ty: field_ty,
            });
            if self.at(TokenKind::Comma) {
                self.bump();
                if self.at(TokenKind::RBrace) {
                    break;
                }
            } else if !self.at(TokenKind::RBrace) {
                self.error_here_expected("Expected `,` or `}` after struct field");
                return None;
            }
        }
        self.expect(TokenKind::RBrace, "Expected `}` after struct declaration")?;
        Some(StructDecl {
            name: name.lexeme,
            fields,
        })
    }

    fn parse_impl_decl(&mut self) -> Option<ImplDecl> {
        self.expect(TokenKind::KwImpl, "Expected `impl`")?;
        let target = self.expect_ident("Expected target type name after `impl`")?;
        self.expect(TokenKind::LBrace, "Expected `{` after impl target")?;
        let mut methods = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            methods.push(self.parse_method_decl(&target.lexeme)?);
        }
        self.expect(TokenKind::RBrace, "Expected `}` after impl block")?;
        Some(ImplDecl {
            target: target.lexeme,
            methods,
        })
    }

    fn parse_method_decl(&mut self, receiver_ty: &str) -> Option<MethodDecl> {
        self.expect(TokenKind::KwFn, "Expected `fn` in impl block")?;
        let name = self.expect_ident("Expected method name after `fn`")?;
        self.expect(TokenKind::LParen, "Expected `(` after method name")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let param_name = self.expect_ident("Expected parameter name")?;
                if param_name.lexeme == "self" {
                    params.push(Param {
                        name: "self".to_string(),
                        ty: TypeName::Named(receiver_ty.to_string()),
                    });
                } else {
                    self.expect(TokenKind::Colon, "Expected `:` after parameter name")?;
                    let param_ty = self.expect_type_name("Expected parameter type after `:`")?;
                    params.push(Param {
                        name: param_name.lexeme,
                        ty: param_ty,
                    });
                }

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

        self.expect(TokenKind::LBrace, "Expected `{` before method body")?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        self.expect(TokenKind::RBrace, "Expected `}` after method body")?;
        Some(MethodDecl {
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
            if self.at(TokenKind::KwImport)
                || self.at(TokenKind::KwFrom)
                || self.at(TokenKind::KwExport)
                || self.at(TokenKind::KwFn)
                || self.at(TokenKind::KwStruct)
                || self.at(TokenKind::KwImpl)
            {
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
