use crate::{RtOption, RtValue};

pub fn some(value: &RtValue) -> RtValue {
    RtValue::Option(RtOption::some(value.clone()))
}

pub fn none() -> RtValue {
    RtValue::Option(RtOption::none())
}
