use crate::{RtError, RtErrorKind, RtOption, RtResult, RtValue};

pub fn some(value: &RtValue) -> RtValue {
    RtValue::Option(RtOption::some(value.clone()))
}

pub fn none() -> RtValue {
    RtValue::Option(RtOption::none())
}

pub fn is_some(value: &RtOption) -> RtValue {
    RtValue::Bool(value.is_some())
}

pub fn is_none(value: &RtOption) -> RtValue {
    RtValue::Bool(value.is_none())
}

pub fn unwrap_some(value: &RtOption) -> RtResult<RtValue> {
    match &value.0 {
        Some(inner) => Ok((**inner).clone()),
        None => Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "cannot unwrap Some from None",
        )),
    }
}
