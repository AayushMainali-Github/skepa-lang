use std::rc::Rc;

use crate::array::RtArray;
use crate::ffi_support::{
    boxed_array, boxed_struct, boxed_value, boxed_vec, clone_value, ffi_try, invalid_argument,
    set_last_error,
};
use crate::value::{RtStruct, RtStructLayout, RtValue};
use crate::vec::RtVec;

#[no_mangle]
pub extern "C" fn skp_rt_array_new(size: i64) -> *mut RtArray {
    match ffi_try(|| {
        if size < 0 {
            return Err(invalid_argument("array size must be non-negative"));
        }
        Ok(boxed_array(RtArray::new(vec![
            RtValue::Unit;
            size as usize
        ])))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_array_repeat(value: *mut RtValue, size: i64) -> *mut RtArray {
    match ffi_try(|| {
        if size < 0 {
            return Err(invalid_argument("array size must be non-negative"));
        }
        Ok(boxed_array(RtArray::repeat(
            clone_value(value)?,
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
pub extern "C" fn skp_rt_array_get(array: *mut RtArray, index: i64) -> *mut RtValue {
    match ffi_try(|| {
        if array.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "array pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::IndexOutOfBounds,
                "array index must be non-negative",
            ));
        }
        unsafe { (*array).get(index as usize) }.map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_array_set(array: *mut RtArray, index: i64, value: *mut RtValue) {
    if let Err(err) = ffi_try(|| {
        if array.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "array pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::IndexOutOfBounds,
                "array index must be non-negative",
            ));
        }
        unsafe { (*array).set(index as usize, clone_value(value)?) }
    }) {
        set_last_error(err);
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_new() -> *mut RtVec {
    boxed_vec(RtVec::new())
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_len(vec: *mut RtVec) -> i64 {
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
pub extern "C" fn skp_rt_vec_push(vec: *mut RtVec, value: *mut RtValue) {
    if let Err(err) = ffi_try(|| {
        if vec.is_null() {
            return Err(invalid_argument("vec pointer must not be null"));
        }
        unsafe { (*vec).push(clone_value(value)?) };
        Ok(())
    }) {
        set_last_error(err);
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_get(vec: *mut RtVec, index: i64) -> *mut RtValue {
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
pub extern "C" fn skp_rt_vec_set(vec: *mut RtVec, index: i64, value: *mut RtValue) {
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
        unsafe { (*vec).set(index as usize, clone_value(value)?) }
    }) {
        set_last_error(err);
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_vec_delete(vec: *mut RtVec, index: i64) -> *mut RtValue {
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
    match ffi_try(|| {
        if field_count < 0 {
            return Err(invalid_argument("field count must be non-negative"));
        }
        Ok(boxed_struct(RtStruct::new(
            Rc::new(RtStructLayout {
                name: format!("Struct{struct_id}"),
                field_names: Vec::new(),
                field_types: Vec::new(),
            }),
            vec![RtValue::Unit; field_count as usize],
        )?))
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_get(value: *mut RtStruct, index: i64) -> *mut RtValue {
    match ffi_try(|| {
        if value.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "struct pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::MissingField,
                "field index must be non-negative",
            ));
        }
        unsafe { (*value).get_field(index as usize) }.map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_struct_set(value: *mut RtStruct, index: i64, field: *mut RtValue) {
    if let Err(err) = ffi_try(|| {
        if value.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "struct pointer must not be null",
            ));
        }
        if index < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::MissingField,
                "field index must be non-negative",
            ));
        }
        unsafe { (*value).set_field(index as usize, clone_value(field)?) }
    }) {
        set_last_error(err);
    }
}
