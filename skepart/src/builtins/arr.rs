use crate::{RtArray, RtResult, RtString, RtValue};

pub fn len(array: &RtArray) -> i64 {
    array.len() as i64
}

pub fn is_empty(array: &RtArray) -> bool {
    array.is_empty()
}

pub fn first(array: &RtArray) -> RtResult<RtValue> {
    array.get(0)
}

pub fn last(array: &RtArray) -> RtResult<RtValue> {
    if array.is_empty() {
        return Err(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "arr.last requires a non-empty array",
        ));
    }
    array.get(array.len() - 1)
}

pub fn join(array: &RtArray, sep: &RtString) -> RtResult<RtString> {
    let mut out = Vec::with_capacity(array.len());
    for item in array.iter() {
        out.push(item.expect_string()?.as_str().to_owned());
    }
    Ok(RtString::from(out.join(sep.as_str())))
}
