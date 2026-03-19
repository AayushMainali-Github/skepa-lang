#[path = "../common.rs"]
mod common;

mod cases {
    use super::common::{assert_has_diag, assert_no_diags, parse_err, parse_ok};
    use skeplib::ast::{
        AssignTarget, BinaryOp, Expr, MatchLiteral, MatchPattern, Stmt, TypeName, UnaryOp,
    };
    use skeplib::parser::Parser;

    mod control_flow;
    mod exprs;
    mod functions_types;
    mod imports_exports;
    mod structs;
}

mod fixtures;
