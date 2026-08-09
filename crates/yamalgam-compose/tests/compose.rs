#![allow(clippy::approx_constant, missing_docs)]

use std::borrow::Cow;
use std::cell::Cell;
use std::rc::Rc;

use yamalgam_compose::{Composer, from_str, from_str_single};
use yamalgam_core::{LoaderConfig, Value};
use yamalgam_parser::resolve::smallvec;
use yamalgam_parser::{Event, ResolveError, ResolvedEventBuf, Resolver};
use yamalgam_scanner::AtomFlags;

#[test]
fn parse_simple_mapping() {
    let v = from_str_single("name: yamalgam\nversion: 0.1").unwrap();
    assert_eq!(v.get("name"), Some(&Value::from("yamalgam")));
}

#[test]
fn parse_nested_config() {
    let input = "database:\n  host: localhost\n  port: 5432\n  credentials:\n    user: admin\n    pass: secret";
    let v = from_str_single(input).unwrap();
    let db = v.get("database").unwrap();
    assert_eq!(db.get("host"), Some(&Value::from("localhost")));
    assert_eq!(db.get("port"), Some(&Value::Integer(5432)));
    let creds = db.get("credentials").unwrap();
    assert_eq!(creds.get("user"), Some(&Value::from("admin")));
}

#[test]
fn parse_sequence_of_strings() {
    let v = from_str_single("- alpha\n- bravo\n- charlie").unwrap();
    assert_eq!(v.get_index(0), Some(&Value::from("alpha")));
    assert_eq!(v.get_index(2), Some(&Value::from("charlie")));
}

#[test]
fn parse_mixed_types() {
    let input = "str: hello\nint: 42\nfloat: 3.14\nbool: true\nnull_val:";
    let v = from_str_single(input).unwrap();
    assert_eq!(v.get("str"), Some(&Value::from("hello")));
    assert_eq!(v.get("int"), Some(&Value::Integer(42)));
    assert_eq!(v.get("float"), Some(&Value::Float(3.14)));
    assert_eq!(v.get("bool"), Some(&Value::Bool(true)));
    assert_eq!(v.get("null_val"), Some(&Value::Null));
}

#[test]
fn parse_multi_document() {
    let docs = from_str("---\na: 1\n---\nb: 2\n...").unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn parse_anchor_alias_roundtrip() {
    let v =
        from_str_single("default: &cfg\n  timeout: 30\nservice:\n  <<: *cfg\n  name: api").unwrap();
    let svc = v.get("service").unwrap();
    assert_eq!(svc.get("timeout"), Some(&Value::Integer(30)));
    assert_eq!(svc.get("name"), Some(&Value::from("api")));
}

#[test]
fn parse_flow_collections() {
    let v = from_str_single("{ports: [80, 443], tls: true}").unwrap();
    let ports = v.get("ports").unwrap();
    assert_eq!(ports.get_index(0), Some(&Value::Integer(80)));
    assert_eq!(ports.get_index(1), Some(&Value::Integer(443)));
}

#[test]
fn parse_empty_string() {
    let docs = from_str("").unwrap();
    assert!(docs.is_empty());
}

#[test]
fn undefined_alias_is_error() {
    assert!(from_str_single("*nope").is_err());
}

/// A test resolver that uppercases all plain scalar values.
struct UppercaseResolver;

impl<'input> Resolver<'input> for UppercaseResolver {
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
            } => {
                let upper = value.to_uppercase();
                Ok(smallvec![Event::Scalar {
                    anchor,
                    tag,
                    value: Cow::Owned(upper),
                    style,
                    chomp,
                    flags: AtomFlags::empty(),
                    span,
                }])
            }
            other => Ok(smallvec![other]),
        }
    }
}

#[test]
fn custom_resolver_transforms_events() {
    let docs = Composer::with_resolver("key: value", UppercaseResolver).unwrap();
    // Both key and value are uppercased since both are plain scalars.
    assert_eq!(docs[0].get("KEY"), Some(&Value::from("VALUE")));
}

struct ConfigProbe {
    observed_depth: Rc<Cell<Option<usize>>>,
}

impl<'input> Resolver<'input> for ConfigProbe {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        Ok(smallvec![event])
    }

    fn on_event_with_config(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        self.observed_depth.set(config.limits.max_depth);
        self.on_event(event)
    }
}

fn config_probe() -> (ConfigProbe, Rc<Cell<Option<usize>>>) {
    let observed_depth = Rc::new(Cell::new(None));
    (
        ConfigProbe {
            observed_depth: Rc::clone(&observed_depth),
        },
        observed_depth,
    )
}

#[test]
fn composer_passes_moderate_config_to_resolver() {
    let (resolver, observed_depth) = config_probe();
    let documents = Composer::with_resolver("value", resolver).unwrap();

    assert_eq!(documents, vec![Value::from("value")]);
    assert_eq!(observed_depth.get(), Some(256));
}

#[test]
fn composer_passes_unchecked_config_to_resolver() {
    let (resolver, observed_depth) = config_probe();
    let documents = Composer::with_resolver_unchecked("value", resolver).unwrap();

    assert_eq!(documents, vec![Value::from("value")]);
    assert_eq!(observed_depth.get(), None);
}

#[test]
fn composer_passes_exact_selected_config_to_resolver() {
    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(7);
    let (resolver, observed_depth) = config_probe();
    let documents = Composer::with_resolver_and_config("value", resolver, &config).unwrap();

    assert_eq!(documents, vec![Value::from("value")]);
    assert_eq!(observed_depth.get(), Some(7));
}
