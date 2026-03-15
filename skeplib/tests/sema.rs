mod common;
mod sema_cases {
    use super::common::{assert_has_diag, sema_err, sema_ok};
    use skeplib::sema::analyze_source;

    mod core;
    mod globals_imports;
    mod packages;
    mod structs;
    mod vec;
}
