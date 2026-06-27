//! `space` is capped at 10, matching `JSON.stringify`. A width over 10 uses 10
//! spaces. A string over 10 characters uses its first 10.

mod common;
use common::*;

use flatted::{stringify, Space};

#[test]
fn width_over_ten_clamps_to_ten() {
    let o = obj(vec![("a", n(1))]);
    let ten = " ".repeat(10);
    let expected = format!("[{{\n{ten}\"a\": 1\n}}]");
    assert_eq!(stringify(&o, None, Some(&Space::Width(20))), expected);
}

#[test]
fn width_ten_is_unchanged() {
    let o = obj(vec![("a", n(1))]);
    let ten = " ".repeat(10);
    let expected = format!("[{{\n{ten}\"a\": 1\n}}]");
    assert_eq!(stringify(&o, None, Some(&Space::Width(10))), expected);
}

#[test]
fn width_zero_is_compact() {
    let o = obj(vec![("a", n(1))]);
    assert_eq!(stringify(&o, None, Some(&Space::Width(0))), r#"[{"a":1}]"#);
}

#[test]
fn string_over_ten_chars_takes_first_ten() {
    let o = obj(vec![("a", n(1))]);
    let indent = "x".repeat(15);
    let ten = "x".repeat(10);
    let expected = format!("[{{\n{ten}\"a\": 1\n}}]");
    assert_eq!(stringify(&o, None, Some(&Space::Str(indent))), expected);
}
