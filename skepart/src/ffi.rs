#![allow(clippy::missing_const_for_thread_local)]

use std::cell::RefCell;
use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::slice;

use crate::array::RtArray;
use crate::builtins;
use crate::host::NoopHost;
use crate::string::RtString;
use crate::value::{RtFunctionRef, RtStruct, RtStructLayout, RtValue};
use crate::vec::RtVec;

thread_local! {
    static LAST_ERROR: RefCell<Option<crate::RtError>> = RefCell::new(None);
}

fn invalid_argument(message: impl Into<String>) -> crate::RtError {
    crate::RtError::new(crate::RtErrorKind::InvalidArgument, message)
}

fn clone_value(ptr: *mut RtValue) -> Result<RtValue, crate::RtError> {
    if ptr.is_null() {
        return Err(invalid_argument("runtime value pointer must not be null"));
    }
    Ok(unsafe { (*ptr).clone() })
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

fn clear_last_error() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn set_last_error(err: crate::RtError) {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(err);
    });
}

fn take_last_error() -> Option<crate::RtError> {
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

fn ffi_try<T, F>(f: F) -> Result<T, crate::RtError>
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

#[no_mangle]
pub extern "C" fn skp_rt_abort_if_error() {
    if let Some(err) = take_last_error() {
        eprintln!("{err}");
        std::process::exit(101);
    }
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
    match ffi_try(|| {
        if value.is_null() {
            return Err(invalid_argument("string pointer must not be null"));
        }
        Ok(unsafe { (*value).len_chars() as i64 })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_string_eq(left: *mut RtString, right: *mut RtString) -> bool {
    match ffi_try(|| {
        if left.is_null() || right.is_null() {
            return Err(invalid_argument("string pointers must not be null"));
        }
        Ok(unsafe { (*left).as_str() == (*right).as_str() })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_contains(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> bool {
    match ffi_try(|| {
        if haystack.is_null() || needle.is_null() {
            return Err(invalid_argument("string pointers must not be null"));
        }
        Ok(unsafe { (*haystack).contains(&*needle) })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_index_of(
    haystack: *mut RtString,
    needle: *mut RtString,
) -> i64 {
    match ffi_try(|| {
        if haystack.is_null() || needle.is_null() {
            return Err(invalid_argument("string pointers must not be null"));
        }
        Ok(unsafe { (*haystack).index_of(&*needle) })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_builtin_str_slice(
    value: *mut RtString,
    start: i64,
    end: i64,
) -> *mut RtString {
    match ffi_try(|| {
        if value.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "string pointer must not be null",
            ));
        }
        if start < 0 || end < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "string slice bounds must be non-negative",
            ));
        }
        let sliced = unsafe { (*value).slice_chars(start as usize..end as usize) }?;
        Ok(boxed_string(sliced))
    }) {
        Ok(value) => value,
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

#[no_mangle]
pub extern "C" fn skp_rt_call_builtin(
    package: *const c_char,
    name: *const c_char,
    argc: i64,
    argv: *const *mut RtValue,
) -> *mut RtValue {
    match ffi_try(|| {
        if package.is_null() || name.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "builtin names must not be null",
            ));
        }
        if argc < 0 {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "argc must be non-negative",
            ));
        }
        let package = c_string(package)?;
        let name = c_string(name)?;
        let args = if argc == 0 {
            Vec::new()
        } else {
            if argv.is_null() {
                return Err(crate::RtError::new(
                    crate::RtErrorKind::InvalidArgument,
                    "argv must not be null when argc > 0",
                ));
            }
            unsafe { slice::from_raw_parts(argv, argc as usize) }
                .iter()
                .map(|arg| clone_value(*arg))
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut host = NoopHost::default();
        builtins::call_with_host(&mut host, &package, &name, &args).map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            boxed_value(RtValue::Unit)
        }
    }
}

#[no_mangle]
pub extern "C" fn skp_rt_call_function(
    function: i32,
    argc: i64,
    _argv: *const *mut RtValue,
) -> *mut RtValue {
    let _ = argc;
    // Native indirect calls are lowered through LLVM-emitted internal dispatch wrappers,
    // not through this exported ABI symbol. Keep the entrypoint explicit and non-panicking
    // so accidental external use fails as a regular runtime argument error.
    set_last_error(crate::RtError::new(
        crate::RtErrorKind::InvalidArgument,
        format!(
            "skp_rt_call_function is not a supported external ABI entrypoint; function id {function} must be dispatched by generated wrappers"
        ),
    ));
    boxed_value(RtValue::Unit)
}

fn c_string(ptr: *const c_char) -> Result<String, crate::RtError> {
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

unsafe fn free_boxed_struct(ptr: *mut RtStruct) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
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
pub unsafe extern "C" fn skp_rt_struct_free(ptr: *mut RtStruct) {
    unsafe { free_boxed_struct(ptr) };
}
