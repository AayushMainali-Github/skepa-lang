use skepart::{RtArray, RtErrorKind, RtString, RtValue};

#[test]
fn arrays_use_copy_on_write_and_preserve_nested_values() {
    let mut first = RtArray::repeat(
        RtValue::Array(RtArray::new(vec![RtValue::Int(1), RtValue::Int(2)])),
        2,
    );
    let second = first.clone();
    first.set(0, RtValue::Int(9)).expect("set should work");

    assert_eq!(first.get(0), Ok(RtValue::Int(9)));
    match second.get(0).expect("nested array should remain") {
        RtValue::Array(inner) => assert_eq!(inner.get(1), Ok(RtValue::Int(2))),
        other => panic!("expected nested array, got {other:?}"),
    }
}

#[test]
fn arrays_report_get_and_set_bounds() {
    let mut array = RtArray::repeat(RtValue::Int(0), 2);
    assert_eq!(
        array.get(3).expect_err("oob").kind,
        RtErrorKind::IndexOutOfBounds
    );
    assert_eq!(
        array.set(2, RtValue::Int(4)).expect_err("oob set").kind,
        RtErrorKind::IndexOutOfBounds
    );
}

#[test]
fn arrays_store_mixed_runtime_values() {
    let array = RtArray::new(vec![
        RtValue::Int(1),
        RtValue::Bool(true),
        RtValue::String(RtString::from("hi")),
    ]);
    assert_eq!(array.get(0), Ok(RtValue::Int(1)));
    assert_eq!(array.get(1), Ok(RtValue::Bool(true)));
    assert_eq!(array.get(2), Ok(RtValue::String(RtString::from("hi"))));
}

#[test]
fn arrays_repeat_backing_isolated_after_write() {
    let mut first = RtArray::repeat(RtValue::Int(3), 4);
    let second = first.clone();
    first.set(3, RtValue::Int(8)).expect("set should work");
    assert_eq!(first.get(3), Ok(RtValue::Int(8)));
    assert_eq!(second.get(3), Ok(RtValue::Int(3)));
}

#[test]
fn arrays_alias_without_write_and_detach_after_write() {
    let mut first = RtArray::new(vec![RtValue::Int(1), RtValue::Int(2)]);
    let second = first.clone();
    assert_eq!(first.get(1), second.get(1));

    first.set(0, RtValue::Int(9)).expect("detach write");
    assert_eq!(first.get(0), Ok(RtValue::Int(9)));
    assert_eq!(second.get(0), Ok(RtValue::Int(1)));
}

#[test]
fn arrays_support_deeper_nested_mixed_values() {
    let inner = RtArray::new(vec![
        RtValue::String(RtString::from("x")),
        RtValue::Bool(false),
    ]);
    let outer = RtArray::new(vec![RtValue::Array(inner.clone()), RtValue::Int(7)]);

    match outer.get(0).expect("nested outer") {
        RtValue::Array(value) => {
            assert_eq!(value.get(0), Ok(RtValue::String(RtString::from("x"))));
            assert_eq!(value.get(1), Ok(RtValue::Bool(false)));
        }
        other => panic!("expected nested array, got {other:?}"),
    }
}
