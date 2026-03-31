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

pub fn call_1_int(host: &mut dyn RtHost, symbol: crate::RtHandle, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_int(symbol, value)?))
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

pub fn call_1_bytes_int(
    host: &mut dyn RtHost,
    symbol: crate::RtHandle,
    value: &crate::RtBytes,
) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.ffi_call_1_bytes_int(symbol, value)?))
}
