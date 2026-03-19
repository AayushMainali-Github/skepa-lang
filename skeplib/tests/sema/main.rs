#[path = "../common.rs"]
mod common;

mod cases {
    use super::common::{assert_has_diag, assert_sema_success, sema_err, sema_ok};
    use skeplib::sema::analyze_source;

    mod core;
    mod globals_imports;
    mod packages;
    mod structs;
    mod vec;
}

mod fixtures;
