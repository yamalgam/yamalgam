# M12 Schema Validation and LSP Delivery Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to work through this plan task-by-task. Track every
> checkbox in place. Do not create a worktree; Clay's workflow uses the current
> branch directly.

**Goal:** Ship native YAML schema validation through `yg`, plus a standalone
YAML language server with diagnostics, completion, hover, and source
navigation for schemas, `$ref`, and `!include`.

**Architecture:** Add `yamalgam-schema` above `yamalgam-core` and
`yamalgam-resolve`. It adapts `yamalgam_core::Value` directly to the current
`jsonschema` custom-representation API, so instances never round-trip through
JSON. A prepared instance view exposes scalar YAML mapping keys through
collision-checked canonical property names while retaining their original
types and spans. Add a source-location tree to the composer, a Rowan/Ropey
syntax model beside the existing CST API, and `yamalgam-lsp` as a synchronous
`lsp-server` frontend. Schema selection, retrieval, validation, and
introspection stay reusable by both CLI and LSP consumers.

**Specification:**
`docs/spec/yg-query-language-spec-draft-01.md` sections 10 and 11 Phase 5,
plus Milestone 12 in `record/superpowers/plans/README.md`.

**Tech stack:** Rust 2024 with MSRV 1.89; `jsonschema` 0.49.8 with default
features disabled; `lsp-server` 0.10.0 and `lsp-types` 0.97.0 for LSP 3.17;
`rowan` 0.16.1 for immutable green/red syntax trees; `ropey` 1.6.1 for text
edits and UTF-16 position conversion; existing Yamalgam parser, composer,
CST, resolver, Librebar HTTP, and `cargo nextest`/`just` gates.

## Build-vs-adopt decision

| Existing project | Adopt | Keep in Yamalgam |
|---|---|---|
| `jsonschema` 0.49.x | Draft 2020-12 evaluation, references, structured errors, official-suite coverage | `Value` adapter, YAML-only preflight, M11 policy/retrieval, source spans, schema navigation |
| `yaml-schema` 0.9.3 | MIT-licensed YAML mapping-key cases, canonical scalar-key projection, source-located CLI expectations | Do not adopt its Saphyr DOM, bespoke keyword engine, typed-`propertyNames` extension, or ambient `reqwest` retrieval |
| Red Hat `yaml-language-server` | Behavior and interoperability baseline, schema-association precedence | Yamalgam parser/CST, native YAML values, multi-document streams, `!include` and data `$ref` |
| `lsp-server` from rust-analyzer | JSON-RPC framing, initialization, request/notification transport, in-memory tests | Document state, validation, completion, hover, definitions, policy |
| SchemaStore | Catalog schema and file-match data | Opt-in retrieval, deterministic selection, budgets, cache, diagnostics |

**Verdict:** EXTEND `jsonschema`, `lsp-server`, and SchemaStore, and port the
useful YAML-boundary cases from `yaml-schema`; building a second JSON Schema
engine would add risk without adding Yamalgam value.

## `yaml-schema` borrowing boundary

Audit snapshot: `yaml-schema` 0.9.3 at commit
`1c9403a7e4eaf2e5de2ea39594279ce73df258fc` (2026-08-06), MIT licensed.

- Borrow its separation between schema loading, validation context, and CLI
  reporting as testable boundary concepts, not as production types.
- Port the scalar mapping-key cases: strings borrow directly; null, Boolean,
  integer, and finite float keys receive their canonical string property name;
  the original key node and span remain available for diagnostics.
- Add a stricter rule absent upstream: two distinct YAML keys that project to
  the same property name are an error, never last-write-wins behavior.
- Port the source-location and structured-output expectations, then express
  them through Yamalgam's richer RFC 6901 paths, spans, codes, document index,
  and file identity.
- Keep JSON Schema `propertyNames` standards-correct: it validates projected
  string property names. Do not copy `yaml-schema`'s extension that treats a
  mapping key as an integer, number, Boolean, or null instance.
- Do not copy its evaluator, typed schema AST, direct filesystem/HTTP loader,
  dot-path diagnostics, first-document-only engine, format validators, or
  Cucumber dependency. `jsonschema`, M11 retrieval, `LocatedValue`, and native
  Rust tests cover those responsibilities.
- Adapt only the cases needed for this boundary. Store the upstream commit,
  source paths, changes, and MIT text beside the adapted tests.

## Global constraints

- Do not write JSON Schema keyword semantics in Yamalgam. Adapt and wrap
  `jsonschema` 0.49.8; pin the exact version because its custom-representation
  API is young.
- Do not convert YAML instances to `serde_json::Value`. The schema document may
  convert after rejecting non-string mapping keys and non-finite numbers,
  because a JSON Schema document is itself JSON-compatible data.
- Expose scalar YAML mapping keys as canonical JSON Schema property names
  without changing the underlying `Value`. Reject complex mapping keys,
  non-finite numbers, and canonical-name collisions with explicit Yamalgam
  diagnostics. Never drop, overwrite, or silently stringify them.
- Configure the linear-time regex backend. Do not enable `jsonschema`'s
  default HTTP, file, or TLS features, and do not use its default backtracking
  regex engine.
- Route every external schema through a Yamalgam `ResourceRetriever` adapter,
  `LoaderConfig.resolution.refs/files/limits`, and the M11 redaction rules. Do
  not create a parallel schema retrieval policy.
- Keep SchemaStore and network schema retrieval disabled by default. Local
  schemas stay capability-rooted to the workspace or explicit schema root.
- Preserve the existing `CstNode`, `CstBuilder`, and `parse_to_cst*` public
  APIs. Add the Rowan syntax API beside them; do not force existing consumers
  through a migration in M12.
- Preserve the current scanner/parser behavior for ordinary loading. Recovery
  is an explicit CST/LSP mode and must always make forward progress.
- Parse and validate every document in a YAML stream independently. Report the
  zero-based document index in CLI data and LSP diagnostic metadata.
- Use UTF-8 byte offsets inside Yamalgam and convert to negotiated LSP UTF-16
  positions only at the protocol boundary.
- Keep LSP stdout exclusively for protocol frames. Logging goes through
  Librebar/tracing to stderr or its configured log destination.
- Keep the `yamalgam` CLI crate thin. Validation algorithms live in
  `yamalgam-schema`; LSP behavior lives in `yamalgam-lsp`.
- Do not add formatting, quick fixes, code actions, workspace symbols,
  persistent schema cache, lockfiles, authenticated retrieval, or HTTP data
  includes in M12.
- Follow RED -> GREEN -> focused verification for every task. Do not manually
  increment any project version.

## File map

- `yamalgam-core/src/located.rs` owns `LocatedValue` and the value-shaped
  source-location tree.
- `yamalgam-core/src/value.rs` owns the single canonical scalar-text helper
  shared by pointer lookup and schema property projection.
- `yamalgam-compose/src/lib.rs` builds located and ordinary `Value` documents
  through one composition path.
- `yamalgam-schema/src/value.rs` adapts core `Value` to `jsonschema::json`.
- `yamalgam-schema/src/preflight.rs` owns prepared instance views, canonical
  scalar-key projection, collision detection, and representation diagnostics.
- `yamalgam-schema/src/schema.rs` converts and compiles schema documents.
- `yamalgam-schema/src/retrieval.rs` bridges M11 retrievers to schema `$ref`.
- `yamalgam-schema/src/diagnostic.rs` owns stable machine-readable validation
  reports.
- `yamalgam-schema/src/selection.rs` owns pragmas, associations, and
  SchemaStore selection.
- `yamalgam-schema/src/navigation.rs` maps instance paths to applicable schema
  nodes for completion and hover.
- `yamalgam-cst/src/syntax.rs` owns Rowan kinds and green/red trees.
- `yamalgam-cst/src/recovery.rs` owns multi-error, forward-progress recovery.
- `yamalgam-cst/src/incremental.rs` owns Ropey text and document-scoped
  reparsing.
- `yamalgam-query/src/validation.rs` owns query-to-schema integration and
  typed runtime failures.
- `yamalgam/src/commands/validate.rs` owns CLI argument validation and output.
- `yamalgam-lsp/src/document.rs` owns one versioned document snapshot.
- `yamalgam-lsp/src/server.rs` owns LSP lifecycle and dispatch.
- `yamalgam-lsp/src/diagnostics.rs`, `completion.rs`, `hover.rs`, and
  `definition.rs` own individual language features.
- `docs/yg/validation.md` and `docs/yg/language-server.md` own end-user
  guidance. The query spec and deviation notes own normative status.

---

### Task 1: Add the native Value adapter and schema crate

**Files:**

- Create `crates/yamalgam-schema/Cargo.toml`.
- Create `crates/yamalgam-schema/src/lib.rs`.
- Create `crates/yamalgam-schema/src/preflight.rs`.
- Create `crates/yamalgam-schema/src/value.rs`.
- Create `crates/yamalgam-schema/tests/upstream/yaml-schema/README.md`.
- Create `crates/yamalgam-schema/tests/upstream/yaml-schema/LICENSE`.
- Modify `crates/yamalgam-core/src/value.rs`.
- Modify `.justfile` with focused M12 recipes.

**Interfaces:**

```rust
pub(crate) struct YamlJson;
pub(crate) struct PreparedInstance<'value> {
    value: &'value yamalgam_core::Value,
    projected_keys: ProjectedKeys,
}
pub(crate) struct YamlNode<'value> {
    value: &'value yamalgam_core::Value,
    projected_keys: &'value ProjectedKeys,
}
pub(crate) struct YamlObject<'value>(YamlNode<'value>);
pub(crate) struct YamlArray<'value>(YamlNode<'value>);
pub(crate) enum YamlNumber {
    Integer(i64),
    Float(f64),
}
pub(crate) enum InstanceStep {
    Sequence(usize),
    MappingKey(usize),
    MappingValue(usize),
}
pub(crate) struct RepresentationError {
    pub kind: RepresentationErrorKind,
    pub instance_path: String,
    pub route: Vec<InstanceStep>,
    pub conflicting_route: Option<Vec<InstanceStep>>,
}

impl<'value> PreparedInstance<'value> {
    pub fn new(value: &'value Value) -> Result<Self, Vec<RepresentationError>>;
    pub fn root(&'value self) -> YamlNode<'value>;
}

impl Value {
    pub fn canonical_scalar_text(&self) -> Option<std::borrow::Cow<'_, str>>;
}
```

- [x] **Step 1: Write adapter conformance tests**

Build the document returned by `jsonschema::json::conformance::document()` as
a `yamalgam_core::Value`, prepare it with `PreparedInstance`, and call:

```rust
let prepared = PreparedInstance::new(&value).expect("conformance value");
jsonschema::json::conformance::assert_conformance::<YamlJson>(&prepared.root());
```

Add focused unit tests beside the private adapter proving mapping lookup is
borrowed, node identity is stable, integers remain integers, string length
counts Unicode scalar values, and `YamlNode::to_value` is used only on cold
paths.

Adapt the projection and rejection boundary from the upstream `yaml-schema`
mapping-key scenarios into unit tests beside `preflight`. Cover string, null,
Boolean, integer, and finite float keys; borrowed string names; canonical
projections; complex keys; non-finite keys; and collisions such as `1`, `1.0`,
and `"1"`. Defer keyword behavior to Task 3 and key-span assertions to Task 5,
where those interfaces exist. Record the upstream commit, original
feature/source paths, semantic changes, and MIT license beside the tests. Do
not add Cucumber or `yaml-schema` as a dependency.

- [x] **Step 2: Run the focused test to verify RED**

```bash
just test-schema value_adapter
```

Expected: fail because the crate and adapter do not exist.

- [x] **Step 3: Add the crate with the exact dependency boundary**

Use this dependency set:

```toml
[dependencies]
jsonschema = { version = "=0.49.8", default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }

[dev-dependencies]
jsonschema = { version = "=0.49.8", default-features = false, features = ["conformance"] }
```

Add `Json`, `Node`, `Object`, `Array`, and `JsonNumber` for the wrapper
types. Use `NodeIdentity::new(value as *const Value as usize)`. Return borrowed
strings and values; store owned property-name strings only for non-string
scalar keys. Build and validate `ProjectedKeys` once in `PreparedInstance`, so
`Object::get`, `Object::members`, and `Node::to_value` share one mapping and
cannot disagree. Put null/Boolean/integer/finite-float/string text conversion
in `Value::canonical_scalar_text`; do not duplicate it in schema or located
pointer code. Do not clone the instance tree.

- [x] **Step 4: Add focused Just recipes**

```make
test-schema filter='':
  cargo nextest run -p yamalgam-schema -E 'test({{filter}})'

test-lsp filter='':
  cargo nextest run -p yamalgam-lsp -E 'test({{filter}})'

feature-matrix:
  cargo hack check --workspace --each-feature --no-dev-deps
```

Make an empty filter run the whole selected crate by branching in the recipe,
rather than passing `test()` to nextest.

- [x] **Step 5: Run the adapter tests to verify GREEN**

```bash
just test-schema value_adapter
just test-schema yaml_schema_compat
just clippy
```

- [x] **Step 6: Commit**

```bash
git add crates/yamalgam-core crates/yamalgam-schema .justfile Cargo.lock
git commit -m "feat(schema): adapt native YAML values"
```

### Task 2: Preserve source locations through composition

**Files:**

- Create `crates/yamalgam-core/src/located.rs`.
- Modify `crates/yamalgam-core/src/lib.rs`.
- Modify `crates/yamalgam-compose/src/lib.rs`.
- Create `crates/yamalgam-compose/tests/located.rs`.

**Interfaces:**

```rust
pub struct LocatedValue {
    pub value: Value,
    pub locations: ValueLocations,
}

pub struct ValueLocations {
    pub span: Span,
    pub kind: LocationKind,
}

pub enum LocationKind {
    Scalar,
    Sequence(Vec<ValueLocations>),
    Mapping(Vec<MappingLocations>),
}

pub struct MappingLocations {
    pub key: ValueLocations,
    pub value: ValueLocations,
}

impl LocatedValue {
    pub fn span_at_pointer(&self, pointer: &str) -> Option<Span>;
}
```

- [x] **Step 1: Write failing located-composition tests**

Cover a nested mapping/sequence, a key containing `/` and `~`, an alias, a
merge key, and two YAML documents. Assert `/service/ports/1` resolves to the
value token span. Alias-root diagnostics point to the alias use; merged child
properties retain the anchor definition span.

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-compose --test located
```

- [x] **Step 3: Add the location tree without changing `Value`**

Refactor the composer's internal `AnchoredValue` to carry `LocatedValue`.
Build value and location children in the same stack frame. Keep
`compose_stream()` by mapping `LocatedValue` to `.value`, and add:

```rust
pub fn compose_stream_located(&mut self) -> Result<Vec<LocatedValue>, ComposeError>;
pub fn from_str_located(input: &str) -> Result<Vec<LocatedValue>, ComposeError>;
pub fn from_str_single_located(input: &str) -> Result<LocatedValue, ComposeError>;
```

Resolve JSON Pointer segments against the semantic `Value` and the parallel
location tree together. Resolve scalar mapping keys through the same canonical
property names as `PreparedInstance`; return no match for complex keys or a
canonical-name collision.

- [x] **Step 4: Re-run existing composer tests and located tests**

```bash
cargo nextest run -p yamalgam-compose
just test-schema value_adapter
```

- [x] **Step 5: Commit**

```bash
git add crates/yamalgam-core crates/yamalgam-compose
git commit -m "feat(compose): retain semantic source locations"
```

### Task 3: Compile Draft 2020-12 schemas safely

**Files:**

- Create `crates/yamalgam-schema/src/schema.rs`.
- Create `crates/yamalgam-schema/src/diagnostic.rs`.
- Modify `crates/yamalgam-schema/src/lib.rs`.
- Create `crates/yamalgam-schema/tests/validation.rs`.
- Create `crates/yamalgam-schema/tests/yaml_schema_compat.rs`.

**Interfaces:**

```rust
pub struct SchemaOptions {
    pub base_uri: Option<String>,
    pub validate_formats: bool,
    pub max_diagnostics: usize,
}

pub struct SchemaValidator {
    schema: serde_json::Value,
    validator: jsonschema::Validator<YamlJson>,
    options: SchemaOptions,
}

impl SchemaValidator {
    pub fn compile(schema: &Value, options: SchemaOptions) -> Result<Self, SchemaError>;
    pub fn compile_located(
        schema: &LocatedValue,
        options: SchemaOptions,
    ) -> Result<Self, SchemaError>;
    pub fn validate(&self, instance: &Value) -> ValidationReport;
    pub fn validate_located(&self, instance: &LocatedValue) -> ValidationReport;
}
```

- [x] **Step 1: Write compilation and validation tests**

Cover Boolean schemas, `type`, `required`, `properties`, `items`,
`prefixItems`, `enum`, `const`, numeric bounds, string length, `pattern`,
`allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`, and local `$defs`/`$ref`.
Use YAML instances with anchors and merge keys to prove validation sees the
composed native value.

Add negative schema tests for non-string schema keys, non-finite schema
numbers, unsupported regex syntax under the linear backend, and invalid
Draft 2020-12 schema structure.

Add YAML-boundary tests proving scalar instance keys participate in
`properties`, `patternProperties`, `additionalProperties`, `required`, and
string-semantic `propertyNames` through their canonical names. Assert complex
keys and projected-name collisions stop validation before the schema engine.
Keep the adapted upstream cases and provenance in `yaml_schema_compat.rs` so
their source remains obvious without exposing the private adapter types.

- [x] **Step 2: Verify RED**

```bash
just test-schema validation
```

- [x] **Step 3: Convert only schema documents to JSON-compatible data**

Write a checked, iterative `schema_value_to_json` conversion. Return:

```rust
#[non_exhaustive]
pub enum SchemaError {
    NonStringKey { path: String },
    NonFiniteNumber { path: String },
    InvalidSchema { message: String, schema_path: String },
    UnsupportedPattern { pattern: String, message: String },
    Retrieval(SchemaRetrieveError),
}
```

Compile through `jsonschema::options_for::<YamlJson>()` with
`Draft::Draft202012`, schema validation enabled,
`.should_validate_formats(options.validate_formats)`, and
`.with_pattern_options(PatternOptions::regex())`.

- [x] **Step 4: Prepare YAML-only instance shapes**

Walk the instance iteratively with `PreparedInstance::new` before calling
`jsonschema`. Borrow string keys and record canonical names for null, Boolean,
integer, and finite float keys. Emit `yaml/complex-key` at a sequence or
mapping key, `yaml/non-finite-number` at the scalar span, and
`yaml/projected-key-collision` at both colliding key spans. If preparation
emits any diagnostic, do not present a partial object to the JSON Schema
engine. Standard `propertyNames` sees canonical strings; retain original key
types and spans for diagnostics and future YAML-specific policy. Carry mapping
entry ordinals in each representation error so `validate_located` can reach
keys that an RFC 6901 pointer cannot name or names ambiguously.

- [x] **Step 5: Verify GREEN and dependency policy**

```bash
just test-schema validation
just deny
just clippy
```

- [x] **Step 6: Commit**

```bash
git add crates/yamalgam-schema Cargo.lock
git commit -m "feat(schema): compile and validate draft 2020-12"
```

### Task 4: Reuse M11 retrieval for schema references

**Files:**

- Create `crates/yamalgam-schema/src/retrieval.rs`.
- Modify `crates/yamalgam-schema/src/schema.rs`.
- Modify `crates/yamalgam-schema/Cargo.toml`.
- Create `crates/yamalgam-schema/tests/retrieval.rs`.

**Interfaces:**

```rust
pub struct SchemaSession {
    retriever: Arc<dyn ResourceRetriever>,
    config: LoaderConfig,
    state: std::sync::Mutex<SchemaSessionState>,
}

pub struct SchemaDocument {
    pub id: ResourceId,
    pub document: LocatedValue,
}

impl SchemaSession {
    pub fn load(&self, id: &ResourceId) -> Result<SchemaDocument, SchemaRetrieveError>;
    pub fn compile(&self, root: SchemaDocument) -> Result<SchemaValidator, SchemaError>;
}
```

- [x] **Step 1: Write policy and reference tests**

Use `MemoryRetriever` for relative, absolute, cyclic, missing, oversized, and
over-budget `$ref` graphs. Use `FileRetriever` under a temporary capability
root for local refs and `LibrebarRetriever`'s existing mock path for HTTPS.
Assert denied schemes/hosts fail before backend calls and public errors redact
URI user information and query values. Drive every case through existing
`ResolutionPolicy.refs`, `files`, and `limits`; prove schema retrieval observes
`ResourceLimits::max_input_bytes` as a per-document ceiling and
`ResolutionLimits::max_fetched_bytes` across the session.

- [x] **Step 2: Verify RED**

```bash
just test-schema retrieval
```

- [x] **Step 3: Build the `jsonschema::Retrieve` bridge**

The bridge converts an absolute schema URI to `ResourceId`, checks the existing
ref mode, scheme, host, file capability, depth, edge count, and byte ceilings,
calls `ResourceRetriever::retrieve` with a bounded `ResourceRequest`,
parses exactly one located YAML or JSON document with Yamalgam, and converts that
schema document through `schema_value_to_json`. Cache canonical resources per
`SchemaSession`. Charge cache hits as edges.

Do not enable `jsonschema`'s `resolve-file` or `resolve-http` features.

- [x] **Step 4: Verify refs and M11 regressions**

```bash
just test-schema retrieval
cargo nextest run -p yamalgam-resolve
just deny
```

- [x] **Step 5: Commit**

```bash
git add crates/yamalgam-schema Cargo.lock
git commit -m "feat(schema): resolve schemas through yamalgam policy"
```

### Task 5: Stabilize machine-readable validation diagnostics

**Files:**

- Modify `crates/yamalgam-schema/src/diagnostic.rs`.
- Modify `crates/yamalgam-schema/src/schema.rs`.
- Create `crates/yamalgam-schema/tests/diagnostics.rs`.

**Interfaces:**

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub keyword: String,
    pub instance_path: String,
    pub schema_path: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub span: Option<Span>,
    pub labels: Vec<yamalgam_core::Label>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub diagnostics: Vec<ValidationDiagnostic>,
}
```

- [ ] **Step 1: Write diagnostic snapshots**

Snapshot required-property, type, enum, additional-properties, pattern,
reference, complex-key, non-finite-number, and projected-key-collision errors.
Include a canonical scalar key whose value fails validation and assert the
diagnostic path uses its projected property name while the span identifies the
original YAML value. Assert deterministic ordering by instance path, schema
path, then code. Assert paths use RFC 6901 escaping and spans come from
`LocatedValue`. A projected-key collision uses the later key as its primary
span and labels the earlier key.

- [ ] **Step 2: Verify RED**

```bash
just test-schema diagnostics
```

- [ ] **Step 3: Map upstream error kinds to stable Yamalgam codes**

Use `schema/<keyword>` for JSON Schema failures, `schema/ref` for retrieval,
and `yaml/complex-key`, `yaml/non-finite-number`, and
`yaml/projected-key-collision` for representation failures. Extract
expected/actual summaries from `ValidationErrorKind` without serializing
complete user values or response bodies. Cap diagnostics at
`SchemaOptions::max_diagnostics` and append one `schema/diagnostic-limit`
warning when truncation occurs.

- [ ] **Step 4: Verify GREEN**

```bash
just test-schema diagnostics
just test-schema
```

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-schema
git commit -m "feat(schema): expose structured validation diagnostics"
```

### Task 6: Select schemas from directives, configuration, and SchemaStore

**Files:**

- Create `crates/yamalgam-schema/src/selection.rs`.
- Modify `crates/yamalgam-schema/src/lib.rs`.
- Modify `crates/yamalgam-schema/Cargo.toml`.
- Create `crates/yamalgam-schema/tests/selection.rs`.
- Create `crates/yamalgam-schema/tests/fixtures/schemastore-catalog.json`.

**Interfaces:**

```rust
pub enum SchemaSource {
    Explicit,
    Pragma,
    InlineProperty,
    Association,
    SchemaStore,
}

pub struct SchemaAssociation {
    pub uri: String,
    pub file_match: Vec<String>,
}

pub struct SelectedSchema {
    pub uri: String,
    pub source: SchemaSource,
}

pub struct SchemaSelector {
    pub associations: Vec<SchemaAssociation>,
    pub schema_store: Option<SchemaCatalog>,
}
```

- [ ] **Step 1: Write precedence and catalog tests**

Assert this exact precedence, from highest to lowest: explicit caller
selection, first-document `# yg-schema-validate:`, compatible
`# yaml-language-server: $schema=`/`# $schema:` modeline, top-level string
`$schema`, configured association, then SchemaStore match. `none` disables
lower sources. Relative directive URIs resolve from the containing file.

- [ ] **Step 2: Verify RED**

```bash
just test-schema selection
```

- [ ] **Step 3: Parse directives from CST comments**

Inspect only comment tokens before the first semantic node in each document.
Require exactly one URI token after the prefix. Return a structured selection
error for malformed or conflicting same-priority directives.

- [ ] **Step 4: Add SchemaStore catalog matching**

Deserialize the official catalog fields used by M12 (`name`, `description`,
`fileMatch`, `url`, `versions`). Compile patterns with `globset`. Normalize
paths to `/`, preserve catalog order as the final tie-breaker, and choose the
longest literal pattern before that tie-breaker. Catalog and schema network
retrieval remain absent unless the caller supplies an enabled `SchemaSession`.

- [ ] **Step 5: Verify GREEN**

```bash
just test-schema selection
just deny
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-schema Cargo.lock
git commit -m "feat(schema): select schemas from standard sources"
```

### Task 7: Build the schema navigator used by editor features

**Files:**

- Create `crates/yamalgam-schema/src/navigation.rs`.
- Modify `crates/yamalgam-schema/src/schema.rs`.
- Create `crates/yamalgam-schema/tests/navigation.rs`.

**Interfaces:**

```rust
pub struct PropertyCandidate {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
}

pub struct SchemaContext {
    pub title: Option<String>,
    pub description: Option<String>,
    pub types: Vec<String>,
    pub enum_values: Vec<String>,
    pub properties: Vec<PropertyCandidate>,
    pub schema_locations: Vec<String>,
}

impl SchemaValidator {
    pub fn context_at(&self, instance_path: &str) -> SchemaContext;
    pub fn definition_for_ref(&self, schema_uri: &str, reference: &str)
        -> Option<SchemaLocation>;
}
```

- [ ] **Step 1: Write navigator tests**

Cover property and item traversal, `prefixItems`, `patternProperties`,
`additionalProperties`, all three combinators, conditionals, local and remote
`$ref`, recursive refs, missing properties, and deterministic de-duplication.

- [ ] **Step 2: Verify RED**

```bash
just test-schema navigation
```

- [ ] **Step 3: Index loaded schema resources**

Retain the located root and every retrieved schema document in
`SchemaValidator`. Index each `$id`, `$anchor`, `$dynamicAnchor`, and JSON
Pointer location together with its resource identity and source span.
Navigate instance paths without rerunning validation. Follow applicators with
a visited `(schema URI, pointer, instance path)` set so recursive schemas stop.

- [ ] **Step 4: Produce completion and hover data**

At mapping paths, return properties not already present. At scalar/sequence
paths, return enum/default/type/description annotations. Union viable
`anyOf`/`oneOf` branches, intersect `allOf` requirements, and sort candidates
by required-first then source order then name.

- [ ] **Step 5: Verify GREEN**

```bash
just test-schema navigation
just test-schema
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-schema
git commit -m "feat(schema): navigate schemas for editor features"
```

### Task 8: Add `validate` to the query engine

**Files:**

- Create `crates/yamalgam-query/src/validation.rs`.
- Modify `crates/yamalgam-query/src/lib.rs`.
- Modify `crates/yamalgam-query/src/engine.rs`.
- Modify `crates/yamalgam-query/src/funs.rs`.
- Modify `crates/yamalgam-query/src/value.rs`.
- Modify `crates/yamalgam-query/Cargo.toml`.
- Create `crates/yamalgam-query/tests/validation.rs`.

**Interfaces:**

```rust
pub trait QuerySchemaResolver {
    fn validator(&self, schema_ref: Option<&str>, input: &YamlVal)
        -> Result<Arc<SchemaValidator>, ValidationFailure>;
}

pub struct ValidationFailure {
    pub report: ValidationReport,
}

impl YamlVal {
    pub fn to_core_value(&self, limits: &ResourceLimits) -> Result<Value, ValueLimitError>;
}
```

- [ ] **Step 1: Write filter tests**

Cover `validate("schema.yaml")`, no-argument `validate` using the input's
pragma-selected schema, success returning the identical metadata-bearing
`YamlVal`, invalid input preserving a typed `ValidationFailure`, disabled
schema I/O, missing schemas, non-string arguments, deep values, and slurped
inputs from different source files.

- [ ] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-query --test validation
```

- [ ] **Step 3: Add a stack-safe query-to-core conversion**

Iteratively convert `YamlVal` data to `Value`, charging depth and collection
limits. Preserve numeric kinds and mapping order. Reject aliases only if they
have not already materialized into query data. Keep query metadata untouched
on the original value.

- [ ] **Step 4: Register both filter arities**

Register `validate/0` and `validate/1`. Obtain `QuerySchemaResolver` from the
run context, validate a converted core value, and return the original
`YamlVal` on success. Attach `ValidationFailure` to the runtime error path so
the CLI can recover the complete structured report after jaq stops.

- [ ] **Step 5: Verify GREEN and query regressions**

```bash
cargo nextest run -p yamalgam-query --test validation
cargo nextest run -p yamalgam-query
just clippy
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-query Cargo.lock
git commit -m "feat(query): validate values against schemas"
```

### Task 9: Add `yg validate` and `yg lint`

**Files:**

- Create `crates/yamalgam/src/commands/validate.rs`.
- Modify `crates/yamalgam/src/commands/mod.rs`.
- Modify `crates/yamalgam/src/lib.rs`.
- Modify `crates/yamalgam/src/main.rs`.
- Modify `crates/yamalgam/src/config.rs`.
- Modify `crates/yamalgam/Cargo.toml`.
- Modify `config/yamalgam.toml.example`.
- Modify `config/yamalgam.yaml.example`.
- Create `crates/yamalgam/tests/validate.rs`.

**Interfaces:**

```rust
#[derive(Args, Clone, Debug)]
pub struct ValidateArgs {
    #[arg(value_name = "FILE")]
    pub files: Vec<Utf8PathBuf>,
    #[arg(long, value_name = "URI")]
    pub schema: Option<String>,
    #[arg(long)]
    pub schema_store: bool,
    #[arg(long, default_value_t = 100)]
    pub max_diagnostics: usize,
}

pub enum Commands {
    #[command(visible_alias = "lint")]
    Validate(ValidateArgs),
    // existing variants remain
}
```

- [ ] **Step 1: Write CLI contract tests**

Cover explicit schema, pragma schema, multiple files and documents, valid and
invalid exits, missing schema, malformed schema, stdin with relative schema
and no base, JSON output, YAML output, `lint` alias, SchemaStore opt-in, and
network-off default. Use exit 0 for valid, 1 for validation findings, 2 for
usage, and 5 for parsing/schema/retrieval failures.

- [ ] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam --test validate
```

- [ ] **Step 3: Wire the thin command**

Load each stream through located composition, select one schema per document,
compile validators through a shared `SchemaSession`, and emit a deterministic
report containing filename, document index, validity, and diagnostics. Reuse
Librebar `--format`; emit JSON with `serde_json` and YAML through the existing
Yamalgam emitter.

- [ ] **Step 4: Add configuration without ambient network access**

Add `schemas` associations and `schema_store.enabled`/`schema_store.url` to
`Config`. Default `enabled` to false and the URL to
`https://www.schemastore.org/api/json/catalog.json`. Reject network URLs
unless the command also enables explicit ref resolution and allowlists the
scheme and host through the existing `LoaderConfig.resolution.refs` path.

- [ ] **Step 5: Verify GREEN**

```bash
cargo nextest run -p yamalgam --test validate
just test
just clippy
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam config Cargo.lock
git commit -m "feat(yg): validate and lint YAML documents"
```

### Task 10: Give parser diagnostics source spans and recover after errors

**Files:**

- Modify `crates/yamalgam-scanner/src/scanner.rs`.
- Modify `crates/yamalgam-scanner/tests/error_kinds.rs`.
- Modify `crates/yamalgam-parser/src/error.rs`.
- Create `crates/yamalgam-cst/src/recovery.rs`.
- Modify `crates/yamalgam-cst/src/lib.rs`.
- Create `crates/yamalgam-cst/tests/recovery.rs`.

**Interfaces:**

```rust
impl ScanError {
    pub const fn span(&self) -> Option<Span>;
}

impl ParseError {
    pub const fn span(&self) -> Option<Span>;
}

pub struct RecoveryOptions {
    pub max_diagnostics: usize,
}

pub struct RecoveryReport<'input> {
    pub cst: CstNode<'input>,
    pub diagnostics: Vec<ParseError>,
}

pub fn parse_to_cst_recovering_with_config<'input>(
    input: &'input str,
    config: &LoaderConfig,
    options: RecoveryOptions,
) -> RecoveryReport<'input>;
```

- [ ] **Step 1: Write span and recovery tests**

Assert exact spans for every `ScanErrorKind` family. Add malformed documents
with valid content before and after errors, multiple errors, multi-document
recovery, a Unicode line, an error at EOF, and an input that previously could
repeat the same failure. Assert byte-for-byte `to_text()`, monotonic recovery
offsets, at most 64 diagnostics by default, and a valid CST node after the
first error.

- [ ] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-scanner --test error_kinds
cargo nextest run -p yamalgam-cst --test recovery
```

- [ ] **Step 3: Attach spans at scanner error construction**

Store an optional span in `ScanError`. Keep `kind` and `message` behavior.
Capture the reader mark before consuming invalid content and the mark after it
when known. Add `ParseError::span()` for every variant. Resource-policy errors
without a source token may return `None`.

- [ ] **Step 4: Add bounded island recovery**

Run the ordinary parser on successive source slices. On error, preserve all
events produced so far, wrap at least one complete failing line in an Error
node, and restart at the next document marker or next line whose indentation
does not exceed the failed construct's starting column. Shift slice-local
marks by the original byte/line base. If the computed boundary does not
advance, consume one Unicode scalar value. Stop at `max_diagnostics` and wrap
the untouched remainder in one final Error node.

- [ ] **Step 5: Verify GREEN and ordinary parser stability**

```bash
cargo nextest run -p yamalgam-scanner --test error_kinds
cargo nextest run -p yamalgam-cst --test recovery
just test-compliance
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-scanner crates/yamalgam-parser crates/yamalgam-cst
git commit -m "feat(cst): recover structured syntax after errors"
```

### Task 11: Add Rowan syntax trees and incremental document reparsing

**Files:**

- Create `crates/yamalgam-cst/src/syntax.rs`.
- Create `crates/yamalgam-cst/src/incremental.rs`.
- Modify `crates/yamalgam-cst/src/lib.rs`.
- Modify `crates/yamalgam-cst/Cargo.toml`.
- Create `crates/yamalgam-cst/tests/syntax.rs`.
- Create `crates/yamalgam-cst/tests/incremental.rs`.

**Interfaces:**

```rust
pub enum SyntaxKind {
    Stream,
    Document,
    Mapping,
    MappingEntry,
    Sequence,
    SequenceEntry,
    Error,
    Scalar,
    Anchor,
    Alias,
    Tag,
    Comment,
    Whitespace,
    Indicator,
}

pub struct SyntaxTree {
    green: rowan::GreenNode,
    diagnostics: Vec<ParseError>,
}

pub struct TextEdit {
    pub range: std::ops::Range<usize>,
    pub replacement: String,
}

pub struct IncrementalDocument {
    text: ropey::Rope,
    syntax: SyntaxTree,
}

impl IncrementalDocument {
    pub fn apply(&mut self, edits: &[TextEdit], config: &LoaderConfig)
        -> Result<ReparseOutcome, EditError>;
    pub fn byte_offset_to_utf16(&self, offset: usize) -> Option<(u32, u32)>;
    pub fn utf16_to_byte_offset(&self, line: u32, character: u32) -> Option<usize>;
}
```

- [ ] **Step 1: Write syntax compatibility tests**

For all 351 YAML Test Suite inputs, assert `SyntaxTree::text() == input` and
the existing `parse_to_cst(input).to_text() == input`. Compare legacy and
Rowan node/token kind sequences on valid inputs. Test `node_at_offset` at
token boundaries and Error nodes.

- [ ] **Step 2: Write edit and position tests**

Cover insertion, replacement, deletion, CRLF, emoji/non-BMP UTF-16 columns,
out-of-range positions, reverse-ordered edit batches, edits inside one
document of a stream, and an edit that changes document markers. Assert one
document subtree is reparsed when safe and the full stream is the explicit
fallback.

- [ ] **Step 3: Verify RED**

```bash
cargo nextest run -p yamalgam-cst --test syntax --test incremental
```

- [ ] **Step 4: Build Rowan from the recovery event stream**

Map existing node/token kinds to `SyntaxKind` and feed every original text
slice to `GreenNodeBuilder`. Keep the legacy builder intact. Expose owned
`SyntaxNode`/`SyntaxToken` aliases and byte ranges; do not leak LSP types into
the CST crate.

- [ ] **Step 5: Reparse one document when safe**

Apply edits to Ropey in descending byte order. Find the smallest enclosing
Document before the edit; reparse and replace that green subtree when document
markers and surrounding indentation remain stable. Fall back to full-stream
reparse for cross-document edits, marker edits, or recovery-boundary changes.
Return `ReparseOutcome::{Document, Full}` for tests and telemetry.

- [ ] **Step 6: Verify GREEN**

```bash
cargo nextest run -p yamalgam-cst --test syntax --test incremental
just test-compliance
just clippy
```

- [ ] **Step 7: Commit**

```bash
git add crates/yamalgam-cst Cargo.lock
git commit -m "feat(cst): add incremental green syntax trees"
```

### Task 12: Add the standalone language-server process

**Files:**

- Create `crates/yamalgam-lsp/Cargo.toml`.
- Create `crates/yamalgam-lsp/src/lib.rs`.
- Create `crates/yamalgam-lsp/src/server.rs`.
- Create `crates/yamalgam-lsp/src/document.rs`.
- Create `crates/yamalgam-lsp/src/config.rs`.
- Create `crates/yamalgam-lsp/tests/protocol.rs`.
- Create `crates/yamalgam/src/bin/yamalgam-lsp.rs`.
- Modify `crates/yamalgam/Cargo.toml`.
- Modify `xtask/src/commands/install.rs`.

**Interfaces:**

```rust
pub fn run_stdio() -> Result<(), ServerError>;
pub fn run(connection: lsp_server::Connection) -> Result<(), ServerError>;

pub struct DocumentState {
    pub uri: lsp_types::Uri,
    pub version: i32,
    pub syntax: IncrementalDocument,
    pub located: Vec<LocatedValue>,
    pub selected_schemas: Vec<SelectedSchema>,
}

pub struct ServerConfig {
    pub schema_associations: Vec<SchemaAssociation>,
    pub schema_store_enabled: bool,
    pub schema_store_url: String,
    pub max_diagnostics: usize,
    pub workspace_roots: Vec<PathBuf>,
}
```

- [ ] **Step 1: Write in-memory lifecycle tests**

Drive `Connection::memory()` through initialize, initialized, didOpen,
incremental didChange, didClose, shutdown, and exit. Assert LSP 3.17
capabilities advertise incremental sync, completion triggers `:` and `-`,
hover, definition, and UTF-16 position encoding.

- [ ] **Step 2: Verify RED**

```bash
just test-lsp protocol
```

- [ ] **Step 3: Add the crate with the narrow dependency set**

```toml
[dependencies]
lsp-server = "0.10.0"
lsp-types = "0.97.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"
yamalgam-compose = { path = "../yamalgam-compose" }
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-cst = { path = "../yamalgam-cst" }
yamalgam-resolve = { path = "../yamalgam-resolve" }
yamalgam-schema = { path = "../yamalgam-schema" }
```

Use one synchronous state-owning loop. Route requests by method and answer
unknown methods with JSON-RPC MethodNotFound. Handle cancellation before
expensive feature work.

- [ ] **Step 4: Build document snapshots**

Apply LSP edits through `IncrementalDocument`, rebuild located semantic values
when composition succeeds, retain recovering syntax when it does not, and
discard out-of-order document versions. Never panic on malformed client data.

- [ ] **Step 5: Ship a second binary without mixing protocols**

`src/bin/yamalgam-lsp.rs` calls `yamalgam_lsp::run_stdio()`. Update xtask
install to copy both `yg` and `yamalgam-lsp`. Do not add `yg lsp`.

- [ ] **Step 6: Verify GREEN**

```bash
just test-lsp protocol
just clippy
```

- [ ] **Step 7: Commit**

```bash
git add crates/yamalgam-lsp crates/yamalgam xtask Cargo.lock
git commit -m "feat(lsp): add server lifecycle and document state"
```

### Task 13: Publish syntax and schema diagnostics

**Files:**

- Create `crates/yamalgam-lsp/src/diagnostics.rs`.
- Modify `crates/yamalgam-lsp/src/document.rs`.
- Modify `crates/yamalgam-lsp/src/server.rs`.
- Create `crates/yamalgam-lsp/tests/diagnostics.rs`.

**Interfaces:**

```rust
pub fn document_diagnostics(
    document: &DocumentState,
    schemas: &SchemaWorkspace,
    config: &ServerConfig,
) -> Vec<lsp_types::Diagnostic>;
```

- [ ] **Step 1: Write protocol diagnostic tests**

Cover scanner, parser, recovery-limit, missing schema, invalid schema,
required property, wrong type, a valid fix clearing diagnostics, multiple YAML
documents, canonical scalar keys, complex keys, projected-key collisions,
included-file syntax errors, and Unicode ranges.
Assert every publish carries the current document version and stale work is
not published.

- [ ] **Step 2: Verify RED**

```bash
just test-lsp diagnostics
```

- [ ] **Step 3: Combine recovery and validation reports**

Convert parse spans by byte offset through the document Rope. Validate only
documents with a complete located semantic value; syntax diagnostics remain
available for partial documents. Select and compile schemas through one
workspace `SchemaSession`, reusing cached validators by canonical schema URI
and content digest.

- [ ] **Step 4: Preserve typed metadata in LSP data**

Set `source = "yamalgam"`, use the stable diagnostic code, map error/warning
severity, and place document index, instance path, schema path, and schema URI
in `Diagnostic.data`. Never put retrieved bodies or unredacted URLs in
messages or data.

- [ ] **Step 5: Verify GREEN**

```bash
just test-lsp diagnostics
just test-schema diagnostics
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-lsp
git commit -m "feat(lsp): publish YAML and schema diagnostics"
```

### Task 14: Add schema-aware completion and hover

**Files:**

- Create `crates/yamalgam-lsp/src/completion.rs`.
- Create `crates/yamalgam-lsp/src/hover.rs`.
- Modify `crates/yamalgam-lsp/src/server.rs`.
- Create `crates/yamalgam-lsp/tests/completion.rs`.
- Create `crates/yamalgam-lsp/tests/hover.rs`.

**Interfaces:**

```rust
pub fn complete(
    document: &DocumentState,
    position: Position,
    schemas: &SchemaWorkspace,
) -> CompletionResponse;

pub fn hover(
    document: &DocumentState,
    position: Position,
    schemas: &SchemaWorkspace,
) -> Option<Hover>;
```

- [ ] **Step 1: Write completion tests**

Test missing property names, required-first ordering, duplicate suppression,
enum values, Booleans/null/numbers without quotes, quoted strings, sequence
items, default snippets, nested objects, multi-document paths, partial
mappings, an Error node before the cursor, and no active schema.

- [ ] **Step 2: Write hover tests**

Test title, description, type union, default, enum, deprecation, active schema
URI, `$ref`-supplied annotations, YAML anchor/alias context, and a cursor on
whitespace or comments.

- [ ] **Step 3: Verify RED**

```bash
just test-lsp completion
just test-lsp hover
```

- [ ] **Step 4: Derive the instance path from syntax**

Walk from `node_at_offset` to the Document root. For mapping entries, decode
the key token with the active YAML tag resolver; for sequences, count semantic
siblings. When recovery prevents an exact path, retain the longest valid
prefix and let `SchemaNavigator` provide candidates there.

- [ ] **Step 5: Render safe completion and hover payloads**

Use plain text unless the client advertises snippets. Preserve the document's
indent width and block/flow context. Return Markdown hover only when the
client supports it; otherwise return plain text. Do not include HTML from
schema descriptions.

- [ ] **Step 6: Verify GREEN**

```bash
just test-lsp completion
just test-lsp hover
just clippy
```

- [ ] **Step 7: Commit**

```bash
git add crates/yamalgam-lsp
git commit -m "feat(lsp): complete and describe schema-backed YAML"
```

### Task 15: Add definition navigation

**Files:**

- Create `crates/yamalgam-lsp/src/definition.rs`.
- Modify `crates/yamalgam-lsp/src/document.rs`.
- Modify `crates/yamalgam-lsp/src/server.rs`.
- Create `crates/yamalgam-lsp/tests/definition.rs`.

**Interfaces:**

```rust
pub struct NavigationIndex {
    pub resolution_report: ResolutionReport,
    pub schema_refs: Vec<SchemaReferenceSite>,
}

pub fn definition(
    document: &DocumentState,
    position: Position,
    workspace: &WorkspaceState,
) -> Option<GotoDefinitionResponse>;
```

- [ ] **Step 1: Write definition tests**

Cover scalar and structured `!include`, local and cross-file data `$ref`, JSON
Pointer fragments, schema `$ref` into `$defs`, a remote schema already in
cache, same-document refs, missing targets, cyclic refs, denied paths, and a
symlink escape. Assert `LocationLink.origin_selection_range` covers only the
reference text and targets the selected node span when available.

- [ ] **Step 2: Verify RED**

```bash
just test-lsp definition
```

- [ ] **Step 3: Build a local-only M11 navigation report**

For each complete document, run a bounded resolution session with memory and
workspace-rooted file retrievers. Record edges and source spans but keep HTTP,
HTTPS, redirects, auth, and out-of-workspace files disabled. Rebuild only when
the document version or referenced file digest changes.

- [ ] **Step 4: Resolve target spans**

Use `ResolutionEdge` for include/data-ref sites and `SchemaNavigator` for
schema refs. Open cached target text through the workspace retriever, parse a
located document, and map RFC 6901 fragments to spans. Return the target file
start when an include selects a whole file without a semantic pointer.

- [ ] **Step 5: Verify GREEN and policy regressions**

```bash
just test-lsp definition
cargo nextest run -p yamalgam-resolve
just clippy
```

- [ ] **Step 6: Commit**

```bash
git add crates/yamalgam-lsp
git commit -m "feat(lsp): navigate includes and references"
```

### Task 16: Document the M12 contract and trace its specification

**Files:**

- Create `docs/yg/validation.md`.
- Create `docs/yg/language-server.md`.
- Modify `docs/yg/README.md`.
- Modify `docs/yg/querying.md`.
- Modify `docs/yg/reference.md`.
- Modify `docs/yg/troubleshooting.md`.
- Modify `docs/README.md`.
- Modify `README.md`.
- Modify `docs/spec/yg-query-language-spec-draft-01.md`.
- Modify `docs/spec/yg-spec-draft-01-implementation-notes.md`.
- Modify `record/superpowers/plans/README.md`.
- Modify `.config/tracey/config.styx`.
- Modify `scripts/check-yg-doc-examples`.

**Steps:**

- [ ] **Step 1: Add executable validation examples**

Document explicit and pragma-selected schemas, `validate(schema)`, `yg
validate`, the `lint` alias, JSON CI output, exit codes, local `$ref`,
SchemaStore opt-in, and network-off defaults. Run each stable CLI example from
the docs harness.

- [ ] **Step 2: Add editor setup and settings**

Document `yamalgam-lsp` stdio setup for VS Code-compatible clients, Neovim,
Zed, and BBEdit/Tinymist-style generic LSP clients. List initialization
settings, schema precedence, workspace-root confinement, supported features,
and M12 exclusions.

- [ ] **Step 3: Make the normative spec match released behavior**

Mark Phase 5 complete. Specify diagnostic fields and exit codes, schema
selection precedence, Draft 2020-12 engine, scalar-key projection,
complex/colliding key and non-finite handling, and local/remote schema policy.
Replace the stale prohibition on `jsonschema` with the native
custom-representation contract. Record the `yaml-schema` borrowing boundary:
adapted tests and canonical key behavior, not its evaluator or typed-key
`propertyNames` extension.

- [ ] **Step 4: Extend traceability**

Add `yamalgam-schema`, `yamalgam-cst`, and `yamalgam-lsp` code/test
roots for the `r` spec. Add `r[impl validate.*]` markers for validation,
selection, CLI, and filter requirements.

- [ ] **Step 5: Run documentation gates**

```bash
just docs-check
tracey query status --format summary
```

- [ ] **Step 6: Commit**

```bash
git add README.md docs record/superpowers/plans/README.md .config/tracey scripts/check-yg-doc-examples
git commit -m "docs(m12): document validation and language services"
```

### Task 17: Run the milestone integration and release gates

**Files:**

- Modify only files required by confirmed review findings.
- Append final command evidence to this plan after every acceptance item passes.

**Steps:**

- [ ] **Step 1: Run focused milestone suites**

```bash
just test-schema
just test-lsp
cargo nextest run -p yamalgam-query --test validation
cargo nextest run -p yamalgam --test validate
cargo nextest run -p yamalgam-cst --test recovery --test syntax --test incremental
```

- [ ] **Step 2: Run repository-wide deterministic gates**

```bash
just test-compliance
just docs-check
just unsafe-census
just feature-matrix
just check
git --no-pager diff --check
git --no-pager status --short
```

- [ ] **Step 3: Audit the complete M12 delta**

Run a fresh full-repository Cased review against the new M12 HEAD and apply
Crustoleum to the Rust surfaces. Pay specific attention to custom-value trait
soundness, stack safety, regex/schema resource exhaustion, retriever policy,
URI redaction, symlink confinement, LSP framing, cancellation/version races,
UTF-16 conversions, and recovery forward progress.

- [ ] **Step 4: Remediate confirmed findings sequentially**

For each confirmed finding: write one focused failing regression, make
the smallest fix, run affected crate tests, commit the code fix, and append the
real code SHA to the audit ledger in a separate commit. Do not mutate immutable
audit artifacts or batch unrelated findings.

- [ ] **Step 5: Re-run every release gate at the final HEAD**

```bash
just test-schema
just test-lsp
just test-compliance
just docs-check
just unsafe-census
just feature-matrix
just check
git --no-pager diff --check
git --no-pager status --short
```

- [ ] **Step 6: Record completion evidence**

Append the final HEAD, audit counts, focused suite counts, and exact gate
results below the acceptance checklist. Commit only the completed plan update:

```bash
git add record/superpowers/plans/2026-08-09-m12-schema-validation-lsp.md
git commit -m "docs(m12): record final verification"
```

## Final acceptance checklist

- [ ] The custom `Value` representation validates YAML instances directly. No
  instance JSON conversion exists.
- [ ] Draft 2020-12 schemas compile with local and policy-approved external
  `$ref`. The configuration disables default `jsonschema` I/O and selects the
  linear regex engine.
- [ ] Scalar mapping keys validate through collision-checked canonical
  property names while retaining original types and spans. Complex keys,
  projected-name collisions, and non-finite numbers produce explicit,
  source-located diagnostics without overwrite or loss.
- [ ] `validate(schema)`, no-argument `validate`, `yg validate`, and `yg lint`
  share the same validator, selection, retrieval, and diagnostics code.
- [ ] CLI reports are deterministic and machine-readable. Valid, invalid,
  usage, and runtime exits match the documented contract.
- [ ] Existing CST APIs remain source-compatible and byte-lossless.
- [ ] Recovery returns useful nodes after syntax errors, emits bounded
  diagnostics, and always advances.
- [ ] Rowan/Ropey document state applies incremental edits and converts UTF-8
  byte offsets to LSP UTF-16 positions correctly.
- [ ] `yamalgam-lsp` passes initialize/open/change/close/shutdown protocol tests
  without writing non-protocol data to stdout.
- [ ] Syntax and schema diagnostics update at the current document version and
  clear after a valid edit.
- [ ] Completion and hover work on valid and partially invalid YAML with schema
  composition and multi-document streams.
- [ ] Go-to-definition resolves schema refs, data refs, and includes while
  enforcing workspace roots and network-off defaults.
- [ ] SchemaStore is interoperable and opt-in. No persistent cache, auth, or
  unbounded catalog/schema fetch exists.
- [ ] End-user docs, normative spec, deviation notes, roadmap, and Tracey agree
  with the released M12 behavior.
- [ ] Fresh Cased/Crustoleum review has no open release findings.
- [ ] `just test-compliance`, `just docs-check`, `just unsafe-census`,
  `just feature-matrix`, and `just check` pass at the final HEAD.

## Plan self-review

- Spec coverage: roadmap validator, filter, CLI, pragma, SchemaStore,
  diagnostics, completion, hover, source navigation, multi-document support,
  and error recovery all map to explicit tasks.
- Dependency direction: core <- parser/compose/CST/resolve <- schema/query <-
  CLI/LSP remains acyclic. `yamalgam-schema` adapts core through a local wrapper.
- Public API compatibility: existing `Value`, composer, parser, resolver, and
  CST entry points remain. Every new behavior has an additive interface.
- Upstream borrowing: adapted `yaml-schema` cases carry source-commit and MIT
  attribution. No Saphyr, Cucumber, duplicate validator, or ambient retriever
  enters the dependency graph.
- Deferred scope: formatting, fixes, persistent cache, lockfiles, auth,
  workspace symbols, and HTTP data includes are explicitly excluded.
