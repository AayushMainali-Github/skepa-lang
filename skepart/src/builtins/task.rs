use super::BuiltinContext;
use crate::{RtFunctionRef, RtHandle, RtHost, RtResult, RtValue};

pub fn test_task(host: &mut dyn RtHost, value: &RtValue) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_store_completed(value.clone())?))
}

pub fn test_channel(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_channel()?))
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

pub fn spawn(ctx: &mut dyn BuiltinContext, function: RtFunctionRef) -> RtResult<RtValue> {
    let value = ctx.call_function(function, &[])?;
    Ok(RtValue::Handle(ctx.host().task_store_completed(value)?))
}

pub fn join(host: &mut dyn RtHost, task: RtHandle) -> RtResult<RtValue> {
    host.task_join(task)
}
