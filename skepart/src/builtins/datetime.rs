use crate::{RtHost, RtResult, RtValue};

pub fn now_unix(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.datetime_now_unix()?))
}

pub fn now_millis(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.datetime_now_millis()?))
}

pub fn from_unix(host: &mut dyn RtHost, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::String(host.datetime_from_unix(value)?))
}

pub fn from_millis(host: &mut dyn RtHost, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::String(host.datetime_from_millis(value)?))
}

pub fn parse_unix(host: &mut dyn RtHost, value: &str) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.datetime_parse_unix(value)?))
}

pub fn component(host: &mut dyn RtHost, name: &str, value: i64) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.datetime_component(name, value)?))
}
