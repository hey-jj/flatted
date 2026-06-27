//! Parse restores cycles and shared references with identity intact. These
//! cases mirror the round-trip assertions in the source test suite.

mod common;
use common::*;

use flatted::{parse, stringify, Value};

/// Helper: assert two values are the same shared node.
fn same(a: &Value, b: &Value) {
    assert!(a.ptr_eq(b), "expected the same shared node");
}

#[test]
fn restore_recursive_array() {
    let a = arr(vec![]);
    push(&a, a.clone());
    let text = stringify(&a, None, None);
    let back = parse(&text, None).unwrap();
    match &back {
        Value::Array(rc) => same(&rc.borrow()[0], &back),
        _ => panic!("expected array"),
    }
}

#[test]
fn restore_recursive_object() {
    let o = obj(vec![]);
    set(&o, "o", o.clone());
    let back = parse(&stringify(&o, None, None), None).unwrap();
    same(&get(&back, "o"), &back);
}

#[test]
fn array_values_round_trip() {
    // a = [a, 1, 'two', true, o, {test:'OK'}, [1,2,3]]
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

    let back = parse(&stringify(&a, None, None), None).unwrap();
    assert_eq!(at(&back, 1), n(1));
    assert_eq!(at(&back, 2), s("two"));
    assert_eq!(at(&back, 3), b(true));
    assert!(at(&back, 4).is_container());
    assert_eq!(at(&back, 5), obj(vec![("test", s("OK"))]));
    assert_eq!(at(&back, 6), arr(vec![n(1), n(2), n(3)]));

    // a[4] === a[4].o && a === a[4].o.a
    let a4 = at(&back, 4);
    same(&a4, &get(&a4, "o"));
    same(&back, &get(&get(&a4, "o"), "a"));
}

#[test]
fn object_values_round_trip() {
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
    set(&o, "test", obj(vec![("test", s("OK"))]));
    set(&o, "array", arr(vec![n(1), n(2), n(3)]));

    let back = parse(&stringify(&o, None, None), None).unwrap();
    assert_eq!(get(&back, "one"), n(1));
    assert_eq!(get(&back, "two"), s("two"));
    assert_eq!(get(&back, "three"), b(true));
    assert!(get(&back, "a").is_container());
    assert_eq!(get(&back, "test"), obj(vec![("test", s("OK"))]));
    assert_eq!(get(&back, "array"), arr(vec![n(1), n(2), n(3)]));

    // o.a === o.a[0] && o === o.a[4]
    let oa = get(&back, "a");
    same(&oa, &at(&oa, 0));
    same(&back, &at(&oa, 4));
}

/// A six-way shared-reference graph re-linked after a round trip.
#[test]
fn recreated_structure() {
    let o = obj(vec![]);
    set(&o, "a", o.clone());
    set(&o, "c", obj(vec![]));
    let d = obj(vec![("a", n(123)), ("b", o.clone())]);
    set(&o, "d", d.clone());
    let c = get(&o, "c");
    set(&c, "e", o.clone());
    set(&c, "f", d.clone());
    set(&o, "b", c.clone());

    let back = parse(&stringify(&o, None, None), None).unwrap();
    same(&get(&back, "b"), &get(&back, "c"));
    same(&get(&get(&back, "c"), "e"), &back);
    assert_eq!(get(&get(&back, "d"), "a"), n(123));
    same(&get(&get(&back, "d"), "b"), &back);
    same(&get(&get(&back, "c"), "f"), &get(&back, "d"));
}

/// The `unique` object is shared at three sites and the same node after parse.
#[test]
fn nested_shared_identity() {
    let text = r#"[{"prop":"1","a":"2","b":"3"},{"value":123},["4","5"],{"e":"6","t":"7","p":4},{},{"b":"8"},"f",{"a":"9"},["10"],"sup",{"a":1,"d":2,"c":"7","z":"11","h":1},{"g":2,"a":"7","b":"12","f":6},{"r":4,"u":"7","c":5}]"#;
    let out = parse(text, None).unwrap();
    // output.b.t.a === 'sup'
    assert_eq!(get(&get(&get(&out, "b"), "t"), "a"), s("sup"));
    // output.a[1].b[0].c === output.b.t
    let left = get(&at(&get(&at(&get(&out, "a"), 1), "b"), 0), "c");
    let right = get(&get(&out, "b"), "t");
    same(&left, &right);
}

/// Several self-referential sub-objects, all restored.
#[test]
fn deep_self_ref_identity() {
    let text = r#"[{"a":"1","b":"0","c":"2"},{"aa":"3"},{"ca":"4","cb":"5","cc":"6","cd":"7","ce":"8","cf":"9"},{"aaa":"10"},{"caa":"4"},{"cba":"5"},{"cca":"2"},{"cda":"4"},"value2","value3","value1"]"#;
    let oo = parse(text, None).unwrap();
    assert_eq!(get(&get(&get(&oo, "a"), "aa"), "aaa"), s("value1"));
    same(&oo, &get(&oo, "b"));
    let c = get(&oo, "c");
    let ca = get(&c, "ca");
    same(&get(&ca, "caa"), &ca);
    let cb = get(&c, "cb");
    same(&get(&cb, "cba"), &cb);
    same(&get(&get(&c, "cc"), "cca"), &c);
    same(&get(&get(&c, "cd"), "cda"), &get(&ca, "caa"));
    assert_eq!(get(&c, "ce"), s("value2"));
    assert_eq!(get(&c, "cf"), s("value3"));
}

/// Arrays that reference a parent and a sibling.
#[test]
fn arrays_referencing_parents() {
    // original.a1.a2[0] = original.a1; original.a4[0] = original.a1.a3[0]
    let a1 = obj(vec![
        ("a2", arr(vec![null()])),
        ("a3", arr(vec![obj(vec![("name", s("whatever"))])])),
    ]);
    let original = obj(vec![("a1", a1.clone()), ("a4", arr(vec![null()]))]);
    let a2 = get(&a1, "a2");
    if let Value::Array(rc) = &a2 {
        rc.borrow_mut()[0] = a1.clone();
    }
    let a3_first = at(&get(&a1, "a3"), 0);
    let a4 = get(&original, "a4");
    if let Value::Array(rc) = &a4 {
        rc.borrow_mut()[0] = a3_first.clone();
    }

    let restored = parse(&stringify(&original, None, None), None).unwrap();
    let r_a1 = get(&restored, "a1");
    same(&at(&get(&r_a1, "a2"), 0), &r_a1);
    same(&at(&get(&restored, "a4"), 0), &at(&get(&r_a1, "a3"), 0));
}

/// Tilde keys and values used as both keys and values, restored intact.
#[test]
fn tilde_keys_restructured() {
    // o = {a:['~','~~','~~~']}; o.a.push(o); o.o=o; o['~']=o['~~']=o['~~~']=o.a
    let o = obj(vec![("a", arr(vec![s("~"), s("~~"), s("~~~")]))]);
    let a = get(&o, "a");
    push(&a, o.clone());
    set(&o, "o", o.clone());
    set(&o, "~", a.clone());
    set(&o, "~~", a.clone());
    set(&o, "~~~", a.clone());

    let out = parse(&stringify(&o, None, None), None).unwrap();
    same(&out, &at(&get(&out, "a"), 3));
    same(&out, &get(&out, "o"));
    same(&get(&out, "~"), &get(&out, "a"));
    same(&get(&out, "~~"), &get(&out, "a"));
    same(&get(&out, "~~~"), &get(&out, "a"));
    let out_a = get(&out, "a");
    // pop returns o, then the remaining three join to '~~~~~~'
    let popped = if let Value::Array(rc) = &out_a {
        rc.borrow_mut().pop().unwrap()
    } else {
        panic!()
    };
    same(&popped, &out);
    let joined: String = if let Value::Array(rc) = &out_a {
        rc.borrow()
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                _ => panic!("expected strings"),
            })
            .collect()
    } else {
        panic!()
    };
    assert_eq!(joined, "~~~~~~");
}

/// Prototype-pollution guard: shared `item` restored as one node.
#[test]
fn shared_item_identity() {
    let text = r#"[{"outer":"1"},["2"],{"a":"3","c":"4","one":"5","many":"6","e":"7"},"b","d",{"name":"8","value":"5"},["5"],"f","TEST"]"#;
    let out = parse(text, None).unwrap();
    // output.outer[0].many[0] === output.outer[0].one
    let inner = at(&get(&out, "outer"), 0);
    same(&at(&get(&inner, "many"), 0), &get(&inner, "one"));
    assert_eq!(get(&get(&inner, "one"), "name"), s("TEST"));
}

/// An empty-string key used as real data, carrying a cycle.
#[test]
fn empty_key_with_cycle() {
    // a = {b:{'':{c:{d:1}}}}; a._circular = a.b['']
    let inner = obj(vec![("c", obj(vec![("d", n(1))]))]);
    let b = obj(vec![("", inner.clone())]);
    let a = obj(vec![("b", b.clone())]);
    set(&a, "_circular", inner.clone());

    let nosj = parse(&stringify(&a, None, None), None).unwrap();
    let circular = get(&nosj, "_circular");
    let via_b = get(&get(&nosj, "b"), "");
    same(&circular, &via_b);
}
