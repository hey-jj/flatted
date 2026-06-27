//! JSON value model that can hold cycles and shared references.
//!
//! [`Value`] mirrors the JSON data model. Arrays and objects sit behind
//! `Rc<RefCell<..>>` so a graph can point back into itself or share a node
//! across several positions. That sharing is the whole point of this crate:
//! a value graph with cycles survives a round trip and keeps its identity.
//!
//! Two array or object handles are "the same node" when their `Rc` allocations
//! match. Use [`Value::ptr_eq`] to test that. Object keys keep insertion order,
//! which is what JSON text encodes and what the format depends on.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// A JSON number.
///
/// Integers and floats stay distinct so `1` serializes as `1`, never `1.0`.
/// Non-finite floats (`NaN`, infinities) serialize as `null`, matching
/// `JSON.stringify`.
#[derive(Clone, Debug, PartialEq)]
pub enum Number {
    /// A signed integer.
    Int(i64),
    /// An unsigned integer past the `i64` range.
    UInt(u64),
    /// A floating point number.
    Float(f64),
}

impl Number {
    /// Render the number as JSON text.
    ///
    /// Non-finite floats become `null` to match `JSON.stringify`.
    pub fn write(&self, out: &mut String) {
        match self {
            Number::Int(n) => out.push_str(&n.to_string()),
            Number::UInt(n) => out.push_str(&n.to_string()),
            Number::Float(f) => {
                if f.is_finite() {
                    out.push_str(&format_float(*f));
                } else {
                    out.push_str("null");
                }
            }
        }
    }
}

/// Format a finite float the way `JSON.stringify` does for the common cases.
///
/// A float with no fractional part prints without a trailing `.0`, so `2.0`
/// becomes `2`. Everything else uses Rust's shortest round-tripping form.
fn format_float(f: f64) -> String {
    if f == f.trunc() && f.abs() < 1e16 {
        format!("{}", f as i64)
    } else {
        let s = format!("{f}");
        s
    }
}

/// A JSON value that may take part in a cyclic or shared graph.
#[derive(Clone)]
pub enum Value {
    /// JSON `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string.
    Str(String),
    /// An array. Shared so cycles and aliasing work.
    Array(Rc<RefCell<Vec<Value>>>),
    /// An object with insertion-ordered keys. Shared so cycles and aliasing work.
    Object(Rc<RefCell<Object>>),
}

/// An object: keys in insertion order with their values.
///
/// Keys stay unique. Inserting an existing key overwrites its value in place
/// and keeps the original position, matching JSON object semantics.
#[derive(Clone, Default)]
pub struct Object {
    entries: Vec<(String, Value)>,
}

impl Object {
    /// Create an empty object.
    pub fn new() -> Self {
        Object {
            entries: Vec::new(),
        }
    }

    /// Insert or overwrite a key, keeping insertion order.
    pub fn insert(&mut self, key: String, value: Value) {
        for entry in &mut self.entries {
            if entry.0 == key {
                entry.1 = value;
                return;
            }
        }
        self.entries.push((key, value));
    }

    /// Look up a key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the object has no keys.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate keys and values in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// The keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(k, _)| k)
    }
}

impl Value {
    /// Build an empty array value.
    pub fn array(items: Vec<Value>) -> Value {
        Value::Array(Rc::new(RefCell::new(items)))
    }

    /// Build an object value.
    pub fn object(object: Object) -> Value {
        Value::Object(Rc::new(RefCell::new(object)))
    }

    /// True when both values are the same shared array or object node.
    ///
    /// Primitives never compare equal here. They carry no identity.
    pub fn ptr_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// True for arrays and objects, the values that get hoisted into the table.
    pub fn is_container(&self) -> bool {
        matches!(self, Value::Array(_) | Value::Object(_))
    }
}

/// Structural equality. Arrays and objects compare by content.
///
/// This walks the graph, so do not call it on cyclic values. Tests that need
/// to compare cyclic graphs use [`Value::ptr_eq`] on the shared nodes instead.
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => *a.borrow() == *b.borrow(),
            (Value::Object(a), Value::Object(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() {
                    return false;
                }
                let equal = a
                    .iter()
                    .zip(b.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && va == vb);
                equal
            }
            _ => false,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Number(n) => write!(f, "Number({n:?})"),
            Value::Str(s) => write!(f, "Str({s:?})"),
            Value::Array(_) => write!(f, "Array(..)"),
            Value::Object(_) => write!(f, "Object(..)"),
        }
    }
}
