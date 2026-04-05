use crate::{RtMap, RtOption, RtValue};

pub fn new() -> RtMap {
    RtMap::new()
}

pub fn len(value: &RtMap) -> RtValue {
    RtValue::Int(value.len() as i64)
}

pub fn has(value: &RtMap, key: &str) -> RtValue {
    RtValue::Bool(value.has(key))
}

pub fn get(value: &RtMap, key: &str) -> RtValue {
    RtValue::Option(match value.get(key) {
        Some(found) => RtOption::some(found),
        None => RtOption::none(),
    })
}

pub fn insert(value: &RtMap, key: &str, item: RtValue) {
    value.insert(key, item);
}

pub fn remove(value: &RtMap, key: &str) -> RtValue {
    RtValue::Option(match value.remove(key) {
        Some(found) => RtOption::some(found),
        None => RtOption::none(),
    })
}
