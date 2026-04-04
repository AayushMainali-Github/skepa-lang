use crate::{RtResultValue, RtValue};

pub fn ok(value: &RtValue) -> RtValue {
    RtValue::Result(RtResultValue::ok(value.clone()))
}

pub fn err(value: &RtValue) -> RtValue {
    RtValue::Result(RtResultValue::err(value.clone()))
}

pub fn is_ok(value: &RtResultValue) -> RtValue {
    RtValue::Bool(value.is_ok())
}

pub fn is_err(value: &RtResultValue) -> RtValue {
    RtValue::Bool(value.is_err())
}
