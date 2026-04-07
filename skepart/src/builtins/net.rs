use crate::{RtHost, RtResult, RtResultValue, RtString, RtValue};

pub fn test_socket(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(
        host.net_alloc_handle(crate::RtHandleKind::Socket)?,
    ))
}

pub fn listen(host: &mut dyn RtHost, address: &str) -> RtResult<RtValue> {
    Ok(match host.net_listen(address) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn connect(host: &mut dyn RtHost, address: &str) -> RtResult<RtValue> {
    Ok(match host.net_connect(address) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn tls_connect(host: &mut dyn RtHost, host_name: &str, port: i64) -> RtResult<RtValue> {
    Ok(match host.net_tls_connect(host_name, port) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn resolve(host: &mut dyn RtHost, host_name: &str) -> RtResult<RtValue> {
    match host.net_resolve(host_name) {
        Ok(ip) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::String(
            ip,
        )))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn parse_url(host: &mut dyn RtHost, url: &str) -> RtResult<RtValue> {
    match host.net_parse_url(url) {
        Ok(parts) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Map(
            parts,
        )))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn fetch(host: &mut dyn RtHost, url: &str, options: &crate::RtMap) -> RtResult<RtValue> {
    match host.net_fetch(url, options) {
        Ok(response) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Map(
            response,
        )))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn accept(host: &mut dyn RtHost, listener: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_accept(listener) {
        Ok(handle) => RtValue::Result(RtResultValue::ok(RtValue::Handle(handle))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn read(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_read(socket) {
        Ok(value) => RtValue::Result(RtResultValue::ok(RtValue::String(value))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn write(host: &mut dyn RtHost, socket: crate::RtHandle, data: &str) -> RtResult<RtValue> {
    Ok(match host.net_write(socket, data) {
        Ok(()) => RtValue::Result(RtResultValue::ok(RtValue::Unit)),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn read_bytes(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_read_bytes(socket) {
        Ok(value) => RtValue::Result(RtResultValue::ok(RtValue::Bytes(value))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn write_bytes(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    data: &crate::RtBytes,
) -> RtResult<RtValue> {
    Ok(match host.net_write_bytes(socket, data) {
        Ok(()) => RtValue::Result(RtResultValue::ok(RtValue::Unit)),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn read_n(host: &mut dyn RtHost, socket: crate::RtHandle, count: i64) -> RtResult<RtValue> {
    Ok(match host.net_read_n(socket, count) {
        Ok(value) => RtValue::Result(RtResultValue::ok(RtValue::Bytes(value))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn local_addr(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_local_addr(socket) {
        Ok(value) => RtValue::Result(RtResultValue::ok(RtValue::String(value))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn peer_addr(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_peer_addr(socket) {
        Ok(value) => RtValue::Result(RtResultValue::ok(RtValue::String(value))),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn flush(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(match host.net_flush(socket) {
        Ok(()) => RtValue::Result(RtResultValue::ok(RtValue::Unit)),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn set_read_timeout(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    millis: i64,
) -> RtResult<RtValue> {
    Ok(match host.net_set_read_timeout(socket, millis) {
        Ok(()) => RtValue::Result(RtResultValue::ok(RtValue::Unit)),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn set_write_timeout(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    millis: i64,
) -> RtResult<RtValue> {
    Ok(match host.net_set_write_timeout(socket, millis) {
        Ok(()) => RtValue::Result(RtResultValue::ok(RtValue::Unit)),
        Err(err) => RtValue::Result(RtResultValue::err(RtValue::String(RtString::from(
            err.to_string(),
        )))),
    })
}

pub fn close(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(socket)?;
    Ok(RtValue::Unit)
}

pub fn close_listener(host: &mut dyn RtHost, listener: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(listener)?;
    Ok(RtValue::Unit)
}
