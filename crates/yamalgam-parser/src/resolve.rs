//! Event-stream resolution middleware.
//!
//! The [`Resolver`] trait is the extension point for transforming a YAML event
//! stream between the parser and downstream consumers (Value builder, CST,
//! serde). It enables `!include` file injection, `$ref` expansion, custom tag
//! processing, and other event-level transforms without coupling those concerns
//! to the parser itself.
//!
//! See ADR-0010 for design rationale.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::{fmt, io};

pub use smallvec::{SmallVec, smallvec};
use yamalgam_core::{LimitExceeded, LoaderConfig, ResolutionMode};

use crate::error::ParseError;
use crate::event::Event;

/// Events produced by a single [`Resolver::on_event`] step.
///
/// Inline capacity of one keeps the common passthrough case (one event in,
/// one event out) entirely on the stack — no heap allocation per event.
pub type ResolvedEventBuf<'input> = SmallVec<[Event<'input>; 1]>;

/// A type-erased resolved-event source for callers with mixed iterator types.
pub type BoxedResolveStream<'input> =
    Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>;

/// Errors that can occur during event resolution.
#[non_exhaustive]
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
    LimitExceeded(LimitExceeded),
    /// External resolution was enabled without a resolver implementation.
    UnsupportedResolutionPolicy {
        /// Whether include resolution was enabled.
        include: bool,
        /// Whether reference resolution was enabled.
        refs: bool,
    },
    /// An application-defined resolution error.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl ResolveError {
    /// Clone this error while preserving its structured outer variant.
    ///
    /// Error sources that do not implement [`Clone`] are reconstructed from
    /// their rendered message. I/O errors also retain their [`io::ErrorKind`].
    #[must_use]
    pub fn clone_structured(&self) -> Self {
        match self {
            Self::Parse(error) => Self::Parse(error.clone()),
            Self::Include { path, source } => Self::Include {
                path: path.clone(),
                source: io::Error::new(source.kind(), source.to_string()),
            },
            Self::Ref { target, source } => Self::Ref {
                target: target.clone(),
                source: Box::new(ClonedError(source.to_string())),
            },
            Self::Cycle { chain } => Self::Cycle {
                chain: chain.clone(),
            },
            Self::LimitExceeded(error) => Self::LimitExceeded(*error),
            Self::UnsupportedResolutionPolicy { include, refs } => {
                Self::UnsupportedResolutionPolicy {
                    include: *include,
                    refs: *refs,
                }
            }
            Self::Custom(source) => Self::Custom(Box::new(ClonedError(source.to_string()))),
        }
    }
}

#[derive(Debug)]
struct ClonedError(String);

impl fmt::Display for ClonedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ClonedError {}

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
            Self::LimitExceeded(error) => write!(f, "limit exceeded: {error}"),
            Self::UnsupportedResolutionPolicy { include, refs } => write!(
                f,
                "enabled resolution policy is unsupported without a resolver (include: {include}, refs: {refs})"
            ),
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
            Self::LimitExceeded(error) => Some(error),
            Self::Cycle { .. } | Self::UnsupportedResolutionPolicy { .. } => None,
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
/// events (the [`smallvec!`] macro is re-exported from this module):
/// - Return `Ok(smallvec![event])` to pass through unchanged.
/// - Return `Ok(smallvec![])` to suppress the event.
/// - Return `Ok(smallvec![e1, e2, ...])` to expand into multiple events.
///
/// The passthrough case stays allocation-free: [`ResolvedEventBuf`] holds one
/// event inline (ADR-0010).
///
/// Resolver-aware entry points delegate enforcement of enabled
/// [`LoaderConfig::resolution`](yamalgam_core::LoaderConfig::resolution) policy
/// to the supplied implementation. [`NoopResolver`] rejects enabled external
/// resolution because built-in include and reference resolvers are not yet
/// available (ADR-0011).
pub trait Resolver<'input> {
    /// Process a single event and return the resulting events.
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError>;

    /// Process one event with the loader configuration selected by the
    /// downstream consumer.
    ///
    /// Existing resolvers may implement only [`Self::on_event`]. Resolvers
    /// that enforce include or reference policy can override this method.
    fn on_event_with_config(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        let _ = config;
        self.on_event(event)
    }
}

/// A resolver that passes all events through unchanged.
///
/// When include or reference resolution is enabled in the selected
/// [`LoaderConfig`], this resolver returns
/// [`ResolveError::UnsupportedResolutionPolicy`] instead of silently ignoring
/// the requested capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopResolver;

impl<'input> Resolver<'input> for NoopResolver {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        Ok(smallvec![event])
    }

    fn on_event_with_config(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        if config.resolution.is_enabled() {
            return Err(ResolveError::UnsupportedResolutionPolicy {
                include: config.resolution.include.mode != ResolutionMode::Disabled,
                refs: config.resolution.refs.mode != ResolutionMode::Disabled,
            });
        }
        self.on_event(event)
    }
}

/// Iterator adapter that applies a [`Resolver`] to an upstream event stream.
///
/// Events from the upstream iterator are passed through the resolver one at a
/// time. When the resolver produces multiple output events, they are buffered
/// and yielded individually. When the resolver returns an empty vec (suppressing
/// an event), the adapter pulls the next upstream event automatically.
pub struct ResolvedEvents<'input, R: Resolver<'input>, I = BoxedResolveStream<'input>>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    upstream: I,
    resolver: R,
    buffer: VecDeque<Event<'input>>,
    config: LoaderConfig,
    resolver_failed: bool,
}

impl<'input, R, I> ResolvedEvents<'input, R, I>
where
    R: Resolver<'input>,
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    /// Create a new `ResolvedEvents` adapter wrapping an upstream event source.
    #[must_use]
    pub fn new(upstream: I, resolver: R) -> Self {
        Self::new_with_config(upstream, resolver, &LoaderConfig::default())
    }

    /// Create a resolved-event adapter with the selected loader configuration.
    #[must_use]
    pub fn new_with_config(upstream: I, resolver: R, config: &LoaderConfig) -> Self {
        Self {
            upstream,
            resolver,
            buffer: VecDeque::new(),
            config: config.clone(),
            resolver_failed: false,
        }
    }
}

impl<'input, R, I> Iterator for ResolvedEvents<'input, R, I>
where
    R: Resolver<'input>,
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    type Item = Result<Event<'input>, ResolveError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.resolver_failed {
            return None;
        }

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

            match self
                .resolver
                .on_event_with_config(upstream_event, &self.config)
            {
                Ok(events) => {
                    let mut drain = events.into_iter();
                    let Some(first) = drain.next() else {
                        // Suppressed — try next upstream event.
                        continue;
                    };
                    // Buffer any extra events, yield the first.
                    self.buffer.extend(drain);
                    return Some(Ok(first));
                }
                Err(e) => {
                    self.resolver_failed = true;
                    return Some(Err(e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::Parser;
    use yamalgam_core::{LoaderConfig, ResolutionMode, ResourceLimit};

    fn map_parse_error<'input>(
        result: Result<Event<'input>, ParseError>,
    ) -> Result<Event<'input>, ResolveError> {
        result.map_err(ResolveError::Parse)
    }

    #[test]
    fn resolved_events_supports_boxed_and_concrete_upstreams() {
        let input = "key: [one, two]";
        let boxed_upstream: Box<dyn Iterator<Item = Result<Event<'_>, ResolveError>> + '_> =
            Box::new(Parser::new(input).map(map_parse_error));
        let boxed: ResolvedEvents<'_, NoopResolver> =
            ResolvedEvents::new(boxed_upstream, NoopResolver);
        let typed = ResolvedEvents::new(Parser::new(input).map(map_parse_error), NoopResolver);

        let boxed = boxed.collect::<Result<Vec<_>, _>>().unwrap();
        let typed = typed.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(typed, boxed);
    }

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
    fn noop_resolver_rejects_enabled_resolution_policy_once() {
        let mut config = LoaderConfig::moderate();
        config.resolution.include.mode = ResolutionMode::Explicit;
        config.resolution.refs.mode = ResolutionMode::Automatic;
        let parser = Parser::with_config("value", &config);
        let mut events = ResolvedEvents::new_with_config(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            NoopResolver,
            &config,
        );

        assert!(matches!(
            events.next(),
            Some(Err(ResolveError::UnsupportedResolutionPolicy {
                include: true,
                refs: true,
            }))
        ));
        assert!(events.next().is_none());
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

    #[test]
    fn structured_clone_preserves_resolver_error_variants() {
        let errors = [
            ResolveError::Include {
                path: PathBuf::from("missing.yaml"),
                source: io::Error::new(io::ErrorKind::NotFound, "missing fixture"),
            },
            ResolveError::Ref {
                target: "#/missing".into(),
                source: Box::new(io::Error::other("missing ref")),
            },
            ResolveError::Cycle {
                chain: vec!["a".into(), "b".into(), "a".into()],
            },
            ResolveError::LimitExceeded(LimitExceeded::new(ResourceLimit::Depth, 2, 1)),
            ResolveError::UnsupportedResolutionPolicy {
                include: true,
                refs: false,
            },
            ResolveError::Custom(Box::new(io::Error::other("custom resolver"))),
        ];

        for error in errors {
            let message = error.to_string();
            let cloned = error.clone_structured();
            assert_eq!(cloned.to_string(), message);
            assert_eq!(
                std::mem::discriminant(&cloned),
                std::mem::discriminant(&error)
            );
        }

        let parse = ResolveError::Parse(ParseError::LimitExceeded(LimitExceeded::new(
            ResourceLimit::Depth,
            2,
            1,
        )));
        assert!(matches!(
            parse.clone_structured(),
            ResolveError::Parse(ParseError::LimitExceeded(error))
                if error.limit == ResourceLimit::Depth
                    && error.observed == 2
                    && error.maximum == 1
        ));

        let include = ResolveError::Include {
            path: PathBuf::from("missing.yaml"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(matches!(
            include.clone_structured(),
            ResolveError::Include { source, .. }
                if source.kind() == io::ErrorKind::PermissionDenied
        ));
    }

    struct ConfigProbe {
        observed_depth: Rc<Cell<Option<usize>>>,
    }

    impl<'input> Resolver<'input> for ConfigProbe {
        fn on_event(
            &mut self,
            event: Event<'input>,
        ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
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

    #[test]
    fn resolved_events_pass_selected_config_to_resolver() {
        let mut config = LoaderConfig::strict();
        config.limits.max_depth = Some(7);
        let observed_depth = Rc::new(Cell::new(None));
        let parser = Parser::with_config("value", &config);

        let events = ResolvedEvents::new_with_config(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            ConfigProbe {
                observed_depth: Rc::clone(&observed_depth),
            },
            &config,
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        assert!(!events.is_empty());
        assert_eq!(observed_depth.get(), Some(7));
    }

    #[test]
    fn custom_resolver_accepts_enabled_resolution_policy() {
        let mut config = LoaderConfig::moderate();
        config.resolution.include.mode = ResolutionMode::Explicit;
        let observed_depth = Rc::new(Cell::new(None));
        let parser = Parser::with_config("value", &config);

        let events = ResolvedEvents::new_with_config(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            ConfigProbe {
                observed_depth: Rc::clone(&observed_depth),
            },
            &config,
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        assert!(!events.is_empty());
        assert_eq!(observed_depth.get(), config.limits.max_depth);
    }
}
