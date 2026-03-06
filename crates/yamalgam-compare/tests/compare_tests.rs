#![allow(missing_docs)]

use yamalgam_compare::{
    CompareResult, SpanSnapshot, TokenSnapshot, compare_token_streams, run_rust_scanner,
};

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

#[test]
fn rust_scanner_produces_stream_markers() {
    let tokens = run_rust_scanner(b"key: value").unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, "StreamStart");
    assert_eq!(tokens[1].kind, "StreamEnd");
    assert!(tokens[0].value.is_none());
    assert!(tokens[1].value.is_none());
}
