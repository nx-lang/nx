# nx-value

`NxValue`: the value a Rust host passes to compiled [NX](https://nxlang.org) and reads back from
it.

It is a JSON-like tree (booleans, integers, floats, strings, lists and records with an optional
type name) and it is the Rust form of an NX canonical value, so a host builds one and passes it as
it is. JSON and MessagePack are encodings of it for a wire or a store, both through `serde`.

```rust
use nx_value::NxValue;

let value = NxValue::from_json_str(r#"{ "$type": "Point", "x": 1, "y": 2.5 }"#).unwrap();
let NxValue::Record { type_name, properties } = &value else { unreachable!() };
assert_eq!(type_name.as_deref(), Some("Point"));
assert_eq!(properties.get("x"), Some(&NxValue::Int(1)));
assert_eq!(value.to_json_string().unwrap(), r#"{"$type":"Point","x":1,"y":2.5}"#);
```

NX has no null: its absent value is the empty list, `NxValue::empty()`. `NxValue::Null` is the
host's spelling of absence, which NX reads as the empty value and writes in the two places the
canonical encoding spells `null`. *Host values* in
[`docs/nx-ir-format.md`](https://github.com/nx-lang/nx/blob/main/docs/nx-ir-format.md) defines the
model.

To run compiled NX with these values, depend on
[`nx-ir-runtime`](https://crates.io/crates/nx-ir-runtime).

The crate is versioned with the NX release it ships in, and its API may change between minor
versions while NX is `0.x`. Depend on it at the same version as `nx-ir-runtime`.
