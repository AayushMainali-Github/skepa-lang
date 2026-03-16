use skepart::{RtArray, RtErrorKind, RtString, RtValue, RtVec};

#[test]
fn arrays_use_copy_on_write_value_storage() {
    let mut first = RtArray::repeat(RtValue::Int(1), 3);
    let second = first.clone();

    first
        .set(1, RtValue::Int(9))
        .expect("array write should succeed");

    assert_eq!(first.get(1), Ok(RtValue::Int(9)));
    assert_eq!(second.get(1), Ok(RtValue::Int(1)));
}

#[test]
fn vecs_use_shared_handle_semantics() {
    let first = RtVec::new();
    let second = first.clone();

    first.push(RtValue::Int(7));

    assert_eq!(second.borrow().as_slice(), &[RtValue::Int(7)]);
}

#[test]
fn strings_track_character_length() {
    let value = RtString::from("naive");
    assert_eq!(value.len_chars(), 5);
}

#[test]
fn arrays_report_bounds_errors() {
    let array = RtArray::repeat(RtValue::Int(0), 2);
    let err = array.get(3).expect_err("index should be rejected");
    assert_eq!(err.kind, RtErrorKind::IndexOutOfBounds);
}

#[test]
fn vec_helpers_support_get_set_and_delete() {
    let vec = RtVec::new();
    vec.push(RtValue::Int(1));
    vec.push(RtValue::Int(2));
    vec.set(1, RtValue::Int(9)).expect("set should work");

    assert_eq!(vec.get(1), Ok(RtValue::Int(9)));
    assert_eq!(vec.delete(0), Ok(RtValue::Int(1)));
    assert_eq!(vec.get(0), Ok(RtValue::Int(9)));
}

#[test]
fn string_builtins_match_current_runtime_shape() {
    let value = RtString::from("skepa-language-benchmark");
    let needle = RtString::from("bench");

    assert_eq!(skepart::str_builtin::len(&value), 24);
    assert_eq!(skepart::str_builtin::index_of(&value, &needle), 15);
    assert!(skepart::str_builtin::contains(
        &RtString::from("language"),
        &RtString::from("gua")
    ));
    assert_eq!(
        skepart::str_builtin::slice(&value, 6, 18).expect("slice should work"),
        RtString::from("language-ben")
    );
}
