//! Flatten a value graph into flatted text.
//!
//! The algorithm hoists every string, array, and object into one flat table.
//! Each is stored once and replaced, where it appeared, by its table index
//! encoded as a decimal string. A breadth-first loop drains the table, so the
//! work is iterative and deep graphs do not overflow the stack.

use std::collections::HashMap;
use std::rc::Rc;

use crate::json::{self, Indent};
use crate::value::{Object, Value};

/// Controls which values `stringify` keeps and how, like the second argument
/// to `JSON.stringify`.
pub enum Replacer<'a> {
    /// Keep only these property keys. The root key is always kept.
    Allowlist(Vec<String>),
    /// Transform each `(key, value)`. Return `None` to drop the value, the
    /// same as returning `undefined` from a JavaScript replacer.
    Func(&'a dyn Fn(&str, &Value) -> Option<Value>),
}

impl<'a> Replacer<'a> {
    /// Apply the replacer to one `(key, value)` pair.
    fn apply(&self, key: &str, value: &Value) -> Option<Value> {
        match self {
            Replacer::Allowlist(keys) => {
                if key.is_empty() || keys.iter().any(|k| k == key) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            Replacer::Func(f) => f(key, value),
        }
    }
}

/// Indentation for pretty output, like the third argument to `JSON.stringify`.
pub enum Space {
    /// Indent each level by this many spaces.
    Width(usize),
    /// Indent each level by this literal string.
    Str(String),
}

impl Space {
    fn to_indent(&self) -> Indent {
        match self {
            Space::Width(0) => Indent::None,
            Space::Width(n) => Indent::With(" ".repeat(*n)),
            Space::Str(s) if s.is_empty() => Indent::None,
            Space::Str(s) => Indent::With(s.clone()),
        }
    }
}

/// Convert a value graph into flatted text.
///
/// The output is always a JSON array. Index 0 holds the root. Cycles and shared
/// references survive because each node is stored once and referenced by index.
///
/// `replacer` filters or transforms values like the `JSON.stringify` replacer.
/// `space` adds indentation inside each hoisted node like the `space` argument.
///
/// ```
/// use flatted::{stringify, Value};
/// assert_eq!(stringify(&Value::array(vec![]), None, None), "[[]]");
/// ```
pub fn stringify(value: &Value, replacer: Option<&Replacer>, space: Option<&Space>) -> String {
    let indent = space.map(|s| s.to_indent()).unwrap_or(Indent::None);

    let mut state = State {
        replacer,
        input: Vec::new(),
        string_index: HashMap::new(),
        container_index: HashMap::new(),
    };

    // Seed the table with the replaced root at index 0. A replacer that drops
    // the root yields an empty array, the same as JSON.stringify(undefined)
    // returning no text.
    match state.replacer_apply("", value) {
        Some(root) => {
            state.set(root);
        }
        None => return "[]".to_string(),
    }

    let mut output: Vec<String> = Vec::new();
    let mut i = 0;
    while i < state.input.len() {
        let node = state.input[i].clone();
        let replaced = state.flatten_children(&node);
        output.push(json::write(&replaced, &indent));
        i += 1;
    }

    let mut result = String::from("[");
    result.push_str(&output.join(","));
    result.push(']');
    result
}

struct State<'a> {
    replacer: Option<&'a Replacer<'a>>,
    input: Vec<Value>,
    string_index: HashMap<String, String>,
    container_index: HashMap<usize, String>,
}

impl<'a> State<'a> {
    fn replacer_apply(&self, key: &str, value: &Value) -> Option<Value> {
        match self.replacer {
            Some(r) => r.apply(key, value),
            None => Some(value.clone()),
        }
    }

    /// Push a value into the table and record its index. Returns the index text.
    fn set(&mut self, value: Value) -> String {
        let index = self.input.len().to_string();
        match &value {
            Value::Str(s) => {
                self.string_index.insert(s.clone(), index.clone());
            }
            Value::Array(rc) => {
                self.container_index
                    .insert(rc_addr_array(rc), index.clone());
            }
            Value::Object(rc) => {
                self.container_index
                    .insert(rc_addr_object(rc), index.clone());
            }
            _ => {}
        }
        self.input.push(value);
        index
    }

    /// Look up an existing index for a string or container, by value or identity.
    fn known(&self, value: &Value) -> Option<String> {
        match value {
            Value::Str(s) => self.string_index.get(s).cloned(),
            Value::Array(rc) => self.container_index.get(&rc_addr_array(rc)).cloned(),
            Value::Object(rc) => self.container_index.get(&rc_addr_object(rc)).cloned(),
            _ => None,
        }
    }

    /// Replace an already-applied value with its index, hoisting on first sight.
    ///
    /// Strings, arrays, and objects become an index string. Null and other
    /// primitives pass through. This mirrors the fall-through in the source
    /// `replace`: non-null object, array, and string share one hoist path.
    fn relate(&mut self, after: Value) -> Value {
        match &after {
            Value::Null => Value::Null,
            Value::Str(_) | Value::Array(_) | Value::Object(_) => {
                if let Some(index) = self.known(&after) {
                    Value::Str(index)
                } else {
                    Value::Str(self.set(after))
                }
            }
            other => other.clone(),
        }
    }

    /// Build a node's flattened form: its own shell with children replaced by
    /// index strings. The node value itself is not re-hoisted, matching the
    /// `firstRun` skip in the source.
    fn flatten_children(&mut self, node: &Value) -> Value {
        match node {
            Value::Array(rc) => {
                let items = rc.borrow().clone();
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    let key = i.to_string();
                    // A dropped array element becomes null, like JSON.stringify.
                    match self.replacer_apply(&key, item) {
                        None => out.push(Value::Null),
                        Some(after) => out.push(self.relate(after)),
                    }
                }
                Value::array(out)
            }
            Value::Object(rc) => {
                let entries: Vec<(String, Value)> = rc
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let mut out = Object::new();
                for (key, val) in &entries {
                    // A dropped object value omits the key, like JSON.stringify.
                    if let Some(after) = self.replacer_apply(key, val) {
                        out.insert(key.clone(), self.relate(after));
                    }
                }
                Value::object(out)
            }
            other => other.clone(),
        }
    }
}

/// Stable address of an array's shared allocation, for identity dedup.
fn rc_addr_array(rc: &Rc<std::cell::RefCell<Vec<Value>>>) -> usize {
    Rc::as_ptr(rc) as *const () as usize
}

/// Stable address of an object's shared allocation, for identity dedup.
fn rc_addr_object(rc: &Rc<std::cell::RefCell<Object>>) -> usize {
    Rc::as_ptr(rc) as *const () as usize
}
