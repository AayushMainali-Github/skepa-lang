use std::ffi::c_char;
use std::rc::Rc;
use std::slice;

use crate::array::RtArray;
use crate::builtins;
use crate::host::NoopHost;
use crate::string::RtString;
use crate::value::{RtFunctionRef, RtStruct, RtStructLayout, RtValue};
use crate::vec::RtVec;

fn clone_value(ptr: *mut RtValue) -> RtValue {
    assert!(!ptr.is_null(), "runtime value pointer must not be null");
    // SAFETY: caller passes pointers previously allocated by this runtime.
    unsafe { (*ptr).clone() }
}

fn boxed_value(value: RtValue) -> *mut RtValue {
    Box::into_raw(Box::new(value))
}

fn boxed_string(value: RtString) -> *mut RtString {
    Box::into_raw(Box::new(value))
}

fn boxed_array(value: RtArray) -> *mut RtArray {
    Box::into_raw(Box::new(value))
}

fn boxed_vec(value: RtVec) -> *mut RtVec {
    Box::into_raw(Box::new(value))
}

fn boxed_struct(value: RtStruct) -> *mut RtStruct {
    Box::into_raw(Box::new(value))
}

#[no_mangle]
pub extern "C" fn skp_rt_string_from_utf8(data: *const u8, len: i64) -> *mut RtString {
    assert!(len >= 0, "string length must be non-negative");
    assert!(!data.is_null(), "string pointer must not be null");
    // SAFETY: caller provides a valid UTF-8 byte slice with the given length.
    let bytes = unsafe { slice::from_raw_parts(data, len as usize) };
    let value = std::str::from_utf8(bytes).expect("runtime string literal must be valid UTF-8");
    boxed_string(RtString::from(value))
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_len(value: *mut RtString) -> i64 {
    assert!(!value.is_null(), "string pointer must not be null");
    // SAFETY: caller passes a valid runtime string pointer.
    unsafe { (*value).len_chars() as i64 }
}

#[no_mangle]
pub extern "C" fn skp_rt_string_eq(left: *mut RtString, right: *mut RtString) -> bool {
    assert!(
        !left.is_null() && !right.is_null(),
        "string pointers must not be null"
    );
    // SAFETY: caller passes valid runtime string pointers.
    unsafe { (*left).as_str() == (*right).as_str() }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_contains(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> bool {
    assert!(
        !haystack.is_null() && !needle.is_null(),
        "string pointers must not be null"
    );
    // SAFETY: caller passes valid runtime string pointers.
    unsafe { (*haystack).contains(&*needle) }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_index_of(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> i64 {
    assert!(
        !haystack.is_null() && !needle.is_null(),
        "string pointers must not be null"
    );
    // SAFETY: caller passes valid runtime string pointers.
    unsafe { (*haystack).index_of(&*needle) }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_slice(
    value: *mut RtString,
    start: i64,
    end: i64,
) -> *mut RtString {
    assert!(!value.is_null(), "string pointer must not be null");
    assert!(
        start >= 0 && end >= 0,
        "string slice bounds must be non-negative"
    );
    // SAFETY: caller passes a valid runtime string pointer.
    let sliced = unsafe { (*value).slice_chars(start as usize..end as usize) }
        .expect("runtime string slice should be valid");
    boxed_string(sliced)
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_int(value: i64) -> *mut RtValue {
    boxed_value(RtValue::Int(value))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_bool(value: bool) -> *mut RtValue {
    boxed_value(RtValue::Bool(value))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_float(value: f64) -> *mut RtValue {
    boxed_value(RtValue::Float(value))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_unit() -> *mut RtValue {
    boxed_value(RtValue::Unit)
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_string(value: *mut RtString) -> *mut RtValue {
    assert!(!value.is_null(), "string pointer must not be null");
    // SAFETY: caller passes a valid runtime string pointer.
    boxed_value(RtValue::String(unsafe { (*value).clone() }))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_array(value: *mut RtArray) -> *mut RtValue {
    assert!(!value.is_null(), "array pointer must not be null");
    // SAFETY: caller passes a valid runtime array pointer.
    boxed_value(RtValue::Array(unsafe { (*value).clone() }))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_vec(value: *mut RtVec) -> *mut RtValue {
    assert!(!value.is_null(), "vec pointer must not be null");
    // SAFETY: caller passes a valid runtime vec pointer.
    boxed_value(RtValue::Vec(unsafe { (*value).clone() }))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_struct(value: *mut RtStruct) -> *mut RtValue {
    assert!(!value.is_null(), "struct pointer must not be null");
    // SAFETY: caller passes a valid runtime struct pointer.
    boxed_value(RtValue::Struct(unsafe { (*value).clone() }))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_function(value: i32) -> *mut RtValue {
    assert!(value >= 0, "function id must be non-negative");
    boxed_value(RtValue::Function(RtFunctionRef(value as u32)))
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_int(value: *mut RtValue) -> i64 {
    clone_value(value)
        .expect_int()
        .expect("expected Int runtime value")
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_bool(value: *mut RtValue) -> bool {
    clone_value(value)
        .expect_bool()
        .expect("expected Bool runtime value")
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_float(value: *mut RtValue) -> f64 {
    clone_value(value)
        .expect_float()
        .expect("expected Float runtime value")
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_string(value: *mut RtValue) -> *mut RtString {
    boxed_string(
        clone_value(value)
            .expect_string()
            .expect("expected String runtime value"),
    )
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_array(value: *mut RtValue) -> *mut RtArray {
    boxed_array(
        clone_value(value)
            .expect_array()
            .expect("expected Array runtime value"),
    )
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_vec(value: *mut RtValue) -> *mut RtVec {
    boxed_vec(
        clone_value(value)
            .expect_vec()
            .expect("expected Vec runtime value"),
    )
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_struct(value: *mut RtValue) -> *mut RtStruct {
    boxed_struct(
        clone_value(value)
            .expect_struct()
            .expect("expected Struct runtime value"),
    )
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_function(value: *mut RtValue) -> i32 {
    clone_value(value)
        .expect_function()
        .expect("expected Function runtime value")
        .0 as i32
}

#[no_mangle]
pub extern "C" fn skp_rt_array_new(size: i64) -> *mut RtArray {
    assert!(size >= 0, "array size must be non-negative");
    boxed_array(RtArray::new(vec![RtValue::Unit; size as usize]))
}

#[no_mangle]
pub extern "C" fn skp_rt_array_repeat(value: *mut RtValue, size: i64) -> *mut RtArray {
    assert!(size >= 0, "array size must be non-negative");
    boxed_array(RtArray::repeat(clone_value(value), size as usize))
}

#[no_mangle]
pub extern "C" fn skp_rt_array_get(array: *mut RtArray, index: i64) -> *mut RtValue {
    assert!(!array.is_null(), "array pointer must not be null");
    assert!(index >= 0, "array index must be non-negative");
    // SAFETY: caller passes a valid runtime array pointer.
    let value = unsafe { (*array).get(index as usize) }.expect("array index should be valid");
    boxed_value(value)
}

#[no_mangle]
pub extern "C" fn skp_rt_array_set(array: *mut RtArray, index: i64, value: *mut RtValue) {
    assert!(!array.is_null(), "array pointer must not be null");
    assert!(index >= 0, "array index must be non-negative");
    // SAFETY: caller passes a valid runtime array pointer.
    unsafe { (*array).set(index as usize, clone_value(value)) }
        .expect("array index should be valid");
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_new() -> *mut RtVec {
    boxed_vec(RtVec::new())
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_len(vec: *mut RtVec) -> i64 {
    assert!(!vec.is_null(), "vec pointer must not be null");
    // SAFETY: caller passes a valid runtime vec pointer.
    unsafe { (*vec).len() as i64 }
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_push(vec: *mut RtVec, value: *mut RtValue) {
    assert!(!vec.is_null(), "vec pointer must not be null");
    // SAFETY: caller passes a valid runtime vec pointer.
    unsafe { (*vec).push(clone_value(value)) };
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_get(vec: *mut RtVec, index: i64) -> *mut RtValue {
    assert!(!vec.is_null(), "vec pointer must not be null");
    assert!(index >= 0, "vec index must be non-negative");
    // SAFETY: caller passes a valid runtime vec pointer.
    let value = unsafe { (*vec).get(index as usize) }.expect("vec index should be valid");
    boxed_value(value)
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_set(vec: *mut RtVec, index: i64, value: *mut RtValue) {
    assert!(!vec.is_null(), "vec pointer must not be null");
    assert!(index >= 0, "vec index must be non-negative");
    // SAFETY: caller passes a valid runtime vec pointer.
    unsafe { (*vec).set(index as usize, clone_value(value)) }.expect("vec index should be valid");
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_delete(vec: *mut RtVec, index: i64) -> *mut RtValue {
    assert!(!vec.is_null(), "vec pointer must not be null");
    assert!(index >= 0, "vec index must be non-negative");
    // SAFETY: caller passes a valid runtime vec pointer.
    let value = unsafe { (*vec).delete(index as usize) }.expect("vec index should be valid");
    boxed_value(value)
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_new(struct_id: i64, field_count: i64) -> *mut RtStruct {
    assert!(field_count >= 0, "field count must be non-negative");
    boxed_struct(
        RtStruct::new(
            Rc::new(RtStructLayout {
                name: format!("Struct{struct_id}"),
                field_names: Vec::new(),
            }),
            vec![RtValue::Unit; field_count as usize],
        )
        .expect("ffi-generated struct layout should be valid"),
    )
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_get(value: *mut RtStruct, index: i64) -> *mut RtValue {
    assert!(!value.is_null(), "struct pointer must not be null");
    assert!(index >= 0, "field index must be non-negative");
    // SAFETY: caller passes a valid runtime struct pointer.
    let field = unsafe { (*value).get_field(index as usize) }.expect("field index should be valid");
    boxed_value(field)
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_set(value: *mut RtStruct, index: i64, field: *mut RtValue) {
    assert!(!value.is_null(), "struct pointer must not be null");
    assert!(index >= 0, "field index must be non-negative");
    // SAFETY: caller passes a valid runtime struct pointer.
    unsafe { (*value).set_field(index as usize, clone_value(field)) }
        .expect("field index should be valid");
}

#[no_mangle]
pub extern "C" fn skp_rt_call_builtin(
    package: *const c_char,
    name: *const c_char,
    argc: i64,
    argv: *const *mut RtValue,
) -> *mut RtValue {
    assert!(
        !package.is_null() && !name.is_null(),
        "builtin names must not be null"
    );
    assert!(argc >= 0, "argc must be non-negative");
    let package = c_string(package);
    let name = c_string(name);
    let args = if argc == 0 {
        Vec::new()
    } else {
        assert!(!argv.is_null(), "argv must not be null when argc > 0");
        // SAFETY: caller passes argc entries.
        unsafe { slice::from_raw_parts(argv, argc as usize) }
            .iter()
            .map(|arg| clone_value(*arg))
            .collect()
    };
    let mut host = NoopHost::default();
    let value = builtins::call_with_host(&mut host, &package, &name, &args)
        .expect("runtime builtin call should succeed");
    boxed_value(value)
}

#[no_mangle]
pub extern "C" fn skp_rt_call_function(
    function: i32,
    argc: i64,
    _argv: *const *mut RtValue,
) -> *mut RtValue {
    let _ = argc;
    panic!("native indirect-call trampoline not implemented for function id {function}");
}

fn c_string(ptr: *const c_char) -> String {
    assert!(!ptr.is_null(), "c string pointer must not be null");
    let mut bytes = Vec::new();
    let mut offset = 0usize;
    loop {
        // SAFETY: caller passes a valid NUL-terminated string.
        let byte = unsafe { ptr.add(offset).read() };
        if byte == 0 {
            break;
        }
        bytes.push(byte as u8);
        offset += 1;
    }
    String::from_utf8(bytes).expect("runtime strings must be valid UTF-8")
}

#[allow(dead_code)]
unsafe fn free_boxed_value(ptr: *mut RtValue) {
    if !ptr.is_null() {
        // SAFETY: ptr came from Box::into_raw in this module.
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[allow(dead_code)]
unsafe fn free_boxed_string(ptr: *mut RtString) {
    if !ptr.is_null() {
        // SAFETY: ptr came from Box::into_raw in this module.
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[allow(dead_code)]
unsafe fn free_boxed_array(ptr: *mut RtArray) {
    if !ptr.is_null() {
        // SAFETY: ptr came from Box::into_raw in this module.
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[allow(dead_code)]
unsafe fn free_boxed_vec(ptr: *mut RtVec) {
    if !ptr.is_null() {
        // SAFETY: ptr came from Box::into_raw in this module.
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[allow(dead_code)]
unsafe fn free_boxed_struct(ptr: *mut RtStruct) {
    if !ptr.is_null() {
        // SAFETY: ptr came from Box::into_raw in this module.
        unsafe { drop(Box::from_raw(ptr)) };
    }
}
