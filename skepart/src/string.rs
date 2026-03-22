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
        if self.0.is_ascii() {
            self.0.len()
        } else {
            self.0.chars().count()
        }
    }

    pub fn contains(&self, needle: &RtString) -> bool {
        self.0.contains(needle.as_str())
    }

    pub fn index_of(&self, needle: &RtString) -> i64 {
        if self.0.is_ascii() && needle.as_str().is_ascii() {
            return self
                .0
                .find(needle.as_str())
                .map(|idx| idx as i64)
                .unwrap_or(-1);
        }
        self.0
            .find(needle.as_str())
            .map(|idx| self.0[..idx].chars().count() as i64)
            .unwrap_or(-1)
    }

    pub fn slice_chars(&self, range: Range<usize>) -> RtResult<Self> {
        if self.0.is_ascii() {
            if range.start > range.end || range.end > self.0.len() {
                return Err(RtError::new(
                    RtErrorKind::IndexOutOfBounds,
                    format!(
                        "str.slice bounds out of range: start={}, end={}, len={}",
                        range.start,
                        range.end,
                        self.0.len()
                    ),
                ));
            }
            return Ok(Self(Rc::from(&self.0[range])));
        }

        let Some(start) = nth_char_boundary(self.0.as_ref(), range.start) else {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    self.0.chars().count()
                ),
            ));
        };
        let Some(end) = nth_char_boundary(self.0.as_ref(), range.end) else {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    self.0.chars().count()
                ),
            ));
        };
        if start > end {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    self.0.chars().count()
                ),
            ));
        }
        Ok(Self(Rc::from(&self.0[start..end])))
    }
}

fn nth_char_boundary(value: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return Some(0);
    }
    let char_len = value.chars().count();
    if index > char_len {
        return None;
    }
    if index == char_len {
        return Some(value.len());
    }
    value.char_indices().nth(index).map(|(offset, _)| offset)
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
