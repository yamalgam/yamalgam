//! Composer — builds a lossy `Value` DOM from resolved parser events.
//!
//! The composer consumes a stream of [`Event`]s (typically produced by a
//! [`Parser`] wrapped in [`ResolvedEvents`]) and assembles one or more
//! [`Value`] documents. Anchors and aliases are resolved during composition,
//! and merge keys (`<<`) are handled per the YAML merge-key spec.
#![deny(unsafe_code)]

use std::collections::HashMap;

use yamalgam_core::tag::Yaml12TagResolver;
use yamalgam_core::tag_resolution::TagResolver;
use yamalgam_core::{Mapping, ResourceLimits, Value};
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
    anchors: HashMap<String, Value>,
    config: ResourceLimits,
    alias_expansion_count: usize,
    tag_resolver: Box<dyn TagResolver>,
}

impl<'input> Composer<'input, ResolvedEvents<'input, NoopResolver>> {
    /// Parse and compose all documents from a YAML string using the default
    /// (no-op) resolver.
    ///
    /// This is the simplest entry point — equivalent to creating a [`Parser`],
    /// wrapping it in [`ResolvedEvents`] with [`NoopResolver`], and calling
    /// [`compose_stream`](Composer::compose_stream).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'input str) -> Result<Vec<Value>, ComposeError> {
        Self::with_resolver(input, NoopResolver)
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

    /// Parse and compose all documents using a custom tag resolver.
    pub fn from_str_with_tag_resolver(
        input: &'input str,
        tag_resolver: impl TagResolver + 'static,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: ResourceLimits::none(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(tag_resolver),
        };
        composer.compose_stream()
    }
}

impl<'input, R: Resolver<'input> + 'input> Composer<'input, ResolvedEvents<'input, R>> {
    /// Compose documents from a YAML string using a custom resolver.
    ///
    /// The resolver intercepts every parser event before it reaches the
    /// composer, enabling custom tag handling, `!include` expansion,
    /// or any other event-level transformation.
    pub fn with_resolver(input: &'input str, resolver: R) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            resolver,
        );
        let mut composer = Composer::new(events);
        composer.compose_stream()
    }
}

impl<'input, I> Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    /// Create a composer from any resolved event iterator (no resource limits).
    #[must_use]
    pub fn new(events: I) -> Self {
        Self {
            events: events.peekable(),
            // y[impl model.serial.anchor-definition]
            // y[impl model.serial.alias-resolution]
            // y[impl model.serial.tree-definition]
            anchors: HashMap::new(),
            config: ResourceLimits::none(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(Yaml12TagResolver),
        }
    }

    /// Create a composer with resource limits from a [`LoaderConfig`].
    #[must_use]
    pub fn new_with_config(events: I, config: &yamalgam_core::LoaderConfig) -> Self {
        Self {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: config.limits.clone(),
            alias_expansion_count: 0,
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
                    let value = self.compose_node()?;

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

        self.compose_node()
    }

    /// Compose a single YAML node from the event stream.
    // y[impl model.repr.node-definition]
    // y[impl model.repr.tag-definition]
    // y[impl flow.alias.error-undefined-anchor]
    // y[impl flow.alias.must-anchor-first]
    // y[impl flow.alias.must-not-specify-properties]
    fn compose_node(&mut self) -> Result<Value, ComposeError> {
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
                    self.anchors.insert(name.into_owned(), resolved.clone());
                    if let Err(msg) = self.config.check_anchor_count(self.anchors.len()) {
                        return Err(ComposeError::LimitExceeded(msg));
                    }
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
                    items.push(self.compose_node()?);
                }
                let value = Value::Sequence(items);
                if let Some(name) = anchor {
                    self.anchors.insert(name.into_owned(), value.clone());
                    if let Err(msg) = self.config.check_anchor_count(self.anchors.len()) {
                        return Err(ComposeError::LimitExceeded(msg));
                    }
                }
                Ok(value)
            }

            Event::MappingStart { anchor, .. } => {
                let mut map = Mapping::new();
                let mut merge_pairs: Vec<(Value, Value)> = Vec::new();

                loop {
                    let Some(peeked) = self.peek_event()? else {
                        return Err(ComposeError::UnexpectedEvent("unterminated mapping".into()));
                    };
                    if matches!(peeked, Event::MappingEnd { .. }) {
                        self.next_event()?;
                        break;
                    }

                    let key = self.compose_node()?;
                    let val = self.compose_node()?;

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
                    }
                    // Explicit keys override merged ones.
                    for (k, v) in map.iter() {
                        merged.insert(k.clone(), v.clone());
                    }
                    map = merged;
                }

                let value = Value::Mapping(map);
                if let Some(name) = anchor {
                    self.anchors.insert(name.into_owned(), value.clone());
                    if let Err(msg) = self.config.check_anchor_count(self.anchors.len()) {
                        return Err(ComposeError::LimitExceeded(msg));
                    }
                }
                Ok(value)
            }

            Event::Alias { name, .. } => {
                self.alias_expansion_count += 1;
                if let Err(msg) = self
                    .config
                    .check_alias_expansions(self.alias_expansion_count)
                {
                    return Err(ComposeError::LimitExceeded(msg));
                }
                let name_str = name.into_owned();
                self.anchors
                    .get(&name_str)
                    .cloned()
                    .ok_or(ComposeError::UndefinedAlias(name_str))
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
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

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
                pairs.push((k.clone(), v.clone()));
            }
            Ok(())
        }
        Value::Sequence(seq) => {
            for item in seq {
                match item {
                    Value::Mapping(m) => {
                        for (k, v) in m.iter() {
                            pairs.push((k.clone(), v.clone()));
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

/// Parse a YAML string into a list of [`Value`] documents.
pub fn from_str(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str(input)
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

    fn compose(input: &str) -> Vec<Value> {
        Composer::from_str(input).unwrap()
    }

    fn compose_one(input: &str) -> Value {
        let docs = compose(input);
        assert_eq!(docs.len(), 1, "expected 1 document, got {}", docs.len());
        docs.into_iter().next().unwrap()
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
