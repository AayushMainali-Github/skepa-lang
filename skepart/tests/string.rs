use skepart::{RtErrorKind, RtString};

#[test]
fn string_counts_chars_not_bytes_for_unicode() {
    let value = RtString::from("naive");
    let unicode = RtString::from("a🙂ब");
    assert_eq!(value.len_chars(), 5);
    assert_eq!(unicode.len_chars(), 3);
}

#[test]
fn string_slice_handles_edges_and_bounds() {
    let value = RtString::from("🙂hello");
    assert_eq!(
        value.slice_chars(0..1).expect("slice should work"),
        RtString::from("🙂")
    );
    assert_eq!(
        value.slice_chars(1..6).expect("slice should work"),
        RtString::from("hello")
    );
    let err = value.slice_chars(3..9).expect_err("slice should fail");
    assert_eq!(err.kind, RtErrorKind::IndexOutOfBounds);
}

#[test]
fn string_contains_and_index_cover_empty_and_missing_needles() {
    let value = RtString::from("skepa-language");
    assert!(value.contains(&RtString::from("language")));
    assert!(value.contains(&RtString::from("")));
    assert_eq!(value.index_of(&RtString::from("epa")), 2);
    assert_eq!(value.index_of(&RtString::from("zzz")), -1);
    assert_eq!(RtString::from("").index_of(&RtString::from("")), 0);
}

#[test]
fn string_handles_empty_and_full_range_slices() {
    let empty = RtString::from("");
    assert_eq!(empty.len_chars(), 0);
    assert_eq!(
        empty.slice_chars(0..0).expect("empty slice"),
        RtString::from("")
    );

    let value = RtString::from("abc");
    assert_eq!(
        value.slice_chars(0..3).expect("full slice"),
        RtString::from("abc")
    );
}

#[test]
fn string_rejects_reversed_and_negative_style_equivalent_bounds() {
    let value = RtString::from("skepa");
    let start = 4;
    let end = 2;
    assert_eq!(
        value.slice_chars(start..end).expect_err("reversed").kind,
        RtErrorKind::IndexOutOfBounds
    );
    assert_eq!(
        value.slice_chars(0..6).expect_err("past end").kind,
        RtErrorKind::IndexOutOfBounds
    );
}

#[test]
fn string_index_and_contains_work_on_unicode_boundaries() {
    let value = RtString::from("a🙂b🙂c");
    assert!(value.contains(&RtString::from("🙂b")));
    assert_eq!(value.index_of(&RtString::from("🙂b")), 1);
    assert_eq!(
        value.slice_chars(1..4).expect("unicode middle slice"),
        RtString::from("🙂b🙂")
    );
}
