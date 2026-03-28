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

pub fn get(value: &RtBytes, index: i64) -> RtResult<RtValue> {
    let index = usize::try_from(index)
        .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative bytes index"))?;
    let byte = value
        .get(index)
        .ok_or_else(|| RtError::index_out_of_bounds(index, value.len()))?;
    Ok(RtValue::Int(i64::from(byte)))
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

pub fn eq(left: &RtBytes, right: &RtBytes) -> RtValue {
    RtValue::Bool(left == right)
}
