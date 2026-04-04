use crate::RtValue;

#[derive(Debug, Clone, PartialEq)]
pub enum RtResultValue {
    Ok(Box<RtValue>),
    Err(Box<RtValue>),
}

impl RtResultValue {
    pub fn ok(value: RtValue) -> Self {
        Self::Ok(Box::new(value))
    }

    pub fn err(value: RtValue) -> Self {
        Self::Err(Box::new(value))
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }
}
