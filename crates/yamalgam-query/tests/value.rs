//! Resolved-value and presentation-metadata semantics for `YamlVal`.

use yamalgam_core::{LoaderConfig, ResourceLimit, TagResolution};
use yamalgam_parser::resolve::smallvec;
use yamalgam_parser::{Event, ResolveError, ResolvedEventBuf, Resolver};
use yamalgam_query::{
    BuildError, Meta, NodeStyle, YamlVal, parse_documents_with_config, parse_str,
    parse_str_with_config, parse_str_with_resolver, parse_str_with_resolver_and_config,
};
use yamalgam_scanner::{AtomFlags, Chomp, ScalarStyle};

#[test]
fn kind_names_follow_yaml_terms() {
    // r[diverge.type]
    assert_eq!(YamlVal::null().type_name(), "null");
    assert_eq!(YamlVal::from(true).type_name(), "boolean");
    assert_eq!(YamlVal::int(3).type_name(), "integer");
    assert_eq!(YamlVal::float(3.5).type_name(), "float");
    assert_eq!(YamlVal::str("hi").type_name(), "string");
    assert_eq!(YamlVal::seq(vec![]).type_name(), "sequence");
    assert_eq!(YamlVal::map(vec![]).type_name(), "mapping");
}

#[test]
fn public_accessors_expose_resolved_values() {
    assert_eq!(YamlVal::from(true).as_boolean(), Some(true));
    assert_eq!(YamlVal::int(7).as_i64(), Some(7));
    assert_eq!(YamlVal::float(2.5).as_f64(), Some(2.5));

    let sequence = YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2)]);
    assert_eq!(sequence.as_sequence().unwrap().len(), 2);

    let mapping = YamlVal::map(vec![(YamlVal::str("answer"), YamlVal::int(42))]);
    let entries = mapping.entries().unwrap().collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, &YamlVal::str("answer"));
    assert_eq!(entries[0].1, &YamlVal::int(42));
}

#[test]
fn kind_is_structure_not_resolution() {
    // r[ext.kind]
    assert_eq!(YamlVal::int(3).kind_name(), "scalar");
    assert_eq!(YamlVal::seq(vec![]).kind_name(), "sequence");
    assert_eq!(YamlVal::map(vec![]).kind_name(), "mapping");
    let aliased = YamlVal::int(3).with_meta(Meta {
        alias_of: Some("a".into()),
        ..Meta::default()
    });
    assert_eq!(aliased.kind_name(), "alias");
}

#[test]
fn meta_accessors_preserve_yaml_presentation() {
    let v = YamlVal::str("x").with_meta(Meta {
        tag: Some("tag:yaml.org,2002:str".into()),
        anchor: Some("base".into()),
        style: Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)),
        raw: Some("x".into()),
        alias_of: None,
        scalar_presentation: None,
    });

    assert_eq!(v.meta().and_then(|m| m.anchor.as_deref()), Some("base"));
    assert_eq!(v.tag_shorthand().as_deref(), Some("!!str"));
}

#[test]
fn parsed_values_preserve_scalar_presentation_metadata() {
    let value = parse_str("|+\n  line\n\n").unwrap().remove(0);
    let presentation = value
        .meta()
        .and_then(|metadata| metadata.scalar_presentation)
        .unwrap();

    assert_eq!(presentation.chomp, Chomp::Keep);
    assert!(presentation.flags.contains(AtomFlags::CHOMP_EXPLICIT));
    assert!(presentation.flags.contains(AtomFlags::TRAILING_LB));
    assert!(!presentation.flags.contains(AtomFlags::DIRECT_OUTPUT));
}

#[test]
fn resolved_equality_is_metadata_blind_and_numeric() {
    // r[comparison.resolved]
    assert_eq!(YamlVal::int(1), YamlVal::float(1.0));
    assert_ne!(YamlVal::int(1), YamlVal::str("1"));
    assert_ne!(YamlVal::null(), YamlVal::from(false));

    let plain = YamlVal::str("hello");
    let quoted = YamlVal::str("hello").with_meta(Meta {
        style: Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)),
        ..Meta::default()
    });
    assert_eq!(plain, quoted);
}

#[test]
fn mapping_equality_is_order_sensitive() {
    // r[comparison.resolved.collection]
    let ab = YamlVal::map(vec![
        (YamlVal::str("a"), YamlVal::int(1)),
        (YamlVal::str("b"), YamlVal::int(2)),
    ]);
    let ba = YamlVal::map(vec![
        (YamlVal::str("b"), YamlVal::int(2)),
        (YamlVal::str("a"), YamlVal::int(1)),
    ]);

    assert_ne!(ab, ba);
}

#[test]
fn cross_type_ordering_follows_jq() {
    // r[comparison.ordering.cross-type]
    let order = [
        YamlVal::null(),
        YamlVal::from(false),
        YamlVal::from(true),
        YamlVal::int(1),
        YamlVal::str("a"),
        YamlVal::seq(vec![]),
        YamlVal::map(vec![]),
    ];

    for pair in order.windows(2) {
        assert!(
            pair[0] < pair[1],
            "{} should sort before {}",
            pair[0].type_name(),
            pair[1].type_name()
        );
    }
}

#[test]
fn numeric_hashes_match_numeric_equality() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash(value: &YamlVal) -> u64 {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    }

    assert_eq!(hash(&YamlVal::int(1)), hash(&YamlVal::float(1.0)));
    assert_eq!(hash(&YamlVal::float(0.0)), hash(&YamlVal::float(-0.0)));
}

#[test]
fn nan_is_a_reflexive_total_order_value() {
    let nan = YamlVal::float(f64::NAN);

    assert_eq!(nan, nan.clone());
    assert_eq!(nan.cmp(&nan), std::cmp::Ordering::Equal);
    assert!(nan < YamlVal::int(-5));
}

#[test]
fn builder_preserves_scalar_metadata() {
    let docs = parse_str("base: &b !custom 'hi'\n").unwrap();
    let value = docs[0].get_str("base").unwrap();
    let meta = value.meta().unwrap();

    assert_eq!(meta.anchor.as_deref(), Some("b"));
    assert_eq!(meta.tag.as_deref(), Some("!custom"));
    assert_eq!(
        meta.style,
        Some(NodeStyle::Scalar(ScalarStyle::SingleQuoted))
    );
    assert_eq!(meta.raw.as_deref(), Some("hi"));
}

#[test]
fn document_iterator_yields_prior_document_before_later_error() {
    let config = LoaderConfig::moderate();
    let mut documents =
        parse_documents_with_config("---\na: 1\n...\n---\n[unterminated\n", &config).unwrap();

    let first = documents.next().unwrap().unwrap();
    assert_eq!(first.get_str("a"), Some(&YamlVal::int(1)));
    assert!(documents.next().unwrap().is_err());
    assert!(documents.next().is_none());
}

#[test]
fn plain_scalars_resolve_using_the_configured_schema() {
    // r[datamodel.schema.default]
    let docs = parse_str("a: 1\nb: yes\nc: 1.5\nd: null\n").unwrap();
    let map = &docs[0];
    assert_eq!(map.get_str("a"), Some(&YamlVal::int(1)));
    assert_eq!(map.get_str("b"), Some(&YamlVal::str("yes")));
    assert_eq!(map.get_str("c"), Some(&YamlVal::float(1.5)));
    assert_eq!(map.get_str("d"), Some(&YamlVal::null()));

    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Failsafe);
    let docs = parse_str_with_config("a: 1\n", &config).unwrap();
    assert_eq!(docs[0].get_str("a"), Some(&YamlVal::str("1")));
}

#[test]
fn aliases_splice_values_with_identity_metadata() {
    let docs = parse_str("a: &x [1, 2]\nb: *x\n").unwrap();
    let map = &docs[0];

    assert_eq!(map.get_str("a"), map.get_str("b"));
    let alias = map.get_str("b").unwrap();
    assert_eq!(alias.meta().unwrap().alias_of.as_deref(), Some("x"));
    assert!(alias.meta().unwrap().anchor.is_none());
}

#[test]
fn merge_keys_are_preserved_instead_of_expanded() {
    // r[ext.merge.preserve]
    let docs = parse_str("base: &b {x: 1}\nchild:\n  <<: *b\n  y: 2\n").unwrap();
    let child = docs[0].get_str("child").unwrap();

    assert!(child.get_str("x").is_none());
    let merge_key = child.keys().unwrap().next().unwrap();
    assert_eq!(merge_key.tag_shorthand().as_deref(), Some("!!merge"));
    assert!(YamlVal::str("not a mapping").keys().is_none());
}

#[test]
fn unknown_alias_reports_its_name() {
    let error = parse_str("a: *nope\n").unwrap_err();

    assert!(matches!(error, BuildError::UnknownAlias { name, .. } if name == "nope"));
}

#[test]
fn parser_builds_multiple_and_explicit_empty_documents() {
    let docs = parse_str("---\n1\n---\n2\n---\n").unwrap();

    assert_eq!(
        docs,
        vec![YamlVal::int(1), YamlVal::int(2), YamlVal::null()]
    );
}

struct RewriteMagic;

impl<'input> Resolver<'input> for RewriteMagic {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        match event {
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                chomp,
                span,
                ..
            } if value == "magic" => Ok(smallvec![Event::Scalar {
                anchor,
                tag,
                value: "42".into(),
                style,
                chomp,
                flags: AtomFlags::empty(),
                span,
            }]),
            other => Ok(smallvec![other]),
        }
    }
}

#[test]
fn resolver_rewrite_reaches_query_builder() {
    let documents = parse_str_with_resolver("magic", RewriteMagic).unwrap();
    assert_eq!(documents, vec![YamlVal::int(42)]);

    let mut config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Failsafe);
    config.resolution.include.enabled = true;
    let documents = parse_str_with_resolver_and_config("magic", RewriteMagic, &config).unwrap();
    assert_eq!(documents, vec![YamlVal::str("42")]);
}

struct FailOnScalar;

impl<'input> Resolver<'input> for FailOnScalar {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        if matches!(event, Event::Scalar { .. }) {
            return Err(ResolveError::Custom(Box::new(std::io::Error::other(
                "query resolver failed",
            ))));
        }
        Ok(smallvec![event])
    }
}

#[test]
fn resolver_failure_stays_typed_in_query_builder() {
    let error = parse_str_with_resolver("value", FailOnScalar).unwrap_err();
    assert!(
        matches!(error, BuildError::Resolve(ResolveError::Custom(_))),
        "got {error:?}"
    );
    assert_eq!(error.to_string(), "query resolver failed");
}

#[test]
fn configured_noop_query_builder_rejects_enabled_resolution_policy() {
    let mut config = LoaderConfig::moderate();
    config.resolution.include.enabled = true;

    assert!(matches!(
        parse_str_with_config("value", &config),
        Err(BuildError::Resolve(
            ResolveError::UnsupportedResolutionPolicy {
                include: true,
                refs: false,
            }
        ))
    ));
}

#[test]
fn explicit_core_tags_force_resolution() {
    let docs = parse_str("a: !!str 5\nb: !!int '5'\nc: !!bool yes\n").unwrap();
    let map = &docs[0];

    assert_eq!(map.get_str("a"), Some(&YamlVal::str("5")));
    assert_eq!(map.get_str("b"), Some(&YamlVal::int(5)));
    assert_eq!(map.get_str("c"), Some(&YamlVal::str("yes")));
}

#[test]
fn builder_enforces_anchor_count_limit() {
    let mut config = LoaderConfig::unchecked();
    config.limits.max_anchor_count = Some(1);

    let error = parse_str_with_config("a: &one 1\nb: &two 2\n", &config).unwrap_err();
    assert!(matches!(
        error,
        BuildError::Limit(error) if error.limit == ResourceLimit::AnchorCount
    ));
}

#[test]
fn builder_enforces_alias_expansion_limit() {
    let mut config = LoaderConfig::unchecked();
    config.limits.max_alias_expansions = Some(1);

    let error = parse_str_with_config("a: &one 1\nb: *one\nc: *one\n", &config).unwrap_err();
    assert!(matches!(
        error,
        BuildError::Limit(error) if error.limit == ResourceLimit::AliasExpansions
    ));
}

#[test]
fn builder_enforces_collection_size_limit() {
    let mut config = LoaderConfig::unchecked();
    config.limits.max_collection_size = Some(1);

    let sequence_error = parse_str_with_config("[1, 2]\n", &config).unwrap_err();
    assert!(matches!(
        sequence_error,
        BuildError::Limit(error) if error.limit == ResourceLimit::CollectionSize
    ));

    let mapping_error = parse_str_with_config("{a: 1, a: 2}\n", &config).unwrap_err();
    assert!(matches!(
        mapping_error,
        BuildError::Limit(error) if error.limit == ResourceLimit::CollectionSize
    ));
}

#[test]
fn builder_enforces_resulting_alias_value_depth() {
    let document = concat!("a: &a0 [0]\n", "b: &a1 [*a0]\n", "c: &a2 [*a1]\n",);

    let mut rejecting = LoaderConfig::unchecked();
    rejecting.limits.max_depth = Some(3);
    let error = parse_str_with_config(document, &rejecting).unwrap_err();
    let message = error.to_string();
    assert!(
        matches!(error, BuildError::Limit(error) if error.limit == ResourceLimit::Depth),
        "unexpected error: {message}"
    );

    let mut accepting = LoaderConfig::unchecked();
    accepting.limits.max_depth = Some(4);
    let documents = parse_str_with_config(document, &accepting).unwrap();
    let alias = &documents[0].get_str("c").unwrap().as_sequence().unwrap()[0];
    assert_eq!(alias.meta().unwrap().alias_of.as_deref(), Some("a1"));
}

#[test]
fn display_is_compact_flow_yaml() {
    let value = parse_str("a: [1, two]\nb: null\n").unwrap().remove(0);

    assert_eq!(value.to_string(), "{a: [1, two], b: null}");
    assert_eq!(
        YamlVal::str("needs: quoting").to_string(),
        "\"needs: quoting\""
    );
    assert_eq!(YamlVal::str("true").to_string(), "\"true\"");
    assert_eq!(YamlVal::float(1.0).to_string(), "1.0");
    assert_eq!(YamlVal::float(f64::INFINITY).to_string(), ".inf");
    assert_eq!(YamlVal::float(f64::NEG_INFINITY).to_string(), "-.inf");
    assert_eq!(YamlVal::float(f64::NAN).to_string(), ".nan");
}

#[test]
fn display_escapes_quoted_strings() {
    assert_eq!(
        YamlVal::str("line 1\n\"line 2\"\\").to_string(),
        "\"line 1\\n\\\"line 2\\\"\\\\\""
    );
}
