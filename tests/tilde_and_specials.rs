//! The format never escapes or marks strings by content. A literal `~` or the
//! text `\x7e` round-trips with no corruption, and an index-looking string
//! value stays a string.

mod common;
use common::*;

use flatted::{parse, stringify};

#[test]
fn backslash_x7e_round_trips() {
    let special = "\\x7e";
    let o = obj(vec![("a", s(special))]);
    let back = parse(&stringify(&o, None, None), None).unwrap();
    assert_eq!(get(&back, "a"), s(special));
}

#[test]
fn tilde_backslash_x7e_round_trips() {
    let special = "~\\x7e";
    let o = obj(vec![("a", s(special))]);
    let back = parse(&stringify(&o, None, None), None).unwrap();
    assert_eq!(get(&back, "a"), s(special));
}

#[test]
fn tilde_inside_text_round_trips() {
    let o = obj(vec![("bar", s("something ~ baz"))]);
    let text = stringify(&o, None, None);
    assert_eq!(text, r#"[{"bar":"1"},"something ~ baz"]"#);
    let back = parse(&text, None).unwrap();
    assert_eq!(get(&back, "bar"), s("something ~ baz"));
}

/// A literal string that looks like an index stays a literal value. The format
/// disambiguates by position, not by the characters.
#[test]
fn index_looking_string_stays_literal() {
    let o = obj(vec![("k", s("3")), ("r", s("k"))]);
    let back = parse(&stringify(&o, None, None), None).unwrap();
    assert_eq!(get(&back, "k"), s("3"));
    assert_eq!(get(&back, "r"), s("k"));
}

/// Several literal numeric-looking strings all survive.
#[test]
fn many_numeric_strings_survive() {
    let a = arr(vec![s("0"), s("1"), s("2"), s("99")]);
    let back = parse(&stringify(&a, None, None), None).unwrap();
    assert_eq!(at(&back, 0), s("0"));
    assert_eq!(at(&back, 1), s("1"));
    assert_eq!(at(&back, 2), s("2"));
    assert_eq!(at(&back, 3), s("99"));
}
