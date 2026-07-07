//! Rebuild a value graph from flatted text.
//!
//! Parsing decodes the flat table, then resolves index pointers back into
//! references. Every string nested inside a table node is a pointer into the
//! table. Strings that sit as top-level table entries are literal values. That
//! split, by position not by content, is how a literal `"3"` stays a string
//! while an index `"3"` becomes a reference. Resolution runs through a queue,
//! so deep graphs do not overflow the stack. When a reviver is present, parse
//! first builds the graph, then walks that graph and applies the reviver.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::json::{self, ParseError};
use crate::value::{Object, Value};

/// Transforms each `(key, value)` during parse, like the `JSON.parse` reviver.
///
/// The root is visited last with key `""`. Members are visited with their
/// property name or array index. The returned value replaces the original.
/// Shared or cyclic containers are walked once for their descendants. The
/// reviver still runs for each holder, so each holder receives its own
/// replacement.
///
/// The reviver takes its value by move and returns a value, because a parse
/// reviver always produces a replacement. The stringify side uses
/// [`crate::Replacer`], which borrows its value and returns `Option<Value>`, so
/// a replacer can keep a value without cloning it or drop it by returning
/// `None`. The two callbacks differ in shape because they do different jobs.
///
/// Deletion is not modeled. A JavaScript reviver returning `undefined` deletes
/// an object property or nulls an array hole. This signature returns a `Value`,
/// so there is no way to signal "drop this". Return the value unchanged to keep
/// it.
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

    let mut resolver = Resolver {
        input,
        parsed: HashSet::new(),
    };

    let value = resolver.resolve_root(root)?;

    match reviver {
        Some(f) => Ok(revive_graph(f, value)),
        None => Ok(value),
    }
}

/// A slot whose pointer target needs resolving after the current node.
#[derive(Clone)]
struct Deferred {
    owner: Slot,
    target: Value,
}

/// A position inside a parent container, used to write a resolved value back.
#[derive(Clone)]
enum Slot {
    Array(Rc<RefCell<Vec<Value>>>, usize),
    Object(Rc<RefCell<Object>>, String),
}

impl Slot {
    /// Read the current value in this slot.
    fn value(&self) -> Value {
        match self {
            Slot::Array(rc, index) => rc.borrow()[*index].clone(),
            Slot::Object(rc, key) => rc.borrow().get(key).cloned().unwrap_or(Value::Null),
        }
    }

    /// Write a resolved value back into its parent. The slot already holds the
    /// array index or object key, so no key argument is needed.
    fn assign(&self, value: Value) {
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

struct Resolver {
    input: Vec<Value>,
    parsed: HashSet<usize>,
}

impl Resolver {
    fn resolve_root(&mut self, root: Value) -> Result<Value, ParseError> {
        if root.is_container() {
            self.parsed.insert(addr(&root));
            let mut lazy: Vec<Deferred> = Vec::new();
            self.resolve_node(&root, &mut lazy)?;

            let mut i = 0;
            while i < lazy.len() {
                let Deferred { owner, target } = lazy[i].clone();
                i += 1;
                self.resolve_node(&target, &mut lazy)?;
                owner.assign(target);
            }
        }

        Ok(root)
    }

    /// Resolve every child of one container node.
    ///
    /// A string child is a pointer. Look up its target. A fresh container
    /// target is deferred to the queue and marked, so it is resolved once and
    /// shared. A primitive or already-seen target is assigned now. A
    /// non-string child is a literal and stays in place.
    fn resolve_node(&mut self, node: &Value, lazy: &mut Vec<Deferred>) -> Result<(), ParseError> {
        match node {
            Value::Array(rc) => {
                let len = rc.borrow().len();
                for index in 0..len {
                    let child = rc.borrow()[index].clone();
                    let slot = Slot::Array(rc.clone(), index);
                    self.resolve_child(child, slot, lazy)?;
                }
            }
            Value::Object(rc) => {
                let keys: Vec<String> = rc.borrow().keys().cloned().collect();
                for key in keys {
                    let child = rc.borrow().get(&key).cloned().unwrap_or(Value::Null);
                    let slot = Slot::Object(rc.clone(), key);
                    self.resolve_child(child, slot, lazy)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn resolve_child(
        &mut self,
        child: Value,
        slot: Slot,
        lazy: &mut Vec<Deferred>,
    ) -> Result<(), ParseError> {
        if let Value::Str(index) = &child {
            let target = self.target(index)?;
            if target.is_container() && self.parsed.insert(addr(&target)) {
                // Defer this fresh container so resolution stays iterative.
                lazy.push(Deferred {
                    owner: slot,
                    target,
                });
            } else {
                slot.assign(target);
            }
        } else {
            slot.assign(child);
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

#[derive(Clone)]
struct ReviveChild {
    key: String,
    slot: Slot,
    value: Value,
}

struct ReviveFrame {
    containers: Vec<ReviveChild>,
    next: usize,
    after: Option<ReviveChild>,
}

impl ReviveFrame {
    fn new(node: &Value, after: Option<ReviveChild>, reviver: Reviver) -> Self {
        let mut containers = Vec::new();

        match node {
            Value::Array(rc) => {
                let len = rc.borrow().len();
                for index in 0..len {
                    let key = index.to_string();
                    let slot = Slot::Array(rc.clone(), index);
                    collect_revive_child(key, slot, reviver, &mut containers);
                }
            }
            Value::Object(rc) => {
                let keys: Vec<String> = rc.borrow().keys().cloned().collect();
                for key in keys {
                    let slot = Slot::Object(rc.clone(), key.clone());
                    collect_revive_child(key, slot, reviver, &mut containers);
                }
            }
            _ => {}
        }

        ReviveFrame {
            containers,
            next: 0,
            after,
        }
    }

    fn next_container(&mut self) -> Option<ReviveChild> {
        let child = self.containers.get(self.next).cloned();
        self.next += usize::from(child.is_some());
        child
    }
}

fn collect_revive_child(
    key: String,
    slot: Slot,
    reviver: Reviver,
    containers: &mut Vec<ReviveChild>,
) {
    let value = slot.value();
    if value.is_container() {
        containers.push(ReviveChild { key, slot, value });
    } else {
        slot.assign(call_reviver(Some(reviver), &key, value));
    }
}

fn revive_graph(reviver: Reviver, root: Value) -> Value {
    if root.is_container() {
        let mut seen = HashSet::new();
        seen.insert(addr(&root));
        let mut stack = vec![ReviveFrame::new(&root, None, reviver)];

        while !stack.is_empty() {
            let child = stack.last_mut().and_then(ReviveFrame::next_container);
            if let Some(child) = child {
                if seen.insert(addr(&child.value)) {
                    stack.push(ReviveFrame::new(&child.value, Some(child.clone()), reviver));
                } else {
                    let revived = call_reviver(Some(reviver), &child.key, child.value);
                    child.slot.assign(revived);
                }
                continue;
            }

            let frame = stack.pop().expect("stack is not empty");
            if let Some(child) = frame.after {
                let revived = call_reviver(Some(reviver), &child.key, child.value);
                child.slot.assign(revived);
            }
        }
    }

    call_reviver(Some(reviver), "", root)
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
