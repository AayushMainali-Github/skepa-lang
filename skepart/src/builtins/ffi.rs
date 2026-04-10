use crate::{RtHost, RtResult, RtResultValue, RtString, RtValue};

pub fn open(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    Ok(match host.ffi_open_library(path) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.message.as_str(),
        )))),
    })
}

pub fn bind(host: &mut dyn RtHost, library: crate::RtHandle, symbol: &str) -> RtResult<RtValue> {
    Ok(match host.ffi_bind_symbol(library, symbol) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.message.as_str(),
        )))),
    })
}

pub fn close_library(host: &mut dyn RtHost, library: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(library)?;
    Ok(RtValue::Unit)
}

pub fn close_symbol(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(symbol)?;
    Ok(RtValue::Unit)
}

pub fn call_0_int(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_0_int(symbol)?))
}

pub fn call(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    signature: &str,
    args: &[RtValue],
) -> RtResult<RtValue> {
    match (signature, args) {
        ("->i64", []) => call_0_int(host, symbol),
        ("->void", []) => call_0_void(host, symbol),
        ("->_Bool", []) => call_0_c_bool(host, symbol),
        ("->i32bool", []) => call_0_i32_bool(host, symbol),
        ("system:->BOOL", []) => call_0_system_i32_bool(host, symbol),
        ("i64->i64", [value]) => call_1_int(host, symbol, value.expect_int()?),
        ("i64->_Bool", [value]) => call_1_i64_c_bool(host, symbol, value.expect_int()?),
        ("i64->i32bool", [value]) => call_1_int_i32_bool(host, symbol, value.expect_int()?),
        ("system:i64->BOOL", [value]) => {
            call_1_int_system_i32_bool(host, symbol, value.expect_int()?)
        }
        ("i64->void", [value]) => call_1_int_void(host, symbol, value.expect_int()?),
        ("cstr->usize", [value]) => {
            call_1_cstr_usize(host, symbol, value.expect_string()?.as_str())
        }
        ("system:cstr->i32", [value]) => {
            call_1_system_cstr_i32(host, symbol, value.expect_string()?.as_str())
        }
        ("cstr->void", [value]) => call_1_cstr_void(host, symbol, value.expect_string()?.as_str()),
        ("system:cstr->void", [value]) => {
            call_1_system_cstr_void(host, symbol, value.expect_string()?.as_str())
        }
        ("cstr,cstr->i32", [left, right]) => call_2_cstr_cstr_i32(
            host,
            symbol,
            left.expect_string()?.as_str(),
            right.expect_string()?.as_str(),
        ),
        ("system:cstr,cstr->i32", [left, right]) => call_2_system_cstr_cstr_i32(
            host,
            symbol,
            left.expect_string()?.as_str(),
            right.expect_string()?.as_str(),
        ),
        ("cstr,usize->usize", [left, right]) => call_2_cstr_usize_usize(
            host,
            symbol,
            left.expect_string()?.as_str(),
            right.expect_int()?,
        ),
        ("i64,i64->i64", [left, right]) => {
            call_2_int_int(host, symbol, left.expect_int()?, right.expect_int()?)
        }
        ("bytes->usize", [value]) => call_1_bytes_usize(host, symbol, &value.expect_bytes()?),
        ("bytes,usize->usize", [value, right]) => {
            call_2_bytes_usize_usize(host, symbol, &value.expect_bytes()?, right.expect_int()?)
        }
        _ => Err(crate::RtError::unsupported_builtin(format!(
            "ffi.call<{signature}>"
        ))),
    }
}

pub fn call_0_void(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    host.ffi_call_0_void(symbol)?;
    Ok(RtValue::Unit)
}

pub fn call_0_c_bool(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_0_c_bool(symbol)?))
}

pub fn call_0_i32_bool(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_0_i32_bool(symbol)?))
}

pub fn call_0_system_i32_bool(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_0_system_i32_bool(symbol)?))
}

pub fn call_1_int(host: &mut dyn RtHost, symbol: crate::RtHandle, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_int(symbol, value)?))
}

pub fn call_1_i64_c_bool(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_1_i64_c_bool(symbol, value)?))
}

pub fn call_1_int_i32_bool(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_1_int_i32_bool(symbol, value)?))
}

pub fn call_1_int_system_i32_bool(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Bool(
        host.ffi_call_1_int_system_i32_bool(symbol, value)?,
    ))
}

pub fn call_1_int_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    host.ffi_call_1_int_void(symbol, value)?;
    Ok(RtValue::Unit)
}

pub fn call_1_cstr_compat_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    #[cfg(windows)]
    {
        call_1_system_cstr_i32(host, symbol, value)
    }
    #[cfg(not(windows))]
    {
        call_1_cstr_usize(host, symbol, value)
    }
}

pub fn call_1_cstr_usize(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_cstr_usize(symbol, value)?))
}

pub fn call_1_system_cstr_i32(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_1_system_cstr_i32(symbol, value)?,
    ))
}

pub fn call_1_cstr_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    host.ffi_call_1_cstr_void(symbol, value)?;
    Ok(RtValue::Unit)
}

pub fn call_1_cstr_platform_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    #[cfg(windows)]
    {
        call_1_system_cstr_void(host, symbol, value)
    }
    #[cfg(not(windows))]
    {
        call_1_cstr_void(host, symbol, value)
    }
}

pub fn call_1_system_cstr_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    host.ffi_call_1_system_cstr_void(symbol, value)?;
    Ok(RtValue::Unit)
}

pub fn call_2_cstr_cstr_i32(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_cstr_cstr_i32(symbol, left, right)?,
    ))
}

pub fn call_2_cstr_compat_cstr_i32(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: &str,
) -> RtResult<RtValue> {
    #[cfg(windows)]
    {
        call_2_system_cstr_cstr_i32(host, symbol, left, right)
    }
    #[cfg(not(windows))]
    {
        call_2_cstr_cstr_i32(host, symbol, left, right)
    }
}

pub fn call_2_system_cstr_cstr_i32(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_system_cstr_cstr_i32(symbol, left, right)?,
    ))
}

pub fn call_2_cstr_usize_usize(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_cstr_usize_usize(symbol, left, right)?,
    ))
}

pub fn call_1_bytes_usize(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &crate::RtBytes,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_bytes_usize(symbol, value)?))
}

pub fn call_2_int_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: i64,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_2_int_int(symbol, left, right)?))
}

pub fn call_2_bytes_usize_usize(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &crate::RtBytes,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_bytes_usize_usize(symbol, value, right)?,
    ))
}
