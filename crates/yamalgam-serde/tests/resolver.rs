#![allow(missing_docs)]

use std::borrow::Cow;

use serde::Deserialize;
use yamalgam_parser::resolve::smallvec;
use yamalgam_parser::{Event, ResolveError, ResolvedEventBuf, Resolver};
use yamalgam_serde::{Error, from_str_with_resolver};

struct RewriteMagic;

impl<'input> Resolver<'input> for RewriteMagic {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        match event {
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                span,
            } if value == "magic" => Ok(smallvec![Event::Scalar {
                anchor,
                tag,
                value: Cow::Borrowed("42"),
                style,
                span,
            }]),
            other => Ok(smallvec![other]),
        }
    }
}

#[test]
fn resolver_rewrite_reaches_deserializer() {
    let value = from_str_with_resolver::<i64, _>("magic", RewriteMagic).unwrap();
    assert_eq!(value, 42);
}

struct FailOnScalar;

impl<'input> Resolver<'input> for FailOnScalar {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        if matches!(event, Event::Scalar { .. }) {
            return Err(ResolveError::Custom(Box::new(std::io::Error::other(
                "serde resolver failed",
            ))));
        }
        Ok(smallvec![event])
    }
}

#[test]
fn resolver_failure_stays_structured_through_serde() {
    let error = from_str_with_resolver::<String, _>("value", FailOnScalar).unwrap_err();
    assert!(
        matches!(error, Error::Resolve(ResolveError::Custom(_))),
        "got {error:?}"
    );
    assert_eq!(error.to_string(), "serde resolver failed");
}

struct OwnScalars;

impl<'input> Resolver<'input> for OwnScalars {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        match event {
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                span,
            } => Ok(smallvec![Event::Scalar {
                anchor,
                tag,
                value: Cow::Owned(value.into_owned()),
                style,
                span,
            }]),
            other => Ok(smallvec![other]),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
enum OwnedColor {
    Red,
}

#[test]
fn owned_resolver_scalars_reach_null_unit_option_and_enum_paths() {
    assert_eq!(
        from_str_with_resolver::<(), _>("null", OwnScalars).unwrap(),
        ()
    );
    assert_eq!(
        from_str_with_resolver::<Option<String>, _>("null", OwnScalars).unwrap(),
        None
    );
    assert_eq!(
        from_str_with_resolver::<Option<String>, _>("value", OwnScalars).unwrap(),
        Some("value".into())
    );
    assert_eq!(
        from_str_with_resolver::<OwnedColor, _>("Red", OwnScalars).unwrap(),
        OwnedColor::Red
    );
}
