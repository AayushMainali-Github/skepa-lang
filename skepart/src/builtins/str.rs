use crate::{RtResult, RtString};

pub fn len(value: &RtString) -> i64 {
    value.len_chars() as i64
}

pub fn contains(haystack: &RtString, needle: &RtString) -> bool {
    haystack.contains(needle)
}

pub fn index_of(haystack: &RtString, needle: &RtString) -> i64 {
    haystack.index_of(needle)
}

pub fn slice(value: &RtString, start: usize, end: usize) -> RtResult<RtString> {
    value.slice_chars(start..end)
}
