use crate::builtins;
use crate::ffi_support::{boxed_value, c_string, clone_value, ffi_try, set_last_error};
use crate::host::NoopHost;
use crate::value::RtValue;
use std::ffi::c_char;
use std::ffi::c_void;
use std::slice;
use std::sync::{Mutex, OnceLock};

fn ffi_host() -> &'static Mutex<NoopHost> {
    static FFI_HOST: OnceLock<Mutex<NoopHost>> = OnceLock::new();
    FFI_HOST.get_or_init(|| Mutex::new(NoopHost::default()))
}

#[no_mangle]
pub unsafe extern "C" fn skp_rt_call_builtin(
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
        let mut runtime = FfiBuiltinRuntime;
        let mut host = ffi_host()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        builtins::call_with_host_runtime(&mut *host, &mut runtime, &package, &name, &args)
            .map(boxed_value)
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
        let result = unsafe {
            crate::ffi_function::skp_rt_call_function(
                function.0 as *mut c_void,
                boxed_args.len() as i64,
                if boxed_args.is_empty() {
                    std::ptr::null()
                } else {
                    boxed_args.as_ptr()
                },
            )
        };
        let value = clone_value(result)?;
        for arg in boxed_args {
            unsafe { drop(Box::from_raw(arg)) };
        }
        unsafe { drop(Box::from_raw(result)) };
        Ok(value)
    }

    fn spawn_function(
        &mut self,
        host: &mut dyn crate::RtHost,
        function: crate::RtFunctionRef,
        args: &[RtValue],
    ) -> crate::RtResult<crate::RtHandle> {
        let args = args.to_vec();
        let task = std::thread::spawn(move || call_wrapped_function(function, &args));
        host.task_store_running(task)
    }
}

fn call_wrapped_function(
    function: crate::RtFunctionRef,
    args: &[RtValue],
) -> crate::RtResult<RtValue> {
    let boxed_args = args
        .iter()
        .cloned()
        .map(crate::ffi_support::boxed_value)
        .collect::<Vec<_>>();
    let result = unsafe {
        crate::ffi_function::skp_rt_call_function(
            function.0 as *mut c_void,
            boxed_args.len() as i64,
            if boxed_args.is_empty() {
                std::ptr::null()
            } else {
                boxed_args.as_ptr()
            },
        )
    };
    let value = clone_value(result)?;
    for arg in boxed_args {
        unsafe { drop(Box::from_raw(arg)) };
    }
    unsafe { drop(Box::from_raw(result)) };
    Ok(value)
}
