#![allow(missing_docs)]

use std::collections::HashMap;

use serde::Deserialize;
use yamalgam_core::{LoaderConfig, ResourceLimits};
use yamalgam_serde::{from_str, from_str_with_config};

/// Build a config carrying only the given limits.
const fn config_with(limits: ResourceLimits) -> LoaderConfig {
    let mut config = LoaderConfig::trusted();
    config.limits = limits;
    config
}

// ---------------------------------------------------------------------------
// Scalar anchors and aliases
// ---------------------------------------------------------------------------

#[test]
fn scalar_anchor_alias() {
    let v: HashMap<String, String> = from_str("a: &val hello\nb: *val").unwrap();
    assert_eq!(v["a"], "hello");
    assert_eq!(v["b"], "hello");
}

#[test]
fn multiple_aliases_same_anchor() {
    let input = "base: &b hello\nx: *b\ny: *b\nz: *b";
    let v: HashMap<String, String> = from_str(input).unwrap();
    assert_eq!(v["base"], "hello");
    assert_eq!(v["x"], "hello");
    assert_eq!(v["y"], "hello");
    assert_eq!(v["z"], "hello");
}

#[test]
fn integer_scalar_anchor_alias() {
    let v: HashMap<String, i64> = from_str("a: &n 42\nb: *n").unwrap();
    assert_eq!(v["a"], 42);
    assert_eq!(v["b"], 42);
}

// ---------------------------------------------------------------------------
// Mapping anchors and aliases
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, PartialEq)]
struct Entry {
    host: String,
    port: u16,
}

#[test]
fn mapping_anchor_alias() {
    let input = "primary: &srv\n  host: db.local\n  port: 5432\nreplica: *srv";
    let v: HashMap<String, Entry> = from_str(input).unwrap();
    assert_eq!(v["primary"], v["replica"]);
    assert_eq!(v["replica"].host, "db.local");
    assert_eq!(v["replica"].port, 5432);
}

// ---------------------------------------------------------------------------
// Sequence anchors and aliases
// ---------------------------------------------------------------------------

#[test]
fn sequence_anchor_alias() {
    let input = "a: &nums\n  - 1\n  - 2\nb: *nums";
    let v: HashMap<String, Vec<i64>> = from_str(input).unwrap();
    assert_eq!(v["a"], vec![1, 2]);
    assert_eq!(v["b"], vec![1, 2]);
}

// ---------------------------------------------------------------------------
// Nested anchors
// ---------------------------------------------------------------------------

#[test]
fn nested_anchor_in_sequence() {
    let input = "- &item\n  name: foo\n  val: 1\n- *item";
    let v: Vec<HashMap<String, serde_json::Value>> = from_str(input).unwrap();
    assert_eq!(v[0], v[1]);
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn undefined_alias_errors() {
    let result = from_str::<HashMap<String, String>>("a: *missing");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("undefined alias"), "got: {err}");
}

// ---------------------------------------------------------------------------
// Resource limits
// ---------------------------------------------------------------------------

#[test]
fn anchor_count_limit_enforced() {
    let limits = ResourceLimits {
        max_anchor_count: Some(1),
        ..ResourceLimits::none()
    };
    // Two anchors should exceed limit of 1.
    let input = "a: &x hello\nb: &y world";
    let result = from_str_with_config::<HashMap<String, String>>(input, &config_with(limits));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("limit exceeded"), "got: {err}");
}

#[test]
fn alias_expansion_limit_enforced() {
    let limits = ResourceLimits {
        max_alias_expansions: Some(1),
        ..ResourceLimits::none()
    };
    // Two alias expansions should exceed limit of 1.
    let input = "base: &b hello\nx: *b\ny: *b";
    let result = from_str_with_config::<HashMap<String, String>>(input, &config_with(limits));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("limit exceeded"), "got: {err}");
}

// ---------------------------------------------------------------------------
// Anchor reuse (later definition wins)
// ---------------------------------------------------------------------------

#[test]
fn anchor_redefinition_uses_latest() {
    let input = "a: &val first\nb: &val second\nc: *val";
    let v: HashMap<String, String> = from_str(input).unwrap();
    assert_eq!(v["a"], "first");
    assert_eq!(v["b"], "second");
    assert_eq!(v["c"], "second");
}

// ---------------------------------------------------------------------------
// Anchors defined inside anchored collections
// ---------------------------------------------------------------------------

#[test]
fn alias_to_scalar_anchor_inside_anchored_collection() {
    // `&inner` lives inside the `&outer` collection. Both the in-collection
    // alias (`c`) and the post-collection alias (`d`) must resolve.
    use yamalgam_core::Value;
    let input = "a: &outer\n  b: &inner 1\n  c: *inner\nd: *inner";
    let v: Value = from_str(input).unwrap();
    assert_eq!(
        v.get("a").and_then(|a| a.get("b")),
        Some(&Value::Integer(1))
    );
    assert_eq!(
        v.get("a").and_then(|a| a.get("c")),
        Some(&Value::Integer(1))
    );
    assert_eq!(v.get("d"), Some(&Value::Integer(1)));
}

#[test]
fn alias_to_outer_collection_containing_inner_anchor() {
    use yamalgam_core::Value;
    let input = "a: &outer\n  b: &inner 1\n  c: *inner\ne: *outer";
    let v: Value = from_str(input).unwrap();
    assert_eq!(v.get("e"), v.get("a"));
    assert!(v.get("e").is_some());
}

#[test]
fn alias_to_collection_anchor_inside_anchored_collection() {
    use yamalgam_core::Value;
    let input = "a: &outer\n  b: &inner\n    c: 1\n  d: *inner\ne: *inner";
    let v: Value = from_str(input).unwrap();
    let expected = v.get("a").and_then(|a| a.get("b")).cloned();
    assert!(expected.is_some());
    assert_eq!(v.get("a").and_then(|a| a.get("d")), expected.as_ref());
    assert_eq!(v.get("e"), expected.as_ref());
}

#[test]
fn self_referential_alias_errors() {
    // `*a` appears before `&a`'s collection is complete — must error like
    // the Composer does, not expand forever.
    use yamalgam_core::Value;
    let result = from_str::<Value>("&a [*a]");
    assert!(result.is_err(), "self-referential alias must be rejected");
}

#[test]
fn forward_alias_inside_anchored_collection_errors() {
    // Alias referencing an anchor that appears later — invalid YAML usage.
    use yamalgam_core::Value;
    let result = from_str::<Value>("a: &outer\n  b: *later\nlater: &later 1");
    assert!(result.is_err(), "forward alias must be rejected");
}
