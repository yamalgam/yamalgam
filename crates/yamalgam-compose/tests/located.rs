#![allow(missing_docs)]

use yamalgam_compose::{from_str, from_str_located, from_str_single_located};

fn span_text(input: &str, span: yamalgam_core::Span) -> &str {
    &input[span.start.offset..span.end.offset]
}

#[test]
fn nested_pointer_resolves_to_the_value_token_span() {
    let input = "service:\n  ports:\n    - 80\n    - 443\n";
    let located = from_str_single_located(input).expect("located document");

    let span = located
        .span_at_pointer("/service/ports/1")
        .expect("second port span");

    assert_eq!(span_text(input, span), "443");
}

#[test]
fn pointer_segments_unescape_slashes_and_tildes() {
    let input = "a/b~c: value\n";
    let located = from_str_single_located(input).expect("located document");

    let span = located
        .span_at_pointer("/a~1b~0c")
        .expect("escaped key span");

    assert_eq!(span_text(input, span), "value");
}

#[test]
fn alias_root_uses_the_alias_span_but_children_keep_definition_spans() {
    let input = "defaults: &cfg\n  timeout: 30\ncopy: *cfg\n";
    let located = from_str_single_located(input).expect("located document");

    let alias_span = located.span_at_pointer("/copy").expect("alias root span");
    let child_span = located
        .span_at_pointer("/copy/timeout")
        .expect("aliased child span");

    // Parser semantic spans cover the alias name; the structural `*` remains
    // available through CST events.
    assert_eq!(span_text(input, alias_span), "cfg");
    assert_eq!(span_text(input, child_span), "30");
}

#[test]
fn merged_properties_keep_the_anchor_definition_span() {
    let input = concat!(
        "defaults: &cfg\n",
        "  timeout: 30\n",
        "service:\n",
        "  <<: *cfg\n",
        "  name: api\n",
    );
    let located = from_str_single_located(input).expect("located document");

    let merged_span = located
        .span_at_pointer("/service/timeout")
        .expect("merged property span");
    let explicit_span = located
        .span_at_pointer("/service/name")
        .expect("explicit property span");

    assert_eq!(span_text(input, merged_span), "30");
    assert_eq!(span_text(input, explicit_span), "api");
}

#[test]
fn every_document_gets_an_independent_location_tree() {
    let input = "---\na: 1\n---\nb: 2\n";
    let located = from_str_located(input).expect("located stream");
    let ordinary = from_str(input).expect("ordinary stream");

    assert_eq!(located.len(), 2);
    assert_eq!(
        span_text(input, located[0].span_at_pointer("/a").unwrap()),
        "1"
    );
    assert_eq!(
        span_text(input, located[1].span_at_pointer("/b").unwrap()),
        "2"
    );
    assert_eq!(
        located
            .iter()
            .map(|document| document.value.clone())
            .collect::<Vec<_>>(),
        ordinary
    );
}
