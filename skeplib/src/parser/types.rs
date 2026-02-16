use crate::ast::TypeName;
use crate::diagnostic::Span;
use crate::token::TokenKind;

use super::Parser;

impl Parser {
    pub(super) fn expect_type_name(&mut self, message: &str) -> Option<TypeName> {
        if self.at(TokenKind::LBracket) {
            self.bump();
            let elem = self.expect_type_name("Expected element type in array type")?;
            self.expect(TokenKind::Semi, "Expected `;` in array type")?;
            let sz = self.expect(TokenKind::IntLit, "Expected integer size in array type")?;
            let size = match sz.lexeme.parse::<usize>() {
                Ok(v) => v,
                Err(_) => {
                    self.error_here_expected("Expected valid integer size in array type");
                    return None;
                }
            };
            self.expect(TokenKind::RBracket, "Expected `]` after array type")?;
            return Some(TypeName::Array {
                elem: Box::new(elem),
                size,
            });
        }
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

    pub(super) fn decode_string_escapes(&mut self, raw: &str, span: Span) -> String {
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
