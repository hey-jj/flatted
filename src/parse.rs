//! Rebuild a value graph from flatted text.
//!
//! Parsing decodes the flat table, then resolves index pointers back into
//! references. Every string nested inside a table node is a pointer into the
//! table. Strings that sit as top-level table entries are literal values. That
//! split, by position not by content, is how a literal `"3"` stays a string
//! while an index `"3"` becomes a reference. Resolution runs through a queue,
//! so deep graphs do not overflow the stack.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::json::{self, ParseError};
use crate::value::{Object, Value};

/// Transforms each `(key, value)` during parse, like the `JSON.parse` reviver.
///
/// The root is visited last with key `""`. Members are visited with their
/// property name or array index. The returned value replaces the original.
pub type Reviver<'a> = &'a dyn Fn(&str, Value) -> Value;

/// Convert flatted text back into a value graph.
///
/// Shared references and cycles are restored. Two pointers to the same table
/// index resolve to the same shared node, so [`Value::ptr_eq`] holds across
/// them.
///
/// `reviver` runs over each resolved value like the `JSON.parse` reviver.
///
/// Returns an error when the text is not valid JSON, when the root is not an
/// array, or when an index points outside the table.
///
/// ```
/// use flatted::{parse, Value};
/// let v = parse("[[\"0\"]]", None).unwrap();
/// // The single array holds itself.
/// if let Value::Array(rc) = &v {
///     assert!(rc.borrow()[0].ptr_eq(&v));
/// }
/// ```
pub fn parse(text: &str, reviver: Option<Reviver>) -> Result<Value, ParseError> {
    let decoded = json::read(text)?;
    let input = match &decoded {
        Value::Array(rc) => rc.borrow().clone(),
        _ => {
            return Err(ParseError {
                message: "flatted text must be a JSON array".to_string(),
                position: 0,
            })
        }
    };

    // The root sits at index 0. An empty table has no root.
    let root = match input.first() {
        Some(v) => v.clone(),
        None => {
            return Ok(call_reviver(reviver, "", Value::Null));
        }
    };

    let resolver = Resolver {
        input,
        parsed: RefCell::new(HashSet::new()),
        reviver,
    };

    let value = if root.is_container() {
        resolver.parsed.borrow_mut().insert(addr(&root));
        let mut lazy: Vec<Deferred> = Vec::new();
        resolver.revive_node(&root, &mut lazy)?;

        let mut i = 0;
        while i < lazy.len() {
            let Deferred { owner, key, target } = lazy[i].clone();
            i += 1;
            resolver.revive_node(&target, &mut lazy)?;
            let revived = call_reviver(reviver, &key, target);
            owner.assign(&key, revived);
        }
        root
    } else {
        root
    };

    Ok(call_reviver(reviver, "", value))
}

/// A slot whose pointer target needs resolving after the current node.
#[derive(Clone)]
struct Deferred {
    owner: Slot,
    key: String,
    target: Value,
}

/// A position inside a parent container, used to write a resolved value back.
#[derive(Clone)]
enum Slot {
    Array(Rc<RefCell<Vec<Value>>>, usize),
    Object(Rc<RefCell<Object>>, String),
}

impl Slot {
    fn assign(&self, _key: &str, value: Value) {
        match self {
            Slot::Array(rc, index) => {
                rc.borrow_mut()[*index] = value;
            }
            Slot::Object(rc, key) => {
                rc.borrow_mut().insert(key.clone(), value);
            }
        }
    }
}

struct Resolver<'a> {
    input: Vec<Value>,
    parsed: RefCell<HashSet<usize>>,
    reviver: Option<Reviver<'a>>,
}

impl<'a> Resolver<'a> {
    /// Resolve every child of one container node.
    ///
    /// A string child is a pointer. Look up its target. A fresh container
    /// target is deferred to the queue and marked, so it is revived once and
    /// shared. A primitive or already-seen target is assigned now through the
    /// reviver. A non-string child is a literal, assigned through the reviver.
    fn revive_node(&self, node: &Value, lazy: &mut Vec<Deferred>) -> Result<(), ParseError> {
        match node {
            Value::Array(rc) => {
                let len = rc.borrow().len();
                for index in 0..len {
                    let key = index.to_string();
                    let child = rc.borrow()[index].clone();
                    let slot = Slot::Array(rc.clone(), index);
                    self.resolve_child(&key, child, slot, lazy)?;
                }
            }
            Value::Object(rc) => {
                let keys: Vec<String> = rc.borrow().keys().cloned().collect();
                for key in keys {
                    let child = rc.borrow().get(&key).cloned().unwrap_or(Value::Null);
                    let slot = Slot::Object(rc.clone(), key.clone());
                    self.resolve_child(&key, child, slot, lazy)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn resolve_child(
        &self,
        key: &str,
        child: Value,
        slot: Slot,
        lazy: &mut Vec<Deferred>,
    ) -> Result<(), ParseError> {
        if let Value::Str(index) = &child {
            let target = self.target(index)?;
            if target.is_container() && self.parsed.borrow_mut().insert(addr(&target)) {
                // Defer this fresh container so resolution stays iterative.
                lazy.push(Deferred {
                    owner: slot,
                    key: key.to_string(),
                    target,
                });
            } else {
                slot.assign(key, call_reviver(self.reviver, key, target));
            }
        } else {
            slot.assign(key, call_reviver(self.reviver, key, child));
        }
        Ok(())
    }

    /// Resolve an index pointer to its table node.
    ///
    /// An index that does not parse or falls outside the table is an error.
    fn target(&self, index: &str) -> Result<Value, ParseError> {
        let i = index.parse::<usize>().map_err(|_| ParseError {
            message: format!("index pointer is not a number: {index:?}"),
            position: 0,
        })?;
        self.input.get(i).cloned().ok_or_else(|| ParseError {
            message: format!("index pointer {i} is out of range"),
            position: 0,
        })
    }
}

/// Run the reviver if present, otherwise return the value unchanged.
fn call_reviver(reviver: Option<Reviver>, key: &str, value: Value) -> Value {
    match reviver {
        Some(f) => f(key, value),
        None => value,
    }
}

/// Stable allocation address of a container, used to mark visited nodes.
fn addr(value: &Value) -> usize {
    match value {
        Value::Array(rc) => Rc::as_ptr(rc) as *const () as usize,
        Value::Object(rc) => Rc::as_ptr(rc) as *const () as usize,
        _ => 0,
    }
}
