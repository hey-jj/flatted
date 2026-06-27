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
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

/// A JSON number.
///
/// Integers and floats stay distinct so `1` serializes as `1`, never `1.0`.
/// Non-finite floats (`NaN`, infinities) serialize as `null`, matching
/// `JSON.stringify`.
///
/// Equality is by representation. `Int` and `UInt` never compare equal even at
/// the same value, so `Int(1) != UInt(1)`. The JSON reader picks `Int` for
/// anything that fits `i64` and `UInt` only past that range, so values parsed
/// from text stay on one path and compare as expected. A `UInt(1)` built by
/// hand will not equal `Int(1)`.
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
    /// Render the number as JSON text into a caller buffer.
    ///
    /// Internal writer entry point for the JSON serializer. Non-finite floats
    /// become `null` to match `JSON.stringify`. The public path is [`Display`],
    /// so `to_string()` and `{}` give the same text.
    pub(crate) fn write(&self, out: &mut String) {
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

/// JSON text for the number. Non-finite floats render as `null`, matching
/// `JSON.stringify`. `NaN` is never equal to itself, so two `Number::Float(NaN)`
/// values compare unequal under the derived `PartialEq` even though both print
/// as `null`.
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(&mut out);
        f.write_str(&out)
    }
}

/// Format a finite float the way `JSON.stringify` does.
///
/// This follows the ECMAScript Number-to-string algorithm. A float with no
/// fractional part prints without a trailing `.0`, so `2.0` becomes `2`. Large
/// and small magnitudes use exponent form on the same thresholds JavaScript
/// uses: magnitude `>= 1e21` or `< 1e-6` switches to `e` notation. Everything
/// in between prints as plain decimal.
fn format_float(f: f64) -> String {
    if f == 0.0 {
        return "0".to_string();
    }
    let sign = if f < 0.0 { "-" } else { "" };

    // Rust's `{:e}` gives the shortest round-tripping mantissa and a base-10
    // exponent. Split it into the significant digits and that exponent.
    let sci = format!("{:e}", f.abs());
    let (mantissa, exp) = sci.split_once('e').expect("scientific form has an e");
    let exp: i32 = exp.parse().expect("exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32; // count of significant digits
    let n = exp + 1; // decimal point position: value = digits * 10^(n - k)

    let body = if k <= n && n <= 21 {
        // Integer, pad with trailing zeros.
        let mut s = digits;
        s.push_str(&"0".repeat((n - k) as usize));
        s
    } else if 0 < n && n <= 21 {
        // Decimal point falls inside the digit run.
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        // Small magnitude, lead with zeros after the point.
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        // Exponent form. One digit before the point, then `e`, sign, exponent.
        let e = n - 1;
        let esign = if e >= 0 { "+" } else { "-" };
        let mantissa = if k == 1 {
            digits
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!("{mantissa}e{esign}{}", e.abs())
    };
    format!("{sign}{body}")
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
/// The walk is cycle-safe. It tracks the container pairs it is comparing and
/// treats a repeat pair as equal, so two equal cyclic graphs return `true`
/// instead of recursing forever. Float members follow `f64` rules, so a graph
/// holding `NaN` never equals itself.
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        let mut visiting = HashSet::new();
        eq_rec(self, other, &mut visiting)
    }
}

/// Compare two values, guarding against cycles with a set of container pairs
/// already on the comparison stack. A repeat pair short-circuits to equal.
fn eq_rec(a: &Value, b: &Value, visiting: &mut HashSet<(usize, usize)>) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            let pair = (Rc::as_ptr(x) as usize, Rc::as_ptr(y) as usize);
            if !visiting.insert(pair) {
                return true;
            }
            let result = {
                let xs = x.borrow();
                let ys = y.borrow();
                xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys.iter())
                        .all(|(va, vb)| eq_rec(va, vb, visiting))
            };
            visiting.remove(&pair);
            result
        }
        (Value::Object(x), Value::Object(y)) => {
            let pair = (Rc::as_ptr(x) as usize, Rc::as_ptr(y) as usize);
            if !visiting.insert(pair) {
                return true;
            }
            let result = {
                let xs = x.borrow();
                let ys = y.borrow();
                xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys.iter())
                        .all(|((ka, va), (kb, vb))| ka == kb && eq_rec(va, vb, visiting))
            };
            visiting.remove(&pair);
            result
        }
        _ => false,
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

/// Compact JSON text for a single value, the same bytes `stringify` writes for
/// one node. This is plain JSON, not the flat table. It walks the graph, so do
/// not format a cyclic value with it. Use [`crate::stringify`] for graphs that
/// may contain cycles.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::json::write(self, &crate::json::Indent::None))
    }
}

/// Drop deeply nested graphs without recursing.
///
/// A naive drop walks the tree on the call stack, so a chain thousands of
/// levels deep would overflow. This drains children into a heap worklist and
/// frees them in a loop. When a node is the last owner of a container, its
/// children move onto the list, so freeing stays flat. Shared and cyclic nodes
/// with other owners are left for those owners to free.
impl Drop for Value {
    fn drop(&mut self) {
        let mut stack: Vec<Value> = Vec::new();
        collect_children(self, &mut stack);
        while let Some(mut value) = stack.pop() {
            collect_children(&mut value, &mut stack);
        }
    }
}

/// If `value` is the last owner of a container, move its children onto `stack`
/// and leave the container empty, so the recursive drop has nothing to descend.
fn collect_children(value: &mut Value, stack: &mut Vec<Value>) {
    match value {
        Value::Array(rc) => {
            if let Some(cell) = Rc::get_mut(rc) {
                stack.append(cell.get_mut());
            }
        }
        Value::Object(rc) => {
            if let Some(cell) = Rc::get_mut(rc) {
                let entries = std::mem::take(&mut cell.get_mut().entries);
                stack.extend(entries.into_iter().map(|(_, v)| v));
            }
        }
        _ => {}
    }
}
