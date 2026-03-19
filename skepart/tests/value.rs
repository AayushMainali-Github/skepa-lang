use skepart::{RtArray, RtErrorKind, RtFunctionRef, RtString, RtStruct, RtValue, RtVec};

#[test]
fn value_accessors_return_expected_values() {
    assert_eq!(RtValue::Int(4).expect_int(), Ok(4));
    assert_eq!(RtValue::Float(2.5).expect_float(), Ok(2.5));
    assert_eq!(RtValue::Bool(true).expect_bool(), Ok(true));
    assert_eq!(
        RtValue::String(RtString::from("hi")).expect_string(),
        Ok(RtString::from("hi"))
    );
    assert_eq!(
        RtValue::Array(RtArray::new(vec![RtValue::Int(1)])).expect_array(),
        Ok(RtArray::new(vec![RtValue::Int(1)]))
    );
    let vec = RtVec::new();
    vec.push(RtValue::Int(9));
    assert_eq!(RtValue::Vec(vec.clone()).expect_vec(), Ok(vec));
    let strukt = RtStruct::named("Pair", vec![RtValue::Int(1)]).expect("struct");
    assert_eq!(RtValue::Struct(strukt.clone()).expect_struct(), Ok(strukt));
    assert_eq!(
        RtValue::Function(RtFunctionRef(3)).expect_function(),
        Ok(RtFunctionRef(3))
    );
}

#[test]
fn value_accessors_report_wrong_type() {
    assert_eq!(
        RtValue::Bool(true)
            .expect_int()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Int(1)
            .expect_string()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Unit.expect_vec().expect_err("wrong type").kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Bool(false)
            .expect_struct()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
}
