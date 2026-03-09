#![allow(clippy::bool_assert_comparison, missing_docs)]

use yamalgam_serde::from_str;

#[test]
fn deserialize_bool_true() {
    assert_eq!(from_str::<bool>("true").unwrap(), true);
}

#[test]
fn deserialize_bool_false() {
    assert_eq!(from_str::<bool>("false").unwrap(), false);
}

#[test]
fn deserialize_integer() {
    assert_eq!(from_str::<i64>("42").unwrap(), 42);
    assert_eq!(from_str::<i32>("-7").unwrap(), -7);
    assert_eq!(from_str::<u64>("0").unwrap(), 0);
}

#[test]
fn deserialize_float() {
    assert_eq!(from_str::<f64>("1.5").unwrap(), 1.5);
    assert!(from_str::<f64>(".nan").unwrap().is_nan());
    assert_eq!(from_str::<f64>(".inf").unwrap(), f64::INFINITY);
    assert_eq!(from_str::<f64>("-.inf").unwrap(), f64::NEG_INFINITY);
}

#[test]
fn deserialize_string() {
    assert_eq!(from_str::<String>("hello").unwrap(), "hello");
    assert_eq!(from_str::<String>("\"quoted\"").unwrap(), "quoted");
    assert_eq!(from_str::<String>("'single'").unwrap(), "single");
}

#[test]
fn deserialize_null() {
    assert_eq!(from_str::<()>("~").unwrap(), ());
    assert_eq!(from_str::<()>("null").unwrap(), ());
    assert_eq!(from_str::<()>("").unwrap(), ());
}

#[test]
fn deserialize_option_some() {
    assert_eq!(from_str::<Option<i64>>("42").unwrap(), Some(42));
}

#[test]
fn deserialize_option_none() {
    assert_eq!(from_str::<Option<i64>>("~").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("null").unwrap(), None);
}

#[test]
fn deserialize_quoted_string_not_bool() {
    // Quoted scalars are always strings -- "true" is not a bool
    assert_eq!(from_str::<String>("\"true\"").unwrap(), "true");
    assert_eq!(from_str::<String>("\"42\"").unwrap(), "42");
}

#[test]
fn deserialize_hex_int() {
    assert_eq!(from_str::<i64>("0xFF").unwrap(), 255);
}

#[test]
fn deserialize_octal_int() {
    assert_eq!(from_str::<i64>("0o17").unwrap(), 15);
}
