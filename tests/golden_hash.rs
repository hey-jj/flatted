//! A language-independent golden over a 1.2 MB real-world file. The hashes are
//! taken over key-sorted, compact JSON, so they do not depend on key order or
//! whitespace. Two gates are checked: the fixture is byte-faithful, and a full
//! flatten then parse returns the same normalized content.

mod common;
use common::*;

use sha2::{Digest, Sha256};

use flatted::{parse, stringify, Value};

const ORIGINAL_HASH: &str = "c76a5329a11de440d28f8d8c4b37aafaa61bca9f1eb41a904b3d46312d5ab565";

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Load a fixture as plain JSON data.
fn load_value(name: &str) -> Value {
    read_json(&fixture(name))
}

/// Gate 1: the fixture matches the known content hash, so the test data is the
/// same real-world file the golden was taken over.
#[test]
fn fixture_content_hash_matches() {
    let value = load_value("65518.json");
    let normalized = canonical_json(&value);
    assert_eq!(sha256_hex(&normalized), ORIGINAL_HASH);
}

/// Gate 2: a full flatten then parse returns the same normalized content. This
/// proves round-trip integrity over a large file with shared structure.
#[test]
fn flatten_then_parse_round_trip_hash() {
    let value = load_value("65518.json");
    let before = canonical_json(&value);

    let flat = stringify(&value, None, None);
    let back = parse(&flat, None).expect("flattened text parses");
    let after = canonical_json(&back);

    assert_eq!(after, before);
    assert_eq!(sha256_hex(&after), ORIGINAL_HASH);
}
