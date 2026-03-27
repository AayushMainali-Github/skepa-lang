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
