use std::ffi::c_void;

use crate::ffi_support::{boxed_value, ffi_try, invalid_argument, set_last_error, take_value};
use crate::value::RtValue;

type RtWrappedFunction = unsafe extern "C" fn(i64, *const *mut RtValue) -> *mut RtValue;

#[no_mangle]
pub unsafe extern "C" fn skp_rt_call_function(
    function: *mut c_void,
    argc: i64,
    argv: *const *mut RtValue,
) -> *mut RtValue {
    match ffi_try(|| {
        if function.is_null() {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                format!("runtime function expected non-null pointer for argc {argc}"),
            ));
        }
        if argc < 0 {
            return Err(invalid_argument(
                "runtime function argc must be non-negative",
            ));
        }
        if argc > 0 && argv.is_null() {
            return Err(invalid_argument(
                "runtime function argv pointer must not be null when argc > 0",
            ));
        }
        let function: RtWrappedFunction = unsafe { std::mem::transmute(function) };
        let result = unsafe { function(argc, argv) };
        take_value(result).map(boxed_value)
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            boxed_value(RtValue::Unit)
        }
    }
}
