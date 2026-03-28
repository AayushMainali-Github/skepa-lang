use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtBytes(Rc<[u8]>);

impl RtBytes {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(Rc::<[u8]>::from(value.into()))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u8>> for RtBytes {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

impl From<&[u8]> for RtBytes {
    fn from(value: &[u8]) -> Self {
        Self::new(value.to_vec())
    }
}
