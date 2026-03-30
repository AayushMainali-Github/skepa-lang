use crate::{RtHost, RtResult, RtValue};

pub fn test_task(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_make_task_handle(0)?))
}

pub fn test_channel(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_make_channel_handle(0)?))
}

pub fn channel(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_channel()?))
}

pub fn send(host: &mut dyn RtHost, channel: crate::RtHandle, value: &RtValue) -> RtResult<RtValue> {
    host.task_send(channel, value.clone())?;
    Ok(RtValue::Unit)
}

pub fn recv(host: &mut dyn RtHost, channel: crate::RtHandle) -> RtResult<RtValue> {
    host.task_recv(channel)
}
