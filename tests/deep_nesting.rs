//! Deep graphs round-trip without overflowing the stack. Both stringify and
//! parse run iteratively, so a chain thousands of levels deep is fine.

mod common;
use common::*;

use flatted::{parse, stringify, Value};

const DEPTH: usize = 100_000;

/// Build a chain of nested single-element arrays with a leaf at the bottom,
/// stringify it, parse it back, and walk down to confirm the leaf survives.
#[test]
fn deep_chain_round_trips() {
    // chain = ["leaf"]; repeat chain = [chain] DEPTH times.
    let mut chain = arr(vec![s("leaf")]);
    for _ in 0..DEPTH {
        chain = arr(vec![chain]);
    }

    let text = stringify(&chain, None, None);
    let back = parse(&text, None).unwrap();

    // Walk down DEPTH levels and check the leaf.
    let mut node = back;
    for _ in 0..DEPTH {
        node = match &node {
            Value::Array(rc) => rc.borrow()[0].clone(),
            _ => panic!("expected array while descending"),
        };
    }
    assert_eq!(at(&node, 0), s("leaf"));
}
