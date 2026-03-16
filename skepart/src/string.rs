use std::ops::Range;
use std::rc::Rc;

use crate::{RtError, RtErrorKind, RtResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtString(Rc<str>);

impl RtString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Rc::<str>::from(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len_chars(&self) -> usize {
        self.0.chars().count()
    }

    pub fn contains(&self, needle: &RtString) -> bool {
        self.0.contains(needle.as_str())
    }

    pub fn index_of(&self, needle: &RtString) -> i64 {
        self.0
            .find(needle.as_str())
            .map(|idx| idx as i64)
            .unwrap_or(-1)
    }

    pub fn slice_chars(&self, range: Range<usize>) -> RtResult<Self> {
        let chars = self.0.chars().collect::<Vec<_>>();
        if range.start > range.end || range.end > chars.len() {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    chars.len()
                ),
            ));
        }
        Ok(Self::new(chars[range].iter().collect::<String>()))
    }
}

impl From<&str> for RtString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RtString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
