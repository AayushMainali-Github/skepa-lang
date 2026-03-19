use std::fs;
use std::path::PathBuf;

#[path = "common.rs"]
mod common;

use skeplib::lexer::lex;
use skeplib::token::TokenKind;

fn kinds(src: &str) -> Vec<TokenKind> {
    let (tokens, diags) = lex(src);
    common::assert_no_diags(&diags);
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
fn lexes_module_and_decl_keywords() {
    let got = kinds("from as export struct impl");
    let want = vec![
        TokenKind::KwFrom,
        TokenKind::KwAs,
        TokenKind::KwExport,
        TokenKind::KwStruct,
        TokenKind::KwImpl,
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
    common::assert_no_diags(&diags);
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
    common::assert_no_diags(&diags);
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
fn ignores_comments_at_end_of_file() {
    let (tokens, diags) = lex("let x = 1; // trailing comment");
    common::assert_no_diags(&diags);
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::KwLet,
            TokenKind::Ident,
            TokenKind::Assign,
            TokenKind::IntLit,
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn ignores_block_comment_with_punctuation_and_keywords_inside() {
    let (tokens, diags) = lex("/* if else == [] {} // not real */ return;");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![TokenKind::KwReturn, TokenKind::Semi, TokenKind::Eof]
    );
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
fn unknown_character_reports_line_and_column() {
    let (_tokens, diags) = lex("let x = 1;\n@");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags.as_slice()[0].span.line, 2);
    assert_eq!(diags.as_slice()[0].span.col, 1);
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
fn single_ampersand_reports_error_and_continues_lexing() {
    let (tokens, diags) = lex("& let ok = 1;");
    assert_eq!(diags.len(), 1);
    assert!(diags.as_slice()[0].message.contains("&&"));
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::KwLet,
            TokenKind::Ident,
            TokenKind::Assign,
            TokenKind::IntLit,
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
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
fn lower_case_type_names_are_plain_identifiers() {
    let (tokens, diags) = lex("int float bool string void");
    assert!(diags.is_empty());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::Ident,
            TokenKind::Ident,
            TokenKind::Ident,
            TokenKind::Ident,
            TokenKind::Ident,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_number_followed_by_identifier_as_separate_tokens() {
    let (tokens, diags) = lex("123abc");
    assert!(diags.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::IntLit);
    assert_eq!(tokens[0].lexeme, "123");
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[1].lexeme, "abc");
    assert_eq!(tokens[2].kind, TokenKind::Eof);
}

#[test]
fn lexes_float_followed_by_identifier_as_separate_tokens() {
    let (tokens, diags) = lex("3.14foo");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::FloatLit);
    assert_eq!(tokens[0].lexeme, "3.14");
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[1].lexeme, "foo");
    assert_eq!(tokens[2].kind, TokenKind::Eof);
}

#[test]
fn lexes_double_dot_as_two_dot_tokens() {
    let (tokens, diags) = lex("1..2");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::IntLit,
            TokenKind::Dot,
            TokenKind::Dot,
            TokenKind::IntLit,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_zero_dot_as_int_then_dot_tokens() {
    let (tokens, diags) = lex("0.");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(got, vec![TokenKind::IntLit, TokenKind::Dot, TokenKind::Eof]);
}

#[test]
fn lexes_leading_dot_number_as_dot_then_int_tokens() {
    let (tokens, diags) = lex(".5");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(got, vec![TokenKind::Dot, TokenKind::IntLit, TokenKind::Eof]);
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
fn tracks_position_after_multiline_block_comment() {
    let (tokens, diags) = lex("/* line1\nline2 */\nfn main");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::KwFn);
    assert_eq!(tokens[0].span.line, 3);
    assert_eq!(tokens[0].span.col, 1);
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[1].lexeme, "main");
}

#[test]
fn lexes_string_with_escape_sequences() {
    let (tokens, diags) = lex("\"a\\n\\t\\\"b\\\\c\"");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::StringLit);
}

#[test]
fn string_literal_keeps_raw_escape_lexeme() {
    let (tokens, diags) = lex("\"a\\n\\\"b\\\\c\"");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::StringLit);
    assert_eq!(tokens[0].lexeme, "\"a\\n\\\"b\\\\c\"");
}

#[test]
fn lexes_long_escaped_string_with_unicode_contents() {
    let src = "\"alpha\\n\\tbeta\\\"quote\\\"\\\\नमस्ते終\"";
    let (tokens, diags) = lex(src);
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::StringLit);
    assert_eq!(tokens[0].lexeme, src);
    assert_eq!(tokens[1].kind, TokenKind::Eof);
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
fn unterminated_string_before_newline_recovers_on_next_line() {
    let (tokens, diags) = lex("\"abc\nlet x = 1;");
    assert_eq!(diags.len(), 1);
    assert!(
        diags.as_slice()[0]
            .message
            .contains("Unterminated string literal")
    );
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::KwLet,
            TokenKind::Ident,
            TokenKind::Assign,
            TokenKind::IntLit,
            TokenKind::Semi,
            TokenKind::Eof,
        ]
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

#[test]
fn comments_split_tokens_cleanly_at_boundaries_and_eof() {
    let (tokens, diags) = lex("foo/* gap */bar// trailing");
    assert!(diags.is_empty(), "diagnostics: {:?}", diags.as_slice());
    assert_eq!(tokens[0].kind, TokenKind::Ident);
    assert_eq!(tokens[0].lexeme, "foo");
    assert_eq!(tokens[1].kind, TokenKind::Ident);
    assert_eq!(tokens[1].lexeme, "bar");
    assert_eq!(tokens[2].kind, TokenKind::Eof);
}

#[test]
fn pathological_recovery_reports_many_bad_chars_and_keeps_valid_tokens() {
    let (tokens, diags) = lex("@#^ let ok = 1; $");
    assert_eq!(diags.len(), 4, "diagnostics: {:?}", diags.as_slice());
    let got: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        got,
        vec![
            TokenKind::KwLet,
            TokenKind::Ident,
            TokenKind::Assign,
            TokenKind::IntLit,
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
}
