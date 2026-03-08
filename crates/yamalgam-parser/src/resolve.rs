//! Event-stream resolution middleware.
//!
//! The [`Resolver`] trait is the extension point for transforming a YAML event
//! stream between the parser and downstream consumers (Value builder, CST,
//! serde). It enables `!include` file injection, `$ref` expansion, custom tag
//! processing, and other event-level transforms without coupling those concerns
//! to the parser itself.
//!
//! See ADR-0007 for design rationale.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::{fmt, io};

use crate::error::ParseError;
use crate::event::Event;

/// Errors that can occur during event resolution.
#[derive(Debug)]
pub enum ResolveError {
    /// A parse error propagated from the upstream parser.
    Parse(ParseError),
    /// An `!include` or similar file-loading directive failed.
    Include {
        /// Path that could not be loaded.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// A `$ref` or cross-document reference failed to resolve.
    Ref {
        /// The reference target that could not be resolved.
        target: String,
        /// Underlying error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A circular reference was detected.
    Cycle {
        /// The chain of references forming the cycle.
        chain: Vec<String>,
    },
    /// A resource limit was exceeded during resolution.
    LimitExceeded(String),
    /// An application-defined resolution error.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Include { path, source } => {
                write!(f, "include failed for {}: {source}", path.display())
            }
            Self::Ref { target, source } => {
                write!(f, "ref resolution failed for {target:?}: {source}")
            }
            Self::Cycle { chain } => {
                write!(f, "circular reference: {}", chain.join(" -> "))
            }
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
            Self::Custom(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Include { source, .. } => Some(source),
            Self::Ref { source, .. } => Some(source.as_ref()),
            Self::Custom(e) => Some(e.as_ref()),
            Self::Cycle { .. } | Self::LimitExceeded(_) => None,
        }
    }
}

impl From<ParseError> for ResolveError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

/// Event-stream middleware that transforms parser events before they reach
/// downstream consumers.
///
/// Implementations receive one event at a time and return zero or more output
/// events:
/// - Return `Ok(vec![event])` to pass through unchanged.
/// - Return `Ok(vec![])` to suppress the event.
/// - Return `Ok(vec![e1, e2, ...])` to expand into multiple events.
pub trait Resolver<'input> {
    /// Process a single event and return the resulting events.
    fn on_event(&mut self, event: Event<'input>) -> Result<Vec<Event<'input>>, ResolveError>;
}

/// A resolver that passes all events through unchanged.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopResolver;

impl<'input> Resolver<'input> for NoopResolver {
    fn on_event(&mut self, event: Event<'input>) -> Result<Vec<Event<'input>>, ResolveError> {
        Ok(vec![event])
    }
}

/// Iterator adapter that applies a [`Resolver`] to an upstream event stream.
///
/// Events from the upstream iterator are passed through the resolver one at a
/// time. When the resolver produces multiple output events, they are buffered
/// and yielded individually. When the resolver returns an empty vec (suppressing
/// an event), the adapter pulls the next upstream event automatically.
pub struct ResolvedEvents<'input, R: Resolver<'input>> {
    upstream: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
    resolver: R,
    buffer: VecDeque<Event<'input>>,
}

impl<'input, R: Resolver<'input>> ResolvedEvents<'input, R> {
    /// Create a new `ResolvedEvents` adapter wrapping an upstream event source.
    #[must_use]
    pub fn new(
        upstream: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
        resolver: R,
    ) -> Self {
        Self {
            upstream,
            resolver,
            buffer: VecDeque::new(),
        }
    }
}

impl<'input, R: Resolver<'input>> Iterator for ResolvedEvents<'input, R> {
    type Item = Result<Event<'input>, ResolveError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Drain buffered events first.
        if let Some(event) = self.buffer.pop_front() {
            return Some(Ok(event));
        }

        // Pull from upstream until we get output or exhaust the stream.
        loop {
            let upstream_event = match self.upstream.next()? {
                Ok(event) => event,
                Err(e) => return Some(Err(e)),
            };

            match self.resolver.on_event(upstream_event) {
                Ok(mut events) => {
                    if events.is_empty() {
                        // Suppressed — try next upstream event.
                        continue;
                    }
                    if events.len() == 1 {
                        return Some(Ok(events.remove(0)));
                    }
                    // Buffer multi-event results, yield the first.
                    let mut drain = events.into_iter();
                    let first = drain.next().expect("non-empty checked above");
                    self.buffer.extend(drain);
                    return Some(Ok(first));
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn noop_resolver_passes_all_events_through() {
        let input = "key: value";
        let direct: Vec<_> = Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap();
        let parser = Parser::new(input);
        let resolved: Vec<_> = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(direct, resolved);
    }

    #[test]
    fn noop_resolver_preserves_event_count() {
        let input = "- a\n- b\n- c";
        let count = Parser::new(input).count();
        let parser = Parser::new(input);
        let resolved_count = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .count();
        assert_eq!(count, resolved_count);
    }

    #[test]
    fn noop_resolver_passes_errors_through() {
        let input = "'unterminated";
        let parser = Parser::new(input);
        let results: Vec<_> = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .collect();
        assert!(results.iter().any(|r| r.is_err()));
    }

    #[test]
    fn resolve_error_from_parse_error() {
        let input = "'bad";
        let results: Vec<Result<Event<'_>, ResolveError>> = Parser::new(input)
            .map(|r| r.map_err(ResolveError::Parse))
            .collect();
        assert!(
            results
                .iter()
                .any(|r| matches!(r, Err(ResolveError::Parse(_))))
        );
    }
}
