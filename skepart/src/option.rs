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

    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}
