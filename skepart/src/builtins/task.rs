use crate::{RtHost, RtResult, RtValue};

pub fn test_task(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_make_task_handle(0)?))
}

pub fn test_channel(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Handle(host.task_make_channel_handle(0)?))
}
