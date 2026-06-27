//! Malformed input returns an error and never panics. An empty table is a
//! documented edge: it has no root, so parse yields null.

mod common;
use common::*;

use flatted::parse;

#[test]
fn non_json_input_errors() {
    assert!(parse("not json", None).is_err());
}

#[test]
fn non_array_root_errors() {
    assert!(parse("{}", None).is_err());
    assert!(parse("42", None).is_err());
    assert!(parse(r#""x""#, None).is_err());
}

#[test]
fn out_of_range_index_errors() {
    // Index 5 points past the one-element table.
    assert!(parse(r#"[["5"]]"#, None).is_err());
}

#[test]
fn empty_table_yields_null() {
    // No index 0 to act as root. The result is null, the documented sentinel.
    assert_eq!(parse("[]", None).unwrap(), null());
}

#[test]
fn trailing_characters_error() {
    assert!(parse("[null] extra", None).is_err());
}

#[test]
fn unterminated_string_errors() {
    assert!(parse(r#"["abc]"#, None).is_err());
}
