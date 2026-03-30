use skepart::{RtErrorKind, RtFunctionRef, RtStruct, RtStructLayout, RtValue};
use std::sync::Arc;

#[test]
fn structs_support_named_and_indexed_field_access() {
    let mut strukt = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Pair".into(),
            field_names: vec!["a".into(), "b".into()],
            field_types: vec![Some("Int"), Some("Function")],
        }),
        vec![RtValue::Int(1), RtValue::Function(RtFunctionRef(9))],
    )
    .expect("valid struct");
    assert_eq!(strukt.get_field(0), Ok(RtValue::Int(1)));
    assert_eq!(
        strukt.get_named_field("b"),
        Ok(RtValue::Function(RtFunctionRef(9)))
    );
    strukt
        .set_field(0, RtValue::Int(7))
        .expect("set field should work");
    assert_eq!(strukt.get_named_field("a"), Ok(RtValue::Int(7)));
}

#[test]
fn structs_report_missing_field_and_layout_mismatches() {
    let strukt = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Only".into(),
            field_names: vec!["x".into()],
            field_types: vec![Some("Int")],
        }),
        vec![RtValue::Int(1)],
    )
    .expect("valid struct");
    assert_eq!(
        strukt.get_field(2).expect_err("bad index").kind,
        RtErrorKind::MissingField
    );
    assert_eq!(
        strukt.get_named_field("y").expect_err("bad name").kind,
        RtErrorKind::MissingField
    );
}

#[test]
fn structs_report_set_field_out_of_range_and_named_layout_mismatch() {
    let strukt = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Mismatch".into(),
            field_names: vec!["left".into(), "right".into()],
            field_types: vec![Some("Int"), Some("Int")],
        }),
        vec![RtValue::Int(1)],
    )
    .expect_err("field count mismatch");
    assert_eq!(strukt.kind, RtErrorKind::MissingField);
}

#[test]
fn structs_reject_layout_and_field_count_mismatch_at_construction() {
    let err = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Mismatch".into(),
            field_names: vec!["left".into(), "right".into()],
            field_types: vec![Some("Int"), Some("Int")],
        }),
        vec![RtValue::Int(1)],
    )
    .expect_err("field count mismatch");
    assert_eq!(err.kind, RtErrorKind::MissingField);
}

#[test]
fn structs_named_constructor_allows_untyped_field_lists() {
    let mut strukt = RtStruct::named("Mismatch", vec![RtValue::Int(1)]).expect("named struct");
    assert_eq!(
        strukt
            .set_field(1, RtValue::Int(2))
            .expect_err("set out of range")
            .kind,
        RtErrorKind::MissingField
    );
    assert_eq!(
        strukt
            .get_named_field("right")
            .expect_err("named layout mismatch")
            .kind,
        RtErrorKind::MissingField
    );
}

#[test]
fn structs_can_nest_other_struct_values() {
    let inner = RtStruct::named("Inner", vec![RtValue::Int(2)]).expect("inner");
    let outer = RtStruct::named("Outer", vec![RtValue::Struct(inner.clone())]).expect("outer");
    assert_eq!(outer.get_field(0), Ok(RtValue::Struct(inner)));
}

#[test]
fn structs_reject_wrong_field_type_when_layout_declares_runtime_types() {
    let mut strukt = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Typed".into(),
            field_names: vec!["a".into(), "b".into()],
            field_types: vec![Some("Int"), Some("Bool")],
        }),
        vec![RtValue::Int(1), RtValue::Bool(true)],
    )
    .expect("typed struct");

    let err = strukt
        .set_field(1, RtValue::Int(2))
        .expect_err("wrong runtime field type");
    assert_eq!(err.kind, RtErrorKind::TypeMismatch);
}

#[test]
fn structs_keep_typed_storage_until_mixed_mutation() {
    let mut strukt = RtStruct::new(
        Arc::new(RtStructLayout {
            name: "Typed".into(),
            field_names: vec!["a".into(), "b".into()],
            field_types: vec![None, None],
        }),
        vec![RtValue::Int(1), RtValue::Int(2)],
    )
    .expect("typed struct");

    strukt.set_field(1, RtValue::Int(9)).expect("typed update");
    assert_eq!(strukt.get_field(0), Ok(RtValue::Int(1)));
    assert_eq!(strukt.get_field(1), Ok(RtValue::Int(9)));

    strukt
        .set_field(0, RtValue::String("mixed".into()))
        .expect("mixed fallback");
    assert_eq!(strukt.get_field(0), Ok(RtValue::String("mixed".into())));
    assert_eq!(strukt.get_field(1), Ok(RtValue::Int(9)));
}
