---
audit_date: 2026-08-11
commit: 7afeea1e98dd987fb8d014e46cc29cc57b2faca5
scope: Full Yamalgam workspace at the M12 schema-validation and language-server release boundary
findings:
  critical: 0
  significant: 8
  moderate: 10
  advisory: 2
  note: 0
---

# Audit: M12 schema validation and language server

M12's deterministic gates are clean, and the scanner, parser, query engine, dependency policy, and workspace unsafe census held up. The release boundary did not. This pass found 20 confirmed findings concentrated where YAML document semantics meet LSP positions, schema caching, and per-keystroke work. Release blocks sit in document context, cache identity, depth handling, and editor hot paths. The remaining correctness, lifecycle, and dependency-fitness work belongs in the same remediation record.

## LSP semantic positioning and request lifecycle

*Editor features are wired end to end, but several independently implemented position and document-context paths disagree with YAML and LSP semantics.*

<a id="leading-document-marker-shifts-lsp-document-indexes"></a>
### A leading YAML document marker shifts every LSP document index

**significant** · `completion.rs:101`, `definition.rs:243`, `diagnostics.rs:229` · effort: small

A stream beginning with `---` still has its first semantic document at index zero. Three LSP paths count that marker as a completed document, then use the shifted number to select a schema. The first document loses completion and hover. Later documents can receive the wrong schema.

```rust crates/yamalgam-lsp/src/diagnostics.rs:229-235
fn document_index_at(document: &DocumentState, offset: usize) -> usize {
    let text = document.syntax.text();
    let offset = offset.min(text.len());
    text[..offset]
        .lines()
        .filter(|line| line.trim_start().starts_with("---"))
        .count()
```

**Remediation:** Derive one semantic document-boundary map from the recovering CST or composed stream. Treat an initial marker as part of document zero and share the map across diagnostics, completion, hover, and definition.

<a id="sequence-item-schema-context-is-unreachable"></a>
### Sequence-item schemas are unreachable from completion and hover

**significant** · `crates/yamalgam-lsp/src/completion.rs:134-170` · effort: medium

Editor paths contain mapping keys but never sequence indexes. Stripping `- ` without appending the semantic item index means navigation sees `/name` where the schema expects `/0/name`. `items` and `prefixItems` context disappears.

```rust crates/yamalgam-lsp/src/completion.rs:159-170
fn mapping_entry(line: &str) -> Option<(String, &str)> {
    let line = line.strip_prefix("- ").unwrap_or(line);
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.starts_with(['[', '{']) {
        return None;
    }
    Some((
        key.trim_matches(|character| character == '\'' || character == '"')
            .to_owned(),
        value.trim(),
    ))
```

**Remediation:** Build paths from recovering CST ancestry with semantic sequence indexes. Completion and hover should share that path code.

<a id="ropey-unicode-lines-desynchronize-lsp-positions"></a>
### Ropey defaults broaden line splitting beyond the LSP position model

**moderate** · `crates/yamalgam-cst/Cargo.toml:12` · effort: small

Ropey's default `unicode_lines` feature recognizes VT, FF, NEL, U+2028, and U+2029 as line breaks. LSP positions don't. A scalar containing one of those characters shifts every later diagnostic and navigation position.

```toml crates/yamalgam-cst/Cargo.toml:12
ropey = "1.6.1"
```

**Remediation:** Disable Ropey defaults and select only `cr_lines` and `simd`. Lock the behavior down with UTF-16 conversion tests.

<a id="documented-example-hover-is-not-implemented"></a>
### Documented JSON Schema example hover is not implemented

**moderate** · `docs/yg/language-server.md:166`, `crates/yamalgam-schema/src/navigation.rs:24-43` · effort: small

The editor guide promises example hover. `SchemaContext` has no examples field, and the renderer has nothing to print. An example-only schema produces no example text and can produce no hover at all.

```markdown docs/yg/language-server.md:166
- Schema title, description, type, enum, default, deprecation, and example hover
```

**Remediation:** Add bounded, deterministic examples to `SchemaContext`. Existing combinator rules should collect them for safe plain-text hover.

<a id="lsp-cancellation-cannot-interrupt-work-and-retains-request-ids"></a>
### LSP cancellation cannot interrupt active work and retains completed request IDs

**moderate** · `crates/yamalgam-lsp/src/server.rs:52-61`, `server.rs:116-143`, `server.rs:197-209` · effort: medium

Requests and cancellation notifications share one synchronous receiver loop. The server can't observe a cancel until the request has finished, then retains the late ID until a future request reuses it.

```rust crates/yamalgam-lsp/src/server.rs:52-61
    while let Ok(message) = connection.receiver.recv() {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                server.handle_request(&connection, request)?;
            }
            Message::Notification(notification) => {
                server.handle_notification(&connection, notification)?;
```

**Remediation:** Track active requests separately and run cancellable work outside the receiver loop. Clear request state on every completion.

<a id="stdio-error-path-can-hang-joining-reader"></a>
### The stdio error path can hang while joining the reader thread

**moderate** · `crates/yamalgam-lsp/src/server.rs:28-33` · effort: small

If `run` returns an error while the client keeps stdin open, `IoThreads::join` waits on the blocking reader before the server can report the original failure.

```rust crates/yamalgam-lsp/src/server.rs:28-33
/// Run the language server over standard input and standard output.
pub fn run_stdio() -> Result<(), ServerError> {
    let (connection, threads) = Connection::stdio();
    let result = run(connection);
    threads.join()?;
    result
```

**Remediation:** Join after clean shutdown or EOF, or use a transport lifecycle that can stop the reader before joining an error path.

<a id="lsp-definition-resource-errors-erased"></a>
### Definition lookup converts resource failures into a successful not-found result

**moderate** · `crates/yamalgam-lsp/src/definition.rs:67-93` · effort: medium

Policy, I/O, size-limit, and UTF-8 failures collapse into `Option::None`. The server returns a successful JSON-RPC null response, making a broken resource indistinguishable from an absent target.

```rust crates/yamalgam-lsp/src/definition.rs:82-89
                .ok()?;
            let resource = retriever
                .retrieve(&ResourceRequest {
                    id: id.clone(),
                    max_bytes: Some(16 * 1024 * 1024),
                })
                .ok()?;
```

**Remediation:** Return `Result<Option<_>, WorkspaceResourceError>`. Keep `Ok(None)` for no target and surface sanitized backend failures separately.

*These failures share one cause: the editor boundary has several local approximations of semantic state where it needs one shared model.*

## Schema compilation, validation, and retrieval boundaries

*Schema handling is policy-gated, but cache identity, depth, work limits, and lock scope aren't yet strong enough for untrusted editor workloads.*

<a id="schema-validator-cache-key-erases-identity-and-options"></a>
### The validator cache key drops canonical identity and diagnostic options

**significant** · `crates/yamalgam-lsp/src/diagnostics.rs:48-58`, `crates/yamalgam-resolve/src/resource.rs:159-169` · effort: small

The cache key renders `ResourceId` through a redacted display form that removes URI queries and omits `max_diagnostics`. Equal schema content can return a validator compiled for another base identity or report cap.

```rust crates/yamalgam-lsp/src/diagnostics.rs:53-58
        let document = self.session.load(&id)?;
        let digest = semantic_digest(&document.document.value);
        let key = (document.id.to_string(), digest);
        if let Some(validator) = self.lock_validators().get(&key) {
            return Ok(Arc::clone(validator));
```

Related: [`schema-session-cache-clones-located-documents`](#schema-session-cache-clones-located-documents).

**Remediation:** Use a typed cache key that retains canonical `ResourceId` and semantic digest. Compile options that change behavior belong in it too.

<a id="schema-native-adapter-unbounded-recursion"></a>
### Native schema materialization recurses without an input depth bound

**significant** · `crates/yamalgam-schema/src/value.rs:318-346` · effort: medium

Public validation accepts programmatic `Value` trees without an instance-depth ceiling. Error materialization and equality recurse through the tree, so a deeply nested value can exhaust the thread stack instead of returning a typed report.

```rust crates/yamalgam-schema/src/value.rs:327-336
        Value::Sequence(items) => JsonValue::Array(
            items
                .iter()
                .map(|value| {
                    materialize(YamlNode {
                        value,
                        projected_keys: node.projected_keys,
                    })
                })
```

**Remediation:** Convert materialization and equality to explicit work stacks and reject excessive instance depth before entering `jsonschema`.

<a id="schema-diagnostic-cap-does-not-bound-work"></a>
### Diagnostic limit caps retention but not validation work

**significant** · `crates/yamalgam-schema/src/schema.rs:171-206`, `diagnostic.rs:46-68` · effort: medium

`max_diagnostics` limits retained output. It still enumerates and converts every error—including string allocation and span lookup—before throwing excess diagnostics away.

```rust crates/yamalgam-schema/src/schema.rs:197-206
        let mut observed = 0_usize;
        let mut diagnostics = Vec::new();
        for error in self.validator.iter_errors(prepared.root()) {
            observed = observed.saturating_add(1);
            retain_diagnostic(
                &mut diagnostics,
                schema_diagnostic(&error, located),
                self.options.max_diagnostics,
            );
        }
```

**Remediation:** Stop after the cap plus one sentinel and record only whether more errors existed.

<a id="schema-session-holds-global-lock-across-retrieval"></a>
### SchemaSession holds its global cache lock across retrieval and parsing

**moderate** · `crates/yamalgam-schema/src/retrieval.rs:122-126`, `retrieval.rs:267-365` · effort: medium

One mutex remains held through pluggable retrieval, UTF-8 decoding, and composition. Unrelated schema loads serialize. A backend that re-enters the session can deadlock.

```rust crates/yamalgam-schema/src/retrieval.rs:122-126
    pub fn load(&self, id: &ResourceId) -> Result<SchemaDocument, SchemaRetrieveError> {
        self.enforce_policy(id, None)?;
        let mut state = self.lock_state();
        self.load_document(&mut state, id)
    }
```

**Remediation:** Reserve per-key work and the aggregate byte budget under the lock. Reject same-key reentrancy. Release the lock for I/O/parsing, then reconcile and publish atomically.

<a id="schema-session-cache-clones-located-documents"></a>
### Schema cache hits deep-clone documents before checking the validator cache

**moderate** · `crates/yamalgam-schema/src/retrieval.rs:272-275`, `crates/yamalgam-lsp/src/diagnostics.rs:53-58` · effort: medium

Every cached editor lookup clones the semantic and location trees, then hashes the schema before discovering the compiled validator already exists.

```rust crates/yamalgam-schema/src/retrieval.rs:272-275
        let canonical = state.aliases.get(requested).unwrap_or(requested).clone();
        if let Some(document) = state.documents.get(&canonical) {
            return Ok(document.clone());
        }
```

Related: [`schema-validator-cache-key-erases-identity-and-options`](#schema-validator-cache-key-erases-identity-and-options).

**Remediation:** Store documents behind `Arc` and retain their semantic digest at load time.

<a id="schema-navigation-recompiles-patterns-per-request"></a>
### Schema navigation recompiles every patternProperties regex on each lookup

**advisory** · `crates/yamalgam-schema/src/navigation.rs:351-355` · effort: small

Completion and hover compile immutable patterns during every instance-path traversal. Pattern-rich schemas pay regex parser and automaton costs on routine cursor movement.

```rust crates/yamalgam-schema/src/navigation.rs:351-355
            if let Some(patterns) = node.get("patternProperties").and_then(JsonValue::as_object) {
                for pattern in patterns.keys() {
                    if Regex::new(pattern).is_ok_and(|regex| regex.is_match(segment)) {
                        matched = true;
                        push_cursor(
```

**Remediation:** Compile patterns while building the navigation index and retain them beside their target cursors.

*The policy boundary is sound. The runtime around it still does too much work under the wrong identity and lock scope.*

## Recovering CST and editor update cost

*Incremental syntax reparsing is local; the surrounding span, composition, selection, and position work repeatedly scans or copies the whole document.*

<a id="recovering-cst-span-normalization-is-quadratic"></a>
### Recovery parsing recomputes spans with repeated prefix scans

**significant** · `crates/yamalgam-cst/src/recovery.rs:78-87`, `recovery.rs:118-130`, `recovery.rs:198-217` · effort: medium

Normalization visits both endpoints of every node and token. Each endpoint recomputes its mark by scanning from the start of the input. Token-rich recovery parses grow quadratically—the wrong curve for incomplete documents on an editor hot path.

```rust crates/yamalgam-cst/src/recovery.rs:118-130
fn normalize_spans(node: &mut CstNode<'_>, input: &str) {
    node.span.start = mark_at_offset(input, node.span.start.offset);
    node.span.end = mark_at_offset(input, node.span.end.offset);
    for child in &mut node.children {
        match child {
            CstElement::Node(child) => normalize_spans(child, input),
            CstElement::Token(token) => {
                token.span.start = mark_at_offset(input, token.span.start.offset);
                token.span.end = mark_at_offset(input, token.span.end.offset);
            }
        }
    }
}
```

**Remediation:** Preserve marks during parsing or normalize every requested offset in one ordered linear scan.

<a id="lsp-didchange-rebuilds-full-document-stages"></a>
### Each LSP change repeats full-document copies and composition

**significant** · `crates/yamalgam-cst/src/incremental.rs:82-100`, `crates/yamalgam-lsp/src/document.rs:58-101` · effort: large

Each protocol edit can materialize the original and updated Rope, then trigger full located composition. Schema selection copies and composes the same text again. Per-keystroke incremental parsing is carrying several full-document passengers.

```rust crates/yamalgam-cst/src/incremental.rs:88-99
        let original = self.text.to_string();
        let ordered = validate_edits(edits, &original)?;
        let document = self.document_reparse_candidate(&ordered, &original);

        for edit in ordered.iter().rev() {
            let start = self.text.byte_to_char(edit.range.start);
            let end = self.text.byte_to_char(edit.range.end);
            self.text.remove(start..end);
            self.text.insert(start, &edit.replacement);
        }

        let updated = self.text.to_string();
```

**Remediation:** Batch protocol changes against one snapshot and materialize final text once. Selection should consume the composed document already in hand.

<a id="lsp-diagnostic-positioning-recopies-source"></a>
### Diagnostic range conversion repeatedly allocates document strings

**significant** · `crates/yamalgam-lsp/src/diagnostics.rs:127-143`, `crates/yamalgam-cst/src/incremental.rs:70-74`, `incremental.rs:139-160` · effort: medium

Each diagnostic converts two endpoints. Every endpoint calls `text().len()`, and `text()` copies the whole Rope before UTF-16 conversion allocates a line prefix. Dozens of diagnostics can copy the document hundreds of times after one edit.

```rust crates/yamalgam-cst/src/incremental.rs:70-74
    /// Return the current source text.
    #[must_use]
    pub fn text(&self) -> String {
        self.text.to_string()
    }
```

**Remediation:** Expose Rope byte length without allocating and count UTF-16 units over chunks. Batch conversions against a versioned line index.

*The CST is incremental. The editor pipeline around it isn't—yet.*

## Located value representation

*The parallel semantic and location trees enable precise diagnostics, but their public shape and construction path neither enforce nor preserve the invariant efficiently.*

<a id="located-value-allows-invalid-parallel-shapes"></a>
### Public located values permit mismatched semantic and location trees

**moderate** · `crates/yamalgam-core/src/located.rs:5-20`, `located.rs:60-80` · effort: medium

Callers can pair unrelated variants, lengths, and mapping order because both trees are public. Pointer traversal trusts the invariant and can return no span or another key's span.

```rust crates/yamalgam-core/src/located.rs:5-20
/// A semantic YAML value and its value-shaped source-location tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LocatedValue {
    /// Composed semantic value.
    pub value: Value,
    /// Source locations with the same shape and child order as `value`.
    pub locations: ValueLocations,
}

/// Source location for one semantic value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueLocations {
    /// Source span for this value.
    pub span: Span,
    /// Child locations matching the value's semantic kind.
    pub kind: LocationKind,
```

Related: [`located-composition-clones-location-subtrees`](#located-composition-clones-location-subtrees).

**Remediation:** Seal construction behind a checked constructor and read-only accessors, or couple each semantic node directly to its span and children.

<a id="located-composition-clones-location-subtrees"></a>
### Value-only composition builds and repeatedly clones location subtrees

**moderate** · `crates/yamalgam-compose/src/lib.rs:269-276`, `lib.rs:392-399` · effort: medium

The composer owns every `LocatedValue` but deep-clones child locations before consuming only their values. Value-only callers also build the complete location tree just to discard it.

```rust crates/yamalgam-compose/src/lib.rs:392-399
                let locations = items.iter().map(|item| item.locations.clone()).collect();
                let value = LocatedValue {
                    value: Value::Sequence(items.into_iter().map(|item| item.value).collect()),
                    locations: ValueLocations {
                        span: encompassing_span(start_span, end_span),
                        kind: LocationKind::Sequence(locations),
                    },
                };
```

Related: [`located-value-allows-invalid-parallel-shapes`](#located-value-allows-invalid-parallel-shapes).

**Remediation:** Move-unzip each located child once and add a value-only accumulator that never builds locations.

*The invariant is useful. It needs to be enforced by construction and paid for only by callers that request it.*

## Retrieval and dependency lifecycle

*The retrieval boundary passes advisory, license, unused-dependency, and workspace unsafe checks. Worker lifecycle and feature scope still need work.*

<a id="http-worker-panic-result-discarded"></a>
### HTTP worker panics are silently discarded during shutdown

**moderate** · `crates/yamalgam-resolve/src/retriever/http.rs:144-156` · effort: small

A worker panic becomes a generic channel closure, then Drop discards the join result and the only remaining panic payload. Operators can't distinguish a worker defect from ordinary shutdown.

```rust crates/yamalgam-resolve/src/retriever/http.rs:144-156
impl Drop for LibrebarRetriever {
    fn drop(&mut self) {
        self.work.take();
        let worker = self
            .worker
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}
```

**Remediation:** Record typed worker termination in shared state and expose fallible explicit shutdown. Drop should emit a structured event as the final fallback.

<a id="librebar-http-enables-unused-tokio-subsystems"></a>
### Librebar HTTP activates Tokio signal and macro features unused by retrieval

**advisory** · `crates/yamalgam-resolve/Cargo.toml:15-18` · effort: medium

Librebar's `http` feature enables Tokio macros and signal handling even though neither production HTTP path uses them. Cargo feature unification adds those subsystems to Yamalgam's shipped graph.

```toml crates/yamalgam-resolve/Cargo.toml:15-18
librebar = { version = "0.6.0", default-features = false, features = ["http"] }
percent-encoding = "2.3"
thiserror = "2.0"
tokio = { version = "1.52", features = ["rt", "sync"] }
```

**Remediation:** Narrow Librebar's Tokio features upstream. Move signal behind its shutdown feature and macros to dev-dependencies before updating Yamalgam.

*No CVE, license, unused-dependency, FFI, or workspace-unsafe release blocker survived this pass.*

## Remediation ledger

| Narrative | Finding | Concern | State | Code commit |
|---|---|---:|---|---|
| LSP semantics | `leading-document-marker-shifts-lsp-document-indexes` | significant | Open | - |
| LSP semantics | `sequence-item-schema-context-is-unreachable` | significant | Open | - |
| LSP semantics | `ropey-unicode-lines-desynchronize-lsp-positions` | moderate | Open | - |
| LSP semantics | `documented-example-hover-is-not-implemented` | moderate | Open | - |
| LSP semantics | `lsp-cancellation-cannot-interrupt-work-and-retains-request-ids` | moderate | Open | - |
| LSP semantics | `stdio-error-path-can-hang-joining-reader` | moderate | Open | - |
| LSP semantics | `lsp-definition-resource-errors-erased` | moderate | Open | - |
| Schema runtime | `schema-validator-cache-key-erases-identity-and-options` | significant | Open | - |
| Schema runtime | `schema-native-adapter-unbounded-recursion` | significant | Open | - |
| Schema runtime | `schema-diagnostic-cap-does-not-bound-work` | significant | Open | - |
| Schema runtime | `schema-session-holds-global-lock-across-retrieval` | moderate | Open | - |
| Schema runtime | `schema-session-cache-clones-located-documents` | moderate | Open | - |
| Schema runtime | `schema-navigation-recompiles-patterns-per-request` | advisory | Open | - |
| Editor cost | `recovering-cst-span-normalization-is-quadratic` | significant | Open | - |
| Editor cost | `lsp-didchange-rebuilds-full-document-stages` | significant | Open | - |
| Editor cost | `lsp-diagnostic-positioning-recopies-source` | significant | Open | - |
| Located values | `located-value-allows-invalid-parallel-shapes` | moderate | Open | - |
| Located values | `located-composition-clones-location-subtrees` | moderate | Open | - |
| Retrieval | `http-worker-panic-result-discarded` | moderate | Open | - |
| Retrieval | `librebar-http-enables-unused-tokio-subsystems` | advisory | Open | - |

---

<sub>Generated 2026-08-11. Source artifacts: `recon.yaml`, `findings.yaml`, `reviewer-verdicts.yaml`, `report.html`.</sub>
