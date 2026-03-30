use crate::builtins;
use crate::ffi_support::{boxed_value, c_string, clone_value, ffi_try, set_last_error};
use crate::host::NoopHost;
use crate::value::RtValue;
use std::cell::RefCell;
use std::ffi::c_char;
use std::ffi::c_void;
use std::slice;

thread_local! {
    static FFI_HOST: RefCell<NoopHost> = RefCell::new(NoopHost::default());
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
        FFI_HOST.with(|host| {
            let mut runtime = FfiBuiltinRuntime;
            builtins::call_with_host_runtime(
                &mut *host.borrow_mut(),
                &mut runtime,
                &package,
                &name,
                &args,
            )
            .map(boxed_value)
        })
    }) {
        Ok(value) => value,
        Err(err) => {
            set_last_error(err);
            boxed_value(RtValue::Unit)
        }
    }
}

struct FfiBuiltinRuntime;

impl builtins::BuiltinRuntime for FfiBuiltinRuntime {
    fn call_function(
        &mut self,
        function: crate::RtFunctionRef,
        args: &[RtValue],
    ) -> crate::RtResult<RtValue> {
        let boxed_args = args
            .iter()
            .cloned()
            .map(crate::ffi_support::boxed_value)
            .collect::<Vec<_>>();
        let result = crate::ffi_function::skp_rt_call_function(
            function.0 as *mut c_void,
            boxed_args.len() as i64,
            if boxed_args.is_empty() {
                std::ptr::null()
            } else {
                boxed_args.as_ptr()
            },
        );
        let value = clone_value(result)?;
        for arg in boxed_args {
            unsafe { drop(Box::from_raw(arg)) };
        }
        unsafe { drop(Box::from_raw(result)) };
        Ok(value)
    }
}
