use crate::{RtHost, RtResult, RtValue};

pub fn test_socket(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(
        host.net_alloc_handle(crate::RtHandleKind::Socket)?,
    ))
}

pub fn listen(host: &mut dyn RtHost, address: &str) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.net_listen(address)?))
}

pub fn connect(host: &mut dyn RtHost, address: &str) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.net_connect(address)?))
}

pub fn tls_connect(host: &mut dyn RtHost, host_name: &str, port: i64) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.net_tls_connect(host_name, port)?))
}

pub fn resolve(host: &mut dyn RtHost, host_name: &str) -> RtResult<RtValue> {
    Ok(RtValue::String(host.net_resolve(host_name)?))
}

pub fn parse_url(host: &mut dyn RtHost, url: &str) -> RtResult<RtValue> {
    Ok(RtValue::Map(host.net_parse_url(url)?))
}

pub fn accept(host: &mut dyn RtHost, listener: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.net_accept(listener)?))
}

pub fn read(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::String(host.net_read(socket)?))
}

pub fn write(host: &mut dyn RtHost, socket: crate::RtHandle, data: &str) -> RtResult<RtValue> {
    host.net_write(socket, data)?;
    Ok(RtValue::Unit)
}

pub fn read_bytes(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::Bytes(host.net_read_bytes(socket)?))
}

pub fn write_bytes(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    data: &crate::RtBytes,
) -> RtResult<RtValue> {
    host.net_write_bytes(socket, data)?;
    Ok(RtValue::Unit)
}

pub fn read_n(host: &mut dyn RtHost, socket: crate::RtHandle, count: i64) -> RtResult<RtValue> {
    Ok(RtValue::Bytes(host.net_read_n(socket, count)?))
}

pub fn local_addr(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::String(host.net_local_addr(socket)?))
}

pub fn peer_addr(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    Ok(RtValue::String(host.net_peer_addr(socket)?))
}

pub fn flush(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    host.net_flush(socket)?;
    Ok(RtValue::Unit)
}

pub fn set_read_timeout(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    millis: i64,
) -> RtResult<RtValue> {
    host.net_set_read_timeout(socket, millis)?;
    Ok(RtValue::Unit)
}

pub fn set_write_timeout(
    host: &mut dyn RtHost,
    socket: crate::RtHandle,
    millis: i64,
) -> RtResult<RtValue> {
    host.net_set_write_timeout(socket, millis)?;
    Ok(RtValue::Unit)
}

pub fn close(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(socket)?;
    Ok(RtValue::Unit)
}

pub fn close_listener(host: &mut dyn RtHost, listener: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(listener)?;
    Ok(RtValue::Unit)
}
