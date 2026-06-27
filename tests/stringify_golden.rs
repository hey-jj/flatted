//! Exact-string checks for `stringify`. The strings are the wire format and
//! must match byte for byte. Cyclic and shared graphs are built by hand so the
//! reference identity matches what the format encodes.

mod common;
use common::*;

use flatted::{stringify, Replacer, Space, Value};

#[test]
fn multiple_null() {
    assert_eq!(
        stringify(&arr(vec![null(), null()]), None, None),
        "[[null,null]]"
    );
}

#[test]
fn empty_array() {
    assert_eq!(stringify(&arr(vec![]), None, None), "[[]]");
}

#[test]
fn empty_object() {
    assert_eq!(stringify(&obj(vec![]), None, None), "[{}]");
}

#[test]
fn recursive_array() {
    let a = arr(vec![]);
    push(&a, a.clone());
    assert_eq!(stringify(&a, None, None), r#"[["0"]]"#);
}

#[test]
fn recursive_object() {
    let o = obj(vec![]);
    set(&o, "o", o.clone());
    assert_eq!(stringify(&o, None, None), r#"[{"o":"0"}]"#);
}

#[test]
fn values_in_array() {
    // a = [a, 1, 'two', true]
    let a = arr(vec![]);
    push(&a, a.clone());
    push(&a, n(1));
    push(&a, s("two"));
    push(&a, b(true));
    assert_eq!(stringify(&a, None, None), r#"[["0",1,"1",true],"two"]"#);
}

#[test]
fn values_in_object() {
    let o = obj(vec![]);
    set(&o, "o", o.clone());
    set(&o, "one", n(1));
    set(&o, "two", s("two"));
    set(&o, "three", b(true));
    assert_eq!(
        stringify(&o, None, None),
        r#"[{"o":"0","one":1,"two":"1","three":true},"two"]"#
    );
}

#[test]
fn object_in_array() {
    // a = [a, 1, 'two', true, o]; o = {o, one, two, three, a}
    let a = arr(vec![]);
    let o = obj(vec![]);
    push(&a, a.clone());
    push(&a, n(1));
    push(&a, s("two"));
    push(&a, b(true));
    push(&a, o.clone());
    set(&o, "o", o.clone());
    set(&o, "one", n(1));
    set(&o, "two", s("two"));
    set(&o, "three", b(true));
    set(&o, "a", a.clone());
    assert_eq!(
        stringify(&a, None, None),
        r#"[["0",1,"1",true,"2"],"two",{"o":"2","one":1,"two":"1","three":true,"a":"0"}]"#
    );
}

#[test]
fn array_in_object() {
    let a = arr(vec![]);
    let o = obj(vec![]);
    push(&a, a.clone());
    push(&a, n(1));
    push(&a, s("two"));
    push(&a, b(true));
    push(&a, o.clone());
    set(&o, "o", o.clone());
    set(&o, "one", n(1));
    set(&o, "two", s("two"));
    set(&o, "three", b(true));
    set(&o, "a", a.clone());
    assert_eq!(
        stringify(&o, None, None),
        r#"[{"o":"0","one":1,"two":"1","three":true,"a":"2"},"two",["2",1,"1",true,"0"]]"#
    );
}

/// Two distinct `{test:'OK'}` objects and two distinct `[1,2,3]` arrays. They
/// are separate references, so the table keeps them separate. The string `"OK"`
/// is shared and stored once.
#[test]
fn objects_in_array() {
    let a = arr(vec![]);
    let o = obj(vec![]);
    push(&a, a.clone());
    push(&a, n(1));
    push(&a, s("two"));
    push(&a, b(true));
    push(&a, o.clone());
    set(&o, "o", o.clone());
    set(&o, "one", n(1));
    set(&o, "two", s("two"));
    set(&o, "three", b(true));
    set(&o, "a", a.clone());

    push(&a, obj(vec![("test", s("OK"))]));
    push(&a, arr(vec![n(1), n(2), n(3)]));
    set(&o, "test", obj(vec![("test", s("OK"))]));
    set(&o, "array", arr(vec![n(1), n(2), n(3)]));

    assert_eq!(
        stringify(&a, None, None),
        r#"[["0",1,"1",true,"2","3","4"],"two",{"o":"2","one":1,"two":"1","three":true,"a":"0","test":"5","array":"6"},{"test":"7"},[1,2,3],{"test":"7"},[1,2,3],"OK"]"#
    );
}

#[test]
fn objects_in_object() {
    let a = arr(vec![]);
    let o = obj(vec![]);
    push(&a, a.clone());
    push(&a, n(1));
    push(&a, s("two"));
    push(&a, b(true));
    push(&a, o.clone());
    set(&o, "o", o.clone());
    set(&o, "one", n(1));
    set(&o, "two", s("two"));
    set(&o, "three", b(true));
    set(&o, "a", a.clone());

    push(&a, obj(vec![("test", s("OK"))]));
    push(&a, arr(vec![n(1), n(2), n(3)]));
    set(&o, "test", obj(vec![("test", s("OK"))]));
    set(&o, "array", arr(vec![n(1), n(2), n(3)]));

    assert_eq!(
        stringify(&o, None, None),
        r#"[{"o":"0","one":1,"two":"1","three":true,"a":"2","test":"3","array":"4"},"two",["2",1,"1",true,"0","5","6"],{"test":"7"},[1,2,3],{"test":"7"},[1,2,3],"OK"]"#
    );
}

/// The circular example from the format notes.
#[test]
fn circular_reference_string() {
    // a = [{}]; a[0].a = a; a.push(a)
    let a = arr(vec![]);
    let inner = obj(vec![]);
    push(&a, inner.clone());
    set(&inner, "a", a.clone());
    push(&a, a.clone());
    assert_eq!(stringify(&a, None, None), r#"[["1","0"],{"a":"0"}]"#);
}

/// Prototype-pollution guard. Only own properties exist in this model, so the
/// shared `item` is stored once at index 5 and referenced by both `one` and
/// `many[0]`.
#[test]
fn shared_item_string() {
    let item = obj(vec![("name", s("TEST"))]);
    set(&item, "value", item.clone());
    let inner = obj(vec![
        ("a", s("b")),
        ("c", s("d")),
        ("one", item.clone()),
        ("many", arr(vec![item.clone()])),
        ("e", s("f")),
    ]);
    let original = obj(vec![("outer", arr(vec![inner]))]);
    assert_eq!(
        stringify(&original, None, None),
        r#"[{"outer":"1"},["2"],{"a":"3","c":"4","one":"5","many":"6","e":"7"},"b","d",{"name":"8","value":"5"},["5"],"f","TEST"]"#
    );
}

/// A `unique` object shared across several positions, plus the string `"sup"`.
#[test]
fn nested_shared_string() {
    let unique = obj(vec![("a", s("sup"))]);
    let nested = obj(vec![
        ("prop", obj(vec![("value", n(123))])),
        (
            "a",
            arr(vec![
                obj(vec![]),
                obj(vec![(
                    "b",
                    arr(vec![obj(vec![
                        ("a", n(1)),
                        ("d", n(2)),
                        ("c", unique.clone()),
                        (
                            "z",
                            obj(vec![
                                ("g", n(2)),
                                ("a", unique.clone()),
                                (
                                    "b",
                                    obj(vec![("r", n(4)), ("u", unique.clone()), ("c", n(5))]),
                                ),
                                ("f", n(6)),
                            ]),
                        ),
                        ("h", n(1)),
                    ])]),
                )]),
            ]),
        ),
        (
            "b",
            obj(vec![("e", s("f")), ("t", unique.clone()), ("p", n(4))]),
        ),
    ]);
    assert_eq!(
        stringify(&nested, None, None),
        r#"[{"prop":"1","a":"2","b":"3"},{"value":123},["4","5"],{"e":"6","t":"7","p":4},{},{"b":"8"},"f",{"a":"9"},["10"],"sup",{"a":1,"d":2,"c":"7","z":"11","h":1},{"g":2,"a":"7","b":"12","f":6},{"r":4,"u":"7","c":5}]"#
    );
}

#[test]
fn tilde_value_string() {
    let o = obj(vec![("bar", s("something ~ baz"))]);
    assert_eq!(
        stringify(&o, None, None),
        r#"[{"bar":"1"},"something ~ baz"]"#
    );
}

/// A graph with several self-referential sub-objects.
#[test]
fn deep_self_ref_string() {
    let o = obj(vec![]);
    set(&o, "a", obj(vec![("aa", obj(vec![("aaa", s("value1"))]))]));
    set(&o, "b", o.clone());
    let c = obj(vec![
        ("ca", obj(vec![])),
        ("cb", obj(vec![])),
        ("cc", obj(vec![])),
        ("cd", obj(vec![])),
        ("ce", s("value2")),
        ("cf", s("value3")),
    ]);
    set(&o, "c", c.clone());
    let ca = get(&c, "ca");
    let cb = get(&c, "cb");
    let cc = get(&c, "cc");
    let cd = get(&c, "cd");
    set(&ca, "caa", ca.clone());
    set(&cb, "cba", cb.clone());
    set(&cc, "cca", c.clone());
    set(&cd, "cda", ca.clone());
    assert_eq!(
        stringify(&o, None, None),
        r#"[{"a":"1","b":"0","c":"2"},{"aa":"3"},{"ca":"4","cb":"5","cc":"6","cd":"7","ce":"8","cf":"9"},{"aaa":"10"},{"caa":"4"},{"cba":"5"},{"cca":"2"},{"cda":"4"},"value2","value3","value1"]"#
    );
}

#[test]
fn indentation_matches_json() {
    // stringify({a:[1]}, null, '  ')
    let o = obj(vec![("a", arr(vec![n(1)]))]);
    let expected = "[{\n  \"a\": \"1\"\n},[\n  1\n]]";
    assert_eq!(
        stringify(&o, None, Some(&Space::Str("  ".to_string()))),
        expected
    );
}

#[test]
fn indentation_numeric_space() {
    // space = 2, single object holding an array of one number
    let o = obj(vec![("a", n(1))]);
    let expected = "[{\n  \"a\": 1\n}]";
    assert_eq!(stringify(&o, None, Some(&Space::Width(2))), expected);
}

#[test]
fn allowlist_replacer() {
    // stringify({a:1, b:{a:1, b:2}}, ['b'])
    let o = obj(vec![
        ("a", n(1)),
        ("b", obj(vec![("a", n(1)), ("b", n(2))])),
    ]);
    let replacer = Replacer::Allowlist(vec!["b".to_string()]);
    assert_eq!(
        stringify(&o, Some(&replacer), None),
        r#"[{"b":"1"},{"b":2}]"#
    );
}

#[test]
fn function_replacer_drops_keys() {
    // stringify(o, (k,v) => (!k || k==='a') ? v : undefined) where o has a,b,c,d
    let o = obj(vec![
        ("a", s("a")),
        ("b", s("b")),
        ("d", obj(vec![("e", n(123))])),
    ]);
    let keep = |key: &str, value: &Value| -> Option<Value> {
        if key.is_empty() || key == "a" {
            Some(value.clone())
        } else {
            None
        }
    };
    let replacer = Replacer::Func(&keep);
    assert_eq!(stringify(&o, Some(&replacer), None), r#"[{"a":"1"},"a"]"#);
}

#[test]
fn function_replacer_keeps_self_ref() {
    // o.a = o; o.b = o; replacer keeps only 'a' (and root)
    let o = obj(vec![]);
    set(&o, "a", o.clone());
    set(&o, "b", o.clone());
    let keep = |key: &str, value: &Value| -> Option<Value> {
        if key.is_empty() || key == "a" {
            Some(value.clone())
        } else {
            None
        }
    };
    let replacer = Replacer::Func(&keep);
    assert_eq!(stringify(&o, Some(&replacer), None), r#"[{"a":"0"}]"#);
}

/// Repeated equal strings dedup to one table entry.
#[test]
fn dedup_repeated_strings() {
    let a = arr(vec![s("x"), s("x"), s("x")]);
    assert_eq!(stringify(&a, None, None), r#"[["1","1","1"],"x"]"#);
}

/// Primitive roots each become a one-element array.
#[test]
fn primitive_roots() {
    assert_eq!(stringify(&s("a"), None, None), r#"["a"]"#);
    assert_eq!(stringify(&n(1), None, None), "[1]");
    assert_eq!(stringify(&null(), None, None), "[null]");
    assert_eq!(stringify(&b(true), None, None), "[true]");
}
