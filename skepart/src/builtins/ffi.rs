use crate::{RtHost, RtResult, RtValue};

pub fn open(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.ffi_open_library(path)?))
}

pub fn bind(host: &mut dyn RtHost, library: crate::RtHandle, symbol: &str) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.ffi_bind_symbol(library, symbol)?))
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
        ("->I", []) => call_0_int(host, symbol),
        ("->V", []) => call_0_void(host, symbol),
        ("->B", []) => call_0_bool(host, symbol),
        ("I->I", [value]) => call_1_int(host, symbol, value.expect_int()?),
        ("I->B", [value]) => call_1_int_bool(host, symbol, value.expect_int()?),
        ("I->V", [value]) => call_1_int_void(host, symbol, value.expect_int()?),
        ("S->I", [value]) => call_1_string_int(host, symbol, value.expect_string()?.as_str()),
        ("S->V", [value]) => call_1_string_void(host, symbol, value.expect_string()?.as_str()),
        ("SS->I", [left, right]) => call_2_string_int(
            host,
            symbol,
            left.expect_string()?.as_str(),
            right.expect_string()?.as_str(),
        ),
        ("SI->I", [left, right]) => call_2_string_int_int(
            host,
            symbol,
            left.expect_string()?.as_str(),
            right.expect_int()?,
        ),
        ("II->I", [left, right]) => {
            call_2_int_int(host, symbol, left.expect_int()?, right.expect_int()?)
        }
        ("Y->I", [value]) => call_1_bytes_int(host, symbol, &value.expect_bytes()?),
        ("YI->I", [value, right]) => {
            call_2_bytes_int_int(host, symbol, &value.expect_bytes()?, right.expect_int()?)
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

pub fn call_0_bool(host: &mut dyn RtHost, symbol: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_0_bool(symbol)?))
}

pub fn call_1_int(host: &mut dyn RtHost, symbol: crate::RtHandle, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_int(symbol, value)?))
}

pub fn call_1_int_bool(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.ffi_call_1_int_bool(symbol, value)?))
}

pub fn call_1_int_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: i64,
) -> RtResult<RtValue> {
    host.ffi_call_1_int_void(symbol, value)?;
    Ok(RtValue::Unit)
}

pub fn call_1_string_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_string_int(symbol, value)?))
}

pub fn call_1_string_void(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &str,
) -> RtResult<RtValue> {
    host.ffi_call_1_string_void(symbol, value)?;
    Ok(RtValue::Unit)
}

pub fn call_2_string_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: &str,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_string_int(symbol, left, right)?,
    ))
}

pub fn call_2_string_int_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: &str,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_string_int_int(symbol, left, right)?,
    ))
}

pub fn call_1_bytes_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &crate::RtBytes,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_bytes_int(symbol, value)?))
}

pub fn call_2_int_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    left: i64,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_2_int_int(symbol, left, right)?))
}

pub fn call_2_bytes_int_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &crate::RtBytes,
    right: i64,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(
        host.ffi_call_2_bytes_int_int(symbol, value, right)?,
    ))
}
