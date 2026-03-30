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
