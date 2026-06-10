#![allow(missing_docs)]
//! Structured errors must survive the erased-serde boundary.
//!
//! `from_str` routes through `erased_serde::deserialize`, which flattens
//! every error to a rendered string. The deserializer stashes the original
//! structured error at its point of origin and the public entry points
//! restore it, so callers can match on `Error` variants and read spans
//! instead of parsing message text.

use serde::Deserialize;
use yamalgam_core::{LoaderConfig, ResourceLimits};
use yamalgam_serde::{Deserializer, Error, from_str, from_str_with_config};

/// Build a config carrying only the given limits.
const fn config_with(limits: ResourceLimits) -> LoaderConfig {
    let mut config = LoaderConfig::trusted();
    config.limits = limits;
    config
}

#[test]
fn from_str_preserves_unexpected_with_span() {
    // deserialize_i64 on a sequence → Unexpected { expected: "scalar" }.
    let err = from_str::<i64>("[1, 2]").unwrap_err();
    match err {
        Error::Unexpected { expected, span, .. } => {
            assert_eq!(expected, "scalar");
            assert!(span.is_some(), "span must survive the erased boundary");
        }
        other => panic!("expected Error::Unexpected, got {other:?}"),
    }
}

#[test]
fn from_str_preserves_parse_error() {
    let err = from_str::<String>("\"unterminated").unwrap_err();
    assert!(matches!(err, Error::Parse(_)), "got {err:?}");
}

#[test]
fn from_str_preserves_undefined_alias() {
    let err = from_str::<Vec<String>>("- *nope").unwrap_err();
    match err {
        Error::UndefinedAlias { name, span } => {
            assert_eq!(name, "nope");
            assert!(span.is_some());
        }
        other => panic!("expected Error::UndefinedAlias, got {other:?}"),
    }
}

#[test]
fn from_str_with_config_preserves_limit_exceeded() {
    let limits = ResourceLimits {
        max_alias_expansions: Some(1),
        ..ResourceLimits::none()
    };
    let input = "base: &b hello\nx: *b\ny: *b";
    let err = from_str_with_config::<std::collections::HashMap<String, String>>(
        input,
        &config_with(limits),
    )
    .unwrap_err();
    assert!(matches!(err, Error::LimitExceeded(_)), "got {err:?}");
}

#[test]
fn more_than_one_document_stays_structured() {
    // check_end errors never cross the erased boundary — guard against
    // the stash machinery degrading them.
    let err = from_str::<i64>("1\n---\n2").unwrap_err();
    assert!(matches!(err, Error::MoreThanOneDocument), "got {err:?}");
}

#[test]
fn documents_iterator_preserves_structured_errors() {
    let mut docs = Deserializer::from_str("---\n42\n---\n*missing").documents::<i64>();
    assert_eq!(docs.next().unwrap().unwrap(), 42);
    let err = docs.next().unwrap().unwrap_err();
    assert!(matches!(err, Error::UndefinedAlias { .. }), "got {err:?}");
}

#[test]
fn untagged_enum_failure_reports_variant_mismatch() {
    // Untagged failures are synthesized inside serde derive code — the
    // stash must not replace them with internal attempt errors.
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    #[expect(dead_code, reason = "variants exist only to drive untagged dispatch")]
    enum IntOrBool {
        Int(i64),
        Bool(bool),
    }
    let err = from_str::<IntOrBool>("neither").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("variant") || msg.contains("untagged"),
        "expected the untagged-enum mismatch message, got: {msg}"
    );
}

#[test]
fn untagged_success_leaves_no_residue() {
    // A succeeding untagged enum must not leave a stale stashed error
    // that pollutes a later failure in the same stream.
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum IntOrString {
        Int(i64),
        Str(String),
    }
    let input = "---\nhello\n---\n- *missing";
    let mut docs = Deserializer::from_str(input).documents::<IntOrString>();
    assert_eq!(
        docs.next().unwrap().unwrap(),
        IntOrString::Str("hello".into())
    );
    let err = docs.next().unwrap().unwrap_err();
    assert!(matches!(err, Error::UndefinedAlias { .. }), "got {err:?}");
}

#[test]
fn rendered_message_unchanged_by_stash() {
    // The restored structured error must render exactly the text callers
    // saw before the stash existed (position included).
    let err = from_str::<std::collections::HashMap<String, i64>>("a: 1\nb: nope").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected integer"), "got: {msg}");
    assert!(msg.contains("line 2"), "got: {msg}");
}
