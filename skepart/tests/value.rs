use skepart::{
    RtArray, RtBytes, RtErrorKind, RtFunctionRef, RtHandle, RtHandleKind, RtMap, RtOption,
    RtString, RtStruct, RtValue, RtVec,
};

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
        RtValue::Bytes(RtBytes::from(vec![1_u8, 2, 3])).expect_bytes(),
        Ok(RtBytes::from(vec![1_u8, 2, 3]))
    );
    assert_eq!(
        RtValue::Option(RtOption::some(RtValue::Int(42))).expect_option(),
        Ok(RtOption::some(RtValue::Int(42)))
    );
    assert_eq!(
        RtValue::Option(RtOption::none()).expect_option(),
        Ok(RtOption::none())
    );
    assert_eq!(
        RtValue::Array(RtArray::new(vec![RtValue::Int(1)])).expect_array(),
        Ok(RtArray::new(vec![RtValue::Int(1)]))
    );
    let vec = RtVec::new();
    vec.push(RtValue::Int(9));
    assert_eq!(RtValue::Vec(vec.clone()).expect_vec(), Ok(vec));
    let map = RtMap::new();
    assert_eq!(RtValue::Map(map.clone()).expect_map(), Ok(map));
    let strukt = RtStruct::named("Pair", vec![RtValue::Int(1)]).expect("struct");
    assert_eq!(RtValue::Struct(strukt.clone()).expect_struct(), Ok(strukt));
    assert_eq!(
        RtValue::Function(RtFunctionRef(3)).expect_function(),
        Ok(RtFunctionRef(3))
    );
    assert_eq!(
        RtValue::Handle(RtHandle {
            id: 7,
            kind: RtHandleKind::Socket,
        })
        .expect_handle(),
        Ok(RtHandle {
            id: 7,
            kind: RtHandleKind::Socket,
        })
    );
    assert_eq!(
        RtValue::Handle(RtHandle {
            id: 8,
            kind: RtHandleKind::Listener,
        })
        .expect_handle_kind(RtHandleKind::Listener),
        Ok(RtHandle {
            id: 8,
            kind: RtHandleKind::Listener,
        })
    );
    assert_eq!(
        RtValue::Handle(RtHandle {
            id: 9,
            kind: RtHandleKind::Task,
        })
        .expect_handle_kind(RtHandleKind::Task),
        Ok(RtHandle {
            id: 9,
            kind: RtHandleKind::Task,
        })
    );
    assert_eq!(
        RtValue::Handle(RtHandle {
            id: 10,
            kind: RtHandleKind::Channel,
        })
        .expect_handle_kind(RtHandleKind::Channel),
        Ok(RtHandle {
            id: 10,
            kind: RtHandleKind::Channel,
        })
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
        RtValue::Int(1)
            .expect_option()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::String(RtString::from("x"))
            .expect_bytes()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Unit.expect_vec().expect_err("wrong type").kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Unit.expect_map().expect_err("wrong type").kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Bool(false)
            .expect_struct()
            .expect_err("wrong type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Unit.expect_handle().expect_err("wrong type").kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        RtValue::Handle(RtHandle {
            id: 9,
            kind: RtHandleKind::Socket,
        })
        .expect_handle_kind(RtHandleKind::Listener)
        .expect_err("wrong handle kind")
        .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn runtime_values_are_send_ready_for_future_task_runtime() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<RtValue>();
    assert_sync::<RtValue>();
}
