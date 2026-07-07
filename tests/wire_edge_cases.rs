//! Edge cases in the wire format that match `JSON.stringify`/`JSON.parse`:
//! control and unicode escaping, empty-string values, a literal string that
//! looks like a live pointer, duplicate keys, and allow-list filtering of array
//! elements.

mod common;
use common::*;

use flatted::{parse, stringify, Replacer};

/// C0 controls use short escapes where they exist, otherwise `\u00xx`.
/// Non-control characters pass through.
#[test]
fn control_characters_escape() {
    let text: String = [
        '\u{01}', '\u{08}', '\u{09}', '\u{0a}', '\u{0c}', '\u{0d}', '\u{1f}',
    ]
    .iter()
    .collect();
    let o = obj(vec![("a", s(&text))]);
    let wire = stringify(&o, None, None);
    // 0x01 and 0x1f have no short escape, so they use \u00xx. The rest use
    // their short forms. Build the expected escape text from pieces so the
    // backslashes stay unambiguous.
    let bs = '\\';
    let escaped = format!("{bs}u0001{bs}b{bs}t{bs}n{bs}f{bs}r{bs}u001f");
    let expected = format!("[{{\"a\":\"1\"}},\"{escaped}\"]");
    assert_eq!(wire, expected);
    // Round-trips back to the same bytes.
    let back = parse(&wire, None).unwrap();
    assert_eq!(get(&back, "a"), s(&text));
}

/// Non-ASCII and astral characters pass through as UTF-8 and round-trip.
#[test]
fn unicode_passes_through() {
    let o = obj(vec![("a", s("héllo 😀 é"))]);
    let wire = stringify(&o, None, None);
    assert_eq!(wire, "[{\"a\":\"1\"},\"héllo 😀 é\"]");
    let back = parse(&wire, None).unwrap();
    assert_eq!(get(&back, "a"), s("héllo 😀 é"));
}

/// An empty string value is hoisted as a normal string entry and round-trips.
#[test]
fn empty_string_value() {
    let o = obj(vec![("a", s(""))]);
    let wire = stringify(&o, None, None);
    assert_eq!(wire, r#"[{"a":"1"},""]"#);
    let back = parse(&wire, None).unwrap();
    assert_eq!(get(&back, "a"), s(""));
}

/// A literal string that equals a live pointer index stays a literal. Position
/// decides pointer versus literal, not the characters. Here `"2"` is a real
/// pointer to the inner object and `"0"` is a literal value.
#[test]
fn literal_string_beside_live_pointer() {
    // o = {a:"0", b:{}}; o.b.self = o.b
    let inner = obj(vec![]);
    set(&inner, "self", inner.clone());
    let o = obj(vec![("a", s("0")), ("b", inner.clone())]);

    let wire = stringify(&o, None, None);
    assert_eq!(wire, r#"[{"a":"1","b":"2"},"0",{"self":"2"}]"#);

    let back = parse(&wire, None).unwrap();
    assert_eq!(get(&back, "a"), s("0"));
    let b = get(&back, "b");
    assert!(get(&b, "self").ptr_eq(&b));
}

/// Duplicate keys in parse input keep the last value, matching `JSON.parse`.
#[test]
fn duplicate_keys_keep_last() {
    let back = parse(r#"[{"a":1,"a":2}]"#, None).unwrap();
    assert_eq!(get(&back, "a"), n(2));
}

/// An allow-list keeps array elements when the array itself is kept.
#[test]
fn allowlist_keeps_array_elements() {
    let o = obj(vec![("a", arr(vec![n(1), n(2), n(3)])), ("b", n(5))]);
    let replacer = Replacer::Allowlist(vec!["a".to_string()]);
    assert_eq!(
        stringify(&o, Some(&replacer), None),
        r#"[{"a":"1"},[1,2,3]]"#
    );
}
