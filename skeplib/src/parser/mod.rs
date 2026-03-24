use crate::ast::{
    ExportDecl, ExportItem, FieldDecl, FnDecl, GlobalLetDecl, ImplDecl, ImportDecl, ImportItem,
    MethodDecl, OperatorDecl, Param, Program, StructDecl, TypeName,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::lexer::lex;
use crate::token::{Token, TokenKind};
use std::collections::{HashMap, HashSet};

mod expr;
mod stmt;
mod types;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderFromImport {
    pub path: Vec<String>,
    pub wildcard: bool,
    pub items: Vec<HeaderImportItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceHeaderInfo {
    pub dependency_paths: Vec<Vec<String>>,
    pub from_imports: Vec<HeaderFromImport>,
    pub local_operator_precedences: HashMap<String, i64>,
    pub local_exported_operator_precedences: HashMap<String, i64>,
    pub reexported_operator_paths: Vec<HeaderFromImport>,
    pub export_all_paths: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    idx: usize,
    diagnostics: DiagnosticBag,
    custom_operator_precedences: HashMap<String, i64>,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            tokens: vec![Token::new(TokenKind::Eof, "", Span::default())],
            idx: 0,
            diagnostics: DiagnosticBag::new(),
            custom_operator_precedences: HashMap::new(),
        }
    }
}

impl Parser {
    pub fn parse_source(source: &str) -> (Program, DiagnosticBag) {
        Self::parse_source_with_operator_precedences(source, HashMap::new())
    }

    pub fn parse_source_with_operator_precedences(
        source: &str,
        mut external_operator_precedences: HashMap<String, i64>,
    ) -> (Program, DiagnosticBag) {
        let (tokens, mut diagnostics) = lex(source);
        let custom_operator_precedences = Self::collect_operator_precedences(&tokens);
        for (name, precedence) in custom_operator_precedences {
            external_operator_precedences.insert(name, precedence);
        }
        let mut parser = Parser {
            tokens,
            idx: 0,
            diagnostics: DiagnosticBag::new(),
            custom_operator_precedences: external_operator_precedences,
        };
        let program = parser.parse_program();
        for d in parser.diagnostics.into_vec() {
            diagnostics.push(d);
        }
        (program, diagnostics)
    }

    pub fn scan_source_headers(source: &str) -> SourceHeaderInfo {
        let (tokens, _diagnostics) = lex(source);
        Self::scan_header_tokens(&tokens)
    }

    fn collect_operator_precedences(tokens: &[Token]) -> HashMap<String, i64> {
        Self::scan_header_tokens(tokens).local_operator_precedences
    }

    fn scan_header_tokens(tokens: &[Token]) -> SourceHeaderInfo {
        let mut out = SourceHeaderInfo::default();
        let mut idx = 0usize;
        let mut brace_depth = 0usize;

        while idx < tokens.len() {
            match tokens[idx].kind {
                TokenKind::LBrace => {
                    brace_depth += 1;
                    idx += 1;
                }
                TokenKind::RBrace => {
                    brace_depth = brace_depth.saturating_sub(1);
                    idx += 1;
                }
                TokenKind::KwOpr if brace_depth == 0 => {
                    let Some(name_tok) = tokens.get(idx + 1) else {
                        break;
                    };
                    if name_tok.kind != TokenKind::Ident {
                        idx += 1;
                        continue;
                    }
                    let mut scan = idx + 2;
                    while let Some(tok) = tokens.get(scan) {
                        match tok.kind {
                            TokenKind::KwPrecedence => {
                                if let Some(value_tok) = tokens.get(scan + 1)
                                    && value_tok.kind == TokenKind::IntLit
                                    && let Ok(precedence) = value_tok.lexeme.parse::<i64>()
                                {
                                    out.local_operator_precedences
                                        .insert(name_tok.lexeme.clone(), precedence);
                                }
                                break;
                            }
                            TokenKind::LBrace | TokenKind::Semi | TokenKind::Eof => break,
                            _ => scan += 1,
                        }
                    }
                    idx += 1;
                }
                TokenKind::KwImport if brace_depth == 0 => {
                    let mut scan = idx + 1;
                    let mut path = Vec::new();
                    while let Some(tok) = tokens.get(scan) {
                        if tok.kind != TokenKind::Ident {
                            break;
                        }
                        path.push(tok.lexeme.clone());
                        scan += 1;
                        if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Dot)) {
                            break;
                        }
                        scan += 1;
                    }
                    if !path.is_empty() {
                        out.dependency_paths.push(path);
                    }
                    idx = scan;
                }
                TokenKind::KwFrom if brace_depth == 0 => {
                    let mut scan = idx + 1;
                    let mut path = Vec::new();
                    while let Some(tok) = tokens.get(scan) {
                        if tok.kind != TokenKind::Ident {
                            break;
                        }
                        path.push(tok.lexeme.clone());
                        scan += 1;
                        if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Dot)) {
                            break;
                        }
                        scan += 1;
                    }
                    if !path.is_empty() {
                        out.dependency_paths.push(path.clone());
                    }
                    if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::KwImport)) {
                        idx = scan;
                        continue;
                    }
                    scan += 1;
                    if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Star)) {
                        out.from_imports.push(HeaderFromImport {
                            path,
                            wildcard: true,
                            items: Vec::new(),
                        });
                        idx = scan;
                        continue;
                    }
                    let mut items = Vec::new();
                    while let Some(tok) = tokens.get(scan) {
                        if tok.kind != TokenKind::Ident {
                            break;
                        }
                        let name = tok.lexeme.clone();
                        scan += 1;
                        let alias =
                            if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::KwAs)) {
                                scan += 1;
                                match tokens.get(scan) {
                                    Some(alias_tok) if alias_tok.kind == TokenKind::Ident => {
                                        scan += 1;
                                        Some(alias_tok.lexeme.clone())
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            };
                        items.push(HeaderImportItem { name, alias });
                        if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Comma)) {
                            break;
                        }
                        scan += 1;
                    }
                    out.from_imports.push(HeaderFromImport {
                        path,
                        wildcard: false,
                        items,
                    });
                    idx = scan;
                }
                TokenKind::KwExport if brace_depth == 0 => {
                    let mut scan = idx + 1;
                    if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Star)) {
                        scan += 1;
                        if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::KwFrom)) {
                            scan += 1;
                            let mut path = Vec::new();
                            while let Some(tok) = tokens.get(scan) {
                                if tok.kind != TokenKind::Ident {
                                    break;
                                }
                                path.push(tok.lexeme.clone());
                                scan += 1;
                                if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Dot))
                                {
                                    break;
                                }
                                scan += 1;
                            }
                            if !path.is_empty() {
                                out.dependency_paths.push(path);
                                out.export_all_paths
                                    .push(out.dependency_paths.last().cloned().unwrap_or_default());
                            }
                        }
                        idx = scan;
                        continue;
                    }
                    if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::LBrace)) {
                        idx = scan;
                        continue;
                    }
                    scan += 1;
                    while let Some(tok) = tokens.get(scan) {
                        if tok.kind == TokenKind::RBrace {
                            scan += 1;
                            break;
                        }
                        if tok.kind != TokenKind::Ident {
                            break;
                        }
                        let local_name = tok.lexeme.clone();
                        scan += 1;
                        let export_name =
                            if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::KwAs)) {
                                scan += 1;
                                match tokens.get(scan) {
                                    Some(alias_tok) if alias_tok.kind == TokenKind::Ident => {
                                        scan += 1;
                                        alias_tok.lexeme.clone()
                                    }
                                    _ => local_name.clone(),
                                }
                            } else {
                                local_name.clone()
                            };
                        if let Some(precedence) =
                            out.local_operator_precedences.get(&local_name).copied()
                        {
                            out.local_exported_operator_precedences
                                .insert(export_name, precedence);
                        }
                        if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Comma)) {
                            continue;
                        }
                        scan += 1;
                    }
                    if matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::KwFrom)) {
                        scan += 1;
                        let mut path = Vec::new();
                        while let Some(tok) = tokens.get(scan) {
                            if tok.kind != TokenKind::Ident {
                                break;
                            }
                            path.push(tok.lexeme.clone());
                            scan += 1;
                            if !matches!(tokens.get(scan).map(|t| t.kind), Some(TokenKind::Dot)) {
                                break;
                            }
                            scan += 1;
                        }
                        if !path.is_empty() {
                            out.dependency_paths.push(path);
                            out.reexported_operator_paths.push(HeaderFromImport {
                                path: out.dependency_paths.last().cloned().unwrap_or_default(),
                                wildcard: false,
                                items: Vec::new(),
                            });
                            if let Some(last) = out.reexported_operator_paths.last_mut() {
                                let mut export_scan = idx + 2;
                                while let Some(tok) = tokens.get(export_scan) {
                                    if tok.kind == TokenKind::RBrace {
                                        break;
                                    }
                                    if tok.kind != TokenKind::Ident {
                                        export_scan += 1;
                                        continue;
                                    }
                                    let name = tok.lexeme.clone();
                                    export_scan += 1;
                                    let alias = if matches!(
                                        tokens.get(export_scan).map(|t| t.kind),
                                        Some(TokenKind::KwAs)
                                    ) {
                                        export_scan += 1;
                                        match tokens.get(export_scan) {
                                            Some(alias_tok)
                                                if alias_tok.kind == TokenKind::Ident =>
                                            {
                                                export_scan += 1;
                                                Some(alias_tok.lexeme.clone())
                                            }
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    };
                                    last.items.push(HeaderImportItem { name, alias });
                                    if matches!(
                                        tokens.get(export_scan).map(|t| t.kind),
                                        Some(TokenKind::Comma)
                                    ) {
                                        export_scan += 1;
                                    }
                                }
                            }
                        }
                    }
                    idx = scan;
                }
                _ => {
                    idx += 1;
                }
            }
        }

        out
    }

    fn parse_program(&mut self) -> Program {
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        let mut globals = Vec::new();
        let mut structs = Vec::new();
        let mut impls = Vec::new();
        let mut operators = Vec::new();
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
                    exports.push(e);
                }
                continue;
            }
            if self.at(TokenKind::KwLet) {
                if let Some(g) = self.parse_global_let_decl() {
                    globals.push(g);
                }
                continue;
            }

            if self.at(TokenKind::KwFn) {
                if let Some(f) = self.parse_function() {
                    functions.push(f);
                }
                continue;
            }
            if self.at(TokenKind::KwOpr) {
                if let Some(operator) = self.parse_operator() {
                    self.custom_operator_precedences
                        .insert(operator.name.clone(), operator.precedence);
                    operators.push(operator);
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
                "Expected top-level declaration (`import`, `from`, `export`, `let`, `struct`, `impl`, `opr`, or `fn`)",
            );
            self.synchronize_toplevel();
        }

        Program {
            imports,
            exports,
            globals,
            structs,
            impls,
            operators,
            functions,
        }
    }

    fn parse_global_let_decl(&mut self) -> Option<GlobalLetDecl> {
        self.expect(TokenKind::KwLet, "Expected `let`")?;
        let name = self.expect_ident("Expected variable name after `let`")?;
        let ty = if self.at(TokenKind::Colon) {
            self.bump();
            Some(self.expect_type_name("Expected type after `:` in global let")?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "Expected `=` in global let declaration")?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::Semi, "Expected `;` after global let declaration")?;
        Some(GlobalLetDecl {
            name: name.lexeme,
            ty,
            value,
        })
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
        if self.at(TokenKind::Star) {
            self.bump();
            self.expect(TokenKind::Semi, "Expected `;` after from-import")?;
            return Some(ImportDecl::ImportFrom {
                path,
                wildcard: true,
                items: Vec::new(),
            });
        }
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
        Some(ImportDecl::ImportFrom {
            path,
            wildcard: false,
            items,
        })
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
        if self.at(TokenKind::Star) {
            self.bump();
            self.expect(TokenKind::KwFrom, "Expected `from` after `export *`")?;
            let path = self.parse_dotted_path("Expected module path after `from`")?;
            self.expect(TokenKind::Semi, "Expected `;` after export declaration")?;
            return Some(ExportDecl::FromAll { path });
        }
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
        if self.at(TokenKind::KwFrom) {
            self.bump();
            let path = self.parse_dotted_path("Expected module path after `from`")?;
            self.expect(TokenKind::Semi, "Expected `;` after export declaration")?;
            return Some(ExportDecl::From { path, items });
        }
        self.expect(TokenKind::Semi, "Expected `;` after export declaration")?;
        Some(ExportDecl::Local { items })
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

    fn parse_operator(&mut self) -> Option<OperatorDecl> {
        self.expect(TokenKind::KwOpr, "Expected `opr`")?;
        let name = self.expect_ident("Expected operator name after `opr`")?;
        self.expect(TokenKind::LParen, "Expected `(` after operator name")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let param_name = self.expect_ident("Expected operator parameter name")?;
                self.expect(
                    TokenKind::Colon,
                    "Expected `:` after operator parameter name",
                )?;
                let param_ty = self.expect_type_name("Expected operator parameter type")?;
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
        self.expect(TokenKind::RParen, "Expected `)` after operator parameters")?;
        self.expect(TokenKind::Arrow, "Expected `->` after operator parameters")?;
        let return_type = self.expect_type_name("Expected operator return type after `->`")?;
        self.expect(
            TokenKind::KwPrecedence,
            "Expected `precedence` after operator return type",
        )?;
        let precedence = self
            .expect(
                TokenKind::IntLit,
                "Expected integer precedence after `precedence`",
            )?
            .lexeme
            .parse::<i64>()
            .ok()?;
        self.expect(TokenKind::LBrace, "Expected `{` before operator body")?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        self.expect(TokenKind::RBrace, "Expected `}` after operator body")?;

        Some(OperatorDecl {
            name: name.lexeme,
            params,
            return_type,
            precedence,
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
                | TokenKind::KwMatch
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
                || self.at(TokenKind::KwLet)
                || self.at(TokenKind::KwFn)
                || self.at(TokenKind::KwOpr)
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
