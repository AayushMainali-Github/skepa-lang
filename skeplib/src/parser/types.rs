use crate::ast::TypeName;
use crate::diagnostic::Span;
use crate::token::TokenKind;

use super::Parser;

impl Parser {
    pub(super) fn expect_type_name(&mut self, message: &str) -> Option<TypeName> {
        if self.at(TokenKind::Ident) && self.current().lexeme == "Fn" {
            self.bump();
            self.expect(TokenKind::LParen, "Expected `(` after `Fn`")?;
            let mut params = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    let ty = self.expect_type_name("Expected function parameter type")?;
                    params.push(ty);
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
                "Expected `)` after function type parameters",
            )?;
            self.expect(
                TokenKind::Arrow,
                "Expected `->` after function type parameters",
            )?;
            let ret = self.expect_type_name("Expected function return type after `->`")?;
            return Some(TypeName::Fn {
                params,
                ret: Box::new(ret),
            });
        }

        if self.at(TokenKind::Ident) && self.current().lexeme == "Vec" {
            self.bump();
            self.expect(TokenKind::LBracket, "Expected `[` after `Vec`")?;
            let elem = self.expect_type_name("Expected vector element type in `Vec[...]`")?;
            if self.at(TokenKind::Comma) {
                self.error_here_expected("`Vec[...]` expects exactly one type argument");
                return None;
            }
            self.expect(TokenKind::RBracket, "Expected `]` after vector type")?;
            return Some(TypeName::Vec {
                elem: Box::new(elem),
            });
        }

        if self.at(TokenKind::Ident) && self.current().lexeme == "Option" {
            self.bump();
            self.expect(TokenKind::LBracket, "Expected `[` after `Option`")?;
            let value = self.expect_type_name("Expected option value type in `Option[...]`")?;
            if self.at(TokenKind::Comma) {
                self.error_here_expected("`Option[...]` expects exactly one type argument");
                return None;
            }
            self.expect(TokenKind::RBracket, "Expected `]` after option type")?;
            return Some(TypeName::Option {
                value: Box::new(value),
            });
        }

        if self.at(TokenKind::Ident) && self.current().lexeme == "Result" {
            self.bump();
            self.expect(TokenKind::LBracket, "Expected `[` after `Result`")?;
            let ok = self.expect_type_name("Expected ok type in `Result[..., ...]`")?;
            self.expect(TokenKind::Comma, "Expected `,` after result ok type")?;
            let err = self.expect_type_name("Expected error type in `Result[..., ...]`")?;
            self.expect(TokenKind::RBracket, "Expected `]` after result type")?;
            return Some(TypeName::Result {
                ok: Box::new(ok),
                err: Box::new(err),
            });
        }

        if self.at(TokenKind::Ident) && self.current().lexeme == "Map" {
            self.bump();
            self.expect(TokenKind::LBracket, "Expected `[` after `Map`")?;
            let key = self.expect_type_name("Expected map key type in `Map[...]`")?;
            self.expect(TokenKind::Comma, "Expected `,` after map key type")?;
            let value = self.expect_type_name("Expected map value type in `Map[...]`")?;
            self.expect(TokenKind::RBracket, "Expected `]` after map type")?;
            if key != TypeName::String {
                self.error_here_expected("`Map[...]` requires `String` keys");
                return None;
            }
            return Some(TypeName::Map {
                value: Box::new(value),
            });
        }

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
            TokenKind::TyBytes => TypeName::Bytes,
            TokenKind::TyVoid => TypeName::Void,
            TokenKind::Ident => {
                let mut name = self.bump().lexeme;
                while self.at(TokenKind::Dot) {
                    self.bump();
                    let seg = self.expect_ident("Expected identifier after `.` in type name")?;
                    name.push('.');
                    name.push_str(&seg.lexeme);
                }
                if (name == "task.Channel" || name == "task.Task") && self.at(TokenKind::LBracket) {
                    self.bump();
                    let type_label = if name == "task.Channel" {
                        "channel value type"
                    } else {
                        "task result type"
                    };
                    let elem =
                        self.expect_type_name(&format!("Expected {type_label} in `{name}[...]`"))?;
                    if self.at(TokenKind::Comma) {
                        self.error_here_expected(&format!(
                            "`{name}[...]` expects exactly one type argument"
                        ));
                        return None;
                    }
                    self.expect(
                        TokenKind::RBracket,
                        &format!("Expected `]` after {name} type"),
                    )?;
                    return Some(TypeName::Named(format!("{name}[{}]", elem.as_str())));
                }
                return Some(TypeName::Named(name));
            }
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
