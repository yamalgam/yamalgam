#![allow(missing_docs)]

use yamalgam_compare::{CompareResult, SpanSnapshot, TokenSnapshot, compare_token_streams};

#[test]
fn identical_streams_match() {
    let tokens = vec![
        TokenSnapshot {
            kind: "StreamStart".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
        TokenSnapshot {
            kind: "StreamEnd".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
    ];
    let result = compare_token_streams(&tokens, &tokens);
    assert!(matches!(result, CompareResult::Match { token_count: 2 }));
}

#[test]
fn different_values_produce_mismatch() {
    let c_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("foo".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let rust_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("bar".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(
        result,
        CompareResult::TokenMismatch { index: 0, .. }
    ));
}

#[test]
fn different_lengths_produce_mismatch() {
    let c_tokens = vec![
        TokenSnapshot {
            kind: "StreamStart".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
        TokenSnapshot {
            kind: "StreamEnd".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
    ];
    let rust_tokens = vec![TokenSnapshot {
        kind: "StreamStart".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(result, CompareResult::TokenMismatch { .. }));
}

#[test]
fn different_kinds_produce_mismatch() {
    let c_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let rust_tokens = vec![TokenSnapshot {
        kind: "Anchor".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(
        result,
        CompareResult::TokenMismatch { index: 0, .. }
    ));
}
