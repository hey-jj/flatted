# flatted

Serialize and parse JSON that contains circular and shared references.

Standard JSON cannot represent a value that points back into itself, and a
plain serializer throws when it meets one. This crate flattens the value graph
into a single JSON array. Every string, array, and object is stored once and
replaced, where it appeared, by its index into that array encoded as a decimal
string. Parsing resolves those indices back into references, so cycles and
shared nodes come back with their identity intact.

The output is valid JSON, but it is not interchangeable with the data it
encodes. Round-trip only through this crate.

## Installation

```toml
[dependencies]
flatted = "0.1"
```

## Usage

Build a value that holds itself, flatten it, and read it back.

```rust
use flatted::{parse, stringify, Object, Value};
use std::cell::RefCell;
use std::rc::Rc;

let object = Rc::new(RefCell::new(Object::new()));
let value = Value::Object(object.clone());
object.borrow_mut().insert("self".to_string(), value.clone());

let text = stringify(&value, None, None);
assert_eq!(text, r#"[{"self":"0"}]"#);

let back = parse(&text, None).unwrap();
if let Value::Object(rc) = &back {
    let inner = rc.borrow().get("self").cloned().unwrap();
    assert!(inner.ptr_eq(&back));
}
```

## API

- `stringify(value, replacer, space)` returns flatted text. `replacer` filters
  or transforms values. `space` adds indentation inside each node.
- `parse(text, reviver)` rebuilds the value graph. `reviver` transforms each
  resolved value. Shared references and cycles are restored.
- `to_json(value)` flattens into plain JSON data, so a host structure can carry
  recursion through a normal serializer.
- `from_json(value)` rebuilds a recursive value from that plain data.

## Value model

`Value` mirrors JSON. Arrays and objects sit behind shared, mutable handles, so
a graph can point back into itself or share a node across positions. Two handles
are the same node when their allocations match. Use `Value::ptr_eq` to test
that. Object keys keep insertion order.

Flatten and parse run iteratively, and deep graphs free without recursing, so a
chain hundreds of thousands of levels deep is safe.

## License

Licensed under the [MIT license](LICENSE).
