use crate::builtins;
use crate::ffi_support::{
    boxed_value, c_string, clone_value, ffi_try, set_last_error,
};
use crate::host::NoopHost;
use crate::value::RtValue;
use std::ffi::c_char;
use std::slice;

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
