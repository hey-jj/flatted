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

#[test]
fn reviver_reads_resolved_nested_container() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "a" {
            let copy = get(&value, "b");
            set(&value, "copy", copy);
        }
        value
    };
    let out = parse(r#"[{"a":"1"},{"b":"2"},{"c":"3"},"x"]"#, Some(&reviver)).unwrap();
    assert_eq!(get(&get(&out, "a"), "copy"), obj(vec![("c", s("x"))]));
}

#[test]
fn reviver_keeps_sibling_container_order() {
    let calls = RefCell::new(Vec::new());
    let reviver = |key: &str, value: Value| -> Value {
        calls.borrow_mut().push(key.to_string());
        value
    };
    parse(r#"[{"a":"1","b":"2"},[],[]]"#, Some(&reviver)).unwrap();
    assert_eq!(
        *calls.borrow(),
        vec![String::from("a"), String::from("b"), String::from("")]
    );
}

#[test]
fn reviver_reads_resolved_shared_reference() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "0" {
            let copy = get(&value, "y");
            set(&value, "copy", copy);
        }
        value
    };
    let out = parse(r#"[["1","2"],{"y":"3"},{"x":"1"},{}]"#, Some(&reviver)).unwrap();
    let first = at(&out, 0);
    assert_eq!(get(&first, "copy"), obj(vec![]));
    assert_eq!(get(&get(&at(&out, 1), "x"), "y"), obj(vec![]));
}

#[test]
fn reviver_reads_cycle_after_deep_child_resolves() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "a" {
            let seen = get(&get(&value, "deep"), "z");
            set(&value, "seen", seen);
        }
        value
    };
    let out = parse(
        r#"[{"a":"1"},{"self":"1","deep":"2"},{"z":"3"},"ok"]"#,
        Some(&reviver),
    )
    .unwrap();
    let a = get(&out, "a");
    assert!(get(&a, "self").ptr_eq(&a));
    assert_eq!(get(&a, "seen"), s("ok"));
}

#[test]
fn reviver_reads_child_to_ancestor_cycle_after_later_child_resolves() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "p" {
            let first = get(&value, "first");
            let back = get(&first, "back");
            let later = get(&back, "later");
            set(&value, "seen", get(&later, "z"));
        }
        value
    };
    let out = parse(
        r#"[{"p":"1"},{"first":"2","later":"3"},{"back":"1"},{"z":"4"},"ok"]"#,
        Some(&reviver),
    )
    .unwrap();
    assert_eq!(get(&get(&out, "p"), "seen"), s("ok"));
}

#[test]
fn reviver_keeps_mixed_key_container_order() {
    let calls = RefCell::new(Vec::new());
    let reviver = |key: &str, value: Value| -> Value {
        calls.borrow_mut().push(key.to_string());
        value
    };
    parse(r#"[{"a":"1","b":7},{"c":"2"},"x"]"#, Some(&reviver)).unwrap();
    assert_eq!(
        *calls.borrow(),
        vec![
            String::from("b"),
            String::from("c"),
            String::from("a"),
            String::from("")
        ]
    );
}

#[test]
fn reviver_reads_shared_container_through_side_path() {
    let reviver = |key: &str, value: Value| -> Value {
        if key == "left" {
            let seen = get(&get(&value, "copy"), "leaf");
            set(&value, "seen", seen);
        }
        value
    };
    let out = parse(
        r#"[{"left":"1","right":"2"},{"copy":"3"},{"via":"3"},{"leaf":"4"},"done"]"#,
        Some(&reviver),
    )
    .unwrap();
    assert_eq!(get(&get(&out, "left"), "seen"), s("done"));
    assert_eq!(get(&get(&get(&out, "right"), "via"), "leaf"), s("done"));
}

#[test]
fn reviver_replaces_each_shared_holder() {
    let reviver = |key: &str, value: Value| -> Value {
        match key {
            "a" => s("first"),
            "b" => s("second"),
            _ => value,
        }
    };
    let out = parse(r#"[{"a":"1","b":"1"},{"v":1}]"#, Some(&reviver)).unwrap();
    assert_eq!(get(&out, "a"), s("first"));
    assert_eq!(get(&out, "b"), s("second"));
}
