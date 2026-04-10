use std::sync::Arc;

use crate::array::RtArray;
use crate::ffi_support::{
    boxed_array, boxed_struct, boxed_value, boxed_vec, clear_last_error, ffi_try, invalid_argument,
    set_last_error,
};
use crate::value::{RtStruct, RtStructLayout, RtValue};
use crate::vec::RtVec;

#[no_mangle]
pub extern "C" fn skp_rt_array_new(size: i64) -> *mut RtArray {
    clear_last_error();
    if size < 0 {
        set_last_error(invalid_argument("array size must be non-negative"));
        return std::ptr::null_mut();
    }
    boxed_array(RtArray::new(vec![RtValue::Unit; size as usize]))
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_array_repeat(value: *mut RtValue, size: i64) -> *mut RtArray {
    match ffi_try(|| {
        if size < 0 {
            return Err(invalid_argument("array size must be non-negative"));
        }
        Ok(boxed_array(RtArray::repeat(
            crate::ffi_support::take_value(value)?,
            size as usize,
        )))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_array_get(array: *mut RtArray, index: i64) -> *mut RtValue {
    clear_last_error();
    if array.is_null() {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "array pointer must not be null",
        ));
        return std::ptr::null_mut();
    }
    if index < 0 {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::IndexOutOfBounds,
            "array index must be non-negative",
        ));
        return std::ptr::null_mut();
    }
    match unsafe { (*array).get(index as usize) } {
        Ok(value) => boxed_value(value),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_array_set(array: *mut RtArray, index: i64, value: *mut RtValue) {
    clear_last_error();
    if array.is_null() {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "array pointer must not be null",
        ));
        return;
    }
    if index < 0 {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::IndexOutOfBounds,
            "array index must be non-negative",
        ));
        return;
    }
    let value = match crate::ffi_support::take_value(value) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            return;
        }
    };
    if let Err(err) = unsafe { (*array).set(index as usize, value) } {
        set_last_error(err);
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_new() -> *mut RtVec {
    boxed_vec(RtVec::new())
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_len(vec: *mut RtVec) -> i64 {
    match ffi_try(|| {
        if vec.is_null() {
            return Err(invalid_argument("vec pointer must not be null"));
        }
        Ok(unsafe { (*vec).len() as i64 })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_push(vec: *mut RtVec, value: *mut RtValue) {
    if let Err(err) = ffi_try(|| {
        if vec.is_null() {
            return Err(invalid_argument("vec pointer must not be null"));
        }
        unsafe { (*vec).push(crate::ffi_support::take_value(value)?) };
        Ok(())
    }) {
        set_last_error(err);
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_get(vec: *mut RtVec, index: i64) -> *mut RtValue {
    match ffi_try(|| {
        if vec.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "vec pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::IndexOutOfBounds,
                "vec index must be non-negative",
            ));
        }
        unsafe { (*vec).get(index as usize) }.map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_set(vec: *mut RtVec, index: i64, value: *mut RtValue) {
    if let Err(err) = ffi_try(|| {
        if vec.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "vec pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::IndexOutOfBounds,
                "vec index must be non-negative",
            ));
        }
        unsafe { (*vec).set(index as usize, crate::ffi_support::take_value(value)?) }
    }) {
        set_last_error(err);
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_delete(vec: *mut RtVec, index: i64) -> *mut RtValue {
    match ffi_try(|| {
        if vec.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "vec pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::IndexOutOfBounds,
                "vec index must be non-negative",
            ));
        }
        unsafe { (*vec).delete(index as usize) }.map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_new(struct_id: i64, field_count: i64) -> *mut RtStruct {
    clear_last_error();
    if field_count < 0 {
        set_last_error(invalid_argument("field count must be non-negative"));
        return std::ptr::null_mut();
    }
    match RtStruct::new(
        Arc::new(RtStructLayout {
            name: format!("Struct{struct_id}"),
            field_names: Vec::new(),
            field_types: Vec::new(),
        }),
        vec![RtValue::Unit; field_count as usize],
    ) {
        Ok(value) => boxed_struct(value),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_struct_get(value: *mut RtStruct, index: i64) -> *mut RtValue {
    clear_last_error();
    if value.is_null() {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "struct pointer must not be null",
        ));
        return std::ptr::null_mut();
    }
    if index < 0 {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::MissingField,
            "field index must be non-negative",
        ));
        return std::ptr::null_mut();
    }
    match unsafe { (*value).get_field(index as usize) } {
        Ok(field) => boxed_value(field),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_struct_set(value: *mut RtStruct, index: i64, field: *mut RtValue) {
    clear_last_error();
    if value.is_null() {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "struct pointer must not be null",
        ));
        return;
    }
    if index < 0 {
        set_last_error(crate::RtError::new(
            crate::RtErrorKind::MissingField,
            "field index must be non-negative",
        ));
        return;
    }
    let field = match crate::ffi_support::take_value(field) {
        Ok(field) => field,
        Err(err) => {
            set_last_error(err);
            return;
        }
    };
    if let Err(err) = unsafe { (*value).set_field(index as usize, field) } {
        set_last_error(err);
    }
}
