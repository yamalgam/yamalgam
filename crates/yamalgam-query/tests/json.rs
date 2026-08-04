//! Strict JSON boundary tests.

use yamalgam_core::LoaderConfig;
use yamalgam_parser::CollectionStyle;
use yamalgam_query::{EmitOpts, NodeStyle, parse_json_with_config};

fn parse(source: &str) -> Result<yamalgam_query::YamlVal, yamalgam_query::YgError> {
    parse_json_with_config(source, &LoaderConfig::moderate())
}

fn compact_json(source: &str) -> String {
    let value = parse(source).unwrap();
    yamalgam_query::emit::to_json(&value, &EmitOpts::compact()).unwrap()
}

#[test]
fn strict_json_rejects_yaml_only_and_extended_syntax() {
    for source in [
        "",
        "a: 1",
        "{a: 1}",
        "{'a': 1}",
        "{\"a\": 1,}",
        "[1,]",
        "---\n1",
        "1\n2",
        "\"\\x41\"",
        "\"line\nbreak\"",
        ".5",
        "+1",
        "01",
        "1.",
        "1e",
        ".inf",
        "~",
        "# comment\n1",
        "&a 1",
        "*a",
        "!!int 1",
    ] {
        assert!(parse(source).is_err(), "accepted invalid JSON: {source:?}");
    }
}

#[test]
fn strict_json_preserves_numbers_escapes_duplicates_and_order() {
    assert_eq!(
        compact_json(
            r#"{"small":1,"large":9223372036854775808,"float":1e+2,"music":"\uD834\uDD1E","a":1,"a":2}"#,
        ),
        r#"{"small":1,"large":9.223372036854776e18,"float":100.0,"music":"𝄞","a":2}"#
    );
}

#[test]
fn strict_json_uses_the_native_builder() {
    let value = parse(r#"{"a":1}"#).unwrap();

    assert_eq!(
        value.meta().and_then(|meta| meta.style),
        Some(NodeStyle::Collection(CollectionStyle::Flow))
    );
    assert_eq!(
        value
            .get_str("a")
            .and_then(|value| value.meta())
            .and_then(|meta| meta.raw.as_deref()),
        Some("1")
    );
}

#[test]
fn strict_json_is_not_capped_by_serde_jsons_dom_recursion_limit() {
    let depth = 140;
    let source = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
    let mut config = LoaderConfig::moderate();
    config.limits.max_depth = Some(depth + 1);

    parse_json_with_config(&source, &config).unwrap();
}

#[test]
fn strict_json_enforces_collection_limits() {
    let mut config = LoaderConfig::unchecked();
    config.limits.max_collection_size = Some(1);

    let error = parse_json_with_config("[1,2]", &config).unwrap_err();

    assert!(error.to_string().contains("collection size"));

    let error = parse_json_with_config("[1,{invalid]", &config).unwrap_err();

    assert!(
        error.to_string().contains("collection size"),
        "oversized collection should fail before parsing its next value: {error}"
    );
}
