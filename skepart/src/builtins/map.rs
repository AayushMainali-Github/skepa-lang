use crate::{RtMap, RtValue};

pub fn new() -> RtMap {
    RtMap::new()
}

pub fn len(value: &RtMap) -> RtValue {
    RtValue::Int(value.len() as i64)
}

pub fn has(value: &RtMap, key: &str) -> RtValue {
    RtValue::Bool(value.has(key))
}

pub fn get(value: &RtMap, key: &str) -> crate::RtResult<RtValue> {
    value.get(key)
}

pub fn insert(value: &RtMap, key: &str, item: RtValue) {
    value.insert(key, item);
}

pub fn remove(value: &RtMap, key: &str) -> crate::RtResult<RtValue> {
    value.remove(key)
}
