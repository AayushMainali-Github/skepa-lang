use crate::{RtBytes, RtError, RtErrorKind, RtOption, RtResult, RtString, RtValue};

pub fn from_string(value: &str) -> RtResult<RtValue> {
    Ok(RtValue::Bytes(RtBytes::from(value.as_bytes())))
}

pub fn to_string(value: &RtBytes) -> RtResult<RtValue> {
    match std::str::from_utf8(value.as_slice()) {
        Ok(text) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::String(
            RtString::from(text),
        )))),
        Err(_) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            RtString::from("bytes.toString expected valid UTF-8 data"),
        )))),
    }
}

pub fn len(value: &RtBytes) -> RtValue {
    RtValue::Int(value.len() as i64)
}

pub fn get(value: &RtBytes, index: i64) -> RtValue {
    let Ok(index) = usize::try_from(index) else {
        return RtValue::Option(RtOption::none());
    };
    match value.get(index) {
        Some(byte) => RtValue::Option(RtOption::some(RtValue::Int(i64::from(byte)))),
        None => RtValue::Option(RtOption::none()),
    }
}

pub fn slice(value: &RtBytes, start: i64, end: i64) -> RtResult<RtValue> {
    let start = usize::try_from(start)
        .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative bytes slice start"))?;
    let end = usize::try_from(end)
        .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative bytes slice end"))?;
    if start > end {
        return Err(RtError::new(
            RtErrorKind::IndexOutOfBounds,
            "bytes slice start cannot be greater than end",
        ));
    }
    let sliced = value
        .slice(start, end)
        .ok_or_else(|| RtError::new(RtErrorKind::IndexOutOfBounds, "bytes slice out of bounds"))?;
    Ok(RtValue::Bytes(sliced))
}

pub fn concat(left: &RtBytes, right: &RtBytes) -> RtValue {
    RtValue::Bytes(left.concat(right))
}

pub fn push(value: &RtBytes, byte: i64) -> RtResult<RtValue> {
    let byte = u8::try_from(byte).map_err(|_| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            "bytes.push expects byte value in range 0..=255",
        )
    })?;
    Ok(RtValue::Bytes(value.push(byte)))
}

pub fn append(left: &RtBytes, right: &RtBytes) -> RtValue {
    RtValue::Bytes(left.append(right))
}
