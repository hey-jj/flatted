//! Float wire text matches `JSON.stringify` across magnitudes. The thresholds
//! are the ECMAScript ones: magnitude `>= 1e21` or `< 1e-6` switches to
//! exponent form, everything in between stays plain decimal.

mod common;
use common::*;

use flatted::stringify;

fn wire(value: f64) -> String {
    stringify(&f(value), None, None)
}

#[test]
fn plain_decimal_range() {
    let cases: &[(f64, &str)] = &[
        (0.1, "[0.1]"),
        (2.0, "[2]"),
        (100.0, "[100]"),
        (3.5, "[3.5]"),
        (123456.789, "[123456.789]"),
        (0.000001, "[0.000001]"),
        (999999999999999900000.0, "[999999999999999900000]"),
        (1e20, "[100000000000000000000]"),
        (1e16, "[10000000000000000]"),
    ];
    for (input, expected) in cases {
        assert_eq!(wire(*input), *expected, "input {input}");
    }
}

#[test]
fn exponent_form_large() {
    let cases: &[(f64, &str)] = &[
        (1e21, "[1e+21]"),
        (1e22, "[1e+22]"),
        (1.5e21, "[1.5e+21]"),
        (1e100, "[1e+100]"),
        (1.7976931348623157e308, "[1.7976931348623157e+308]"),
    ];
    for (input, expected) in cases {
        assert_eq!(wire(*input), *expected, "input {input}");
    }
}

#[test]
fn exponent_form_small() {
    let cases: &[(f64, &str)] = &[(1e-7, "[1e-7]"), (1.1e-7, "[1.1e-7]"), (5e-324, "[5e-324]")];
    for (input, expected) in cases {
        assert_eq!(wire(*input), *expected, "input {input}");
    }
}

#[test]
fn negative_floats_match() {
    assert_eq!(wire(-1.5e21), "[-1.5e+21]");
    assert_eq!(wire(-0.000001), "[-0.000001]");
    assert_eq!(wire(-123.45), "[-123.45]");
}

/// Negative zero prints as `0`, matching `JSON.stringify(-0)`.
#[test]
fn negative_zero_is_zero() {
    assert_eq!(wire(-0.0), "[0]");
    assert_eq!(wire(0.0), "[0]");
}

/// Boundaries that must stay plain decimal, one tick below each threshold.
#[test]
fn threshold_boundaries_stay_plain() {
    assert_eq!(wire(1e-6), "[0.000001]");
    assert_eq!(wire(1e20), "[100000000000000000000]");
}
