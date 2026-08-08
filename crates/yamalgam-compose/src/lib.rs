//! Composer — builds a lossy `Value` DOM from resolved parser events.
//!
//! The composer consumes a stream of [`Event`]s (typically produced by a
//! [`Parser`] wrapped in [`ResolvedEvents`]) and assembles one or more
//! [`Value`] documents. Anchors and aliases are resolved during composition,
//! and merge keys (`<<`) are handled per the YAML merge-key spec.
#![deny(unsafe_code)]

use std::collections::HashMap;

use yamalgam_core::tag_resolution::TagResolver;
use yamalgam_core::{LoaderConfig, Mapping, ResourceLimits, Value};
use yamalgam_parser::{
    Event, NoopResolver, ParseError, Parser, ResolveError, ResolvedEvents, Resolver,
};
use yamalgam_scanner::ScalarStyle;

/// Errors that can occur during composition (event-to-Value conversion).
#[derive(Debug)]
pub enum ComposeError {
    /// An error propagated from the resolver layer.
    Resolve(ResolveError),
    /// An alias references an anchor that has not been defined.
    UndefinedAlias(String),
    /// An event was encountered that is not valid in the current context.
    UnexpectedEvent(String),
    /// A resource limit was exceeded during composition.
    LimitExceeded(String),
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(e) => write!(f, "{e}"),
            Self::UndefinedAlias(name) => write!(f, "undefined alias: *{name}"),
            Self::UnexpectedEvent(msg) => write!(f, "unexpected event: {msg}"),
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
        }
    }
}

impl std::error::Error for ComposeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolve(e) => Some(e),
            Self::UndefinedAlias(_) | Self::UnexpectedEvent(_) | Self::LimitExceeded(_) => None,
        }
    }
}

impl From<ResolveError> for ComposeError {
    fn from(e: ResolveError) -> Self {
        Self::Resolve(e)
    }
}

impl From<ParseError> for ComposeError {
    fn from(e: ParseError) -> Self {
        Self::Resolve(ResolveError::Parse(e))
    }
}

struct AnchoredValue {
    value: Value,
    materialized_weight: usize,
}

/// Builds [`Value`] documents from a resolved event stream.
///
/// The composer consumes events one at a time, tracking anchors for alias
/// resolution. Each call to [`compose_stream`](Composer::compose_stream)
/// produces a `Vec<Value>` — one entry per YAML document in the stream.
pub struct Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    events: std::iter::Peekable<I>,
    anchors: HashMap<String, AnchoredValue>,
    config: ResourceLimits,
    alias_expansion_count: usize,
    alias_materialized_bytes: usize,
    tag_resolver: Box<dyn TagResolver>,
}

impl<'input> Composer<'input, ResolvedEvents<'input, NoopResolver>> {
    /// Parse and compose all documents from a YAML string using the default
    /// (no-op) resolver.
    ///
    /// This is the simplest entry point — equivalent to creating a [`Parser`],
    /// wrapping it in [`ResolvedEvents`] with [`NoopResolver`], and calling
    /// [`compose_stream`](Composer::compose_stream). It applies
    /// [`LoaderConfig::default`] resource limits.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'input str) -> Result<Vec<Value>, ComposeError> {
        Self::with_resolver(input, NoopResolver)
    }

    /// Parse and compose all documents with no resource limits.
    ///
    /// Use only for trusted input. Prefer [`Self::from_str`] for ordinary
    /// input.
    pub fn from_str_unchecked(input: &'input str) -> Result<Vec<Value>, ComposeError> {
        Self::with_resolver_unchecked(input, NoopResolver)
    }

    /// Parse and compose all documents from a YAML string with resource limits.
    pub fn from_str_with_config(
        input: &'input str,
        config: &yamalgam_core::LoaderConfig,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::with_config(input, config);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer::new_with_config(events, config);
        composer.compose_stream()
    }

    /// Parse and compose all documents using a custom tag resolver and
    /// [`LoaderConfig::default`] resource limits.
    pub fn from_str_with_tag_resolver(
        input: &'input str,
        tag_resolver: impl TagResolver + 'static,
    ) -> Result<Vec<Value>, ComposeError> {
        Self::from_str_with_tag_resolver_and_config(input, tag_resolver, &LoaderConfig::default())
    }

    /// Parse and compose all documents with a custom tag resolver and no
    /// resource limits.
    ///
    /// Use only for trusted input. Prefer
    /// [`Self::from_str_with_tag_resolver`] for ordinary input.
    pub fn from_str_with_tag_resolver_unchecked(
        input: &'input str,
        tag_resolver: impl TagResolver + 'static,
    ) -> Result<Vec<Value>, ComposeError> {
        Self::from_str_with_tag_resolver_and_config(input, tag_resolver, &LoaderConfig::unchecked())
    }

    fn from_str_with_tag_resolver_and_config(
        input: &'input str,
        tag_resolver: impl TagResolver + 'static,
        config: &LoaderConfig,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::with_config(input, config);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer::new_with_config(events, config);
        composer.tag_resolver = Box::new(tag_resolver);
        composer.compose_stream()
    }
}

impl<'input, R: Resolver<'input> + 'input> Composer<'input, ResolvedEvents<'input, R>> {
    /// Compose documents from a YAML string using a custom resolver.
    ///
    /// The resolver intercepts every parser event before it reaches the
    /// composer, enabling custom tag handling, `!include` expansion,
    /// or any other event-level transformation. This applies
    /// [`LoaderConfig::default`] resource limits.
    pub fn with_resolver(input: &'input str, resolver: R) -> Result<Vec<Value>, ComposeError> {
        Self::with_resolver_and_config(input, resolver, &LoaderConfig::default())
    }

    /// Compose documents with a custom event resolver and no resource limits.
    ///
    /// Use only for trusted input. Prefer [`Self::with_resolver`] for ordinary
    /// input.
    pub fn with_resolver_unchecked(
        input: &'input str,
        resolver: R,
    ) -> Result<Vec<Value>, ComposeError> {
        Self::with_resolver_and_config(input, resolver, &LoaderConfig::unchecked())
    }

    fn with_resolver_and_config(
        input: &'input str,
        resolver: R,
        config: &LoaderConfig,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::with_config(input, config);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            resolver,
        );
        let mut composer = Composer::new_with_config(events, config);
        composer.compose_stream()
    }
}

impl<'input, I> Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    /// Create a composer from any resolved event iterator with
    /// [`LoaderConfig::default`] resource limits.
    #[must_use]
    pub fn new(events: I) -> Self {
        Self::new_with_config(events, &LoaderConfig::default())
    }

    /// Create a composer from an event iterator with no resource limits.
    ///
    /// Use only for trusted events. Prefer [`Self::new`] for ordinary input.
    #[must_use]
    pub fn new_unchecked(events: I) -> Self {
        Self::new_with_config(events, &LoaderConfig::unchecked())
    }

    /// Create a composer with resource limits from a [`LoaderConfig`].
    #[must_use]
    pub fn new_with_config(events: I, config: &yamalgam_core::LoaderConfig) -> Self {
        Self {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: config.limits.clone(),
            alias_expansion_count: 0,
            alias_materialized_bytes: 0,
            tag_resolver: Box::new(config.tag_resolution),
        }
    }

    /// Compose all documents from the event stream.
    ///
    /// Returns one [`Value`] per YAML document. An empty input produces an
    /// empty vec.
    // y[impl model.loading.well-formed]
    // y[impl model.loading.reject-ill-formed]
    pub fn compose_stream(&mut self) -> Result<Vec<Value>, ComposeError> {
        // Consume StreamStart.
        self.expect_event(|e| matches!(e, Event::StreamStart), "StreamStart")?;

        let mut docs = Vec::new();

        loop {
            let Some(peeked) = self.peek_event()? else {
                break;
            };

            match peeked {
                Event::StreamEnd => {
                    self.next_event()?;
                    break;
                }
                Event::VersionDirective { .. } | Event::TagDirective { .. } => {
                    // Skip directives.
                    self.next_event()?;
                }
                Event::DocumentStart { .. } => {
                    // Explicit document.
                    self.next_event()?;
                    self.anchors.clear();

                    let value = self.compose_document_content()?;

                    // Consume DocumentEnd.
                    self.consume_document_end()?;
                    docs.push(value);
                }
                _ => {
                    // Implicit document.
                    self.anchors.clear();
                    let value = self.compose_node(0)?;

                    // Consume optional DocumentEnd.
                    if let Some(Event::DocumentEnd { .. }) = self.peek_event()? {
                        self.next_event()?;
                    }
                    docs.push(value);
                }
            }
        }

        Ok(docs)
    }

    /// Compose the content of an explicit document (after DocumentStart).
    ///
    /// If the next event is DocumentEnd, the document is empty (Null).
    fn compose_document_content(&mut self) -> Result<Value, ComposeError> {
        let Some(peeked) = self.peek_event()? else {
            return Ok(Value::Null);
        };

        if matches!(peeked, Event::DocumentEnd { .. }) {
            return Ok(Value::Null);
        }

        self.compose_node(0)
    }

    /// Compose a single YAML node from the event stream.
    // y[impl model.repr.node-definition]
    // y[impl model.repr.tag-definition]
    // y[impl flow.alias.error-undefined-anchor]
    // y[impl flow.alias.must-anchor-first]
    // y[impl flow.alias.must-not-specify-properties]
    fn compose_node(&mut self, depth: usize) -> Result<Value, ComposeError> {
        self.config
            .check_depth(depth)
            .map_err(ComposeError::LimitExceeded)?;
        let event = self
            .next_event()?
            .ok_or_else(|| ComposeError::UnexpectedEvent("unexpected end of events".into()))?;

        match event {
            Event::Scalar {
                anchor,
                value,
                style,
                ..
            } => {
                let resolved = self.resolve_scalar(&value, style);
                if let Some(name) = anchor {
                    self.remember_anchor(name.into_owned(), &resolved)?;
                }
                Ok(resolved)
            }

            Event::SequenceStart { anchor, .. } => {
                let mut items = Vec::new();
                loop {
                    let Some(peeked) = self.peek_event()? else {
                        return Err(ComposeError::UnexpectedEvent(
                            "unterminated sequence".into(),
                        ));
                    };
                    if matches!(peeked, Event::SequenceEnd { .. }) {
                        self.next_event()?;
                        break;
                    }
                    self.next_collection_size(items.len())?;
                    items.push(self.compose_node(child_depth(depth)?)?);
                }
                let value = Value::Sequence(items);
                if let Some(name) = anchor {
                    self.remember_anchor(name.into_owned(), &value)?;
                }
                Ok(value)
            }

            Event::MappingStart { anchor, .. } => {
                let mut map = Mapping::new();
                let mut merge_pairs: Vec<(Value, Value)> = Vec::new();
                let mut pair_count = 0_usize;

                loop {
                    let Some(peeked) = self.peek_event()? else {
                        return Err(ComposeError::UnexpectedEvent("unterminated mapping".into()));
                    };
                    if matches!(peeked, Event::MappingEnd { .. }) {
                        self.next_event()?;
                        break;
                    }

                    pair_count = self.next_collection_size(pair_count)?;
                    let nested_depth = child_depth(depth)?;
                    let key = self.compose_node(nested_depth)?;
                    let val = self.compose_node(nested_depth)?;

                    // Merge key handling: `<<` key.
                    if is_merge_key(&key) {
                        collect_merge_pairs(&val, &mut merge_pairs, &self.config, 0)?;
                    } else {
                        map.insert(key, val);
                    }
                }

                // Apply merge pairs (explicit keys take precedence).
                if !merge_pairs.is_empty() {
                    let mut merged = Mapping::new();
                    for (k, v) in merge_pairs {
                        merged.insert(k, v);
                        self.config
                            .check_collection_size(merged.len())
                            .map_err(ComposeError::LimitExceeded)?;
                    }
                    // Explicit keys override merged ones.
                    for (k, v) in map.iter() {
                        merged.insert(k.clone(), v.clone());
                        self.config
                            .check_collection_size(merged.len())
                            .map_err(ComposeError::LimitExceeded)?;
                    }
                    map = merged;
                }

                let value = Value::Mapping(map);
                if let Some(name) = anchor {
                    self.remember_anchor(name.into_owned(), &value)?;
                }
                Ok(value)
            }

            Event::Alias { name, .. } => {
                self.alias_expansion_count =
                    self.alias_expansion_count.checked_add(1).ok_or_else(|| {
                        ComposeError::LimitExceeded("alias expansion count overflow".into())
                    })?;
                if let Err(msg) = self
                    .config
                    .check_alias_expansions(self.alias_expansion_count)
                {
                    return Err(ComposeError::LimitExceeded(msg));
                }
                let name_str = name.into_owned();
                let anchored = self
                    .anchors
                    .get(&name_str)
                    .ok_or(ComposeError::UndefinedAlias(name_str))?;
                self.alias_materialized_bytes = checked_materialized_total(
                    self.alias_materialized_bytes,
                    anchored.materialized_weight,
                    &self.config,
                )?;
                Ok(anchored.value.clone())
            }

            other => Err(ComposeError::UnexpectedEvent(format!("{other:?}"))),
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Peek at the next non-structural event without consuming it.
    ///
    /// Skips Comment, BlockEntry, KeyIndicator, and ValueIndicator events.
    /// Returns `Ok(None)` when the stream is exhausted. If the peeked value
    /// is an error, it is consumed and returned immediately.
    fn peek_event(&mut self) -> Result<Option<&Event<'input>>, ComposeError> {
        // Consume and discard structural events until we find a semantic one.
        loop {
            // Determine action without holding a borrow.
            let action = match self.events.peek() {
                Some(Err(_)) => 1,                             // error
                Some(Ok(event)) if event.is_structural() => 2, // skip
                Some(Ok(_)) => 3,                              // return ref
                None => 0,                                     // exhausted
            };
            match action {
                0 => return Ok(None),
                1 => {
                    let err = self.events.next().unwrap().unwrap_err();
                    return Err(ComposeError::Resolve(err));
                }
                2 => {
                    self.events.next();
                }
                _ => {
                    // action == 3: peek again to return a fresh reference.
                    return Ok(self.events.peek().and_then(|r| r.as_ref().ok()));
                }
            }
        }
    }

    /// Consume and return the next non-structural event.
    ///
    /// Skips Comment, BlockEntry, KeyIndicator, and ValueIndicator events.
    fn next_event(&mut self) -> Result<Option<Event<'input>>, ComposeError> {
        loop {
            match self.events.next() {
                Some(Ok(event)) if event.is_structural() => continue,
                Some(Ok(event)) => return Ok(Some(event)),
                Some(Err(e)) => return Err(ComposeError::Resolve(e)),
                None => return Ok(None),
            }
        }
    }

    /// Consume the next event, verifying it matches the predicate.
    fn expect_event(
        &mut self,
        predicate: impl Fn(&Event<'input>) -> bool,
        expected: &str,
    ) -> Result<Event<'input>, ComposeError> {
        let event = self.next_event()?.ok_or_else(|| {
            ComposeError::UnexpectedEvent(format!("expected {expected}, got EOF"))
        })?;
        if !predicate(&event) {
            return Err(ComposeError::UnexpectedEvent(format!(
                "expected {expected}, got {event:?}"
            )));
        }
        Ok(event)
    }

    /// Consume a `DocumentEnd` event (explicit or implicit).
    fn consume_document_end(&mut self) -> Result<(), ComposeError> {
        if let Some(Event::DocumentEnd { .. }) = self.peek_event()? {
            self.next_event()?;
        }
        Ok(())
    }

    /// Resolve a scalar value based on its style.
    ///
    /// Plain scalars are dispatched to the configured [`TagResolver`].
    /// All other styles produce strings.
    fn resolve_scalar(&self, value: &str, style: ScalarStyle) -> Value {
        match style {
            ScalarStyle::Plain => self.tag_resolver.resolve_scalar(value),
            _ => Value::String(value.to_owned()),
        }
    }

    fn next_collection_size(&self, current: usize) -> Result<usize, ComposeError> {
        let next = current
            .checked_add(1)
            .ok_or_else(|| ComposeError::LimitExceeded("collection size overflow".into()))?;
        self.config
            .check_collection_size(next)
            .map_err(ComposeError::LimitExceeded)?;
        Ok(next)
    }

    fn remember_anchor(&mut self, name: String, value: &Value) -> Result<(), ComposeError> {
        let materialized_weight = materialized_weight(value)?;
        self.anchors.insert(
            name,
            AnchoredValue {
                value: value.clone(),
                materialized_weight,
            },
        );
        self.config
            .check_anchor_count(self.anchors.len())
            .map_err(ComposeError::LimitExceeded)
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn child_depth(depth: usize) -> Result<usize, ComposeError> {
    depth
        .checked_add(1)
        .ok_or_else(|| ComposeError::LimitExceeded("depth overflow".into()))
}

fn materialized_weight(value: &Value) -> Result<usize, ComposeError> {
    let mut total = 0_usize;
    let mut pending = vec![value];

    while let Some(value) = pending.pop() {
        total = total
            .checked_add(std::mem::size_of::<Value>())
            .ok_or_else(|| {
                ComposeError::LimitExceeded("anchor materialized weight overflow".into())
            })?;
        match value {
            Value::String(string) => {
                total = total.checked_add(string.len()).ok_or_else(|| {
                    ComposeError::LimitExceeded("anchor materialized weight overflow".into())
                })?;
            }
            Value::Sequence(items) => pending.extend(items),
            Value::Mapping(mapping) => {
                for (key, value) in mapping.iter() {
                    pending.push(key);
                    pending.push(value);
                }
            }
            Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Float(_) => {}
        }
    }

    Ok(total)
}

fn checked_materialized_total(
    current: usize,
    increment: usize,
    limits: &ResourceLimits,
) -> Result<usize, ComposeError> {
    let next = current
        .checked_add(increment)
        .ok_or_else(|| ComposeError::LimitExceeded("alias materialization size overflow".into()))?;
    limits
        .check_output_size(next)
        .map_err(ComposeError::LimitExceeded)?;
    Ok(next)
}

/// Check whether a value is the merge key sentinel `<<`.
fn is_merge_key(key: &Value) -> bool {
    matches!(key, Value::String(s) if s == "<<")
}

/// Collect key-value pairs from a merge source into `pairs`.
///
/// If `val` is a Mapping, its entries are collected. If `val` is a Sequence of
/// Mappings, all entries from all mappings are collected (in order). Non-mapping
/// values are rejected as errors per the YAML merge-key spec.
///
/// The `depth` parameter tracks recursion into sequences; `config` enforces
/// [`max_merge_depth`](ResourceLimits::max_merge_depth).
fn collect_merge_pairs(
    val: &Value,
    pairs: &mut Vec<(Value, Value)>,
    config: &ResourceLimits,
    depth: usize,
) -> Result<(), ComposeError> {
    if let Err(msg) = config.check_merge_depth(depth) {
        return Err(ComposeError::LimitExceeded(msg));
    }
    match val {
        Value::Mapping(m) => {
            for (k, v) in m.iter() {
                push_merge_pair(pairs, k, v, config)?;
            }
            Ok(())
        }
        Value::Sequence(seq) => {
            for item in seq {
                match item {
                    Value::Mapping(m) => {
                        for (k, v) in m.iter() {
                            push_merge_pair(pairs, k, v, config)?;
                        }
                    }
                    other => {
                        return Err(ComposeError::UnexpectedEvent(format!(
                            "merge key (<<) sequence item must be a mapping, got {other:?}"
                        )));
                    }
                }
            }
            Ok(())
        }
        _ => Err(ComposeError::UnexpectedEvent(
            "merge key (<<) value must be a mapping or sequence of mappings".into(),
        )),
    }
}

fn push_merge_pair(
    pairs: &mut Vec<(Value, Value)>,
    key: &Value,
    value: &Value,
    config: &ResourceLimits,
) -> Result<(), ComposeError> {
    let next = pairs
        .len()
        .checked_add(1)
        .ok_or_else(|| ComposeError::LimitExceeded("collection size overflow".into()))?;
    config
        .check_collection_size(next)
        .map_err(ComposeError::LimitExceeded)?;
    pairs.push((key.clone(), value.clone()));
    Ok(())
}

/// Parse a YAML string into a list of [`Value`] documents.
pub fn from_str(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str(input)
}

/// Parse a YAML string into documents with no resource limits.
///
/// Use only for trusted input. Prefer [`from_str`] for ordinary input.
pub fn from_str_unchecked(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str_unchecked(input)
}

/// Parse a YAML string into a single [`Value`].
/// Returns Null for empty input, error for multiple documents.
pub fn from_str_single(input: &str) -> Result<Value, ComposeError> {
    let mut docs = Composer::from_str(input)?;
    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.remove(0)),
        n => Err(ComposeError::UnexpectedEvent(format!(
            "expected 1 document, got {n}",
        ))),
    }
}

/// Parse one YAML document with no resource limits.
///
/// Use only for trusted input. Prefer [`from_str_single`] for ordinary input.
pub fn from_str_single_unchecked(input: &str) -> Result<Value, ComposeError> {
    let mut docs = Composer::from_str_unchecked(input)?;
    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.remove(0)),
        n => Err(ComposeError::UnexpectedEvent(format!(
            "expected 1 document, got {n}",
        ))),
    }
}

/// Parse a YAML string into a list of [`Value`] documents with resource limits.
pub fn from_str_with_config(
    input: &str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str_with_config(input, config)
}

/// Parse a YAML string into a single [`Value`] with resource limits.
pub fn from_str_single_with_config(
    input: &str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<Value, ComposeError> {
    let mut docs = Composer::from_str_with_config(input, config)?;
    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.remove(0)),
        n => Err(ComposeError::UnexpectedEvent(format!(
            "expected 1 document, got {n}",
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const DEEP_WEIGHT_WORKER: &str = "YAMALGAM_DEEP_WEIGHT_WORKER";

    fn nested_flow_sequence(depth: usize) -> String {
        format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
    }

    fn assert_limit_error(result: Result<Vec<Value>, ComposeError>) {
        let err = result.expect_err("expected a resource-limit error");
        match err {
            ComposeError::LimitExceeded(_)
            | ComposeError::Resolve(ResolveError::LimitExceeded(_))
            | ComposeError::Resolve(ResolveError::Parse(ParseError::LimitExceeded(_))) => {}
            ComposeError::Resolve(ResolveError::Parse(ParseError::Scan(error)))
                if error.kind == yamalgam_scanner::scanner::ScanErrorKind::LimitExceeded => {}
            other => panic!("expected a resource-limit error, got {other:?}"),
        }
    }

    fn compose(input: &str) -> Vec<Value> {
        Composer::from_str(input).unwrap()
    }

    fn compose_one(input: &str) -> Value {
        let docs = compose(input);
        assert_eq!(docs.len(), 1, "expected 1 document, got {}", docs.len());
        docs.into_iter().next().unwrap()
    }

    #[test]
    fn ordinary_string_entry_points_use_moderate_limits() {
        let input = nested_flow_sequence(257);

        assert_limit_error(Composer::from_str(&input));
        assert_limit_error(Composer::with_resolver(&input, NoopResolver));
        assert_limit_error(Composer::from_str_with_tag_resolver(
            &input,
            yamalgam_core::tag::Yaml12TagResolver,
        ));

        let parser = Parser::new(&input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        assert_limit_error(Composer::new(events).compose_stream());
    }

    #[test]
    fn unchecked_string_entry_points_preserve_unlimited_behavior() {
        let input = nested_flow_sequence(257);

        assert!(Composer::from_str_unchecked(&input).is_ok());
        assert!(Composer::with_resolver_unchecked(&input, NoopResolver).is_ok());
        assert!(
            Composer::from_str_with_tag_resolver_unchecked(
                &input,
                yamalgam_core::tag::Yaml12TagResolver,
            )
            .is_ok()
        );
        assert!(from_str_unchecked(&input).is_ok());
        assert!(from_str_single_unchecked(&input).is_ok());

        let parser = Parser::new(&input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        assert!(Composer::new_unchecked(events).compose_stream().is_ok());
    }

    fn compose_with_unchecked_parser(
        input: &str,
        config: &LoaderConfig,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        Composer::new_with_config(events, config).compose_stream()
    }

    fn collection_config(max: usize) -> LoaderConfig {
        LoaderConfig {
            limits: ResourceLimits {
                max_collection_size: Some(max),
                ..ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        }
    }

    fn output_config(max_output_bytes: Option<usize>) -> LoaderConfig {
        LoaderConfig {
            limits: ResourceLimits {
                max_alias_expansions: None,
                max_output_bytes,
                ..ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        }
    }

    #[test]
    fn composer_honors_custom_depth_at_the_exact_boundary() {
        let config = LoaderConfig {
            limits: ResourceLimits {
                max_depth: Some(1),
                ..ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        };

        assert!(compose_with_unchecked_parser("[0]", &config).is_ok());
        assert!(matches!(
            compose_with_unchecked_parser("[[0]]", &config),
            Err(ComposeError::LimitExceeded(_))
        ));
    }

    #[test]
    fn composer_enforces_sequence_size_for_unchecked_event_sources() {
        assert!(matches!(
            compose_with_unchecked_parser("[1, 2, 3]", &collection_config(2)),
            Err(ComposeError::LimitExceeded(_))
        ));
    }

    #[test]
    fn composer_counts_duplicate_mapping_entries_toward_the_limit() {
        assert!(matches!(
            compose_with_unchecked_parser("{a: 1, a: 2, a: 3}", &collection_config(2)),
            Err(ComposeError::LimitExceeded(_))
        ));
    }

    #[test]
    fn composer_limits_materialized_merge_entries() {
        let input = "one: &one {a: 1, b: 2}\ntwo: &two {c: 3, d: 4}\nout: {<<: [*one, *two]}";
        assert!(matches!(
            compose_with_unchecked_parser(input, &collection_config(3)),
            Err(ComposeError::LimitExceeded(_))
        ));
    }

    #[test]
    fn materialized_weight_counts_value_storage_and_string_bytes() {
        let value = Value::Sequence(vec![Value::String("alpha".into()), Value::Integer(7)]);
        let expected = 3 * std::mem::size_of::<Value>() + "alpha".len();

        assert_eq!(materialized_weight(&value).unwrap(), expected);
    }

    #[test]
    fn alias_materialization_budget_honors_exact_boundary() {
        let input = "base: &base [payload]\none: *base\ntwo: *base";
        let anchored = Value::Sequence(vec![Value::String("payload".into())]);
        let weight = materialized_weight(&anchored).unwrap();

        assert!(compose_with_unchecked_parser(input, &output_config(Some(weight * 2))).is_ok());
        assert!(matches!(
            compose_with_unchecked_parser(input, &output_config(Some(weight * 2 - 1))),
            Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
        ));
    }

    #[test]
    fn alias_materialization_budget_is_size_weighted_and_cumulative() {
        let input = "small: &small x\nlarge: &large [one, two, three, four]\na: *small\nb: *large";
        let small = materialized_weight(&Value::String("x".into())).unwrap();
        let large = materialized_weight(&Value::Sequence(vec![
            Value::String("one".into()),
            Value::String("two".into()),
            Value::String("three".into()),
            Value::String("four".into()),
        ]))
        .unwrap();

        assert!(matches!(
            compose_with_unchecked_parser(input, &output_config(Some(small + large - 1))),
            Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
        ));
    }

    #[test]
    fn alias_materialization_arithmetic_overflow_is_a_limit_error() {
        assert!(matches!(
            checked_materialized_total(usize::MAX, 1, &ResourceLimits::none()),
            Err(ComposeError::LimitExceeded(message))
                if message == "alias materialization size overflow"
        ));
    }

    #[test]
    fn anchor_redefinition_replaces_value_and_cached_weight() {
        let input = "first: &ref x\none: *ref\nreplace: &ref [one, two, three]\ntwo: *ref";
        let small = materialized_weight(&Value::String("x".into())).unwrap();
        let redefined = Value::Sequence(vec![
            Value::String("one".into()),
            Value::String("two".into()),
            Value::String("three".into()),
        ]);
        let large = materialized_weight(&redefined).unwrap();
        let mut exact = output_config(Some(small + large));
        exact.limits.max_anchor_count = Some(1);
        let documents = compose_with_unchecked_parser(input, &exact).unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].get("two"), Some(&redefined));

        let mut below = output_config(Some(small + large - 1));
        below.limits.max_anchor_count = Some(1);
        assert!(matches!(
            compose_with_unchecked_parser(input, &below),
            Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
        ));
    }

    #[test]
    fn aliases_cannot_resolve_anchors_from_a_previous_document() {
        let input = "---\na: &ref value\n---\nb: *ref";
        assert!(matches!(
            compose_with_unchecked_parser(input, &output_config(None)),
            Err(ComposeError::UndefinedAlias(name)) if name == "ref"
        ));
    }

    #[test]
    fn alias_materialization_bytes_are_cumulative_across_documents() {
        let input = "---\na: &ref [payload]\nb: *ref\n---\nc: &ref [payload]\nd: *ref";
        let anchored = Value::Sequence(vec![Value::String("payload".into())]);
        let weight = materialized_weight(&anchored).unwrap();

        assert!(compose_with_unchecked_parser(input, &output_config(Some(weight * 2))).is_ok());
        assert!(matches!(
            compose_with_unchecked_parser(input, &output_config(Some(weight * 2 - 1))),
            Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
        ));
    }

    #[test]
    fn unlimited_output_budget_allows_cumulative_alias_materialization() {
        let input = "base: &base [one, two, three]\na: *base\nb: *base\nc: *base";
        assert!(compose_with_unchecked_parser(input, &output_config(None)).is_ok());
    }

    #[test]
    fn materialized_weight_is_stack_safe_on_a_deep_public_value() {
        if std::env::var_os(DEEP_WEIGHT_WORKER).is_some() {
            let mut value = Value::Null;
            for _ in 0..100_000 {
                value = Value::Sequence(vec![value]);
            }
            assert_eq!(
                materialized_weight(&value).unwrap(),
                100_001 * std::mem::size_of::<Value>()
            );
            std::mem::forget(value);
            return;
        }

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .env(DEEP_WEIGHT_WORKER, "1")
            .args([
                "--exact",
                "tests::materialized_weight_is_stack_safe_on_a_deep_public_value",
                "--nocapture",
            ])
            .output()
            .expect("run deep materialized-weight worker");
        assert!(
            output.status.success(),
            "deep weight worker failed: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn empty_document() {
        let docs = compose("");
        assert!(docs.is_empty(), "expected no documents, got {docs:?}");
    }

    #[test]
    fn null_document() {
        assert_eq!(compose_one("~"), Value::Null);
        assert_eq!(compose_one("null"), Value::Null);
    }

    #[test]
    fn plain_scalar() {
        assert_eq!(compose_one("hello"), Value::String("hello".into()));
    }

    #[test]
    fn quoted_scalar_is_string() {
        assert_eq!(compose_one("'true'"), Value::String("true".into()));
        assert_eq!(compose_one("\"42\""), Value::String("42".into()));
    }

    #[test]
    fn integer_scalar() {
        assert_eq!(compose_one("42"), Value::Integer(42));
    }

    #[test]
    fn float_scalar() {
        assert_eq!(compose_one("1.5"), Value::Float(1.5));
    }

    #[test]
    fn bool_scalar() {
        assert_eq!(compose_one("true"), Value::Bool(true));
    }

    #[test]
    fn simple_sequence() {
        let val = compose_one("- a\n- b\n- c");
        assert_eq!(
            val,
            Value::Sequence(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ])
        );
    }

    #[test]
    fn simple_mapping() {
        let val = compose_one("name: Clay\nage: 25");
        assert_eq!(val.get("name"), Some(&Value::String("Clay".into())));
        assert_eq!(val.get("age"), Some(&Value::Integer(25)));
    }

    #[test]
    fn nested_mapping() {
        let val = compose_one("outer:\n  inner: value");
        let outer = val.get("outer").expect("missing 'outer'");
        assert_eq!(outer.get("inner"), Some(&Value::String("value".into())));
    }

    #[test]
    fn sequence_of_mappings() {
        let val = compose_one("- name: Alice\n  val: 1\n- name: Bob\n  val: 2");
        let seq = match &val {
            Value::Sequence(s) => s,
            other => panic!("expected sequence, got {other:?}"),
        };
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].get("name"), Some(&Value::String("Alice".into())));
        assert_eq!(seq[0].get("val"), Some(&Value::Integer(1)));
        assert_eq!(seq[1].get("name"), Some(&Value::String("Bob".into())));
        assert_eq!(seq[1].get("val"), Some(&Value::Integer(2)));
    }

    #[test]
    fn flow_sequence() {
        let val = compose_one("[1, 2, 3]");
        assert_eq!(
            val,
            Value::Sequence(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])
        );
    }

    #[test]
    fn flow_mapping() {
        let val = compose_one("{a: 1, b: 2}");
        assert_eq!(val.get("a"), Some(&Value::Integer(1)));
        assert_eq!(val.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn anchor_and_alias() {
        let val = compose_one("a: &anchor hello\nb: *anchor");
        assert_eq!(val.get("a"), Some(&Value::String("hello".into())));
        assert_eq!(val.get("b"), Some(&Value::String("hello".into())));
    }

    #[test]
    fn anchor_on_collection() {
        let val = compose_one("a: &items\n  - 1\n  - 2\nb: *items");
        let expected_seq = Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]);
        assert_eq!(val.get("a"), Some(&expected_seq));
        assert_eq!(val.get("b"), Some(&expected_seq));
    }

    #[test]
    fn multiple_documents() {
        let docs = compose("---\na: 1\n---\nb: 2");
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].get("a"), Some(&Value::Integer(1)));
        assert_eq!(docs[1].get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn empty_mapping_value() {
        let val = compose_one("key:");
        assert_eq!(val.get("key"), Some(&Value::Null));
    }

    #[test]
    fn block_literal_scalar() {
        let val = compose_one("text: |\n  line1\n  line2\n");
        let text = val.get("text").expect("missing 'text'");
        assert_eq!(text, &Value::String("line1\nline2\n".into()));
    }

    #[test]
    fn merge_key() {
        let input = "\
defaults: &defaults
  adapter: postgres
  host: localhost
production:
  <<: *defaults
  host: prod-server
  database: myapp";
        let val = compose_one(input);
        let prod = val.get("production").expect("missing 'production'");
        // Merged from defaults.
        assert_eq!(prod.get("adapter"), Some(&Value::String("postgres".into())));
        // Overridden by explicit key.
        assert_eq!(prod.get("host"), Some(&Value::String("prod-server".into())));
        assert_eq!(prod.get("database"), Some(&Value::String("myapp".into())));
    }

    #[test]
    fn undefined_alias_errors() {
        let result = Composer::from_str("a: *undefined");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ComposeError::UndefinedAlias(ref name) if name == "undefined"),
            "expected UndefinedAlias, got {err:?}"
        );
    }

    #[test]
    fn anchor_count_limit() {
        use yamalgam_core::LoaderConfig;
        let config = LoaderConfig {
            limits: yamalgam_core::ResourceLimits {
                max_anchor_count: Some(2),
                ..yamalgam_core::ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        };
        let parser = Parser::with_config("a: &a 1\nb: &b 2\nc: &c 3", &config);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer::new_with_config(events, &config);
        let result = composer.compose_stream();
        assert!(matches!(result, Err(ComposeError::LimitExceeded(_))));
    }

    #[test]
    fn alias_expansion_limit() {
        use yamalgam_core::LoaderConfig;
        let config = LoaderConfig {
            limits: yamalgam_core::ResourceLimits {
                max_alias_expansions: Some(1),
                ..yamalgam_core::ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        };
        let parser = Parser::with_config("a: &ref val\nb: *ref\nc: *ref", &config);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer::new_with_config(events, &config);
        let result = composer.compose_stream();
        assert!(matches!(result, Err(ComposeError::LimitExceeded(_))));
    }

    #[test]
    fn invalid_merge_value_errors() {
        let result = Composer::from_str("a: 1\n<<: 2");
        assert!(
            matches!(result, Err(ComposeError::UnexpectedEvent(_))),
            "expected error for non-mapping merge value, got {result:?}"
        );
    }

    #[test]
    fn invalid_merge_sequence_item_errors() {
        let result = Composer::from_str("<<:\n  - 42");
        assert!(
            matches!(result, Err(ComposeError::UnexpectedEvent(_))),
            "expected error for non-mapping item in merge sequence, got {result:?}"
        );
    }

    #[test]
    fn compose_with_failsafe_schema() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Failsafe);
        let docs = Composer::from_str_with_config("true", &config).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::String("true".into()));
    }

    #[test]
    fn compose_with_yaml11_booleans() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Yaml11);
        let docs = Composer::from_str_with_config("yes", &config).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::Bool(true));
    }

    #[test]
    fn compose_with_json_schema() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Json);
        let docs = Composer::from_str_with_config("True", &config).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::String("True".into()));
    }

    #[test]
    fn compose_default_is_yaml12() {
        let docs = Composer::from_str("yes").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::String("yes".into()));
    }

    #[test]
    fn compose_with_custom_tag_resolver() {
        use yamalgam_core::tag_resolution::TagResolver;

        struct MagicResolver;
        impl TagResolver for MagicResolver {
            fn resolve_scalar(&self, value: &str) -> Value {
                if value == "MAGIC" {
                    Value::Integer(42)
                } else {
                    Value::String(value.to_owned())
                }
            }
        }

        let docs = Composer::from_str_with_tag_resolver("MAGIC", MagicResolver).unwrap();
        assert_eq!(docs[0], Value::Integer(42));

        let docs = Composer::from_str_with_tag_resolver("true", MagicResolver).unwrap();
        assert_eq!(docs[0], Value::String("true".into()));
    }
}
