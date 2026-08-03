//! Resolved-value and presentation-metadata semantics for `YamlVal`.

use yamalgam_query::value::{Meta, NodeStyle, YamlVal};
use yamalgam_scanner::ScalarStyle;

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
    });

    assert_eq!(v.meta().and_then(|m| m.anchor.as_deref()), Some("base"));
    assert_eq!(v.tag_shorthand().as_deref(), Some("!!str"));
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
