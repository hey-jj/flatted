//! The reviver runs over each resolved value. It sees the root with key `""`
//! and members with their property name or index. The returned value replaces
//! the original.

mod common;
use common::*;

use std::cell::RefCell;

use flatted::{parse, Value};

/// Replace the value at key `a` with the string `b`.
#[test]
fn reviver_replaces_member() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "a" {
            s("b")
        } else {
            value
        }
    };
    let out = parse(r#"[{"a":"1"},"a"]"#, Some(&reviver)).unwrap();
    assert_eq!(out, obj(vec![("a", s("b"))]));
}

/// A date-like string is revived into a tagged marker, but only when the key
/// is not the root. This mirrors the `key !== ''` guard in the source test.
#[test]
fn reviver_skips_root_key() {
    let is_date_like = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, ':' | '.' | 'Z' | 'T' | '-'))
    };
    let reviver = |key: &str, value: Value| -> Value {
        if !key.is_empty() {
            if let Value::Str(text) = &value {
                if is_date_like(text) {
                    return obj(vec![("__date", value.clone())]);
                }
            }
        }
        value
    };
    // {sub:{one23:123, date:'2020-01-01T00:00:00.000Z'}}
    let date = "2020-01-01T00:00:00.000Z";
    let input = obj(vec![(
        "sub",
        obj(vec![("one23", n(123)), ("date", s(date))]),
    )]);
    let text = flatted::stringify(&input, None, None);
    let out = parse(&text, Some(&reviver)).unwrap();
    let revived = get(&get(&out, "sub"), "date");
    assert_eq!(revived, obj(vec![("__date", s(date))]));
}

/// The reviver fires on a non-object root with key `""`.
#[test]
fn reviver_fires_on_null_root() {
    let calls = RefCell::new(Vec::new());
    let reviver = |key: &str, value: Value| -> Value {
        calls.borrow_mut().push(key.to_string());
        value
    };
    let out = parse("[null]", Some(&reviver)).unwrap();
    assert_eq!(out, null());
    assert_eq!(*calls.borrow(), vec![String::from("")]);
}

/// The first reviver call is the root with `""`, then members by name.
#[test]
fn reviver_records_key_sequence() {
    let calls = RefCell::new(Vec::new());
    let reviver = |key: &str, value: Value| -> Value {
        calls.borrow_mut().push(key.to_string());
        value
    };
    // [{"a":"1"},"x"] -> member 'a' then root ''
    parse(r#"[{"a":"1"},"x"]"#, Some(&reviver)).unwrap();
    let seq = calls.borrow().clone();
    assert_eq!(seq.first().map(String::as_str), Some("a"));
    assert_eq!(seq.last().map(String::as_str), Some(""));
}

/// A reviver can mutate the root during its visit, adding a self-reference.
#[test]
fn reviver_can_add_self_reference() {
    let reviver = |key: &str, value: Value| -> Value {
        if key.is_empty() {
            if let Value::Object(_) = &value {
                set(&value, "b", value.clone());
            }
        }
        value
    };
    let out = parse(r#"[{"a":"0"}]"#, Some(&reviver)).unwrap();
    assert!(get(&out, "a").ptr_eq(&out));
    assert!(get(&out, "b").ptr_eq(&out));
}
