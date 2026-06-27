//! Large real-world fixtures parse into a container. The `toolData` field is
//! already a flat table, so feeding it back through the parser rebuilds the
//! object graph.

mod common;
use common::*;

use flatted::{from_json, parse, Value};

/// Pull the `toolData` field out of a fixture and serialize it to JSON text.
///
/// The field is already a flat table, so this text is valid flatted input.
fn tool_data_text(fixture_name: &str) -> String {
    let root = read_json(&fixture(fixture_name));
    let tool_data = get(&root, "toolData");
    let mut out = String::new();
    write_compact(&tool_data, &mut out);
    out
}

fn write_compact(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::Str(text) => {
            out.push('"');
            for c in text.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\u{08}' => out.push_str("\\b"),
                    '\u{09}' => out.push_str("\\t"),
                    '\u{0a}' => out.push_str("\\n"),
                    '\u{0c}' => out.push_str("\\f"),
                    '\u{0d}' => out.push_str("\\r"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
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
                for c in key.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out.push(':');
                write_compact(val, out);
            }
            out.push('}');
        }
    }
}

#[test]
fn tool_data_65515_parses_to_container() {
    let table = tool_data_text("65515.json");
    let result = parse(&table, None).expect("toolData is valid flatted text");
    assert!(result.is_container());
}

#[test]
fn tool_data_65518_parses_to_container() {
    let table = tool_data_text("65518.json");
    let result = parse(&table, None).expect("toolData is valid flatted text");
    assert!(result.is_container());
}

/// The from_json entry point gives the same container.
#[test]
fn from_json_on_tool_data() {
    let table = tool_data_text("65515.json");
    let value = parse(&table, None).unwrap();
    // from_json expects the plain table. Round-trip via to_json to get it.
    let plain = flatted::to_json(&value).unwrap();
    let restored = from_json(&plain).unwrap();
    assert!(restored.is_container());
}
