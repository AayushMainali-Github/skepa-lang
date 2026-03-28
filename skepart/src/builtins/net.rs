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

pub fn close(host: &mut dyn RtHost, socket: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(socket)?;
    Ok(RtValue::Unit)
}

pub fn close_listener(host: &mut dyn RtHost, listener: crate::RtHandle) -> RtResult<RtValue> {
    host.net_close_handle(listener)?;
    Ok(RtValue::Unit)
}
