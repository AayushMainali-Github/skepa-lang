use std::fs;
use std::path::PathBuf;

use skeplib::lexer::lex;
use skeplib::token::TokenKind;

fn kinds(src: &str) -> Vec<TokenKind> {
    let (tokens, diags) = lex(src);
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got: {:?}",
        diags.as_slice()
    );
    tokens.into_iter().map(|t| t.kind).collect()
}

#[test]
fn lexes_keywords_and_types() {
    let got = kinds(
        "import fn let if else while for break continue return match true false Int Float Bool String Void",
    );
    let want = vec![
        TokenKind::KwImport,
        TokenKind::KwFn,
        TokenKind::KwLet,
        TokenKind::KwIf,
        TokenKind::KwElse,
        TokenKind::KwWhile,
        TokenKind::KwFor,
        TokenKind::KwBreak,
        TokenKind::KwContinue,
        TokenKind::KwReturn,
        TokenKind::KwMatch,
        TokenKind::KwTrue,
        TokenKind::KwFalse,
        TokenKind::TyInt,
        TokenKind::TyFloat,
        TokenKind::TyBool,
        TokenKind::TyString,
        TokenKind::TyVoid,
        TokenKind::Eof,
    ];
    assert_eq!(got, want);
}

#[test]
fn lexes_operators_and_punctuation() {
    let got = kinds("()[]{}.,:; -> => = + - * / % ! == != < <= > >= && || |");
    let want = vec![
        TokenKind::LParen,
        TokenKind::RParen,
        TokenKind::LBracket,
        TokenKind::RBracket,
        TokenKind::LBrace,
        TokenKind::RBrace,
        TokenKind::Dot,
        TokenKind::Comma,
        TokenKind::Colon,
        TokenKind::Semi,
        TokenKind::Arrow,
        TokenKind::FatArrow,
        TokenKind::Assign,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Bang,
        TokenKind::EqEq,
        TokenKind::Neq,
        TokenKind::Lt,
        TokenKind::Lte,
        TokenKind::Gt,
        TokenKind::Gte,
        TokenKind::AndAnd,
        TokenKind::OrOr,
        TokenKind::Pipe,
        TokenKind::Eof,
    ];
    assert_eq!(got, want);
}

#[test]
fn lexes_literals() {
    let (tokens, diags) = lex("123 3.14 \"hello\" true false");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::IntLit);
    assert_eq!(tokens[1].kind, TokenKind::FloatLit);
    assert_eq!(tokens[2].kind, TokenKind::StringLit);
    assert_eq!(tokens[2].lexeme, "\"hello\"");
    assert_eq!(tokens[3].kind, TokenKind::KwTrue);
    assert_eq!(tokens[4].kind, TokenKind::KwFalse);
}

#[test]
fn ignores_single_and_block_comments() {
    let (tokens, diags) = lex("let x = 1; // comment\n/* multi */ let y = 2;");
    assert!(diags.is_empty());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    let want = vec![
        TokenKind::KwLet,
        TokenKind::Ident,
        TokenKind::Assign,
        TokenKind::IntLit,
        TokenKind::Semi,
        TokenKind::KwLet,
        TokenKind::Ident,
        TokenKind::Assign,
        TokenKind::IntLit,
        TokenKind::Semi,
        TokenKind::Eof,
    ];
    assert_eq!(got, want);
}

#[test]
fn reports_unterminated_string() {
    let (_tokens, diags) = lex("\"hello");
    assert_eq!(diags.len(), 1);
    assert!(diags.as_slice()[0].message.contains("Unterminated string"));
}

#[test]
fn reports_unknown_character() {
    let (_tokens, diags) = lex("@");
    assert_eq!(diags.len(), 1);
    assert!(diags.as_slice()[0].message.contains("Unexpected character"));
}

#[test]
fn lexes_complete_fixture_program() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("full_program.sk");
    let src = fs::read_to_string(path).expect("fixture file should exist");
    let (tokens, diags) = lex(&src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert!(tokens.len() > 20);
    assert_eq!(tokens.last().map(|t| t.kind), Some(TokenKind::Eof));
}

#[test]
fn reports_unterminated_block_comment() {
    let (_tokens, diags) = lex("/* never ends");
    assert_eq!(diags.len(), 1);
    assert!(
        diags.as_slice()[0]
            .message
            .contains("Unterminated block comment")
    );
}

#[test]
fn reports_single_ampersand_and_pipe() {
    let (_tokens, diags) = lex("&");
    assert_eq!(diags.len(), 1);
    assert!(diags.as_slice()[0].message.contains("&&"));
}

#[test]
fn lexes_match_arrow_and_pipe_tokens() {
    let got = kinds("match (x) { 1 | 2 => { } _ => { } }");
    assert!(got.contains(&TokenKind::KwMatch));
    assert!(got.contains(&TokenKind::FatArrow));
    assert!(got.contains(&TokenKind::Pipe));
}

#[test]
fn continues_after_error_and_lexes_rest() {
    let (tokens, diags) = lex("@ let x = 1;");
    assert_eq!(diags.len(), 1);
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    let want = vec![
        TokenKind::KwLet,
        TokenKind::Ident,
        TokenKind::Assign,
        TokenKind::IntLit,
        TokenKind::Semi,
        TokenKind::Eof,
    ];
    assert_eq!(got, want);
}

#[test]
fn lexes_identifiers_with_underscore_and_digits() {
    let (tokens, diags) = lex("_x foo_2 bar99");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::Ident);
    assert_eq!(tokens[0].lexeme, "_x");
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[1].lexeme, "foo_2");
    assert_eq!(tokens[2].kind, TokenKind::Ident);
    assert_eq!(tokens[2].lexeme, "bar99");
}

#[test]
fn lexes_int_then_dot_then_int_when_not_float_form() {
    let (tokens, diags) = lex("12. x");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::IntLit);
    assert_eq!(tokens[0].lexeme, "12");
    assert_eq!(tokens[1].kind, TokenKind::Dot);
    assert_eq!(tokens[2].kind, TokenKind::Ident);
}

#[test]
fn tracks_token_spans_line_and_column() {
    let (tokens, diags) = lex("let x = 1;\nreturn x;");
    assert!(diags.is_empty());

    assert_eq!(tokens[0].kind, TokenKind::KwLet);
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.col, 1);

    let ret = tokens
        .iter()
        .find(|t| t.kind == TokenKind::KwReturn)
        .expect("return token exists");
    assert_eq!(ret.span.line, 2);
    assert_eq!(ret.span.col, 1);
}

#[test]
fn tracks_string_span_start_position() {
    let (tokens, diags) = lex("  \"abc\"");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::StringLit);
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.col, 3);
}

#[test]
fn lexes_string_with_escape_sequences() {
    let (tokens, diags) = lex("\"a\\n\\t\\\"b\\\\c\"");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::StringLit);
}

#[test]
fn lexes_empty_input_to_only_eof() {
    let (tokens, diags) = lex("");
    assert!(diags.is_empty());
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn lexes_float_then_dot_chain() {
    let (tokens, diags) = lex("1.2.3");
    assert!(diags.is_empty());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::FloatLit,
            TokenKind::Dot,
            TokenKind::IntLit,
            TokenKind::Eof
        ]
    );
}

#[test]
fn reports_unterminated_string_with_trailing_escape() {
    let (_tokens, diags) = lex("\"abc\\");
    assert_eq!(diags.len(), 1);
    assert!(
        diags.as_slice()[0]
            .message
            .contains("Unterminated string literal")
    );
}

#[test]
fn reports_unterminated_block_comment_across_newline() {
    let (_tokens, diags) = lex("/* line1\nline2");
    assert_eq!(diags.len(), 1);
    assert!(
        diags.as_slice()[0]
            .message
            .contains("Unterminated block comment")
    );
}

#[test]
fn keywords_inside_identifiers_are_not_keywords() {
    let (tokens, diags) = lex("imported fnx returnValue trueish false0");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::Ident);
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[2].kind, TokenKind::Ident);
    assert_eq!(tokens[3].kind, TokenKind::Ident);
    assert_eq!(tokens[4].kind, TokenKind::Ident);
}
