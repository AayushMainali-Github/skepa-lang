use crate::RtValue;

#[derive(Debug, Clone, PartialEq)]
pub struct RtOption(pub Option<Box<RtValue>>);

impl RtOption {
    pub fn some(value: RtValue) -> Self {
        Self(Some(Box::new(value)))
    }

    pub fn none() -> Self {
        Self(None)
    }
}
