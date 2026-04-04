use crate::{RtResultValue, RtValue};

pub fn ok(value: &RtValue) -> RtValue {
    RtValue::Result(RtResultValue::ok(value.clone()))
}

pub fn err(value: &RtValue) -> RtValue {
    RtValue::Result(RtResultValue::err(value.clone()))
}
