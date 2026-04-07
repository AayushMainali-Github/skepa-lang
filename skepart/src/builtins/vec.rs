use crate::{RtResult, RtValue, RtVec};

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

pub fn set(vec: &RtVec, index: usize, value: RtValue) -> RtResult<()> {
    vec.set(index, value)
}

pub fn delete(vec: &RtVec, index: usize) -> RtResult<RtValue> {
    vec.delete(index)
}
