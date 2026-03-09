#![allow(missing_docs)]

use std::collections::HashMap;

use yamalgam_serde::from_str;

#[test]
fn deserialize_sequence() {
    let v: Vec<i64> = from_str("- 1\n- 2\n- 3").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn deserialize_flow_sequence() {
    let v: Vec<String> = from_str("[a, b, c]").unwrap();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn deserialize_mapping() {
    let v: HashMap<String, i64> = from_str("x: 1\ny: 2").unwrap();
    assert_eq!(v["x"], 1);
    assert_eq!(v["y"], 2);
}

#[test]
fn deserialize_flow_mapping() {
    let v: HashMap<String, String> = from_str("{a: b, c: d}").unwrap();
    assert_eq!(v["a"], "b");
    assert_eq!(v["c"], "d");
}

#[test]
fn deserialize_nested_mapping() {
    let v: HashMap<String, HashMap<String, i64>> = from_str("outer:\n  inner: 42").unwrap();
    assert_eq!(v["outer"]["inner"], 42);
}

#[test]
fn deserialize_sequence_of_mappings() {
    let v: Vec<HashMap<String, String>> = from_str("- name: alice\n- name: bob").unwrap();
    assert_eq!(v[0]["name"], "alice");
    assert_eq!(v[1]["name"], "bob");
}

#[test]
fn deserialize_mapping_of_sequences() {
    let v: HashMap<String, Vec<i64>> = from_str("nums:\n  - 1\n  - 2").unwrap();
    assert_eq!(v["nums"], vec![1, 2]);
}

#[test]
fn deserialize_empty_sequence() {
    let v: Vec<i64> = from_str("[]").unwrap();
    assert!(v.is_empty());
}

#[test]
fn deserialize_empty_mapping() {
    let v: HashMap<String, String> = from_str("{}").unwrap();
    assert!(v.is_empty());
}
