//! `to_json` and `from_json` bridge to plain JSON so a host structure can carry
//! recursion through a normal serializer.

mod common;
use common::*;

use flatted::{from_json, to_json, Value};

/// Serialize an acyclic plain value to compact JSON, for wire comparison.
fn compact(value: &Value) -> String {
    let mut out = String::new();
    write_compact(value, &mut out);
    out
}

fn write_compact(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => num.write(out),
        Value::Str(text) => {
            out.push('"');
            for c in text.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Value::Array(rc) => {
            out.push('[');
            for (i, item) in rc.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(item, out);
            }
            out.push(']');
        }
        Value::Object(rc) => {
            out.push('{');
            for (i, (key, val)) in rc.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(key);
                out.push('"');
                out.push(':');
                write_compact(val, out);
            }
            out.push('}');
        }
    }
}

/// to_json returns the flat table as plain JSON. Its compact form equals the
/// flatted wire string.
#[test]
fn to_json_matches_wire_string() {
    let v = arr(vec![obj(vec![("a", arr(vec![n(1)]))]), null()]);
    let table = to_json(&v).unwrap();
    assert_eq!(compact(&table), flatted::stringify(&v, None, None));
}

/// The map-entries fixture: to_json of [['test','value']].
#[test]
fn recursive_map_to_json() {
    let entries = arr(vec![arr(vec![s("test"), s("value")])]);
    let table = to_json(&entries).unwrap();
    assert_eq!(compact(&table), r#"[["1"],["2","3"],"test","value"]"#);
}

/// from_json restores the entries from the plain table.
#[test]
fn recursive_map_from_json() {
    let entries = arr(vec![arr(vec![s("test"), s("value")])]);
    let table = to_json(&entries).unwrap();
    let restored = from_json(&table).unwrap();
    assert_eq!(at(&at(&restored, 0), 0), s("test"));
    assert_eq!(at(&at(&restored, 0), 1), s("value"));
}

/// from_json is the inverse of to_json for an acyclic value.
#[test]
fn to_json_then_from_json_is_identity() {
    let v = arr(vec![
        obj(vec![("x", n(1)), ("y", s("hi"))]),
        arr(vec![b(true), null()]),
    ]);
    let table = to_json(&v).unwrap();
    let restored = from_json(&table).unwrap();
    assert_eq!(restored, v);
}

/// A self-referential value survives the to_json / from_json round trip.
#[test]
fn self_reference_survives_round_trip() {
    // same = {}; same.same = same; entries = [['same', same]]
    let same = obj(vec![]);
    set(&same, "same", same.clone());
    let entries = arr(vec![arr(vec![s("same"), same.clone()])]);

    let table = to_json(&entries).unwrap();
    let restored = from_json(&table).unwrap();
    let value = at(&at(&restored, 0), 1);
    assert!(get(&value, "same").ptr_eq(&value));
}
