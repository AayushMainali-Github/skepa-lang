#![allow(clippy::missing_const_for_thread_local)]

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::array::RtArray;
use crate::map::RtMap;
use crate::string::RtString;
use crate::value::{RtStruct, RtValue};
use crate::vec::RtVec;

thread_local! {
    static LAST_ERROR: RefCell<Option<crate::RtError>> = RefCell::new(None);
}

pub fn invalid_argument(message: impl Into<String>) -> crate::RtError {
    crate::RtError::new(crate::RtErrorKind::InvalidArgument, message)
}

pub fn clear_last_error() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

pub fn set_last_error(err: crate::RtError) {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(err);
    });
}

pub fn take_last_error() -> Option<crate::RtError> {
    LAST_ERROR.with(|slot| slot.borrow_mut().take())
}

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(msg) => *msg,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(msg) => (*msg).to_string(),
            Err(_) => "runtime ffi panic".to_string(),
        },
    }
}

pub fn ffi_try<T, F>(f: F) -> Result<T, crate::RtError>
where
    F: FnOnce() -> Result<T, crate::RtError>,
{
    clear_last_error();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => Err(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            panic_payload_message(payload),
        )),
    }
}

pub fn clone_value(ptr: *mut RtValue) -> Result<RtValue, crate::RtError> {
    if ptr.is_null() {
        return Err(invalid_argument("runtime value pointer must not be null"));
    }
    Ok(unsafe { (*ptr).clone() })
}

pub fn take_value(ptr: *mut RtValue) -> Result<RtValue, crate::RtError> {
    if ptr.is_null() {
        return Err(invalid_argument("runtime value pointer must not be null"));
    }
    Ok(unsafe { *Box::from_raw(ptr) })
}

pub fn boxed_value(value: RtValue) -> *mut RtValue {
    Box::into_raw(Box::new(value))
}

pub fn boxed_string(value: RtString) -> *mut RtString {
    Box::into_raw(Box::new(value))
}

pub fn boxed_array(value: RtArray) -> *mut RtArray {
    Box::into_raw(Box::new(value))
}

pub fn boxed_vec(value: RtVec) -> *mut RtVec {
    Box::into_raw(Box::new(value))
}

pub fn boxed_map(value: RtMap) -> *mut RtMap {
    Box::into_raw(Box::new(value))
}

pub fn boxed_struct(value: RtStruct) -> *mut RtStruct {
    Box::into_raw(Box::new(value))
}

pub fn c_string(ptr: *const c_char) -> Result<String, crate::RtError> {
    if ptr.is_null() {
        return Err(invalid_argument("c string pointer must not be null"));
    }
    let mut bytes = Vec::new();
    let mut offset = 0usize;
    loop {
        let byte = unsafe { ptr.add(offset).read() };
        if byte == 0 {
            break;
        }
        bytes.push(byte as u8);
        offset += 1;
    }
    String::from_utf8(bytes).map_err(|_| invalid_argument("runtime strings must be valid UTF-8"))
}

unsafe fn free_boxed_value(ptr: *mut RtValue) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

unsafe fn free_boxed_string(ptr: *mut RtString) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

unsafe fn free_boxed_array(ptr: *mut RtArray) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

unsafe fn free_boxed_vec(ptr: *mut RtVec) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

unsafe fn free_boxed_map(ptr: *mut RtMap) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

unsafe fn free_boxed_struct(ptr: *mut RtStruct) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_abort_if_error() {
    if let Some(err) = take_last_error() {
        eprintln!("{err}");
        std::process::exit(101);
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_raise_division_by_zero() {
    set_last_error(crate::RtError::new(
        crate::RtErrorKind::DivisionByZero,
        "division by zero",
    ));
    skp_rt_abort_if_error();
}

#[no_mangle]
pub extern "C" fn skp_rt_raise_negative_shift_count() {
    set_last_error(crate::RtError::new(
        crate::RtErrorKind::InvalidArgument,
        "negative shift count",
    ));
    skp_rt_abort_if_error();
}

#[no_mangle]
pub extern "C" fn skp_rt_last_error_kind() -> i32 {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref().map(|err| &err.kind) {
        Some(crate::RtErrorKind::DivisionByZero) => 1,
        Some(crate::RtErrorKind::IndexOutOfBounds) => 2,
        Some(crate::RtErrorKind::TypeMismatch) => 3,
        Some(crate::RtErrorKind::MissingField) => 4,
        Some(crate::RtErrorKind::InvalidArgument) => 5,
        Some(crate::RtErrorKind::UnsupportedBuiltin) => 6,
        Some(crate::RtErrorKind::Io) => 7,
        Some(crate::RtErrorKind::Process) => 8,
        None => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_value_free(ptr: *mut RtValue) {
    unsafe { free_boxed_value(ptr) };
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_string_free(ptr: *mut RtString) {
    unsafe { free_boxed_string(ptr) };
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_array_free(ptr: *mut RtArray) {
    unsafe { free_boxed_array(ptr) };
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_vec_free(ptr: *mut RtVec) {
    unsafe { free_boxed_vec(ptr) };
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_map_free(ptr: *mut RtMap) {
    unsafe { free_boxed_map(ptr) };
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_struct_free(ptr: *mut RtStruct) {
    unsafe { free_boxed_struct(ptr) };
}
