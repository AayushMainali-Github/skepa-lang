use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtBytes(Arc<[u8]>);

impl RtBytes {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(Arc::<[u8]>::from(value.into()))
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

    pub fn get(&self, index: usize) -> Option<u8> {
        self.0.get(index).copied()
    }

    pub fn slice(&self, start: usize, end: usize) -> Option<Self> {
        self.0.get(start..end).map(Self::from)
    }

    pub fn concat(&self, other: &Self) -> Self {
        let mut bytes = Vec::with_capacity(self.len() + other.len());
        bytes.extend_from_slice(self.as_slice());
        bytes.extend_from_slice(other.as_slice());
        Self::from(bytes)
    }

    pub fn push(&self, byte: u8) -> Self {
        let mut bytes = Vec::with_capacity(self.len() + 1);
        bytes.extend_from_slice(self.as_slice());
        bytes.push(byte);
        Self::from(bytes)
    }

    pub fn append(&self, other: &Self) -> Self {
        self.concat(other)
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
