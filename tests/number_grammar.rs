//! The reader enforces the JSON number grammar. Malformed numbers are an error,
//! not a silent accept.

mod common;
use common::*;

use flatted::parse;

#[test]
fn leading_zero_is_rejected() {
    assert!(parse("[01]", None).is_err());
    assert!(parse("[00]", None).is_err());
    assert!(parse("[-01]", None).is_err());
}

#[test]
fn trailing_decimal_point_is_rejected() {
    assert!(parse("[1.]", None).is_err());
}

#[test]
fn decimal_point_without_integer_part_is_rejected() {
    assert!(parse("[.5]", None).is_err());
}

#[test]
fn empty_exponent_is_rejected() {
    assert!(parse("[1e]", None).is_err());
    assert!(parse("[1e+]", None).is_err());
}

/// Well-formed numbers parse. The outer array is the flat table, so index 0 is
/// the root value.
#[test]
fn well_formed_numbers_still_parse() {
    assert_eq!(parse("[0]", None).unwrap(), n(0));
    assert_eq!(parse("[10]", None).unwrap(), n(10));
    assert_eq!(parse("[-5]", None).unwrap(), n(-5));
    assert_eq!(parse("[0.5]", None).unwrap(), f(0.5));
    assert_eq!(parse("[1e3]", None).unwrap(), f(1000.0));
    assert_eq!(parse("[1.5e-3]", None).unwrap(), f(0.0015));
}
