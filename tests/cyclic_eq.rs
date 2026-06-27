//! Structural equality survives cycles. Two equal self-referential graphs
//! compare equal instead of overflowing the stack.

mod common;
use common::*;

use flatted::Value;

/// Two objects that each hold themselves under the same key are equal.
#[test]
fn equal_self_referential_objects() {
    let a = obj(vec![]);
    set(&a, "self", a.clone());
    let b = obj(vec![]);
    set(&b, "self", b.clone());
    assert_eq!(a, b);
}

/// Two arrays that each hold themselves are equal.
#[test]
fn equal_self_referential_arrays() {
    let a = arr(vec![]);
    push(&a, a.clone());
    let b = arr(vec![]);
    push(&b, b.clone());
    assert_eq!(a, b);
}

/// A two-node cycle compares equal to another two-node cycle of the same shape.
#[test]
fn equal_two_node_cycles() {
    let a1 = obj(vec![]);
    let a2 = obj(vec![]);
    set(&a1, "next", a2.clone());
    set(&a2, "next", a1.clone());

    let b1 = obj(vec![]);
    let b2 = obj(vec![]);
    set(&b1, "next", b2.clone());
    set(&b2, "next", b1.clone());

    assert_eq!(a1, b1);
}

/// Different keys on otherwise-cyclic objects are not equal.
#[test]
fn cyclic_objects_with_different_keys_differ() {
    let a = obj(vec![("tag", s("a"))]);
    set(&a, "self", a.clone());
    let b = obj(vec![("tag", s("b"))]);
    set(&b, "self", b.clone());
    assert_ne!(a, b);
}

/// A graph holding NaN never equals itself, following f64 rules.
#[test]
fn nan_breaks_equality() {
    let a = arr(vec![f(f64::NAN)]);
    let b = arr(vec![f(f64::NAN)]);
    assert_ne!(a, b);
}

/// Acyclic equality still works after the cycle-safe rewrite.
#[test]
fn acyclic_equality_holds() {
    let a = obj(vec![("x", n(1)), ("y", arr(vec![s("z"), b(true)]))]);
    let b = obj(vec![("x", n(1)), ("y", arr(vec![s("z"), b(true)]))]);
    assert_eq!(a, b);
    assert_eq!(Value::Null, Value::Null);
}
