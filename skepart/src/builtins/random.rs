use crate::{RtHost, RtResult, RtValue};

pub fn seed(host: &mut dyn RtHost, value: i64) -> RtResult<RtValue> {
    host.random_seed(value)?;
    Ok(RtValue::Unit)
}

pub fn int(host: &mut dyn RtHost, min: i64, max: i64) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.random_int(min, max)?))
}

pub fn float(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::Float(host.random_float()?))
}
