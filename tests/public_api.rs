//! The collection traits on `Object` and the `Display` paths on `Number` and
//! `Value`.

mod common;
use common::*;

use flatted::{Number, Object, Value};

/// `Number` renders JSON text through `Display`.
#[test]
fn number_display() {
    assert_eq!(Number::Int(123).to_string(), "123");
    assert_eq!(Number::Float(3.5).to_string(), "3.5");
    assert_eq!(Number::Float(1e21).to_string(), "1e+21");
    // Non-finite floats render as null, like JSON.stringify.
    assert_eq!(Number::Float(f64::NAN).to_string(), "null");
    assert_eq!(Number::Float(f64::INFINITY).to_string(), "null");
}

/// `Value` renders compact JSON text through `Display`.
#[test]
fn value_display() {
    let v = obj(vec![("a", n(1)), ("b", arr(vec![s("x"), b(true), null()]))]);
    assert_eq!(v.to_string(), r#"{"a":1,"b":["x",true,null]}"#);
}

/// `Object` collects from an iterator of pairs.
#[test]
fn object_from_iterator() {
    let pairs = vec![
        ("a".to_string(), n(1)),
        ("b".to_string(), n(2)),
        ("a".to_string(), n(3)),
    ];
    let object: Object = pairs.into_iter().collect();
    // Last write wins, first position kept.
    assert_eq!(object.len(), 2);
    assert_eq!(object.keys().collect::<Vec<_>>(), vec!["a", "b"]);
    assert_eq!(object["a"], n(3));
}

/// `&Object` and `Object` both drive a for loop.
#[test]
fn object_into_iterator() {
    let object: Object = vec![("a".to_string(), n(1)), ("b".to_string(), n(2))]
        .into_iter()
        .collect();

    let mut by_ref = Vec::new();
    for (k, v) in &object {
        by_ref.push((k.clone(), v.clone()));
    }
    assert_eq!(
        by_ref,
        vec![("a".to_string(), n(1)), ("b".to_string(), n(2))]
    );

    let by_value: Vec<_> = object.into_iter().collect();
    assert_eq!(
        by_value,
        vec![("a".to_string(), n(1)), ("b".to_string(), n(2))]
    );
}

/// `&mut Object` lets a loop mutate values in place.
#[test]
fn object_iter_mut() {
    let mut object: Object = vec![("a".to_string(), n(1))].into_iter().collect();
    for (_, v) in &mut object {
        *v = n(9);
    }
    assert_eq!(object["a"], n(9));
}

/// `Index` reads a key and `values` lists values in order.
#[test]
fn object_index_and_values() {
    let object: Object = vec![("a".to_string(), n(1)), ("b".to_string(), s("x"))]
        .into_iter()
        .collect();
    assert_eq!(object["a"], n(1));
    assert_eq!(object["b"], s("x"));
    assert_eq!(
        object.values().cloned().collect::<Vec<_>>(),
        vec![n(1), s("x")]
    );
}

/// `get_mut` and `remove` round out the map.
#[test]
fn object_get_mut_and_remove() {
    let mut object: Object = vec![("a".to_string(), n(1)), ("b".to_string(), n(2))]
        .into_iter()
        .collect();
    *object.get_mut("a").unwrap() = n(10);
    assert_eq!(object["a"], n(10));

    let removed = object.remove("a");
    assert_eq!(removed, Some(n(10)));
    assert_eq!(object.get("a"), None);
    // Order of the rest is kept.
    assert_eq!(object.keys().collect::<Vec<_>>(), vec!["b"]);
}

/// Indexing a missing key panics, matching slice indexing.
#[test]
#[should_panic]
fn object_index_missing_panics() {
    let object: Object = vec![("a".to_string(), n(1))].into_iter().collect();
    let _ = &object["missing"];
}

/// A reviver cannot delete. Returning the value unchanged is the only no-op.
/// This documents the gap: there is no "drop" signal.
#[test]
fn reviver_has_no_delete() {
    use flatted::parse;
    let reviver = |_key: &str, value: Value| -> Value { value };
    let out = parse(r#"[{"a":1}]"#, Some(&reviver)).unwrap();
    assert_eq!(get(&out, "a"), n(1));
}
