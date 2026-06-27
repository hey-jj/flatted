//! A large acyclic fixture round-trips, and a value referenced twice is stored
//! once and restored as one shared node.

mod common;
use common::*;

use flatted::{parse, stringify};

/// 100 flat records round-trip with their content intact.
#[test]
fn data_json_round_trips() {
    let value = read_json(&fixture("data.json"));
    let back = parse(&stringify(&value, None, None), None).unwrap();
    assert_eq!(back, value);
}

/// The same record placed twice is stored once and shared after parse.
#[test]
fn repeated_record_is_shared() {
    let records = read_json(&fixture("data.json"));
    let first = at(&records, 0);
    let pair = arr(vec![first.clone(), first.clone()]);

    let text = stringify(&pair, None, None);
    // The second slot is an index pointer to the first record, not a copy.
    // Both resolve to the same shared node.
    let back = parse(&text, None).unwrap();
    let a = at(&back, 0);
    let b = at(&back, 1);
    assert!(a.ptr_eq(&b));
}
