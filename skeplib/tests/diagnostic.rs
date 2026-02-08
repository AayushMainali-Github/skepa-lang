use skeplib::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticLevel, Span};

#[test]
fn span_creation_and_ordering() {
    let s = Span::new(2, 5, 1, 3);
    assert_eq!(s.start, 2);
    assert_eq!(s.end, 5);
    assert_eq!(s.line, 1);
    assert_eq!(s.col, 3);
}

#[test]
fn span_merge_keeps_outer_bounds() {
    let a = Span::new(10, 20, 2, 4);
    let b = Span::new(3, 8, 1, 1);
    let merged = a.merge(b);
    assert_eq!(merged.start, 3);
    assert_eq!(merged.end, 20);
    assert_eq!(merged.line, 1);
    assert_eq!(merged.col, 1);
}

#[test]
fn diagnostic_format_includes_line_col() {
    let d = Diagnostic::error("bad token", Span::new(0, 1, 7, 9));
    let rendered = d.to_string();
    assert!(rendered.contains("error"));
    assert!(rendered.contains("7:9"));
    assert!(rendered.contains("bad token"));
}

#[test]
fn diagnostic_bag_collects_multiple_errors_without_panicking() {
    let mut bag = DiagnosticBag::new();
    bag.error("first", Span::new(0, 1, 1, 1));
    bag.warning("second", Span::new(2, 3, 1, 3));
    bag.error("third", Span::new(4, 5, 2, 1));

    assert_eq!(bag.len(), 3);
    assert!(!bag.is_empty());
    assert_eq!(bag.as_slice()[0].level, DiagnosticLevel::Error);
    assert_eq!(bag.as_slice()[1].level, DiagnosticLevel::Warning);
    assert_eq!(bag.as_slice()[2].message, "third");
}

#[test]
fn span_merge_prefers_earliest_start_location() {
    let a = Span::new(0, 2, 1, 1);
    let b = Span::new(5, 9, 4, 3);
    let merged = a.merge(b);
    assert_eq!(merged.start, 0);
    assert_eq!(merged.end, 9);
    assert_eq!(merged.line, 1);
    assert_eq!(merged.col, 1);
}

#[test]
fn diagnostic_bag_into_vec_keeps_order() {
    let mut bag = DiagnosticBag::new();
    bag.error("a", Span::new(0, 1, 1, 1));
    bag.warning("b", Span::new(2, 3, 1, 3));
    bag.error("c", Span::new(4, 5, 1, 5));

    let out = bag.into_vec();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].message, "a");
    assert_eq!(out[1].message, "b");
    assert_eq!(out[2].message, "c");
}
