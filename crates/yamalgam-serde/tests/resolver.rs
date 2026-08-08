#![allow(missing_docs)]

use std::borrow::Cow;

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
