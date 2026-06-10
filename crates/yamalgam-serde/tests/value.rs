//! Deserializing YAML into yamalgam-core's `Value` through the streaming
//! serde path. This is the bridge the compliance round-trip harness relies
//! on: `from_str::<Value>()` must agree with the Composer.

use yamalgam_core::{Mapping, Value};
use yamalgam_serde::{Deserializer, from_str};

#[test]
fn scalars_into_value() {
    assert_eq!(from_str::<Value>("42").unwrap(), Value::Integer(42));
    assert_eq!(from_str::<Value>("true").unwrap(), Value::Bool(true));
    assert_eq!(from_str::<Value>("~").unwrap(), Value::Null);
    assert_eq!(from_str::<Value>("3.5").unwrap(), Value::Float(3.5));
    assert_eq!(
        from_str::<Value>("hello").unwrap(),
        Value::String("hello".into())
    );
    // Quoted scalars stay strings regardless of content.
    assert_eq!(
        from_str::<Value>("\"42\"").unwrap(),
        Value::String("42".into())
    );
}

#[test]
fn collections_into_value() {
    let v: Value = from_str("items:\n  - 1\n  - two\n  - true").unwrap();
    let expected_items = Value::Sequence(vec![
        Value::Integer(1),
        Value::String("two".into()),
        Value::Bool(true),
    ]);
    assert_eq!(v.get("items"), Some(&expected_items));
}

#[test]
fn non_string_keys_into_value() {
    let v: Value = from_str("1: int key\ntrue: bool key\n~: null key").unwrap();
    let Value::Mapping(m) = &v else {
        panic!("expected mapping, got {v:?}");
    };
    assert_eq!(
        m.get(&Value::Integer(1)),
        Some(&Value::String("int key".into()))
    );
    assert_eq!(
        m.get(&Value::Bool(true)),
        Some(&Value::String("bool key".into()))
    );
    assert_eq!(m.get(&Value::Null), Some(&Value::String("null key".into())));
}

#[test]
fn anchors_into_value() {
    let v: Value = from_str("a: &x 1\nb: *x").unwrap();
    assert_eq!(v.get("a"), Some(&Value::Integer(1)));
    assert_eq!(v.get("b"), Some(&Value::Integer(1)));
}

#[test]
fn nested_mapping_into_value() {
    let v: Value = from_str("outer:\n  inner: 42").unwrap();
    let mut inner = Mapping::new();
    inner.insert(Value::String("inner".into()), Value::Integer(42));
    assert_eq!(v.get("outer"), Some(&Value::Mapping(inner)));
}

#[test]
fn multi_doc_into_values() {
    let docs: Vec<Value> = Deserializer::from_str("---\n1\n---\ntwo")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::Integer(1), Value::String("two".into())]);
}

#[test]
fn empty_input_into_value_is_null() {
    assert_eq!(from_str::<Value>("").unwrap(), Value::Null);
}
