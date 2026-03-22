use std::ops::Range;
use std::rc::Rc;

use crate::{RtError, RtErrorKind, RtResult};

#[derive(Debug, Clone)]
enum RtStringRepr {
    Owned(Rc<str>),
    AsciiSlice { base: Rc<str>, range: Range<usize> },
}

#[derive(Debug, Clone)]
pub struct RtString(RtStringRepr);

impl RtString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(RtStringRepr::Owned(Rc::<str>::from(value.into())))
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            RtStringRepr::Owned(value) => value,
            RtStringRepr::AsciiSlice { base, range } => &base[range.clone()],
        }
    }

    pub fn len_chars(&self) -> usize {
        let value = self.as_str();
        if value.is_ascii() {
            value.len()
        } else {
            value.chars().count()
        }
    }

    pub fn contains(&self, needle: &RtString) -> bool {
        self.as_str().contains(needle.as_str())
    }

    pub fn index_of(&self, needle: &RtString) -> i64 {
        let value = self.as_str();
        let needle = needle.as_str();
        if value.is_ascii() && needle.is_ascii() {
            return self
                .as_str()
                .find(needle)
                .map(|idx| idx as i64)
                .unwrap_or(-1);
        }
        value
            .find(needle)
            .map(|idx| value[..idx].chars().count() as i64)
            .unwrap_or(-1)
    }

    pub fn slice_chars(&self, range: Range<usize>) -> RtResult<Self> {
        let value = self.as_str();
        if value.is_ascii() {
            if range.start > range.end || range.end > value.len() {
                return Err(RtError::new(
                    RtErrorKind::IndexOutOfBounds,
                    format!(
                        "str.slice bounds out of range: start={}, end={}, len={}",
                        range.start,
                        range.end,
                        value.len()
                    ),
                ));
            }
            return Ok(Self(self.ascii_slice_repr(range)));
        }

        let Some(start) = nth_char_boundary(value, range.start) else {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    value.chars().count()
                ),
            ));
        };
        let Some(end) = nth_char_boundary(value, range.end) else {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    value.chars().count()
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
                    value.chars().count()
                ),
            ));
        }
        Ok(Self(RtStringRepr::Owned(Rc::from(&value[start..end]))))
    }

    fn ascii_slice_repr(&self, range: Range<usize>) -> RtStringRepr {
        match &self.0 {
            RtStringRepr::Owned(base) => RtStringRepr::AsciiSlice {
                base: base.clone(),
                range,
            },
            RtStringRepr::AsciiSlice {
                base,
                range: outer,
            } => RtStringRepr::AsciiSlice {
                base: base.clone(),
                range: (outer.start + range.start)..(outer.start + range.end),
            },
        }
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

impl PartialEq for RtString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for RtString {}
