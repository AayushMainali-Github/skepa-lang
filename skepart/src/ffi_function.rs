use std::ffi::c_void;

use crate::ffi_support::{boxed_value, set_last_error};
use crate::value::RtValue;

#[no_mangle]
pub extern "C" fn skp_rt_call_function(
    function: *mut c_void,
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
            "skp_rt_call_function is not a supported external ABI entrypoint; function pointer {:?} must be dispatched by generated wrappers",
            function
        ),
    ));
    boxed_value(RtValue::Unit)
}
