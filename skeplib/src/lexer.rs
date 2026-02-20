use crate::diagnostic::{DiagnosticBag, Span};
use crate::token::{Token, TokenKind};

pub fn lex(source: &str) -> (Vec<Token>, DiagnosticBag) {
    let mut lexer = Lexer::new(source);
    lexer.lex_all();
    (lexer.tokens, lexer.diagnostics)
}

struct Lexer {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    col: usize,
    tokens: Vec<Token>,
    diagnostics: DiagnosticBag,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            idx: 0,
            line: 1,
            col: 1,
            tokens: Vec::new(),
            diagnostics: DiagnosticBag::new(),
        }
    }

    fn lex_all(&mut self) {
        while !self.is_eof() {
            if self.skip_ws_or_comment() {
                continue;
            }
            self.lex_token();
        }
        self.tokens
            .push(Token::new(TokenKind::Eof, "", self.current_span(self.idx)));
    }

    fn lex_token(&mut self) {
        let start = self.idx;
        let line = self.line;
        let col = self.col;
        let c = self.peek().unwrap_or('\0');

        if Self::is_ident_start(c) {
            self.lex_ident_or_keyword(start, line, col);
            return;
        }

        if c.is_ascii_digit() {
            self.lex_number(start, line, col);
            return;
        }

        match c {
            '"' => self.lex_string(start, line, col),
            '(' => self.single(TokenKind::LParen, start, line, col),
            ')' => self.single(TokenKind::RParen, start, line, col),
            '[' => self.single(TokenKind::LBracket, start, line, col),
            ']' => self.single(TokenKind::RBracket, start, line, col),
            '{' => self.single(TokenKind::LBrace, start, line, col),
            '}' => self.single(TokenKind::RBrace, start, line, col),
            ',' => self.single(TokenKind::Comma, start, line, col),
            '.' => self.single(TokenKind::Dot, start, line, col),
            ':' => self.single(TokenKind::Colon, start, line, col),
            ';' => self.single(TokenKind::Semi, start, line, col),
            '+' => self.single(TokenKind::Plus, start, line, col),
            '*' => self.single(TokenKind::Star, start, line, col),
            '/' => self.single(TokenKind::Slash, start, line, col),
            '%' => self.single(TokenKind::Percent, start, line, col),
            '-' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    self.push_token(TokenKind::Arrow, start, line, col);
                } else {
                    self.push_token(TokenKind::Minus, start, line, col);
                }
            }
            '=' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.push_token(TokenKind::EqEq, start, line, col);
                } else {
                    self.push_token(TokenKind::Assign, start, line, col);
                }
            }
            '!' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.push_token(TokenKind::Neq, start, line, col);
                } else {
                    self.push_token(TokenKind::Bang, start, line, col);
                }
            }
            '<' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.push_token(TokenKind::Lte, start, line, col);
                } else {
                    self.push_token(TokenKind::Lt, start, line, col);
                }
            }
            '>' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.push_token(TokenKind::Gte, start, line, col);
                } else {
                    self.push_token(TokenKind::Gt, start, line, col);
                }
            }
            '&' => {
                self.bump();
                if self.peek() == Some('&') {
                    self.bump();
                    self.push_token(TokenKind::AndAnd, start, line, col);
                } else {
                    self.diagnostics.error(
                        "Unexpected '&'. Did you mean '&&'?",
                        Span::new(start, self.idx, line, col),
                    );
                }
            }
            '|' => {
                self.bump();
                if self.peek() == Some('|') {
                    self.bump();
                    self.push_token(TokenKind::OrOr, start, line, col);
                } else {
                    self.diagnostics.error(
                        "Unexpected '|'. Did you mean '||'?",
                        Span::new(start, self.idx, line, col),
                    );
                }
            }
            _ => {
                self.bump();
                self.diagnostics.error(
                    format!("Unexpected character '{c}'"),
                    Span::new(start, self.idx, line, col),
                );
            }
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize, line: usize, col: usize) {
        self.bump();
        while matches!(self.peek(), Some(ch) if Self::is_ident_continue(ch)) {
            self.bump();
        }
        let lexeme = self.slice(start, self.idx);
        let kind = match lexeme.as_str() {
            "import" => TokenKind::KwImport,
            "from" => TokenKind::KwFrom,
            "as" => TokenKind::KwAs,
            "export" => TokenKind::KwExport,
            "fn" => TokenKind::KwFn,
            "struct" => TokenKind::KwStruct,
            "impl" => TokenKind::KwImpl,
            "let" => TokenKind::KwLet,
            "if" => TokenKind::KwIf,
            "else" => TokenKind::KwElse,
            "while" => TokenKind::KwWhile,
            "for" => TokenKind::KwFor,
            "break" => TokenKind::KwBreak,
            "continue" => TokenKind::KwContinue,
            "return" => TokenKind::KwReturn,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            "Int" => TokenKind::TyInt,
            "Float" => TokenKind::TyFloat,
            "Bool" => TokenKind::TyBool,
            "String" => TokenKind::TyString,
            "Void" => TokenKind::TyVoid,
            _ => TokenKind::Ident,
        };
        self.tokens.push(Token::new(
            kind,
            lexeme,
            Span::new(start, self.idx, line, col),
        ));
    }

    fn lex_number(&mut self, start: usize, line: usize, col: usize) {
        self.bump();
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.bump();
        }

        let mut kind = TokenKind::IntLit;
        if self.peek() == Some('.') && matches!(self.peek_next(), Some(ch) if ch.is_ascii_digit()) {
            kind = TokenKind::FloatLit;
            self.bump();
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.bump();
            }
        }

        let lexeme = self.slice(start, self.idx);
        self.tokens.push(Token::new(
            kind,
            lexeme,
            Span::new(start, self.idx, line, col),
        ));
    }

    fn lex_string(&mut self, start: usize, line: usize, col: usize) {
        self.bump();
        let mut terminated = false;
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.bump();
                terminated = true;
                break;
            }
            if ch == '\\' {
                self.bump();
                if self.peek().is_some() {
                    self.bump();
                }
                continue;
            }
            if ch == '\n' {
                break;
            }
            self.bump();
        }

        if !terminated {
            self.diagnostics.error(
                "Unterminated string literal",
                Span::new(start, self.idx, line, col),
            );
            return;
        }

        let lexeme = self.slice(start, self.idx);
        self.tokens.push(Token::new(
            TokenKind::StringLit,
            lexeme,
            Span::new(start, self.idx, line, col),
        ));
    }

    fn skip_ws_or_comment(&mut self) -> bool {
        let mut progressed = false;

        loop {
            while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
                self.bump();
                progressed = true;
            }

            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                progressed = true;
                self.bump();
                self.bump();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.bump();
                }
                continue;
            }

            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                progressed = true;
                let start = self.idx;
                let line = self.line;
                let col = self.col;
                self.bump();
                self.bump();
                let mut terminated = false;
                while !self.is_eof() {
                    if self.peek() == Some('*') && self.peek_next() == Some('/') {
                        self.bump();
                        self.bump();
                        terminated = true;
                        break;
                    }
                    self.bump();
                }
                if !terminated {
                    self.diagnostics.error(
                        "Unterminated block comment",
                        Span::new(start, self.idx, line, col),
                    );
                }
                continue;
            }

            break;
        }

        progressed
    }

    fn single(&mut self, kind: TokenKind, start: usize, line: usize, col: usize) {
        self.bump();
        self.push_token(kind, start, line, col);
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, line: usize, col: usize) {
        let lexeme = self.slice(start, self.idx);
        self.tokens.push(Token::new(
            kind,
            lexeme,
            Span::new(start, self.idx, line, col),
        ));
    }

    fn current_span(&self, start: usize) -> Span {
        Span::new(start, self.idx, self.line, self.col)
    }

    fn slice(&self, start: usize, end: usize) -> String {
        self.chars[start..end].iter().collect()
    }

    fn bump(&mut self) -> Option<char> {
        if self.is_eof() {
            return None;
        }
        let ch = self.chars[self.idx];
        self.idx += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.idx + 1).copied()
    }

    fn is_eof(&self) -> bool {
        self.idx >= self.chars.len()
    }

    fn is_ident_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }

    fn is_ident_continue(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }
}
