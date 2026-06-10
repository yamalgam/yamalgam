#![allow(missing_docs)]
//! Behavioral parity with serde_yaml.
//!
//! Tests modeled on serde_yaml's test suite (dual MIT/Apache-2.0) so that
//! migrating code keeps its semantics. Intentional divergences are marked
//! `DIVERGENCE:` with the rationale — each one is a deliberate choice, not
//! an accident.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use yamalgam_serde::{Deserializer, from_str};

// ---------------------------------------------------------------------------
// Scalar typing
// ---------------------------------------------------------------------------

#[test]
fn yaml12_core_bools_only() {
    // YAML 1.2 core schema: only true/false are bools. The 1.1 forms stay
    // strings — same as serde_yaml.
    assert!(from_str::<bool>("true").unwrap());
    assert!(!from_str::<bool>("false").unwrap());
    assert_eq!(from_str::<String>("yes").unwrap(), "yes");
    assert_eq!(from_str::<String>("on").unwrap(), "on");
    assert_eq!(from_str::<String>("y").unwrap(), "y");
}

#[test]
fn special_floats() {
    assert!(from_str::<f64>(".nan").unwrap().is_nan());
    assert!(from_str::<f64>(".NaN").unwrap().is_nan());
    assert_eq!(from_str::<f64>(".inf").unwrap(), f64::INFINITY);
    assert_eq!(from_str::<f64>("-.inf").unwrap(), f64::NEG_INFINITY);
    assert_eq!(from_str::<f64>("+.Inf").unwrap(), f64::INFINITY);
}

#[test]
fn leading_zero_is_decimal() {
    // YAML 1.2 core schema: ints are [-+]?[0-9]+, so leading zeros are
    // decimal (1.1-style bare octal is gone). Same as serde_yaml.
    assert_eq!(from_str::<i64>("0755").unwrap(), 755);
}

#[test]
fn prefixed_radix_integers() {
    assert_eq!(from_str::<i64>("0x1F").unwrap(), 31);
    assert_eq!(from_str::<i64>("0o17").unwrap(), 15);
}

#[test]
fn huge_integer_falls_back_to_string() {
    // Out-of-range integers are not silently truncated.
    let s: String = from_str("99999999999999999999999999").unwrap();
    assert_eq!(s, "99999999999999999999999999");
}

#[test]
fn quoted_scalars_never_resolve() {
    assert_eq!(from_str::<String>("\"true\"").unwrap(), "true");
    assert_eq!(from_str::<String>("'123'").unwrap(), "123");
    assert_eq!(from_str::<String>("\"~\"").unwrap(), "~");
}

#[test]
fn borrowed_strings_zero_copy() {
    let input = "plain scalar text";
    let s: &str = from_str(input).unwrap();
    assert_eq!(s, "plain scalar text");
    // Same pointer region — actually borrowed, not copied.
    assert_eq!(s.as_ptr(), input.as_ptr());
}

// ---------------------------------------------------------------------------
// Null and Option
// ---------------------------------------------------------------------------

#[test]
fn null_forms() {
    assert_eq!(from_str::<Option<i64>>("~").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("null").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("Null").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("NULL").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("").unwrap(), None);
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithOptional {
    required: String,
    optional: Option<u32>,
}

#[test]
fn missing_key_is_none() {
    let v: WithOptional = from_str("required: here").unwrap();
    assert_eq!(v.optional, None);
}

#[test]
fn explicit_null_value_is_none() {
    let v: WithOptional = from_str("required: here\noptional:").unwrap();
    assert_eq!(v.optional, None);
}

// ---------------------------------------------------------------------------
// Mapping key types
// ---------------------------------------------------------------------------

#[test]
fn integer_keys() {
    let v: BTreeMap<i64, String> = from_str("1: one\n2: two\n-3: minus three").unwrap();
    assert_eq!(v[&1], "one");
    assert_eq!(v[&2], "two");
    assert_eq!(v[&-3], "minus three");
}

#[test]
fn bool_keys() {
    let v: BTreeMap<bool, String> = from_str("true: yep\nfalse: nope").unwrap();
    assert_eq!(v[&true], "yep");
    assert_eq!(v[&false], "nope");
}

#[test]
fn duplicate_keys_last_wins() {
    // DIVERGENCE: serde_yaml errors on duplicate mapping keys; yamalgam
    // keeps the last value, matching its Composer (and serde_json).
    let v: HashMap<String, i64> = from_str("k: 1\nk: 2").unwrap();
    assert_eq!(v["k"], 2);
}

// ---------------------------------------------------------------------------
// Block scalars
// ---------------------------------------------------------------------------

#[test]
fn literal_block_scalar() {
    let v: HashMap<String, String> = from_str("text: |\n  line one\n  line two\n").unwrap();
    assert_eq!(v["text"], "line one\nline two\n");
}

#[test]
fn literal_block_scalar_strip_chomping() {
    let v: HashMap<String, String> = from_str("text: |-\n  no trailing newline\n").unwrap();
    assert_eq!(v["text"], "no trailing newline");
}

#[test]
fn folded_block_scalar() {
    let v: HashMap<String, String> = from_str("text: >\n  folded into\n  one line\n").unwrap();
    assert_eq!(v["text"], "folded into one line\n");
}

#[test]
fn keep_chomping_preserves_newlines() {
    let v: HashMap<String, String> = from_str("text: |+\n  kept\n\n").unwrap();
    assert_eq!(v["text"], "kept\n\n");
}

// ---------------------------------------------------------------------------
// Anchors and aliases
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, PartialEq, Clone)]
struct Endpoint {
    host: String,
    port: u16,
}

#[test]
fn anchored_mapping_reused() {
    let input = "default: &default\n  host: localhost\n  port: 5432\nreplica: *default";
    let v: HashMap<String, Endpoint> = from_str(input).unwrap();
    assert_eq!(v["default"], v["replica"]);
}

#[test]
fn anchor_in_sequence() {
    let input = "- &first one\n- two\n- *first";
    let v: Vec<String> = from_str(input).unwrap();
    assert_eq!(v, vec!["one", "two", "one"]);
}

#[test]
fn merge_key_is_literal() {
    // DIVERGENCE: like serde_yaml's streaming Deserializer, `<<` is NOT
    // auto-applied — it surfaces as a literal "<<" key. The Composer DOES
    // apply merge semantics; pick the pipeline that matches your needs.
    use yamalgam_core::Value;
    let input = "base: &b\n  x: 1\nderived:\n  <<: *b\n  y: 2";
    let v: Value = from_str(input).unwrap();
    let derived = v.get("derived").expect("derived key");
    assert_eq!(
        derived.get("<<").and_then(|m| m.get("x")),
        Some(&Value::Integer(1))
    );
    assert_eq!(derived.get("y"), Some(&Value::Integer(2)));
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, PartialEq)]
enum Message {
    Ping,
    Text(String),
    Move { x: i32, y: i32 },
}

#[test]
fn enum_representations() {
    assert_eq!(from_str::<Message>("Ping").unwrap(), Message::Ping);
    assert_eq!(
        from_str::<Message>("Text: hello").unwrap(),
        Message::Text("hello".into())
    );
    assert_eq!(
        from_str::<Message>("Move:\n  x: 1\n  y: -1").unwrap(),
        Message::Move { x: 1, y: -1 }
    );
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum IntOrList {
    Int(i64),
    List(Vec<i64>),
}

#[test]
fn untagged_enum() {
    assert_eq!(from_str::<IntOrList>("3").unwrap(), IntOrList::Int(3));
    assert_eq!(
        from_str::<IntOrList>("[1, 2]").unwrap(),
        IntOrList::List(vec![1, 2])
    );
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum Shape {
    Circle { radius: f64 },
    Square { side: f64 },
}

#[test]
fn internally_tagged_enum() {
    assert_eq!(
        from_str::<Shape>("kind: Circle\nradius: 2.5").unwrap(),
        Shape::Circle { radius: 2.5 }
    );
    assert_eq!(
        from_str::<Shape>("side: 4.0\nkind: Square").unwrap(),
        Shape::Square { side: 4.0 }
    );
}

// ---------------------------------------------------------------------------
// Struct attributes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, PartialEq)]
struct Outer {
    name: String,
    #[serde(flatten)]
    inner: Inner,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Inner {
    a: i64,
    b: i64,
}

#[test]
fn flattened_struct() {
    let v: Outer = from_str("name: test\na: 1\nb: 2").unwrap();
    assert_eq!(
        v,
        Outer {
            name: "test".into(),
            inner: Inner { a: 1, b: 2 }
        }
    );
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Strict {
    only: String,
}

#[test]
fn deny_unknown_fields_rejects() {
    let result = from_str::<Strict>("only: this\nextra: nope");
    assert!(result.is_err());
}

#[derive(Debug, Deserialize, PartialEq)]
struct Renamed {
    #[serde(rename = "kebab-case-key")]
    value: i64,
}

#[test]
fn renamed_field() {
    let v: Renamed = from_str("kebab-case-key: 7").unwrap();
    assert_eq!(v.value, 7);
}

#[test]
fn unknown_fields_skipped_with_nested_collections() {
    // Unknown fields containing deep structure must be skipped cleanly.
    #[derive(Debug, Deserialize, PartialEq)]
    struct Sparse {
        keep: i64,
    }
    let input = "ignore:\n  nested:\n    - 1\n    - {a: [2, 3]}\nkeep: 9\nalso_ignore: [x, y]";
    let v: Sparse = from_str(input).unwrap();
    assert_eq!(v.keep, 9);
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

#[test]
fn empty_input_is_unit() {
    from_str::<()>("").unwrap();
    assert_eq!(from_str::<Option<i64>>("").unwrap(), None);
}

#[test]
fn document_end_marker_ignored() {
    let v: i64 = from_str("42\n...\n").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn multiple_documents_rejected_by_from_str() {
    let err = from_str::<i64>("1\n---\n2").unwrap_err();
    assert!(err.to_string().contains("more than one document"));
}

#[test]
fn documents_iterator_heterogeneous_framing() {
    let input = "%YAML 1.2\n---\n1\n...\n---\n2\n";
    let docs: Vec<i64> = Deserializer::from_str(input)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![1, 2]);
}

// ---------------------------------------------------------------------------
// Errors carry positions
// ---------------------------------------------------------------------------

#[test]
fn type_error_includes_position() {
    let err = from_str::<HashMap<String, i64>>("a: 1\nb: not_a_number").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("line 2"),
        "expected position in error, got: {msg}"
    );
}
