use crate::{RtBytes, RtError, RtErrorKind, RtResult, RtString, RtValue};

pub fn from_string(value: &str) -> RtResult<RtValue> {
    Ok(RtValue::Bytes(RtBytes::from(value.as_bytes())))
}

pub fn to_string(value: &RtBytes) -> RtResult<RtValue> {
    let text = std::str::from_utf8(value.as_slice()).map_err(|_| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            "bytes.toString expected valid UTF-8 data",
        )
    })?;
    Ok(RtValue::String(RtString::from(text)))
}

pub fn len(value: &RtBytes) -> RtValue {
    RtValue::Int(value.len() as i64)
}
