use crate::{RtArray, RtOption, RtResult, RtString, RtValue};

pub fn len(array: &RtArray) -> i64 {
    array.len() as i64
}

pub fn is_empty(array: &RtArray) -> bool {
    array.is_empty()
}

pub fn first(array: &RtArray) -> RtValue {
    match array.get(0) {
        Ok(value) => RtValue::Option(RtOption::some(value)),
        Err(_) => RtValue::Option(RtOption::none()),
    }
}

pub fn last(array: &RtArray) -> RtValue {
    if array.is_empty() {
        return RtValue::Option(RtOption::none());
    }
    match array.get(array.len() - 1) {
        Ok(value) => RtValue::Option(RtOption::some(value)),
        Err(_) => RtValue::Option(RtOption::none()),
    }
}

pub fn join(array: &RtArray, sep: &RtString) -> RtResult<RtString> {
    let mut out = Vec::with_capacity(array.len());
    for item in array.iter() {
        out.push(item.expect_string()?.as_str().to_owned());
    }
    Ok(RtString::from(out.join(sep.as_str())))
}
