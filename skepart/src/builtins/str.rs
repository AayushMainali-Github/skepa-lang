use crate::{RtResult, RtResultValue, RtString, RtValue};

pub fn len(value: &RtString) -> i64 {
    value.len_chars() as i64
}

pub fn contains(haystack: &RtString, needle: &RtString) -> bool {
    haystack.contains(needle)
}

pub fn starts_with(value: &RtString, prefix: &RtString) -> bool {
    value.as_str().starts_with(prefix.as_str())
}

pub fn ends_with(value: &RtString, suffix: &RtString) -> bool {
    value.as_str().ends_with(suffix.as_str())
}

pub fn trim(value: &RtString) -> RtString {
    RtString::from(value.as_str().trim())
}

pub fn to_lower(value: &RtString) -> RtString {
    RtString::from(value.as_str().to_lowercase())
}

pub fn to_upper(value: &RtString) -> RtString {
    RtString::from(value.as_str().to_uppercase())
}

pub fn index_of(haystack: &RtString, needle: &RtString) -> i64 {
    haystack.index_of(needle)
}

pub fn last_index_of(haystack: &RtString, needle: &RtString) -> i64 {
    let value = haystack.as_str();
    value
        .rfind(needle.as_str())
        .map(|byte_index| value[..byte_index].chars().count() as i64)
        .unwrap_or(-1)
}

pub fn replace(value: &RtString, from: &RtString, to: &RtString) -> RtString {
    RtString::from(value.as_str().replace(from.as_str(), to.as_str()))
}

pub fn repeat(value: &RtString, count: i64) -> RtResult<RtValue> {
    let count = usize::try_from(count).map_err(|_| {
        crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "str.repeat count must be non-negative",
        )
    })?;
    if value.as_str().len().checked_mul(count).is_none() {
        return Err(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "str.repeat result is too large",
        ));
    }
    Ok(RtValue::String(RtString::from(
        value.as_str().repeat(count),
    )))
}

pub fn is_empty(value: &RtString) -> bool {
    value.as_str().is_empty()
}

pub fn slice(value: &RtString, start: usize, end: usize) -> RtResult<RtValue> {
    match value.slice_chars(start..end) {
        Ok(sliced) => Ok(RtValue::Result(RtResultValue::ok(RtValue::String(sliced)))),
        Err(err) => Ok(RtValue::Result(RtResultValue::err(RtValue::String(
            RtString::from(err.to_string()),
        )))),
    }
}
