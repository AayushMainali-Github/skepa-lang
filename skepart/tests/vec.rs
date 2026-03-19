use skepart::{RtErrorKind, RtFunctionRef, RtString, RtStruct, RtValue, RtVec};

#[test]
fn vecs_share_aliasing_and_support_boundary_mutations() {
    let first = RtVec::new();
    let second = first.clone();
    first.push(RtValue::Int(1));
    first.push(RtValue::Int(2));
    first.push(RtValue::Int(3));
    second.set(0, RtValue::Int(9)).expect("set should work");
    assert_eq!(first.get(0), Ok(RtValue::Int(9)));
    assert_eq!(second.delete(2), Ok(RtValue::Int(3)));
    assert_eq!(first.len(), 2);
}

#[test]
fn vecs_report_empty_and_oob_errors() {
    let vec = RtVec::new();
    assert_eq!(
        vec.get(0).expect_err("empty get").kind,
        RtErrorKind::IndexOutOfBounds
    );
    assert_eq!(
        vec.delete(0).expect_err("empty delete").kind,
        RtErrorKind::IndexOutOfBounds
    );
}

#[test]
fn vecs_hold_structs_functions_and_strings() {
    let vec = RtVec::new();
    vec.push(RtValue::Struct(
        RtStruct::named("Pair", vec![RtValue::Int(1)]).expect("struct"),
    ));
    vec.push(RtValue::Function(RtFunctionRef(7)));
    vec.push(RtValue::String(RtString::from("done")));
    assert!(matches!(vec.get(0), Ok(RtValue::Struct(_))));
    assert_eq!(vec.get(1), Ok(RtValue::Function(RtFunctionRef(7))));
    assert_eq!(vec.get(2), Ok(RtValue::String(RtString::from("done"))));
}

#[test]
fn vecs_survive_large_mutation_sequences() {
    let vec = RtVec::new();
    for i in 0..64 {
        vec.push(RtValue::Int(i));
    }
    for i in 0..32 {
        vec.set(i, RtValue::Int(i as i64 * 10)).expect("set");
    }
    for _ in 0..16 {
        vec.delete(0).expect("delete");
    }
    assert_eq!(vec.len(), 48);
    assert_eq!(vec.get(0), Ok(RtValue::Int(160)));
}

#[test]
fn vecs_delete_middle_and_last_boundary_positions() {
    let vec = RtVec::new();
    vec.push(RtValue::Int(1));
    vec.push(RtValue::Int(2));
    vec.push(RtValue::Int(3));
    vec.push(RtValue::Int(4));

    assert_eq!(vec.delete(1), Ok(RtValue::Int(2)));
    assert_eq!(vec.delete(2), Ok(RtValue::Int(4)));
    assert_eq!(vec.len(), 2);
    assert_eq!(vec.get(0), Ok(RtValue::Int(1)));
    assert_eq!(vec.get(1), Ok(RtValue::Int(3)));
}

#[test]
fn vecs_nested_runtime_values_observe_shared_aliasing() {
    let outer = RtVec::new();
    let inner = RtVec::new();
    inner.push(RtValue::Int(5));
    outer.push(RtValue::Vec(inner.clone()));

    let alias = outer.clone();
    inner.set(0, RtValue::Int(9)).expect("nested set");

    match alias.get(0).expect("nested vec") {
        RtValue::Vec(value) => assert_eq!(value.get(0), Ok(RtValue::Int(9))),
        other => panic!("expected nested vec, got {other:?}"),
    }
}
