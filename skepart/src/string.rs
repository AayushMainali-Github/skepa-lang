use std::ops::Range;
use std::sync::Arc;

use crate::{RtError, RtErrorKind, RtResult};

#[derive(Debug, Clone, Copy)]
struct RtStringMeta {
    is_ascii: bool,
    len_chars: usize,
}

#[derive(Debug, Clone)]
struct RtStringStorage(Arc<str>);

#[derive(Debug, Clone)]
struct RtStringView {
    storage: RtStringStorage,
    bytes: Range<usize>,
    meta: RtStringMeta,
}

#[derive(Debug, Clone)]
enum RtStringRepr {
    NativeView(RtStringView),
    RuntimeObject(RtStringStorage),
}

#[derive(Debug, Clone)]
pub struct RtString {
    repr: RtStringRepr,
}

impl RtString {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            repr: RtStringRepr::RuntimeObject(RtStringStorage(Arc::<str>::from(value))),
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.repr {
            RtStringRepr::NativeView(view) => &view.storage.0[view.bytes.clone()],
            RtStringRepr::RuntimeObject(storage) => &storage.0,
        }
    }

    pub fn len_chars(&self) -> usize {
        self.meta().len_chars
    }

    pub fn contains(&self, needle: &RtString) -> bool {
        let haystack = self.as_str();
        let needle_str = needle.as_str();
        if self.meta().is_ascii && needle.meta().is_ascii && needle_str.len() == 1 {
            return haystack.as_bytes().contains(&needle_str.as_bytes()[0]);
        }
        haystack.contains(needle_str)
    }

    pub fn index_of(&self, needle: &RtString) -> i64 {
        let value = self.as_str();
        let needle_str = needle.as_str();
        if self.meta().is_ascii && needle.meta().is_ascii {
            if needle_str.len() == 1 {
                return value
                    .as_bytes()
                    .iter()
                    .position(|byte| *byte == needle_str.as_bytes()[0])
                    .map(|idx| idx as i64)
                    .unwrap_or(-1);
            }
            return value.find(needle_str).map(|idx| idx as i64).unwrap_or(-1);
        }
        value
            .find(needle_str)
            .map(|idx| value[..idx].chars().count() as i64)
            .unwrap_or(-1)
    }

    pub fn slice_chars(&self, range: Range<usize>) -> RtResult<Self> {
        if range.start > range.end || range.end > self.meta().len_chars {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    self.meta().len_chars
                ),
            ));
        }

        if self.meta().is_ascii {
            return Ok(Self {
                repr: RtStringRepr::NativeView(self.slice_view(
                    range.start..range.end,
                    RtStringMeta {
                        is_ascii: true,
                        len_chars: range.end - range.start,
                    },
                )),
            });
        }

        let value = self.as_str();
        let Some(start) = nth_char_boundary(value, range.start) else {
            return Err(RtError::new(
                RtErrorKind::IndexOutOfBounds,
                format!(
                    "str.slice bounds out of range: start={}, end={}, len={}",
                    range.start,
                    range.end,
                    self.meta().len_chars
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
                    self.meta().len_chars
                ),
            ));
        };
        Ok(Self {
            repr: RtStringRepr::NativeView(self.slice_view(
                start..end,
                RtStringMeta {
                    is_ascii: false,
                    len_chars: range.end - range.start,
                },
            )),
        })
    }

    fn meta(&self) -> RtStringMeta {
        match &self.repr {
            RtStringRepr::NativeView(view) => view.meta,
            RtStringRepr::RuntimeObject(storage) => meta_for_str(&storage.0),
        }
    }

    fn slice_view(&self, bytes: Range<usize>, meta: RtStringMeta) -> RtStringView {
        match &self.repr {
            RtStringRepr::NativeView(view) => RtStringView {
                storage: view.storage.clone(),
                bytes: (view.bytes.start + bytes.start)..(view.bytes.start + bytes.end),
                meta,
            },
            RtStringRepr::RuntimeObject(storage) => RtStringView {
                storage: storage.clone(),
                bytes,
                meta,
            },
        }
    }
}

fn meta_for_str(value: &str) -> RtStringMeta {
    if value.is_ascii() {
        RtStringMeta {
            is_ascii: true,
            len_chars: value.len(),
        }
    } else {
        RtStringMeta {
            is_ascii: false,
            len_chars: value.chars().count(),
        }
    }
}

fn nth_char_boundary(value: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return Some(0);
    }
    if index > value.chars().count() {
        return None;
    }
    if index == value.chars().count() {
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
