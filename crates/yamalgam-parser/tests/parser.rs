#![allow(missing_docs)]

use pretty_assertions::assert_eq;
use yamalgam_parser::{CollectionStyle, Event, Parser, ScalarStyle};

#[test]
fn empty_stream() {
    let events: Vec<_> = Parser::new("").collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}

#[test]
fn whitespace_only_stream() {
    let events: Vec<_> = Parser::new("   \n\n  ")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}

#[test]
fn iterator_returns_none_after_stream_end() {
    let mut parser = Parser::new("");
    let e1 = parser.next();
    assert!(e1.is_some());
    let e2 = parser.next();
    assert!(e2.is_some());
    let e3 = parser.next();
    assert!(e3.is_none());
    // Subsequent calls also return None (fused behavior).
    assert!(parser.next().is_none());
}

#[test]
fn iterator_stops_on_error() {
    // Feed a broken token stream: scanner error should propagate.
    use yamalgam_scanner::Token;
    use yamalgam_scanner::scanner::ScanError;

    let tokens: Vec<Result<Token<'_>, ScanError>> = vec![Err(ScanError {
        message: "boom".to_string(),
    })];
    let mut parser = Parser::from_tokens(tokens.into_iter());
    let result = parser.next();
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
    // After error, iterator is done.
    assert!(parser.next().is_none());
}

// -- Document handling tests --

#[test]
fn implicit_document_with_scalar() {
    let events: Vec<_> = Parser::new("hello").collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(events.len(), 5);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(
        events[1],
        Event::DocumentStart { implicit: true, .. }
    ));
    assert!(matches!(events[2], Event::Scalar { .. }));
    assert!(matches!(
        events[3],
        Event::DocumentEnd { implicit: true, .. }
    ));
    assert!(matches!(events[4], Event::StreamEnd));
}

#[test]
fn explicit_document_start() {
    let events: Vec<_> = Parser::new("---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[1],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn explicit_document_end() {
    let events: Vec<_> = Parser::new("hello\n...")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[3],
        Event::DocumentEnd {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn version_directive_as_event() {
    let events: Vec<_> = Parser::new("%YAML 1.2\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[1],
        Event::VersionDirective {
            major: 1,
            minor: 2,
            ..
        }
    ));
    assert!(matches!(
        events[2],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn tag_directive_as_event() {
    let events: Vec<_> = Parser::new("%TAG !e! tag:example.com,2000:\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[1], Event::TagDirective { .. }));
    if let Event::TagDirective {
        ref handle,
        ref prefix,
        ..
    } = events[1]
    {
        assert_eq!(handle.as_ref(), "!e!");
        assert_eq!(prefix.as_ref(), "tag:example.com,2000:");
    }
}

#[test]
fn multi_document() {
    let events: Vec<_> = Parser::new("hello\n---\nworld")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let doc_start_count = events
        .iter()
        .filter(|e| matches!(e, Event::DocumentStart { .. }))
        .count();
    assert_eq!(doc_start_count, 2);
}

#[test]
fn empty_document() {
    let events: Vec<_> = Parser::new("---\n...")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(explicit), Scalar(""), DocEnd(explicit), StreamEnd
    assert_eq!(events.len(), 5);
    assert!(matches!(events[2], Event::Scalar { .. }));
    if let Event::Scalar { ref value, .. } = events[2] {
        assert_eq!(value.as_ref(), "");
    }
}

#[test]
fn multiple_directives() {
    let events: Vec<_> = Parser::new("%YAML 1.2\n%TAG !! tag:yaml.org,2002:\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, VersionDirective, TagDirective, DocStart, Scalar, DocEnd, StreamEnd
    assert_eq!(events.len(), 7);
    assert!(matches!(events[1], Event::VersionDirective { .. }));
    assert!(matches!(events[2], Event::TagDirective { .. }));
    assert!(matches!(
        events[3],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn document_end_then_new_document() {
    let events: Vec<_> = Parser::new("hello\n...\nworld")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(i), Scalar(hello), DocEnd(explicit), DocStart(i), Scalar(world), DocEnd(i), StreamEnd
    assert_eq!(events.len(), 8);
    assert!(matches!(
        events[3],
        Event::DocumentEnd {
            implicit: false,
            ..
        }
    ));
    assert!(matches!(
        events[4],
        Event::DocumentStart { implicit: true, .. }
    ));
}

// -- Scalar with anchor/tag tests --

#[test]
fn plain_scalar() {
    let events: Vec<_> = Parser::new("hello").collect::<Result<Vec<_>, _>>().unwrap();
    if let Event::Scalar {
        ref value,
        style,
        ref anchor,
        ref tag,
        ..
    } = events[2]
    {
        assert_eq!(value.as_ref(), "hello");
        assert_eq!(style, ScalarStyle::Plain);
        assert!(anchor.is_none());
        assert!(tag.is_none());
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn anchored_scalar() {
    let events: Vec<_> = Parser::new("&foo hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar {
        ref anchor,
        ref value,
        ..
    } = events[2]
    {
        assert_eq!(anchor.as_deref(), Some("foo"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn tagged_scalar() {
    let events: Vec<_> = Parser::new("!!str hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar {
        ref tag, ref value, ..
    } = events[2]
    {
        assert_eq!(tag.as_deref(), Some("!!str"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn anchor_and_tag_on_scalar() {
    let events: Vec<_> = Parser::new("&a !!str hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar {
        ref anchor,
        ref tag,
        ref value,
        ..
    } = events[2]
    {
        assert_eq!(anchor.as_deref(), Some("a"));
        assert_eq!(tag.as_deref(), Some("!!str"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn tag_before_anchor_on_scalar() {
    let events: Vec<_> = Parser::new("!!str &a hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar {
        ref anchor,
        ref tag,
        ref value,
        ..
    } = events[2]
    {
        assert_eq!(anchor.as_deref(), Some("a"));
        assert_eq!(tag.as_deref(), Some("!!str"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn quoted_scalar_styles() {
    let single = Parser::new("'hello'")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let double = Parser::new("\"hello\"")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        single[2],
        Event::Scalar {
            style: ScalarStyle::SingleQuoted,
            ..
        }
    ));
    assert!(matches!(
        double[2],
        Event::Scalar {
            style: ScalarStyle::DoubleQuoted,
            ..
        }
    ));
}

#[test]
fn block_scalar_literal() {
    let events: Vec<_> = Parser::new("|\n  hello\n  world")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { style, .. } = events[2] {
        assert_eq!(style, ScalarStyle::Literal);
    } else {
        panic!("expected Scalar");
    }
}

#[test]
fn block_scalar_folded() {
    let events: Vec<_> = Parser::new(">\n  hello\n  world")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { style, .. } = events[2] {
        assert_eq!(style, ScalarStyle::Folded);
    } else {
        panic!("expected Scalar");
    }
}

// -- Alias tests --

#[test]
fn alias_event() {
    let events: Vec<_> = Parser::new("- &anchor hello\n- *anchor")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let alias = events.iter().find(|e| matches!(e, Event::Alias { .. }));
    assert!(alias.is_some(), "expected Alias event in {:?}", events);
    if let Some(Event::Alias { name, .. }) = alias {
        assert_eq!(name.as_ref(), "anchor");
    }
}

// -- Block sequence tests --

#[test]
fn block_sequence() {
    let events: Vec<_> = Parser::new("- a\n- b\n- c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[2],
        Event::SequenceStart {
            style: CollectionStyle::Block,
            ..
        }
    ));
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c"]);
    assert!(matches!(events[6], Event::SequenceEnd { .. }));
}

#[test]
fn nested_block_sequence() {
    let events: Vec<_> = Parser::new("- - a\n  - b\n- c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let seq_starts = events
        .iter()
        .filter(|e| matches!(e, Event::SequenceStart { .. }))
        .count();
    assert_eq!(seq_starts, 2);
}

#[test]
fn anchored_sequence() {
    let events: Vec<_> = Parser::new("&seq\n- a\n- b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::SequenceStart { ref anchor, .. } = events[2] {
        assert_eq!(anchor.as_deref(), Some("seq"));
    } else {
        panic!("expected SequenceStart, got {:?}", events[2]);
    }
}

#[test]
fn empty_block_sequence_entries() {
    let events: Vec<_> = Parser::new("-\n- a\n-")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["", "a", ""]);
}

#[test]
fn tagged_sequence() {
    let events: Vec<_> = Parser::new("!!seq\n- a")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::SequenceStart { ref tag, .. } = events[2] {
        assert_eq!(tag.as_deref(), Some("!!seq"));
    } else {
        panic!("expected SequenceStart, got {:?}", events[2]);
    }
}

#[test]
fn anchor_only_empty_scalar() {
    // Anchor with no content following (document ends) — should emit empty scalar.
    let events: Vec<_> = Parser::new("---\n&a")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar {
        ref anchor,
        ref value,
        ..
    } = events[2]
    {
        assert_eq!(anchor.as_deref(), Some("a"));
        assert_eq!(value.as_ref(), "");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn sequence_event_count() {
    // "- a\n- b" should produce:
    // StreamStart, DocStart(i), SeqStart, Scalar(a), Scalar(b), SeqEnd, DocEnd(i), StreamEnd
    let events: Vec<_> = Parser::new("- a\n- b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 8);
}

// === Block mapping tests ===

#[test]
fn block_mapping() {
    let events: Vec<_> = Parser::new("a: b\nc: d")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[2],
        Event::MappingStart {
            style: CollectionStyle::Block,
            ..
        }
    ));
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c", "d"]);
}

#[test]
fn mapping_with_explicit_key() {
    let events: Vec<_> = Parser::new("? a\n: b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::MappingStart { .. }));
}

#[test]
fn empty_value_in_mapping() {
    let events: Vec<_> = Parser::new("a:\nb: c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "", "b", "c"]);
}

#[test]
fn nested_mapping_in_sequence() {
    let events: Vec<_> = Parser::new("- a: b\n  c: d\n- e")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::MappingStart { .. }))
    );
}

// === Flow sequence tests ===

#[test]
fn flow_sequence() {
    let events: Vec<_> = Parser::new("[a, b, c]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[2],
        Event::SequenceStart {
            style: CollectionStyle::Flow,
            ..
        }
    ));
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c"]);
}

#[test]
fn empty_flow_sequence() {
    let events: Vec<_> = Parser::new("[]").collect::<Result<Vec<_>, _>>().unwrap();
    assert!(matches!(
        events[2],
        Event::SequenceStart {
            style: CollectionStyle::Flow,
            ..
        }
    ));
    assert!(matches!(events[3], Event::SequenceEnd { .. }));
}

#[test]
fn nested_flow_sequences() {
    let events: Vec<_> = Parser::new("[[a, b], [c]]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let seq_starts = events
        .iter()
        .filter(|e| matches!(e, Event::SequenceStart { .. }))
        .count();
    assert_eq!(seq_starts, 3);
}

#[test]
fn flow_sequence_with_implicit_mapping() {
    let events: Vec<_> = Parser::new("[a: b]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::MappingStart { .. }))
    );
}

// === Flow mapping tests ===

#[test]
fn flow_mapping() {
    let events: Vec<_> = Parser::new("{a: b, c: d}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[2],
        Event::MappingStart {
            style: CollectionStyle::Flow,
            ..
        }
    ));
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c", "d"]);
}

#[test]
fn empty_flow_mapping() {
    let events: Vec<_> = Parser::new("{}").collect::<Result<Vec<_>, _>>().unwrap();
    assert!(matches!(
        events[2],
        Event::MappingStart {
            style: CollectionStyle::Flow,
            ..
        }
    ));
    assert!(matches!(events[3], Event::MappingEnd { .. }));
}

#[test]
fn flow_mapping_empty_value() {
    let events: Vec<_> = Parser::new("{a:, b: c}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "", "b", "c"]);
}

#[test]
fn nested_flow_in_block() {
    let events: Vec<_> = Parser::new("key: {a: b}\nother: [1, 2]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::MappingStart {
            style: CollectionStyle::Flow,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::SequenceStart {
            style: CollectionStyle::Flow,
            ..
        }
    )));
}

// === Indentless sequence tests ===

#[test]
fn indentless_sequence_in_mapping() {
    let events: Vec<_> = Parser::new("key:\n- a\n- b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::SequenceStart { .. }))
    );
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["key", "a", "b"]);
}

#[test]
fn indentless_sequence_with_nested_mapping() {
    let events: Vec<_> = Parser::new("key:\n- a: b\n- c: d")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let map_starts = events
        .iter()
        .filter(|e| matches!(e, Event::MappingStart { .. }))
        .count();
    // outer mapping + 2 inner mappings per sequence entry
    assert!(
        map_starts >= 3,
        "expected at least 3 MappingStart events, got {map_starts}"
    );
}

// === Complex nesting tests ===

#[test]
fn mapping_of_sequences() {
    let events: Vec<_> = Parser::new("a:\n  - 1\n  - 2\nb:\n  - 3")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let seq_starts = events
        .iter()
        .filter(|e| matches!(e, Event::SequenceStart { .. }))
        .count();
    assert_eq!(seq_starts, 2);
}

#[test]
fn sequence_of_mappings() {
    let events: Vec<_> = Parser::new("- a: 1\n  b: 2\n- c: 3")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let map_starts = events
        .iter()
        .filter(|e| matches!(e, Event::MappingStart { .. }))
        .count();
    assert_eq!(map_starts, 2);
}

#[test]
fn deeply_nested() {
    let events: Vec<_> = Parser::new("a:\n  b:\n    c: d")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let map_starts = events
        .iter()
        .filter(|e| matches!(e, Event::MappingStart { .. }))
        .count();
    assert_eq!(map_starts, 3);
}

#[test]
fn flow_inside_block_sequence() {
    let events: Vec<_> = Parser::new("- [a, b]\n- {c: d}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::SequenceStart {
            style: CollectionStyle::Flow,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::MappingStart {
            style: CollectionStyle::Flow,
            ..
        }
    )));
}

// -- Trailing comma fixes --

#[test]
fn trailing_comma_in_flow_sequence() {
    let events: Vec<_> = Parser::new("[a, b, ]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b"]); // NO empty scalar for trailing comma
}

#[test]
fn trailing_comma_in_flow_mapping() {
    let events: Vec<_> = Parser::new("{a: b, c: d, }")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c", "d"]); // NO empty key/value for trailing comma
}

// -- Tag/anchor on empty scalar in block sequence --

#[test]
fn tag_on_empty_scalar_in_sequence() {
    let events: Vec<_> = Parser::new("- !!str\n- a")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Should NOT produce SequenceStart for !!str — should be tagged empty scalar
    let tagged_scalar = events
        .iter()
        .find(|e| matches!(e, Event::Scalar { tag: Some(_), .. }));
    assert!(
        tagged_scalar.is_some(),
        "expected tagged empty scalar, got: {:?}",
        events
    );
}

#[test]
fn anchor_on_empty_scalar_in_sequence() {
    let events: Vec<_> = Parser::new("- &a\n- a")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Should NOT produce SequenceStart for &a — should be anchored empty scalar
    let anchored_scalar = events.iter().find(|e| {
        matches!(
            e,
            Event::Scalar {
                anchor: Some(_),
                ..
            }
        )
    });
    assert!(
        anchored_scalar.is_some(),
        "expected anchored empty scalar, got: {:?}",
        events
    );
}

// -- Bare document end markers (M7A3) --

#[test]
fn bare_document_end_between_documents() {
    // M7A3: two `...` with a comment between — the second `...` is a bare
    // marker in the directive prologue, not a document boundary.
    let events: Vec<_> = Parser::new(
        "Bare\ndocument\n...\n# No document\n...\n|\n%!PS-Adobe-2.0 # Not the first line\n",
    )
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scalar { value, .. } = e {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        scalars,
        vec!["Bare document", "%!PS-Adobe-2.0 # Not the first line\n"]
    );
    let doc_starts = events
        .iter()
        .filter(|e| matches!(e, Event::DocumentStart { .. }))
        .count();
    assert_eq!(doc_starts, 2);
}

#[test]
fn directive_then_document_end_is_error() {
    // B63P: `%YAML 1.2` then `...` without `---` is invalid.
    let result: Result<Vec<_>, _> = Parser::new("%YAML 1.2\n...\n").collect();
    assert!(
        result.is_err(),
        "directives followed by ... without --- should error"
    );
}

// -- LoaderConfig / max_depth tests --

#[test]
fn max_depth_rejects_deep_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(3);

    let input = "a:\n  b:\n    c:\n      d:\n        e: too deep";
    let result: Vec<_> = Parser::with_config(input, &config).collect();
    assert!(result.iter().any(|r| r.is_err()));
}

#[test]
fn max_depth_allows_within_limit() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(10);

    let input = "a:\n  b:\n    c: ok";
    let result: Result<Vec<_>, _> = Parser::with_config(input, &config).collect();
    assert!(result.is_ok());
}

#[test]
fn max_depth_rejects_deep_flow_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(2);

    let input = "{a: {b: {c: deep}}}";
    let result: Vec<_> = Parser::with_config(input, &config).collect();
    assert!(result.iter().any(|r| r.is_err()));
}

#[test]
fn parser_new_has_no_limits() {
    // Parser::new() should work with no limits (backward compat)
    let input = "a:\n  b:\n    c:\n      d:\n        e:\n          f: deep";
    let result: Result<Vec<_>, _> = Parser::new(input).collect();
    assert!(result.is_ok());
}
