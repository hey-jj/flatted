//! Serialize and parse JSON that contains circular and shared references.
//!
//! Standard JSON cannot represent a value that points back into itself, and a
//! plain serializer throws when it meets one. This crate flattens the value
//! graph into a single JSON array. Every string, array, and object is stored
//! once and replaced, where it appeared, by its index into that array encoded
//! as a decimal string. Parsing resolves those indices back into references, so
//! cycles and shared nodes come back with their identity intact.
//!
//! The output is always a JSON array. It is valid JSON, but it is not
//! interchangeable with the data it encodes. Round-trip only through this
//! crate: [`parse`] after [`stringify`]. Feeding flatted text to a plain JSON
//! reader loses the structure, and feeding plain JSON to [`parse`] has no index
//! table to resolve.
//!
//! # Value model
//!
//! [`Value`] mirrors JSON, with arrays and objects behind shared, mutable
//! handles so a graph can contain cycles. After a round trip, shared nodes
//! compare equal under [`Value::ptr_eq`].
//!
//! # Example
//!
//! ```
//! use flatted::{parse, stringify, Value, Object};
//! use std::rc::Rc;
//! use std::cell::RefCell;
//!
//! // Build an object that holds itself under key "o".
//! let object = Rc::new(RefCell::new(Object::new()));
//! let value = Value::Object(object.clone());
//! object.borrow_mut().insert("o".to_string(), value.clone());
//!
//! let text = stringify(&value, None, None);
//! assert_eq!(text, r#"[{"o":"0"}]"#);
//!
//! let back = parse(&text, None).unwrap();
//! if let Value::Object(rc) = &back {
//!     let inner = rc.borrow().get("o").cloned().unwrap();
//!     assert!(inner.ptr_eq(&back));
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod json;
mod parse;
mod stringify;
mod value;

pub use json::ParseError;
pub use parse::{parse, Reviver};
pub use stringify::{stringify, Replacer, Space};
pub use value::{Number, Object, Value};

/// Flatten a value into the parsed JSON form, keeping recursion.
///
/// This is [`stringify`] followed by a JSON parse of the result. The output is
/// the flat table as plain JSON data: an array of nodes with string indices
/// still in place. Embed it in a host structure and serialize that with any
/// JSON writer without losing the cycles.
///
/// The `Result` exists for type symmetry with [`from_json`]. The error arm is
/// unreachable in practice: `stringify` always emits text that this crate's own
/// reader accepts. The signature keeps a `Result` so callers handle both sides
/// of the bridge the same way.
///
/// ```
/// use flatted::{to_json, Value};
/// let v = Value::array(vec![Value::Str("a".to_string())]);
/// // The flat table parsed back into plain JSON.
/// let table = to_json(&v).unwrap();
/// assert!(matches!(table, Value::Array(_)));
/// ```
pub fn to_json(value: &Value) -> Result<Value, ParseError> {
    json::read(&stringify(value, None, None))
}

/// Rebuild a recursive value from the flat table produced by [`to_json`].
///
/// This serializes the plain table back to JSON text, then runs the full
/// [`parse`] to restore references. It is the inverse of [`to_json`].
///
/// ```
/// use flatted::{from_json, to_json, Value};
/// let v = Value::array(vec![Value::Str("a".to_string())]);
/// let table = to_json(&v).unwrap();
/// let restored = from_json(&table).unwrap();
/// assert_eq!(restored, v);
/// ```
pub fn from_json(value: &Value) -> Result<Value, ParseError> {
    parse(&json::write(value, &json::Indent::None), None)
}
