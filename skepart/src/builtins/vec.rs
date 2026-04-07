use crate::{RtOption, RtResult, RtValue, RtVec};

pub fn new() -> RtVec {
    RtVec::new()
}

pub fn len(vec: &RtVec) -> i64 {
    vec.len() as i64
}

pub fn push(vec: &RtVec, value: RtValue) {
    vec.push(value);
}

pub fn get(vec: &RtVec, index: usize) -> RtResult<RtValue> {
    vec.get(index)
}

pub fn try_get(vec: &RtVec, index: i64) -> RtValue {
    let Ok(index) = usize::try_from(index) else {
        return RtValue::Option(RtOption::none());
    };
    match vec.get(index) {
        Ok(value) => RtValue::Option(RtOption::some(value)),
        Err(_) => RtValue::Option(RtOption::none()),
    }
}

pub fn set(vec: &RtVec, index: usize, value: RtValue) -> RtResult<()> {
    vec.set(index, value)
}

pub fn delete(vec: &RtVec, index: usize) -> RtResult<RtValue> {
    vec.delete(index)
}
