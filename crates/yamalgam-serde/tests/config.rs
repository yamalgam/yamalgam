#![allow(missing_docs)]

use std::io::Cursor;

use yamalgam_core::{LoaderConfig, TagResolution};
use yamalgam_parser::{ParseError, ResolveError};
use yamalgam_serde::{
    Deserializer, Error, from_reader, from_reader_unchecked, from_str, from_str_unchecked,
    from_str_with_config,
};

fn nested_flow_sequence(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

fn assert_limit_error<T>(result: Result<T, Error>) {
    let err = match result {
        Ok(_) => panic!("expected a resource-limit error"),
        Err(error) => error,
    };
    match err {
        Error::LimitExceeded(_) | Error::Parse(ParseError::LimitExceeded(_)) => {}
        Error::Parse(ParseError::Scan(error))
            if error.message.starts_with("depth ") && error.message.contains("exceeds maximum") => {
        }
        other => panic!("expected a resource-limit error, got {other:?}"),
    }
}

#[test]
fn ordinary_serde_entry_points_use_moderate_limits() {
    let input = nested_flow_sequence(257);

    assert_limit_error(from_str::<serde_json::Value>(&input));

    let result = Deserializer::from_str(&input)
        .documents::<serde_json::Value>()
        .next()
        .expect("one document result");
    assert_limit_error(result);

    assert_limit_error(from_reader::<_, serde_json::Value>(Cursor::new(
        input.as_bytes(),
    )));
}

#[test]
fn unchecked_serde_entry_points_preserve_unlimited_behavior() {
    let input = nested_flow_sequence(257);

    assert!(from_str_unchecked::<serde_json::Value>(&input).is_ok());
    assert!(
        Deserializer::from_str_unchecked(&input)
            .documents::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .is_ok()
    );
    assert!(from_reader_unchecked::<_, serde_json::Value>(Cursor::new(input.as_bytes())).is_ok());
}

#[test]
fn yaml11_tag_resolution_bool_yes() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    let v: bool = from_str_with_config("yes", &config).unwrap();
    assert!(v);
}

#[test]
fn yaml11_tag_resolution_bool_no() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    let v: bool = from_str_with_config("no", &config).unwrap();
    assert!(!v);
}

#[test]
fn yaml11_tag_resolution_bool_on() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    let v: bool = from_str_with_config("on", &config).unwrap();
    assert!(v);
}

#[test]
fn yaml11_tag_resolution_bool_off() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    let v: bool = from_str_with_config("off", &config).unwrap();
    assert!(!v);
}

#[test]
fn yaml12_rejects_yes_as_bool() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml12);
    let result = from_str_with_config::<bool>("yes", &config);
    assert!(result.is_err());
}

#[test]
fn failsafe_keeps_strings() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Failsafe);
    // Failsafe: "true" stays a string, not a bool.
    let v: String = from_str_with_config("true", &config).unwrap();
    assert_eq!(v, "true");
}

#[test]
fn failsafe_integer_stays_string() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Failsafe);
    let v: String = from_str_with_config("42", &config).unwrap();
    assert_eq!(v, "42");
}

#[test]
fn json_schema_strict_bool() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Json);
    let v: bool = from_str_with_config("true", &config).unwrap();
    assert!(v);
    // JSON schema doesn't accept "True" (case-sensitive).
    let result = from_str_with_config::<bool>("True", &config);
    assert!(result.is_err());
}

#[test]
fn strict_limits_depth() {
    let config = LoaderConfig::strict(); // max_depth: 64
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&"  ".repeat(i));
        input.push_str(&format!("k{i}:\n"));
    }
    input.push_str(&"  ".repeat(100));
    input.push_str("val");
    let result = from_str_with_config::<serde_json::Value>(&input, &config);
    assert!(result.is_err());
}

#[test]
fn config_with_documents_iterator() {
    let config = LoaderConfig::moderate();
    let input = "---\n42\n---\n99";
    let docs: Vec<i64> = Deserializer::from_str_with_config(input, &config)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}

#[test]
fn configured_noop_deserializer_rejects_enabled_resolution_policy() {
    let mut config = LoaderConfig::moderate();
    config.resolution.refs.enabled = true;

    assert!(matches!(
        from_str_with_config::<String>("value", &config),
        Err(Error::Resolve(ResolveError::UnsupportedResolutionPolicy {
            include: false,
            refs: true,
        }))
    ));
}

#[test]
fn yaml11_octal_integer() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    // YAML 1.1 octal: 0644 = 420
    let v: i64 = from_str_with_config("0644", &config).unwrap();
    assert_eq!(v, 420);
}

#[test]
fn default_config_uses_yaml12() {
    // Default config = moderate = Yaml12.
    let config = LoaderConfig::default();
    let v: bool = from_str_with_config("true", &config).unwrap();
    assert!(v);
    // "yes" is a string in YAML 1.2.
    let v: String = from_str_with_config("yes", &config).unwrap();
    assert_eq!(v, "yes");
}
