use crate::{RtError, RtErrorKind, RtResult, RtResultValue, RtValue};

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

pub fn unwrap_ok(value: &RtResultValue) -> RtResult<RtValue> {
    match value {
        RtResultValue::Ok(inner) => Ok((**inner).clone()),
        RtResultValue::Err(_) => Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "cannot unwrap Ok from Err",
        )),
    }
}

pub fn unwrap_err(value: &RtResultValue) -> RtResult<RtValue> {
    match value {
        RtResultValue::Err(inner) => Ok((**inner).clone()),
        RtResultValue::Ok(_) => Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "cannot unwrap Err from Ok",
        )),
    }
}
