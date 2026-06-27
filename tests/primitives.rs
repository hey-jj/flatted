//! Primitive roots round-trip through stringify and parse unchanged.

mod common;
use common::*;

use flatted::{parse, stringify};

#[test]
fn number_round_trips() {
    let v = n(1);
    assert_eq!(parse(&stringify(&v, None, None), None).unwrap(), v);
}

#[test]
fn bool_round_trips() {
    let v = b(false);
    assert_eq!(parse(&stringify(&v, None, None), None).unwrap(), v);
}

#[test]
fn null_round_trips() {
    let v = null();
    assert_eq!(parse(&stringify(&v, None, None), None).unwrap(), v);
}

#[test]
fn string_round_trips() {
    let v = s("test");
    assert_eq!(parse(&stringify(&v, None, None), None).unwrap(), v);
}

#[test]
fn float_round_trips() {
    let v = f(3.5);
    assert_eq!(parse(&stringify(&v, None, None), None).unwrap(), v);
}

/// An integer stays an integer in text: 123, not 123.0.
#[test]
fn integer_has_no_decimal_point() {
    assert_eq!(stringify(&n(123), None, None), "[123]");
}
