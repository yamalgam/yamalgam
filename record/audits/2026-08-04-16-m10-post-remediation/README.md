---
audit_date: 2026-08-04
project: yamalgam
commit: a1123b4998722ad943980ef7525a26cc07a2b9ff
scope: Full Repository Post-Remediation
auditor: "Codex (GPT-5-generated, independently GPT-5-verified)"
verification:
  confirmed: 15
  adjusted: 12
  disputed: 1
findings:
  critical: 0
  significant: 8
  moderate: 4
  advisory: 7
  note: 8
---

# Audit: yamalgam — Full Repository Post-Remediation

This full-workspace audit at commit `a1123b4998722ad943980ef7525a26cc07a2b9ff` finds
strong parser primitives and explicit dependency policy, but inconsistent safe defaults
across public loading APIs.

**The Loading and Composition Surface** contains the dominant risk: composer, serde, and
CST conveniences bypass depth or resource controls, while owned-tree construction adds
width and alias multipliers.

**The Error Preservation Surface** loses structured failures at three boundaries, while
**The Public API and Type Surface** is broadly disciplined but retains allocation and
ergonomics costs.

**The Feature Completeness Surface** exposes resolver and configuration promises not yet
honored plus eight valid CST round-trip failures; **The Runtime Performance Surface**
has one structural tree-rewalk cost, and **The Supply Chain Surface** is clean outside
one benchmark-only advisory edge and two unused declarations.

Independent GPT-5 review confirmed 15 findings, adjusted 12, and disputed one that was
removed; the immediate priority is to make bounded loading structural across every
public entry point.

## The Loading and Composition Surface

*The workspace defines strong resource controls, but several public convenience paths
bypass them before recursively building owned trees.*

### composer-default-path-has-unbounded-recursion

Default composition recursively descends untrusted YAML without a depth bound

**significant** · criterion `11.5` · `crates/yamalgam-compose/src/lib.rs:272-285` · effort: medium

compose_node recursively descends every nested sequence and mapping without carrying
depth. The public from_str and from_str_single paths reach it through Parser::new and
Composer::new, both of which use ResourceLimits::none rather than LoaderConfig's
moderate default. Deeply nested external YAML can therefore grow the Rust call stack
until the process aborts; dropping the resulting recursive Value can repeat the same
failure. Even Composer::new_with_config relies on its event source to enforce max_depth
instead of enforcing the advertised limit locally.

```rust crates/yamalgam-compose/src/lib.rs:272-285
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
```

> A valid document only needs enough nesting to turn the convenience API's recursion
> into a process abort.

**Related:** [The default serde entry point explicitly disables every loader limit](#serde-default-disables-loader-limits), [CST traversal and destruction remain recursively stack-bound](#cst-recursive-traversals-have-no-depth-bound).

**Remediation:** Thread a depth argument through compose_node, call self.config.check_depth before
descending, and use checked_add for child depth. Route the convenience from_str paths
through LoaderConfig::default; retain unlimited behavior only behind an explicitly named
unchecked constructor.

### serde-default-disables-loader-limits

The default serde entry point explicitly disables every loader limit

**significant** · criterion `11.5` · `crates/yamalgam-serde/src/de.rs:52-59` · effort: small

yamalgam_serde::from_str and from_reader use this constructor. Nested collections drive
recursive serde visitors with no parser depth ceiling, while alias and anchor counters
are checked against an all-None ResourceLimits value. Adversarial YAML can consequently
exhaust the call stack or memory instead of returning Error::LimitExceeded. This
bypasses LoaderConfig::default, which is documented as the moderate safe profile.

```rust crates/yamalgam-serde/src/de.rs:52-59
    pub fn from_str(input: &'input str) -> Self {
        let parser = Parser::new(input);
        Self {
            events: (Box::new(parser) as Box<dyn Iterator<Item = _> + 'input>).peekable(),
            tag_resolver: Box::new(Yaml12TagResolver),
            limits: ResourceLimits::none(),
            finished: false,
            at_document_start: false,
```

> The safest-looking serde entry point is the unlimited one; deep nesting and alias
> amplification arrive through the ordinary API.

**Related:** [Default composition recursively descends untrusted YAML without a depth bound](#composer-default-path-has-unbounded-recursion), [CST traversal and destruction remain recursively stack-bound](#cst-recursive-traversals-have-no-depth-bound), [Serde from_reader erases I/O failures into the Custom data-error variant](#from-reader-erases-io-errors).

**Remediation:** Construct the parser and deserializer from LoaderConfig::default in from_str, and expose
the current behavior under an explicit from_str_unchecked API. Keep from_str_with_config
for callers selecting strict, trusted, or custom limits.

### cst-recursive-traversals-have-no-depth-bound

CST traversal and destruction remain recursively stack-bound

**significant** · criterion `11.5` · `crates/yamalgam-cst/src/lib.rs:141-152` · effort: medium

to_text, node_at_offset, and leaf_count recursively traverse CstNode, and the derived
tree destruction path is recursive as well. parse_to_cst accepts external text through
Parser::new, whose limits are disabled, and no config-aware convenience function exists.
A sufficiently deep valid document can therefore build a CST and then abort while
traversing, formatting, or merely dropping it.

```rust crates/yamalgam-cst/src/lib.rs:141-152
    pub fn to_text(&self) -> String {
        let mut buf = String::new();
        self.collect_text(&mut buf);
        buf
    }

    fn collect_text(&self, buf: &mut String) {
        for child in &self.children {
            match child {
                CstElement::Node(node) => node.collect_text(buf),
                CstElement::Token(token) => buf.push_str(&token.text),
            }
```

> Once the deep tree exists, I get several chances to exhaust the stack: traversal,
> formatting, or drop.

**Related:** [Default composition recursively descends untrusted YAML without a depth bound](#composer-default-path-has-unbounded-recursion), [The default serde entry point explicitly disables every loader limit](#serde-default-disables-loader-limits), [CST recovery discards the only structured parse error](#cst-discards-parse-error).

**Remediation:** Add parse_to_cst_with_config and make parse_to_cst use LoaderConfig::default. Convert
whole-tree traversals and destruction to explicit worklists, or document and enforce a
construction-time maximum depth that makes their recursion safe.

### composer-mapping-insertion-is-quadratic

Composer mapping insertion is quadratic in mapping width

**significant** · criterion `12.1` · `crates/yamalgam-core/src/value.rs:67-77`, `crates/yamalgam-compose/src/lib.rs:296-317` · effort: medium · <img src="assets/sparkline-composer-mapping-insertion-is-quadratic.svg" height="14" alt="commit activity for composer-mapping-insertion-is-quadratic" />

`Mapping` preserves order in a `Vec`, and `Composer` calls `insert` once for every
non-merge mapping pair. Inserting n distinct scalar keys therefore performs n(n-1)/2
equality checks; collection-valued keys make each check recursive as well. This path
consumes untrusted YAML, and the composer does not enforce `max_collection_size`, so the
strict input-byte limit still permits a wide document that spends quadratic CPU before
composition completes.

```rust crates/yamalgam-core/src/value.rs:67-77
    /// Inserts a key/value pair. If the key already exists, its value is
    /// replaced and the old value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        for entry in &mut self.entries {
            if entry.0 == key {
                let old = std::mem::replace(&mut entry.1, value);
                return Some(old);
            }
        }
        self.entries.push((key, value));
        None
```

> A wide mapping with distinct keys makes each new pair pay for every pair already
> accepted.

**Related:** [Alias expansion deep-clones entire DOM subtrees under a count-only budget](#composer-alias-deep-clone-is-size-unbudgeted), [Borrowed string lookup allocates an owned Value key](#borrowed-value-get-allocates-key).

**Remediation:** Keep the ordered entry vector, but add a randomized side index from equality-safe scalar
keys to entry positions and use the linear fallback only for keys such as NaN or
collection-valued keys whose current `PartialEq` semantics cannot safely back `Eq +
Hash`. Enforce `max_collection_size` in the sequence and mapping composition loops as a
defense-in-depth bound, and add a wide-map benchmark.

### composer-alias-deep-clone-is-size-unbudgeted

Alias expansion deep-clones entire DOM subtrees under a count-only budget

**significant** · criterion `3.1` · `crates/yamalgam-compose/src/lib.rs:343-355`, `crates/yamalgam-core/src/value.rs:23-41` · effort: medium · <img src="assets/sparkline-composer-alias-deep-clone-is-size-unbudgeted.svg" height="14" alt="commit activity for composer-alias-deep-clone-is-size-unbudgeted" />

Core `Value` derives `Clone` over owned `String`, `Vec<Value>`, and `Mapping`, so
`.cloned()` recursively allocates and copies the complete anchored subtree for every
alias. The only guard counts alias occurrences: cloning a large but legal anchor 100
times under the strict profile, or 10,000 times under the default profile, can multiply
memory and CPU far beyond the input-size limit without exceeding the alias count.

```rust crates/yamalgam-compose/src/lib.rs:343-355
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
```

> The counter prices every alias equally, so I spend each allowed reference on the
> largest anchor in the document.

**Related:** [Composer mapping insertion is quadratic in mapping width](#composer-mapping-insertion-is-quadratic).

**Remediation:** Store a cached node/byte weight with each anchored value and charge that weight to an
expanded-node or expanded-byte budget before cloning. Reject the alias when the
cumulative expansion would exceed the configured collection/output bound. A future
shareable DOM representation can make immutable alias clones cheap, but the
size-weighted budget is the compatible fix for the current public `Value` API.

*Verdict: Parser limits are available and tested, yet composer, serde, and CST defaults do not
consistently inherit them. Wide mappings and alias materialization add CPU and memory
multipliers after parsing, so safe construction needs to be structural rather than an
opt-in convention.*

---

## The Error Preservation Surface

*Typed scanner, parser, and serde errors are available, but three adapters flatten or
discard them at consumer boundaries.*

### flow-pair-finalization-swallows-scanner-errors

Synthetic flow-pair finalization consumes scanner errors while calculating a span

**moderate** · criterion `5.3` · `crates/yamalgam-parser/src/parser.rs:1167-1179` · effort: small

peek_token obtains its value through self.tokens.next().transpose(), so an Err consumes
the scanner error. Converting that result with ok() replaces ParseError::Scan with None,
emits a default-span MappingEnd, and advances parser state. Invalid input immediately
following an implicit mapping in a flow sequence can therefore lose the original
machine-readable ScanErrorKind and message; a later generic EOF error may replace it.
Six similar ok().flatten() span probes make the same fragile assumption that a token was
already buffered.

```rust crates/yamalgam-parser/src/parser.rs:1167-1179
    fn parse_flow_sequence_entry_mapping_end(
        &mut self,
    ) -> Result<Option<Event<'input>>, ParseError> {
        let span = self
            .peek_token()
            .ok()
            .flatten()
            .map_or_else(Span::default, |t| Span {
                start: t.atom.span.start,
                end: t.atom.span.start,
            });
        self.state = ParserState::FlowSequenceEntry;
        Ok(Some(Event::MappingEnd { span }))
```

> If EOF lands in the flow-pair state, the parser turns the scanner failure into a
> successful MappingEnd event.

**Related:** [CST recovery discards the only structured parse error](#cst-discards-parse-error).

**Remediation:** Propagate the error with self.peek_token()? and map only the Option. Sweep all seven
ok().flatten() span probes, retaining a default span only for genuine None, and add a
malformed flow-pair regression that asserts the original scanner error kind survives.

### cst-discards-parse-error

CST recovery discards the only structured parse error

**moderate** · criterion `5.3` · `crates/yamalgam-cst/src/lib.rs:548-548` · effort: small

parse_to_cst never returns Result. On malformed external input, handle_error creates a
NodeKind::Error containing only the remaining source text as a Whitespace token, then
discards ParseError. Callers can detect that recovery happened but cannot obtain the
error variant, message, or diagnostic span, leaving no useful failure information
outside reparsing the source.

```rust crates/yamalgam-cst/src/lib.rs:548-548
        let _ = error; // consumed
```

**Related:** [CST traversal and destruction remain recursively stack-bound](#cst-recursive-traversals-have-no-depth-bound).

**Remediation:** Preserve ParseError in a diagnostic collection returned alongside the CST, or add
structured error data to error nodes without affecting leaf-text round trips. Expose a
Result- or report-returning convenience API while retaining the current recovery API
where needed.

### from-reader-erases-io-errors

Serde from_reader erases I/O failures into the Custom data-error variant

**moderate** · criterion `5.4` · `crates/yamalgam-serde/src/lib.rs:84-91` · effort: small

Error::Custom is documented for failures produced by serde Deserialize implementations.
Mapping std::io::Error into that same String variant makes reader failures
indistinguishable from schema or visitor failures and causes Error::source to return
None. Callers cannot inspect ErrorKind, downcast the source, or apply retry policy
without parsing display text.

```rust crates/yamalgam-serde/src/lib.rs:84-91
pub fn from_reader<R: std::io::Read, T: serde::de::DeserializeOwned>(
    mut reader: R,
) -> Result<T, Error> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| Error::Custom(format!("I/O error: {e}")))?;
    from_str(&buf)
```

**Related:** [The default serde entry point explicitly disables every loader limit](#serde-default-disables-loader-limits).

**Remediation:** Add Error::Io(#[source] std::io::Error), map read_to_string failures into it, and return
that source from std::error::Error::source. Keep Custom reserved for serde-generated
data-model errors.

*Verdict: The core error architecture remains strong. The gaps are localized: one parser span
probe consumes a scanner error, CST recovery throws away its diagnostic, and reader I/O
becomes an untyped string.*

---

## The Public API and Type Surface

*The public APIs are mostly idiomatic, but a few ownership, lifetime, and collection
contracts impose unnecessary allocation or constrain callers.*

### borrowed-value-get-allocates-key

Borrowed string lookup allocates an owned Value key

**advisory** · criterion `3.3 - Functions Accept References When Callee Does Not Need Ownership` · `crates/yamalgam-core/src/value.rs:128-134` · effort: trivial

The public API correctly accepts a borrowed str, but each lookup converts it into an
owned String and wraps it in Value before performing a linear scan. The temporary
allocation is discarded immediately, so repeated lookups pay a per-call allocator and
copy cost without any ownership requirement.

```rust crates/yamalgam-core/src/value.rs:128-134
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Mapping(m) => m.get(&Self::String(key.to_owned())),
            _ => None,
        }
    }
```

**Related:** [Mapping exposes iterator methods but omits standard IntoIterator implementations](#mapping-omits-intoiterator).

**Remediation:** Add a borrowed string-key lookup on Mapping that directly matches Value::String
candidates against &str, then delegate Value::get to it.

### serde-peek-clones-owned-event

Serde unit deserialization clones an event before consuming the same event

**advisory** · criterion `3.1 - .clone() Is Not Used to Appease the Borrow Checker` · `crates/yamalgam-serde/src/de.rs:647-656` · effort: small

Event contains Cow-backed scalar text. Cloning an Event::Scalar whose Cow is owned
allocates and copies the scalar, after which next_event or expect_scalar consumes the
original event. The same residual peek-clone-consume shape exists in enum
deserialization, while null probing separately clones the scalar Cow to release its
borrow.

```rust crates/yamalgam-serde/src/de.rs:647-656
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let event = self.peek_event()?.clone();
        match event {
            Event::StreamEnd | Event::DocumentEnd { .. } => {
                // Empty document = null.
                let _ = self.next_event()?;
                visitor.visit_unit()
            }
            Event::Scalar { .. } => {
                let (value, style, span) = self.expect_scalar()?;
```

**Related:** [Parser consumers erase concrete event and resolver types on hot paths](#parser-consumers-erase-concrete-hot-path-types).

**Remediation:** Extract only a small borrowed discriminant from the peek, end that borrow, and then
consume the owned event. For enum deserialization, match directly on next_event where
the value is needed after consumption.

### custom-tag-resolver-requires-static

Synchronous custom tag resolution unnecessarily requires a static resolver

**advisory** · criterion `4.2 - 'static Bounds Are Not Used to Work Around Lifetime Complexity` · `crates/yamalgam-compose/src/lib.rs:106-123` · effort: medium

The method consumes the entire stream and returns owned Values before the call ends, yet
the Box<dyn TagResolver> field forces the resolver to be static. A resolver borrowing a
caller-owned lookup table or configuration is rejected even when that state outlives the
synchronous call.

```rust crates/yamalgam-compose/src/lib.rs:106-123
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
```

**Related:** [Parser consumers erase concrete event and resolver types on hot paths](#parser-consumers-erase-concrete-hot-path-types).

**Remediation:** Parameterize Composer over its resolver type, or add a resolver lifetime to the trait
object so this entry point can accept borrowed implementations.

### mapping-omits-intoiterator

Mapping exposes iterator methods but omits standard IntoIterator implementations

**note** · criterion `9 - Trait Design (missing standard IntoIterator implementation)` · `crates/yamalgam-core/src/value.rs:86-99` · effort: small

Mapping behaves as a collection but cannot participate in idiomatic `for (key, value) in
&mapping` iteration, and callers cannot consume it into entries because the backing Vec
is private. The inherent iter method duplicates the role of the standard conversion
trait.

```rust crates/yamalgam-core/src/value.rs:86-99
    /// Returns an iterator over keys.
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over `(key, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
```

**Related:** [Borrowed string lookup allocates an owned Value key](#borrowed-value-get-allocates-key), [Mapping manually implements the same Default that derive would generate](#mapping-default-manually-matches-derive).

**Remediation:** Implement IntoIterator for Mapping, &Mapping, and &mut Mapping, delegating to the
backing Vec iterators while retaining iter, keys, and values as convenience methods.

### mapping-default-manually-matches-derive

Mapping manually implements the same Default that derive would generate

**note** · criterion `9.4 - Standard Traits Are Derived When Possible` · `crates/yamalgam-core/src/value.rs:114-118` · effort: trivial

Mapping contains only a Vec, and Mapping::new constructs exactly Vec::new. The manual
implementation therefore has no semantic distinction from #[derive(Default)] and creates
an unnecessary maintenance site.

```rust crates/yamalgam-core/src/value.rs:114-118
impl Default for Mapping {
    fn default() -> Self {
        Self::new()
    }
}
```

**Related:** [Mapping exposes iterator methods but omits standard IntoIterator implementations](#mapping-omits-intoiterator).

**Remediation:** Add Default to Mapping's derive list and remove the manual implementation.

### compare-streams-use-index-loops

Stream comparison manually indexes two slices in lockstep

**note** · criterion `10.1 - Iterator Combinators Replace Manual Index Loops` · `crates/yamalgam-compare/src/compare.rs:66-77` · effort: trivial

The loop computes a minimum length solely to make parallel indexing safe. zip with
enumerate expresses the lockstep traversal directly and removes the need to repeat the
same index across both slices. The same pattern recurs in both event comparison
functions.

```rust crates/yamalgam-compare/src/compare.rs:66-77
    let common_len = c_tokens.len().min(rust_tokens.len());

    for i in 0..common_len {
        if !tokens_match(&c_tokens[i], &rust_tokens[i]) {
            return CompareResult::TokenMismatch {
                index: i,
                c_token: c_tokens[i].clone(),
                rust_token: rust_tokens[i].clone(),
                context: context_slice(c_tokens, i),
            };
        }
    }
```

**Remediation:** Iterate with c_tokens.iter().zip(rust_tokens).enumerate(), cloning only the mismatching
pair when constructing the owned result.

### with-variables-collect-target-inferred

with_variables relies on distant field inference for collect

**note** · criterion `10.5 - collect() Specifies the Target Type` · `crates/yamalgam-query/src/engine.rs:103-107` · effort: trivial

The collect target is determined only by RunCtx::variables, outside the method body.
This violates the repository rubric's explicit-target rule and makes the allocation
shape less obvious during local review.

```rust crates/yamalgam-query/src/engine.rs:103-107
    /// Attach values for the globals declared while compiling the filter.
    #[must_use]
    pub fn with_variables(mut self, variables: impl IntoIterator<Item = YamlVal>) -> Self {
        self.variables = variables.into_iter().collect();
        self
```

**Remediation:** Use collect::<Vec<_>>() here and add explicit targets to the analogous inference-only
collect calls in query operations and CLI assembly.

*Verdict: No type-safety defect emerged. The actionable items are ergonomic and
performance-facing: borrowed lookup still allocates, event peeking clones owned data,
collection iteration is incomplete, and boxed resolver plumbing rejects useful borrowed
implementations.*

---

## The Feature Completeness Surface

*Several public promises are broader than the behavior users can currently reach,
including an intentionally deferred loader policy and universal CST losslessness.*

### resolver-middleware-cannot-reach-most-documented-consumers

Resolver middleware cannot reach the documented CST and serde consumers

**significant** · criterion `14.5` · `crates/yamalgam-serde/src/de.rs:22-28` · effort: medium

The resolver module documents middleware between the parser and the Value builder, CST,
and serde, and the README diagrams the same fan-out. Only Composer actually exposes a
resolver-aware entry point. Deserializer has only string constructors; its private event
field cannot accept ResolvedEvents because their error item is ResolveError rather than
ParseError. CstBuilder::build has the same ParseError-only item constraint, while the
query builder hard-codes Parser. A custom resolver therefore cannot be composed with
most of the consumers the public architecture promises.

```rust crates/yamalgam-serde/src/de.rs:22-28
pub struct Deserializer<'input> {
    /// Event source (parser or resolved events), wrapped in Peekable.
    events: Peekable<Box<dyn Iterator<Item = Result<Event<'input>, ParseError>> + 'input>>,
    /// Tag resolver for plain scalar typing.
    tag_resolver: Box<dyn TagResolver>,
    /// Resource limits for anchor/alias caps and future extensions.
    limits: ResourceLimits,
```

**Related:** [Deferred loader resolution policy is publicly accepted but never enforced](#loader-resolution-policy-is-entirely-unwired).

**Remediation:** Add public event-source or resolver-aware constructors for serde and CST, using a shared
resolved-event error contract. Route their string convenience constructors through those
entry points and add one custom-resolver integration test per downstream consumer.

### loader-resolution-policy-is-entirely-unwired

Deferred loader resolution policy is publicly accepted but never enforced

**advisory** · criterion `14.1` · `crates/yamalgam-core/src/loader.rs:32-48`, `docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md:152-156`, `docs/plans/README.md:152-155` · effort: small

Outside its constructors and tests, no crate reads config.resolution or any
IncludePolicy or RefPolicy field. Parser and scanner retain only config.limits;
composer, serde, and query retain limits and tag_resolution. The accepted ADR explicitly
defers enforcement until the resolver layer exists, and Milestone 11 remains not
started, so this is an intentional public placeholder rather than an active exploit or
data-loss path. The actionable gap is that with-config APIs accept enabled policies
without resolving them or reporting that the requested capability is not implemented.

```rust crates/yamalgam-core/src/loader.rs:32-48
/// Top-level loader configuration.
///
/// Combines [`ResourceLimits`] (caps on sizes, depth, expansions) with
/// [`ResolutionPolicy`] (include/ref handling).
///
/// The [`Default`] implementation returns [`LoaderConfig::moderate()`].
#[derive(Debug, Clone, PartialEq, Eq)]
// y[impl model.loading.well-formed]
// y[impl model.loading.reject-ill-formed]
pub struct LoaderConfig {
    /// Resource limits applied during loading.
    pub limits: ResourceLimits,
    /// Policy for resolving `!include` and `$ref` references.
    pub resolution: ResolutionPolicy,
    /// Tag resolution scheme for plain scalar typing.
    pub tag_resolution: TagResolution,
}
```

**Related:** [Resolver middleware cannot reach the documented CST and serde consumers](#resolver-middleware-cannot-reach-most-documented-consumers).

**Remediation:** Until built-in resolvers are wired, reject enabled resolution policies at public loading
entry points with a typed unsupported-capability error, or hide the policy behind an
explicitly unstable surface. Keep the full resolver and policy implementation in M11,
where every path, host, recursion, byte-budget, and timeout field can be enforced.

### lossless-cst-corrupts-eight-valid-flow-documents

The lossless CST duplicates closing tokens in eight valid YAML cases

**significant** · criterion `14.1` · `crates/yamalgam-compare/tests/cst_round_trip.rs:13-34` · effort: medium

yamalgam-cst's crate docs and parse_to_cst rustdoc promise that every input byte is
reproduced, which is the crate's defining capability. The compliance harness instead
accepts inequality for eight non-failing YAML Test Suite documents. Synthetic
flow-mapping end spans cause to_text() to emit an additional closing bracket or brace,
so parse and re-emit corrupts valid source for these documented cases.

```rust crates/yamalgam-compare/tests/cst_round_trip.rs:13-34
/// Known round-trip failures that are understood and acceptable.
///
/// STALENESS: if a case in this list passes, the test panics — remove it.
///
/// Categories:
/// - Flow collection close token duplication (extra `]` or `}`)
/// - Comment/whitespace ordering when comments arrive out of source order
/// - Edge cases with complex flow nesting or empty values
const ROUND_TRIP_ALLOWLIST: &[&str] = &[
    // --- Flow collection close token duplication ---
    // (Single-pair flow-SEQUENCE mappings were fixed by zero-width
    // MappingStart/End spans; the remaining cases involve flow-MAPPING
    // synthetic ends.)
    "57H4", // nested flow — extra `]`
    "6PBE", // flow mapping — extra `}`
    "7ZZ5", // flow mapping — extra `}`
    "AZ63", // flow mapping — extra `}`
    "RLU9", // flow mapping — extra `}`
    "S3PD", // flow mapping — extra `}`
    "S9E8", // flow mapping — extra `}`
    "SKE5", // flow mapping — extra `}`
];
```

> The round-trip API promises preservation; these valid fixtures make it manufacture
> closing tokens instead.

**Remediation:** Correct flow-mapping synthetic-end span/token ownership so each source delimiter is
emitted exactly once, remove the allowlist as cases pass, and keep the universal rustdoc
claim only once the suite enforces it without exemptions.

### config-template-advertises-silently-ignored-settings

The shipped configuration template advertises settings the CLI deliberately ignores

**moderate** · criterion `14.1` · `config/yamalgam.toml.example:33-52` · effort: trivial

The CLI Config type contains only log_level and log_dir. It has no otel_endpoint,
api_url, features, telemetry, output, format, or color fields. Serde accepts unknown
keys, and the integration suite explicitly asserts that unknown configuration fields are
ignored. Uncommenting the template's suggested settings therefore succeeds while
changing nothing, making configuration mistakes and nonexistent capabilities
indistinguishable.

```toml config/yamalgam.toml.example:33-52
# Config options:
# - log_dir = "/path/to/log/dir"
# - otel_endpoint = "http://127.0.0.1:4317"

# ------------------------------------------------------------------------------
# Add your configuration options below
# ------------------------------------------------------------------------------

# Example: API endpoint
# api_url = "https://api.example.com"

# Example: Feature flags
# [features]
# experimental = false
# telemetry = true

# Example: Output settings
# [output]
# format = "text"  # "text", "json", "table"
# color = "auto"   # "auto", "always", "never"
```

**Remediation:** Reduce both TOML and YAML templates to fields the CLI actually consumes and add
deny_unknown_fields so misspellings fail loudly. Add future settings to the templates
only with their Config fields, behavior, and integration tests.

### multidocument-rustdoc-example-is-never-compiled

The sole multi-document rustdoc example is ignored and incomplete

**note** · criterion `14.1` · `crates/yamalgam-serde/src/de.rs:87-100` · effort: trivial

cargo test --doc reports green without compiling the public API's only multi-document
example. As written, the snippet also lacks a use yamalgam_serde::Deserializer import,
so removing ignore would expose the failure. API or trait-bound changes can therefore
break the documented entry point without any documentation test detecting it.

````rust crates/yamalgam-serde/src/de.rs:87-100
    /// Create a streaming iterator over documents in a multi-document YAML
    /// stream.
    ///
    /// Each call to `Iterator::next()` deserializes one document into `T`.
    /// Iteration stops at `StreamEnd` or on the first error.
    ///
    /// ```ignore
    /// let docs: Vec<i64> = Deserializer::from_str("---\n42\n---\n99")
    ///     .documents::<i64>()
    ///     .collect::<Result<_, _>>()
    ///     .unwrap();
    /// assert_eq!(docs, vec![42, 99]);
    /// ```
    pub const fn documents<T>(self) -> crate::Documents<'input, T>
````

**Remediation:** Add the public import, change the fence to a normal Rust doctest, and leave the existing
assertion as the executable multi-document contract.

*Verdict: This surface has the largest documentation-to-runtime gaps. The resolver architecture is
only partly composable, deferred loader resolution settings are accepted without an
unsupported-capability error, eight valid flow documents violate the lossless-CST
contract, and shipped configuration examples advertise ignored settings.*

---

## The Runtime Performance Surface

*The query and scanner pipelines are correct but retain repeated whole-tree work and
avoidable short-lived allocations on hot paths.*

### query-builder-rewalks-every-completed-node

The query builder allocates a depth stack and rewalks every completed node

**significant** · criterion `12.5` · `crates/yamalgam-query/src/value.rs:210-230`, `crates/yamalgam-query/src/build.rs:287-292` · effort: medium · <img src="assets/sparkline-query-builder-rewalks-every-completed-node.svg" height="14" alt="commit activity for query-builder-rewalks-every-completed-node" />

`Builder::finish` invokes `check_depth` for every completed scalar, sequence, and
mapping. Scalars therefore pay a fresh `Vec` allocation for a one-node check, and each
collection walks its entire completed subtree. Nested documents revisit each descendant
once per ancestor, adding O(nodes * depth) work even though the recursive builder
already knows each child's depth and the configured depth bound.

```rust crates/yamalgam-query/src/value.rs:210-230
    pub(crate) fn check_depth(&self, limits: &yamalgam_core::ResourceLimits) -> Result<(), String> {
        let mut stack = vec![(self, 0_usize)];
        while let Some((value, depth)) = stack.pop() {
            limits.check_depth(depth)?;
            let child_depth = depth
                .checked_add(1)
                .ok_or_else(|| "value depth overflow".to_owned())?;
            match &value.data {
                Data::Seq(items) => {
                    stack.extend(items.iter().map(|item| (item, child_depth)));
                }
                Data::Map(mapping) => {
                    stack.extend(
                        mapping
                            .0
                            .iter()
                            .flat_map(<[_; 2]>::from)
                            .map(|item| (item, child_depth)),
                    );
                }
                Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_) => {}
```

**Related:** [Identity inventory builds a heap-backed YAML path node for every mapping key](#identity-inventory-allocates-path-node-per-map-key).

**Remediation:** Have the private node builder return `(YamlVal, subtree_height)`, compute collection
height from child heights, and store `(YamlVal, subtree_height)` for anchors. Alias
nodes then carry the cached target height, so the expanded-depth check remains correct
without allocating a traversal stack or rewalking completed subtrees.

### identity-inventory-allocates-path-node-per-map-key

Identity inventory builds a heap-backed YAML path node for every mapping key

**advisory** · criterion `12.5` · `crates/yamalgam-query/src/value.rs:763-780` · effort: medium · <img src="assets/sparkline-identity-inventory-allocates-path-node-per-map-key.svg" height="14" alt="commit activity for identity-inventory-allocates-path-node-per-map-key" />

Both `aliases` and `orphan_anchors` traverse the complete document through this visitor.
For every mapping entry, including documents with no aliases in key position, the
visitor materializes the jq path encoding `{"$key": key}` and immediately pops it. That
creates several heap allocations per key solely to maintain traversal state; the path is
only copied into output when an alias is found.

```rust crates/yamalgam-query/src/value.rs:763-780
        match &value.data {
            Data::Seq(items) => {
                for (index, item) in items.iter().enumerate() {
                    path.push(YamlVal::from(index));
                    self.visit(item, path);
                    path.pop();
                }
            }
            Data::Map(mapping) => {
                for (key, value) in &mapping.0 {
                    path.push(YamlVal::map(vec![(YamlVal::str("$key"), key.clone())]));
                    self.visit(key, path);
                    path.pop();
                    path.push(key.clone());
                    self.visit(value, path);
                    path.pop();
                }
            }
```

**Related:** [The query builder allocates a depth stack and rewalks every completed node](#query-builder-rewalks-every-completed-node).

**Remediation:** Track traversal state as a reusable `Vec<PathPart<'_>>` with borrowed `Key`, `MapKey`,
and numeric index variants. Materialize `YamlVal` path elements only when recording an
encountered alias. This preserves the public path format while making alias-free
traversal allocation-free after the path vector reaches capacity.

### string-range-retains-intermediate-string

String slicing allocates and copies through an intermediate String

**advisory** · criterion `2.5` · `crates/yamalgam-query/src/valt.rs:135-150`, `crates/yamalgam-query/src/valt.rs:477-484` · effort: trivial · <img src="assets/sparkline-string-range-retains-intermediate-string.svg" height="14" alt="commit activity for string-range-retains-intermediate-string" />

A jq string range first collects the selected characters into an owned `String`.
`YamlVal::str` then converts that `String` to `Rc<str>`, which requires a second
allocation and copy for the reference-counted layout. The same module already has
`char_byte_range` and uses a borrowed substring in the range-update path, so this read
path retains a duplicate allocation the adjacent implementation avoids.

```rust crates/yamalgam-query/src/valt.rs:135-150
    // r[impl filters.slice]
    fn range(self, range: Range<&Self>) -> ValR<Self> {
        let (start, end) = range_bounds(range)?;
        match self.data {
            Data::Seq(items) => {
                let (skip, take) = skip_take(start, end, items.len());
                Ok(Self::seq(
                    items.iter().skip(skip).take(take).cloned().collect(),
                ))
            }
            Data::Str(string) => {
                let (skip, take) = skip_take(start, end, string.chars().count());
                Ok(Self::str(
                    string.chars().skip(skip).take(take).collect::<String>(),
                ))
            }
```

**Remediation:** Reuse `char_byte_range`: compute `(start_byte, end_byte)` from `skip` and `take`, then
call `Self::str(&string[start_byte..end_byte])`. This removes the intermediate String
and leaves the single allocation required for Rc<str>. Add ASCII and multibyte range
tests to pin jq character-index semantics.

### parser-consumers-erase-concrete-hot-path-types

Parser consumers erase concrete event and resolver types on hot paths

**note** · criterion `9.1` · `crates/yamalgam-serde/src/de.rs:22-26`, `crates/yamalgam-parser/src/resolve.rs:125-136`, `crates/yamalgam-compose/src/lib.rs:97-102` · effort: medium · <img src="assets/sparkline-parser-consumers-erase-concrete-hot-path-types.svg" height="14" alt="commit activity for parser-consumers-erase-concrete-hot-path-types" />

`Deserializer::from_str` and `from_str_with_config` always construct a concrete `Parser`
and a statically known tag-resolution value, but immediately box both. `ResolvedEvents`
is generic over its resolver while independently boxing its upstream iterator, and all
composer string constructors pass the same concrete parser adapter. The vtable calls are
real, but the accepted architecture treats this indirection as noise unless profiling
proves otherwise, and serde documents the type erasure as a deliberate way to limit
monomorphization.

```rust crates/yamalgam-serde/src/de.rs:22-26
pub struct Deserializer<'input> {
    /// Event source (parser or resolved events), wrapped in Peekable.
    events: Peekable<Box<dyn Iterator<Item = Result<Event<'input>, ParseError>> + 'input>>,
    /// Tag resolver for plain scalar typing.
    tag_resolver: Box<dyn TagResolver>,
```

**Remediation:** Profile representative serde, composer, and resolved-event workloads first. Only if
dynamic dispatch is measurable should Deserializer or ResolvedEvents gain generic
iterator/resolver forms, with boxed aliases retained to control monomorphization and
preserve the intentional erased API.

### scanner-error-branches-remain-in-hot-functions

Scanner error construction remains inline in tokenization hot loops

**note** · criterion `12.9` · `crates/yamalgam-scanner/src/scanner.rs:354-375` · effort: small · <img src="assets/sparkline-scanner-error-branches-remain-in-hot-functions.svg" height="14" alt="commit activity for scanner-error-branches-remain-in-hot-functions" />

`scan_to_next_token` is entered for the whitespace boundary before ordinary tokens, yet
its rare invalid-tab diagnostic is lexically inline in the loop. The scanner repeats
this pattern across token-fetch functions with no explicit `#[cold]` boundary. Source
placement alone does not establish generated instruction layout or a measurable scanner
cost, so this is a possible code-layout optimization rather than a demonstrated hot-path
regression.

```rust crates/yamalgam-scanner/src/scanner.rs:354-375
    fn scan_to_next_token(&mut self) {
        self.tab_in_preceding_whitespace = false;
        let mut at_line_start = self.reader.mark().column == 0;
        let mut tab_at_flow_line_start = false;
        loop {
            while let Some(c) = self.reader.peek() {
                if c == ' ' {
                    self.reader.advance();
                } else if c == '\t' {
                    // YAML §6.1: tabs must not be used for indentation.
                    // In block context, tabs at the start of a line (before
                    // any non-whitespace) are indentation — reject them.
                    // Only when there's an active block structure (indent >= 0),
                    // so tabs before the first block token (like `\t[`) are fine.
                    // cref: fy_scan_to_next_token (fy-parse.c)
                    // y[impl struct.indent.tab-forbidden]
                    if at_line_start && self.flow_level == 0 && self.indent >= 0 {
                        self.error = Some(ScanError {
                            kind: ScanErrorKind::InvalidTab,
                            message: "tab character used for indentation".to_string(),
                        });
                        return;
```

**Remediation:** Benchmark representative scanning or inspect optimized codegen before changing the
layout. If the diagnostic blocks remain in hot code and the cost is measurable, route
scanner failures through a small `#[cold]` and `#[inline(never)]` helper.

*Verdict: The largest cost is structural: the query builder rewalks every completed subtree.
Identity inventory and string slicing add smaller allocation multipliers, while boxed
parser consumers and inline scanner error construction are lower-priority tuning
opportunities.*

---

## The Supply Chain Surface

*Dependency controls are explicit and passing, with one accepted benchmark-only advisory
edge and two unused xtask declarations.*

### xtask-unused-serde-and-toml

xtask retains two unused direct dependencies

**note** · criterion `7.4` · `xtask/Cargo.toml:14-15` · effort: trivial

cargo-machete reports both declarations unused, and a repository search finds no serde
or toml API use under xtask. They remain from the removed benchmark-code-generation
path, adding unnecessary direct dependency and feature edges even though both crates
already occur transitively elsewhere in the workspace.

```toml xtask/Cargo.toml:14-15
serde = { version = "1.0", features = ["derive"] }
toml = "1.1"
```

**Remediation:** Remove serde and toml from xtask/Cargo.toml, then run the xtask checks and the workspace
dependency checks.

### yamlstar-pulls-unmaintained-dlopen-derive

Optional yamlstar benchmarks retain an unmaintained proc-macro dependency

**advisory** · criterion `7.5 / 13.5` · `.config/deny.toml:55-58` · effort: small

Enabling yamalgam-bench's optional yamlstar feature resolves yamlstar 0.1.18 through
dlopen 0.1.8 to the unmaintained dlopen_derive 0.1.4 proc macro. Cargo Audit reports
RUSTSEC-2023-0051. The edge is isolated from the shipped yg graph and explicitly
monitored, so it is maintenance and build-host exposure rather than a production-runtime
vulnerability.

```toml .config/deny.toml:55-58
    # dlopen_derive 0.1.4 - via yamlstar (optional benchmark peer)
    # Unmaintained, but yamlstar is only used for comparative benchmarks
    # All Cargo Deny entry points use --all-features so this entry stays live
    "RUSTSEC-2023-0051",
```

**Remediation:** Remove yamlstar from the comparative peer set unless the GraalVM comparison remains
valuable; otherwise replace or fork its dlopen integration so all-features builds no
longer compile dlopen_derive 0.1.4.

*Verdict: The shipped graph has no active RustSec vulnerability. The remaining work is
housekeeping: remove unused xtask dependencies and decide whether the optional yamlstar
comparison still justifies compiling an unmaintained proc macro in all-feature builds.*

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---------|---------|----------|--------|--------|
| _The Loading and Composition Surface_ |  |  |  |  |
| [composer-default-path-has-unbounded-recursion](#composer-default-path-has-unbounded-recursion) | significant | `crates/yamalgam-compose/src/lib.rs:272-285` | medium | related: [serde-default-disables-loader-limits](#serde-default-disables-loader-limits), [cst-recursive-traversals-have-no-depth-bound](#cst-recursive-traversals-have-no-depth-bound) |
| [serde-default-disables-loader-limits](#serde-default-disables-loader-limits) | significant | `crates/yamalgam-serde/src/de.rs:52-59` | small | related: [composer-default-path-has-unbounded-recursion](#composer-default-path-has-unbounded-recursion), [cst-recursive-traversals-have-no-depth-bound](#cst-recursive-traversals-have-no-depth-bound), [from-reader-erases-io-errors](#from-reader-erases-io-errors) |
| [cst-recursive-traversals-have-no-depth-bound](#cst-recursive-traversals-have-no-depth-bound) | significant | `crates/yamalgam-cst/src/lib.rs:141-152` | medium | related: [composer-default-path-has-unbounded-recursion](#composer-default-path-has-unbounded-recursion), [serde-default-disables-loader-limits](#serde-default-disables-loader-limits), [cst-discards-parse-error](#cst-discards-parse-error) |
| [composer-mapping-insertion-is-quadratic](#composer-mapping-insertion-is-quadratic) | significant | `crates/yamalgam-core/src/value.rs:67-77` | medium | related: [composer-alias-deep-clone-is-size-unbudgeted](#composer-alias-deep-clone-is-size-unbudgeted), [borrowed-value-get-allocates-key](#borrowed-value-get-allocates-key) |
| [composer-alias-deep-clone-is-size-unbudgeted](#composer-alias-deep-clone-is-size-unbudgeted) | significant | `crates/yamalgam-compose/src/lib.rs:343-355` | medium | related: [composer-mapping-insertion-is-quadratic](#composer-mapping-insertion-is-quadratic) |
| _The Error Preservation Surface_ |  |  |  |  |
| [flow-pair-finalization-swallows-scanner-errors](#flow-pair-finalization-swallows-scanner-errors) | moderate | `crates/yamalgam-parser/src/parser.rs:1167-1179` | small | related: [cst-discards-parse-error](#cst-discards-parse-error) |
| [cst-discards-parse-error](#cst-discards-parse-error) | moderate | `crates/yamalgam-cst/src/lib.rs:548-548` | small | related: [cst-recursive-traversals-have-no-depth-bound](#cst-recursive-traversals-have-no-depth-bound) |
| [from-reader-erases-io-errors](#from-reader-erases-io-errors) | moderate | `crates/yamalgam-serde/src/lib.rs:84-91` | small | related: [serde-default-disables-loader-limits](#serde-default-disables-loader-limits) |
| _The Public API and Type Surface_ |  |  |  |  |
| [borrowed-value-get-allocates-key](#borrowed-value-get-allocates-key) | advisory | `crates/yamalgam-core/src/value.rs:128-134` | trivial | related: [mapping-omits-intoiterator](#mapping-omits-intoiterator) |
| [serde-peek-clones-owned-event](#serde-peek-clones-owned-event) | advisory | `crates/yamalgam-serde/src/de.rs:647-656` | small | related: [parser-consumers-erase-concrete-hot-path-types](#parser-consumers-erase-concrete-hot-path-types) |
| [custom-tag-resolver-requires-static](#custom-tag-resolver-requires-static) | advisory | `crates/yamalgam-compose/src/lib.rs:106-123` | medium | related: [parser-consumers-erase-concrete-hot-path-types](#parser-consumers-erase-concrete-hot-path-types) |
| [mapping-omits-intoiterator](#mapping-omits-intoiterator) | note | `crates/yamalgam-core/src/value.rs:86-99` | small | related: [borrowed-value-get-allocates-key](#borrowed-value-get-allocates-key), [mapping-default-manually-matches-derive](#mapping-default-manually-matches-derive) |
| [mapping-default-manually-matches-derive](#mapping-default-manually-matches-derive) | note | `crates/yamalgam-core/src/value.rs:114-118` | trivial | related: [mapping-omits-intoiterator](#mapping-omits-intoiterator) |
| [compare-streams-use-index-loops](#compare-streams-use-index-loops) | note | `crates/yamalgam-compare/src/compare.rs:66-77` | trivial | — |
| [with-variables-collect-target-inferred](#with-variables-collect-target-inferred) | note | `crates/yamalgam-query/src/engine.rs:103-107` | trivial | — |
| _The Feature Completeness Surface_ |  |  |  |  |
| [resolver-middleware-cannot-reach-most-documented-consumers](#resolver-middleware-cannot-reach-most-documented-consumers) | significant | `crates/yamalgam-serde/src/de.rs:22-28` | medium | related: [loader-resolution-policy-is-entirely-unwired](#loader-resolution-policy-is-entirely-unwired) |
| [loader-resolution-policy-is-entirely-unwired](#loader-resolution-policy-is-entirely-unwired) | advisory | `crates/yamalgam-core/src/loader.rs:32-48` | small | related: [resolver-middleware-cannot-reach-most-documented-consumers](#resolver-middleware-cannot-reach-most-documented-consumers) |
| [lossless-cst-corrupts-eight-valid-flow-documents](#lossless-cst-corrupts-eight-valid-flow-documents) | significant | `crates/yamalgam-compare/tests/cst_round_trip.rs:13-34` | medium | — |
| [config-template-advertises-silently-ignored-settings](#config-template-advertises-silently-ignored-settings) | moderate | `config/yamalgam.toml.example:33-52` | trivial | — |
| [multidocument-rustdoc-example-is-never-compiled](#multidocument-rustdoc-example-is-never-compiled) | note | `crates/yamalgam-serde/src/de.rs:87-100` | trivial | — |
| _The Runtime Performance Surface_ |  |  |  |  |
| [query-builder-rewalks-every-completed-node](#query-builder-rewalks-every-completed-node) | significant | `crates/yamalgam-query/src/value.rs:210-230` | medium | related: [identity-inventory-allocates-path-node-per-map-key](#identity-inventory-allocates-path-node-per-map-key) |
| [identity-inventory-allocates-path-node-per-map-key](#identity-inventory-allocates-path-node-per-map-key) | advisory | `crates/yamalgam-query/src/value.rs:763-780` | medium | related: [query-builder-rewalks-every-completed-node](#query-builder-rewalks-every-completed-node) |
| [string-range-retains-intermediate-string](#string-range-retains-intermediate-string) | advisory | `crates/yamalgam-query/src/valt.rs:135-150` | trivial | — |
| [parser-consumers-erase-concrete-hot-path-types](#parser-consumers-erase-concrete-hot-path-types) | note | `crates/yamalgam-serde/src/de.rs:22-26` | medium | — |
| [scanner-error-branches-remain-in-hot-functions](#scanner-error-branches-remain-in-hot-functions) | note | `crates/yamalgam-scanner/src/scanner.rs:354-375` | small | — |
| _The Supply Chain Surface_ |  |  |  |  |
| [xtask-unused-serde-and-toml](#xtask-unused-serde-and-toml) | note | `xtask/Cargo.toml:14-15` | trivial | — |
| [yamlstar-pulls-unmaintained-dlopen-derive](#yamlstar-pulls-unmaintained-dlopen-derive) | advisory | `.config/deny.toml:55-58` | small | — |

<sub>
Generated 2026-08-04 at commit a1123b4. Intermediate artifacts: recon.yaml,
findings.yaml, reviewer-verdicts.yaml, and report.html. Provenance: Codex
(GPT-5-generated, independently GPT-5-verified).
</sub>
