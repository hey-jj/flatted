//! Shared helpers for the test suite: value builders and a canonical hash.

#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use flatted::{Number, Object, Value};

/// Build a string value.
pub fn s(text: &str) -> Value {
    Value::Str(text.to_string())
}

/// Build an integer number value.
pub fn n(value: i64) -> Value {
    Value::Number(Number::Int(value))
}

/// Build a float number value.
pub fn f(value: f64) -> Value {
    Value::Number(Number::Float(value))
}

/// Build a boolean value.
pub fn b(value: bool) -> Value {
    Value::Bool(value)
}

/// The JSON null value.
pub fn null() -> Value {
    Value::Null
}

/// Build a shared array node from items.
pub fn arr(items: Vec<Value>) -> Value {
    Value::array(items)
}

/// Build a shared object node from key/value pairs, in order.
pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let object: Object = pairs
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    Value::object(object)
}

/// Push a value onto an array node.
pub fn push(array: &Value, value: Value) {
    if let Value::Array(rc) = array {
        rc.borrow_mut().push(value);
    } else {
        panic!("push called on non-array");
    }
}

/// Set a key on an object node.
pub fn set(object: &Value, key: &str, value: Value) {
    if let Value::Object(rc) = object {
        rc.borrow_mut().insert(key.to_string(), value);
    } else {
        panic!("set called on non-object");
    }
}

/// Read an object key, panicking if missing or not an object.
pub fn get(object: &Value, key: &str) -> Value {
    match object {
        Value::Object(rc) => rc
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("missing key {key}")),
        _ => panic!("get called on non-object"),
    }
}

/// Read an array element by index.
pub fn at(array: &Value, index: usize) -> Value {
    match array {
        Value::Array(rc) => rc.borrow()[index].clone(),
        _ => panic!("at called on non-array"),
    }
}

/// A fresh empty array handle, for building cyclic graphs.
pub fn empty_array() -> Rc<RefCell<Vec<Value>>> {
    Rc::new(RefCell::new(Vec::new()))
}

/// A fresh empty object handle, for building cyclic graphs.
pub fn empty_object() -> Rc<RefCell<Object>> {
    Rc::new(RefCell::new(Object::new()))
}

/// Sort object keys recursively and emit JSON the way `jq --sort-keys -r .`
/// does: two-space indent, `": "` after keys, a trailing newline. Used by the
/// golden hash test so output is stable across implementations.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, 0, &mut out);
    out.push('\n');
    out
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_canonical(value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::Str(text) => write_string(text, out),
        Value::Array(rc) => {
            let items = rc.borrow();
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                indent(depth + 1, out);
                write_canonical(item, depth + 1, out);
            }
            out.push('\n');
            indent(depth, out);
            out.push(']');
        }
        Value::Object(rc) => {
            let borrow = rc.borrow();
            if borrow.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut keys: Vec<&String> = borrow.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                indent(depth + 1, out);
                write_string(key, out);
                out.push_str(": ");
                write_canonical(borrow.get(key).unwrap(), depth + 1, out);
            }
            out.push('\n');
            indent(depth, out);
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
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

/// Load a fixture file from `tests/fixtures`.
pub fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// Read plain JSON text into a [`Value`]. Used to load fixtures as ordinary
/// data, not as flatted text.
pub fn read_json(text: &str) -> Value {
    let mut p = JsonReader {
        bytes: text.as_bytes(),
        text,
        pos: 0,
    };
    p.skip_ws();
    let v = p.value();
    p.skip_ws();
    assert_eq!(p.pos, p.bytes.len(), "trailing characters in JSON");
    v
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    pos: usize,
}

impl JsonReader<'_> {
    fn skip_ws(&mut self) {
        while let Some(c) = self.bytes.get(self.pos) {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn value(&mut self) -> Value {
        match self.bytes.get(self.pos) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Value::Str(self.string()),
            Some(b't') => {
                self.pos += 4;
                Value::Bool(true)
            }
            Some(b'f') => {
                self.pos += 5;
                Value::Bool(false)
            }
            Some(b'n') => {
                self.pos += 4;
                Value::Null
            }
            _ => self.number(),
        }
    }

    fn array(&mut self) -> Value {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return arr(items);
        }
        loop {
            self.skip_ws();
            items.push(self.value());
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => panic!("bad array"),
            }
        }
        arr(items)
    }

    fn object(&mut self) -> Value {
        self.pos += 1;
        let mut object = Object::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Value::object(object);
        }
        loop {
            self.skip_ws();
            let key = self.string();
            self.skip_ws();
            assert_eq!(self.bytes.get(self.pos), Some(&b':'));
            self.pos += 1;
            self.skip_ws();
            let value = self.value();
            object.insert(key, value);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => panic!("bad object"),
            }
        }
        Value::object(object)
    }

    fn string(&mut self) -> String {
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.bytes.get(self.pos) {
                Some(b'"') => {
                    self.pos += 1;
                    return out;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let c = self.bytes[self.pos];
                    self.pos += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4();
                            if (0xD800..=0xDBFF).contains(&cp) {
                                self.pos += 2; // skip \u
                                let low = self.hex4();
                                let combined = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                out.push(char::from_u32(combined).unwrap());
                            } else {
                                out.push(char::from_u32(cp).unwrap());
                            }
                        }
                        _ => panic!("bad escape"),
                    }
                }
                Some(_) => {
                    let ch = self.text[self.pos..].chars().next().unwrap();
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
                None => panic!("unterminated string"),
            }
        }
    }

    fn hex4(&mut self) -> u32 {
        let hex = &self.text[self.pos..self.pos + 4];
        self.pos += 4;
        u32::from_str_radix(hex, 16).unwrap()
    }

    fn number(&mut self) -> Value {
        let start = self.pos;
        let mut is_float = false;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        while let Some(c) = self.bytes.get(self.pos) {
            match c {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let slice = &self.text[start..self.pos];
        if is_float {
            Value::Number(Number::Float(slice.parse().unwrap()))
        } else if let Ok(i) = slice.parse::<i64>() {
            Value::Number(Number::Int(i))
        } else if let Ok(u) = slice.parse::<u64>() {
            Value::Number(Number::UInt(u))
        } else {
            Value::Number(Number::Float(slice.parse().unwrap()))
        }
    }
}
