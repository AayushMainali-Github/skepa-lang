use std::ffi::c_void;

use skepart::{RtFunctionRef, RtHandle, RtHandleKind, RtValue};

unsafe extern "C" {
    fn skp_rt_string_from_utf8(data: *const u8, len: i64) -> *mut c_void;
    fn skp_rt_string_eq(left: *mut c_void, right: *mut c_void) -> bool;
    fn skp_rt_builtin_str_len(value: *mut c_void) -> i64;
    fn skp_rt_value_from_int(value: i64) -> *mut c_void;
    fn skp_rt_value_from_unit() -> *mut c_void;
    fn skp_rt_value_from_string(value: *mut c_void) -> *mut c_void;
    fn skp_rt_value_to_int(value: *mut c_void) -> i64;
    fn skp_rt_value_from_function(value: *mut c_void) -> *mut c_void;
    fn skp_rt_value_to_function(value: *mut c_void) -> *mut c_void;
    fn skp_rt_value_from_handle(value: *mut c_void) -> *mut c_void;
    fn skp_rt_value_to_handle(value: *mut c_void) -> *mut c_void;
    fn skp_rt_array_repeat(value: *mut c_void, size: i64) -> *mut c_void;
    fn skp_rt_array_get(array: *mut c_void, index: i64) -> *mut c_void;
    fn skp_rt_vec_new() -> *mut c_void;
    fn skp_rt_vec_push(vec: *mut c_void, value: *mut c_void);
    fn skp_rt_vec_get(vec: *mut c_void, index: i64) -> *mut c_void;
    fn skp_rt_struct_new(struct_id: i64, field_count: i64) -> *mut c_void;
    fn skp_rt_struct_set(value: *mut c_void, index: i64, field: *mut c_void);
    fn skp_rt_struct_get(value: *mut c_void, index: i64) -> *mut c_void;
    fn skp_rt_call_builtin(
        package: *const i8,
        name: *const i8,
        argc: i64,
        argv: *const *mut c_void,
    ) -> *mut c_void;
    fn skp_rt_call_function(
        function: *mut c_void,
        argc: i64,
        argv: *const *mut c_void,
    ) -> *mut c_void;
    fn skp_rt_last_error_kind() -> i32;
    fn skp_rt_value_free(ptr: *mut c_void);
    fn skp_rt_string_free(ptr: *mut c_void);
    fn skp_rt_array_free(ptr: *mut c_void);
    fn skp_rt_vec_free(ptr: *mut c_void);
    fn skp_rt_struct_free(ptr: *mut c_void);
}

unsafe extern "C" fn ffi_add_one(argc: i64, argv: *const *mut c_void) -> *mut c_void {
    assert_eq!(argc, 1);
    let arg = unsafe { *argv };
    let value = unsafe { skp_rt_value_to_int(arg) };
    unsafe { skp_rt_value_from_int(value + 1) }
}

#[test]
fn ffi_string_and_value_roundtrip_surfaces_work() {
    let bytes = "🙂ok".as_bytes();
    let string_ptr = unsafe { skp_rt_string_from_utf8(bytes.as_ptr(), bytes.len() as i64) };
    assert_eq!(unsafe { skp_rt_builtin_str_len(string_ptr) }, 3);
    let equal_ptr = unsafe { skp_rt_string_from_utf8(bytes.as_ptr(), bytes.len() as i64) };
    let other_ptr = unsafe { skp_rt_string_from_utf8("nope".as_ptr(), 4) };
    assert!(unsafe { skp_rt_string_eq(string_ptr, equal_ptr) });
    assert!(!unsafe { skp_rt_string_eq(string_ptr, other_ptr) });

    let int_ptr = unsafe { skp_rt_value_from_int(42) };
    assert_eq!(unsafe { skp_rt_value_to_int(int_ptr) }, 42);

    let unit_ptr = unsafe { skp_rt_value_from_unit() };
    let unit = unsafe { (*(unit_ptr as *mut RtValue)).clone() };
    assert!(matches!(unit, RtValue::Unit));
}

#[test]
fn ffi_function_and_container_surfaces_work() {
    let raw_fn = 7usize as *mut c_void;
    let fn_ptr = unsafe { skp_rt_value_from_function(raw_fn) };
    assert_eq!(unsafe { skp_rt_value_to_function(fn_ptr) }, raw_fn);
    assert_eq!(
        unsafe { (*(fn_ptr as *mut RtValue)).expect_function().expect("fn") },
        RtFunctionRef(7)
    );

    let repeated = unsafe { skp_rt_array_repeat(skp_rt_value_from_int(9), 2) };
    let second = unsafe { skp_rt_array_get(repeated, 1) };
    assert_eq!(
        unsafe { (*(second as *mut RtValue)).expect_int().expect("int") },
        9
    );

    let vec_ptr = unsafe { skp_rt_vec_new() };
    unsafe { skp_rt_vec_push(vec_ptr, skp_rt_value_from_int(5)) };
    let got = unsafe { skp_rt_vec_get(vec_ptr, 0) };
    assert_eq!(
        unsafe { (*(got as *mut RtValue)).expect_int().expect("int") },
        5
    );

    let raw_handle = Box::into_raw(Box::new(RtHandle {
        id: 7,
        kind: RtHandleKind::Socket,
    })) as *mut c_void;
    let handle_value = unsafe { skp_rt_value_from_handle(raw_handle) };
    let roundtrip = unsafe { skp_rt_value_to_handle(handle_value) } as *mut RtHandle;
    assert_eq!(
        unsafe {
            (*(handle_value as *mut RtValue))
                .expect_handle()
                .expect("handle")
        },
        RtHandle {
            id: 7,
            kind: RtHandleKind::Socket
        }
    );
    assert_eq!(
        unsafe { *roundtrip },
        RtHandle {
            id: 7,
            kind: RtHandleKind::Socket
        }
    );
    unsafe {
        drop(Box::from_raw(raw_handle as *mut RtHandle));
        drop(Box::from_raw(roundtrip));
    }
}

#[test]
fn ffi_struct_helpers_and_builtin_dispatch_surface_work() {
    let strukt = unsafe { skp_rt_struct_new(1, 2) };
    unsafe {
        skp_rt_struct_set(strukt, 0, skp_rt_value_from_int(11));
        skp_rt_struct_set(strukt, 1, skp_rt_value_from_int(22));
    }
    let field = unsafe { skp_rt_struct_get(strukt, 1) };
    assert_eq!(
        unsafe { (*(field as *mut RtValue)).expect_int().expect("int") },
        22
    );
    let pkg = c"str";
    let name = c"len";
    let arg = unsafe { skp_rt_string_from_utf8("hello".as_ptr(), 5) };
    let boxed_arg = unsafe { skp_rt_value_from_string(arg) };
    let argv = [boxed_arg];
    let boxed = unsafe { skp_rt_call_builtin(pkg.as_ptr(), name.as_ptr(), 1, argv.as_ptr()) };
    assert_eq!(
        unsafe { (*(boxed as *mut RtValue)).expect_int().expect("int") },
        5
    );
}

#[test]
fn ffi_records_runtime_error_after_failed_builtin() {
    let pkg = c"str";
    let name = c"len";
    let bad_arg = unsafe { skp_rt_value_from_int(1) };
    let argv = [bad_arg];
    let _ = unsafe { skp_rt_call_builtin(pkg.as_ptr(), name.as_ptr(), 1, argv.as_ptr()) };
    assert_eq!(unsafe { skp_rt_last_error_kind() }, 3);
    unsafe { skp_rt_value_free(bad_arg) };
}

#[test]
fn ffi_builtin_host_state_persists_for_net_handles() {
    let pkg = c"net";
    let make = c"__testSocket";
    let close = c"close";
    let socket = unsafe { skp_rt_call_builtin(pkg.as_ptr(), make.as_ptr(), 0, std::ptr::null()) };
    assert!(matches!(
        unsafe { (*(socket as *mut RtValue)).clone() },
        RtValue::Handle(_)
    ));
    let argv = [socket];
    let result = unsafe { skp_rt_call_builtin(pkg.as_ptr(), close.as_ptr(), 1, argv.as_ptr()) };
    assert!(matches!(
        unsafe { (*(result as *mut RtValue)).clone() },
        RtValue::Unit
    ));
    assert_eq!(unsafe { skp_rt_last_error_kind() }, 0);
    unsafe {
        skp_rt_value_free(socket);
        skp_rt_value_free(result);
    }
}

#[test]
fn ffi_records_invalid_argument_for_null_and_negative_inputs() {
    let _ = unsafe { skp_rt_builtin_str_len(std::ptr::null_mut()) };
    assert_eq!(unsafe { skp_rt_last_error_kind() }, 5);

    let fn_ptr = unsafe { skp_rt_value_from_function(std::ptr::null_mut()) };
    assert!(!fn_ptr.is_null());
    assert!(matches!(
        unsafe { (*(fn_ptr as *mut RtValue)).clone() },
        RtValue::Function(RtFunctionRef(0))
    ));
}

#[test]
fn ffi_exports_free_helpers_for_boxed_runtime_values() {
    let string = unsafe { skp_rt_string_from_utf8("hello".as_ptr(), 5) };
    let array = unsafe { skp_rt_array_repeat(skp_rt_value_from_int(3), 2) };
    let vec = unsafe { skp_rt_vec_new() };
    let strukt = unsafe { skp_rt_struct_new(1, 1) };
    let value = unsafe { skp_rt_value_from_int(9) };

    unsafe {
        skp_rt_string_free(string);
        skp_rt_array_free(array);
        skp_rt_vec_free(vec);
        skp_rt_struct_free(strukt);
        skp_rt_value_free(value);
    }
}

#[test]
fn ffi_call_function_dispatches_wrapped_runtime_functions() {
    let arg = unsafe { skp_rt_value_from_int(41) };
    let argv = [arg];
    let result = unsafe { skp_rt_call_function(ffi_add_one as *mut c_void, 1, argv.as_ptr()) };
    let value = unsafe { (*(result as *mut RtValue)).clone() };
    assert_eq!(value.expect_int().expect("int"), 42);
    assert_eq!(unsafe { skp_rt_last_error_kind() }, 0);
    unsafe {
        skp_rt_value_free(arg);
        skp_rt_value_free(result);
    }
}

#[test]
fn ffi_call_function_rejects_invalid_external_abi_use() {
    let result = unsafe { skp_rt_call_function(std::ptr::null_mut(), 0, std::ptr::null()) };
    let value = unsafe { (*(result as *mut RtValue)).clone() };
    assert!(matches!(value, RtValue::Unit));
    assert_eq!(unsafe { skp_rt_last_error_kind() }, 5);
    unsafe { skp_rt_value_free(result) };
}
