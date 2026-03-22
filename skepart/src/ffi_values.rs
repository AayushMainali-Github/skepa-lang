use std::slice;

use crate::array::RtArray;
use crate::ffi_support::{
    boxed_array, boxed_string, boxed_struct, boxed_value, boxed_vec, clear_last_error, clone_value,
    ffi_try, invalid_argument, set_last_error,
};
use crate::string::RtString;
use crate::value::{RtFunctionRef, RtStruct, RtValue};
use crate::vec::RtVec;

#[no_mangle]
pub extern "C" fn skp_rt_string_from_utf8(data: *const u8, len: i64) -> *mut RtString {
    match ffi_try(|| {
        if len < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "string length must be non-negative",
            ));
        }
        if data.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "string pointer must not be null",
            ));
        }
        let bytes = unsafe { slice::from_raw_parts(data, len as usize) };
        let value = std::str::from_utf8(bytes).map_err(|_| {
            crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "runtime string literal must be valid UTF-8",
            )
        })?;
        Ok(boxed_string(RtString::from(value)))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_len(value: *mut RtString) -> i64 {
    clear_last_error();
    if value.is_null() {
        set_last_error(invalid_argument("string pointer must not be null"));
        return 0;
    }
    unsafe { (*value).len_chars() as i64 }
}

#[no_mangle]
pub extern "C" fn skp_rt_string_eq(left: *mut RtString, right: *mut RtString) -> bool {
    clear_last_error();
    if left.is_null() || right.is_null() {
        set_last_error(invalid_argument("string pointers must not be null"));
        return false;
    }
    unsafe { (*left).as_str() == (*right).as_str() }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_contains(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> bool {
    clear_last_error();
    if haystack.is_null() || needle.is_null() {
        set_last_error(invalid_argument("string pointers must not be null"));
        return false;
    }
    unsafe { (*haystack).contains(&*needle) }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_index_of(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> i64 {
    clear_last_error();
    if haystack.is_null() || needle.is_null() {
        set_last_error(invalid_argument("string pointers must not be null"));
        return 0;
    }
    unsafe { (*haystack).index_of(&*needle) }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_slice(
    value: *mut RtString,
    start: i64,
    end: i64,
) -> *mut RtString {
    clear_last_error();
    if value.is_null() {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "string pointer must not be null",
        ));
        return std::ptr::null_mut();
    }
    if start < 0 || end < 0 {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "string slice bounds must be non-negative",
        ));
        return std::ptr::null_mut();
    }
    match unsafe { (*value).slice_chars(start as usize..end as usize) } {
        Ok(sliced) => boxed_string(sliced),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
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
    match ffi_try(|| {
        if value.is_null() {
            return Err(invalid_argument("string pointer must not be null"));
        }
        Ok(boxed_value(RtValue::String(unsafe { (*value).clone() })))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_array(value: *mut RtArray) -> *mut RtValue {
    match ffi_try(|| {
        if value.is_null() {
            return Err(invalid_argument("array pointer must not be null"));
        }
        Ok(boxed_value(RtValue::Array(unsafe { (*value).clone() })))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_vec(value: *mut RtVec) -> *mut RtValue {
    match ffi_try(|| {
        if value.is_null() {
            return Err(invalid_argument("vec pointer must not be null"));
        }
        Ok(boxed_value(RtValue::Vec(unsafe { (*value).clone() })))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_struct(value: *mut RtStruct) -> *mut RtValue {
    match ffi_try(|| {
        if value.is_null() {
            return Err(invalid_argument("struct pointer must not be null"));
        }
        Ok(boxed_value(RtValue::Struct(unsafe { (*value).clone() })))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_from_function(value: i32) -> *mut RtValue {
    match ffi_try(|| {
        if value < 0 {
            return Err(invalid_argument("function id must be non-negative"));
        }
        Ok(boxed_value(RtValue::Function(RtFunctionRef(value as u32))))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_int(value: *mut RtValue) -> i64 {
    match ffi_try(|| clone_value(value)?.expect_int()) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_bool(value: *mut RtValue) -> bool {
    match ffi_try(|| clone_value(value)?.expect_bool()) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_float(value: *mut RtValue) -> f64 {
    match ffi_try(|| clone_value(value)?.expect_float()) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0.0
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_string(value: *mut RtValue) -> *mut RtString {
    match ffi_try(|| clone_value(value)?.expect_string().map(boxed_string)) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_array(value: *mut RtValue) -> *mut RtArray {
    match ffi_try(|| clone_value(value)?.expect_array().map(boxed_array)) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_vec(value: *mut RtValue) -> *mut RtVec {
    match ffi_try(|| clone_value(value)?.expect_vec().map(boxed_vec)) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_struct(value: *mut RtValue) -> *mut RtStruct {
    match ffi_try(|| clone_value(value)?.expect_struct().map(boxed_struct)) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_value_to_function(value: *mut RtValue) -> i32 {
    match ffi_try(|| {
        clone_value(value)?
            .expect_function()
            .map(|value| value.0 as i32)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}
