# M11 External Resource Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Track every
> checkbox in place. Do not create a worktree; Clay's workflow uses the current
> branch directly.

**Goal:** Ship pluggable retrieval for memory, capability-rooted files, and
Librebar HTTP. Use it for shared `$ref`/`!include` resolution, policy modes,
deterministic reports, and complete `yg` integration.

**Architecture:** Add a `yamalgam-resolve` crate above the parser and below
query/CLI consumers. It owns resource identities and synchronous retrievers.
It also owns the neutral YAML tree, cache, sessions, anchor namespacing,
reports, and built-in middleware. The parser remains I/O-free. Query adapts
between `OwnedNode` and `YamlVal`. This preserves node origins and lets explicit
filters reuse the same session. Disabled policy retains the current
`NoopResolver` fast path.

**ADRs:**

- `record/decisions/0015-pluggable-retrievers-with-shared-resolution-sessions.md`
- `record/decisions/0016-use-three-mode-resolution-policy.md`
- `record/decisions/0017-use-sole-key-uri-references.md`
- `record/decisions/0018-use-structured-include-aggregation.md`
- `record/decisions/0019-record-resolution-reports-and-integrity-metadata.md`

**Design:**
`record/superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md`.

**Tech stack:** Rust 2024 with MSRV 1.89 and Yamalgam's event middleware. File
support uses `cap-std` 4.x and `globset` 0.4.x. URI support uses `url` 2.5.x
and `percent-encoding` 2.3.x. HTTP uses Librebar 0.6.0 and Tokio. Tests use
`cargo nextest` and the existing `just` quality gates.

## Global constraints

- Keep the parser free of filesystem, network, Tokio, Librebar, and URL
  dependencies.
- Keep feature logic out of the `yamalgam` binary crate except CLI argument,
  input/output, and dependency wiring.
- Do not replace any Yamalgam scanner, parser, composer, serde, CST, or query
  path with another YAML implementation.
- Do not add HTTP includes, redirects, cookies, authentication, persistent
  caching, lockfiles, top-level URL inputs, or `$ref` sibling merging.
- Do not perform retrieval unless the relevant capability is explicitly or
  automatically enabled.
- Preserve native YAML types, tags, anchors, aliases, presentation metadata,
  document order, and the CST source round trip.
- Treat externally retrieved bytes as exactly one YAML document. Top-level
  input streams may remain multi-document; each root document receives its own
  session and report.
- Charge every attempted resolution edge, including cache hits. Charge every
  materialization site and synthesized aggregate storage with checked
  arithmetic.
- Resolve deny rules before allow rules, policy before I/O, and HTTP
  scheme/host/userinfo checks before a request reaches Librebar.
- Use deterministic traversal order and normalized root-relative path ordering;
  never depend on filesystem enumeration or locale.
- Preserve structured error variants and their existing display context across
  parser, query, and CLI boundaries.
- Follow RED -> GREEN -> focused verification for every task. Do not batch all
  tests until the end.
- Do not manually increment the project version.

## File map

- `yamalgam-core/src/loader.rs` owns resolution modes, shared file/network
  policy, presets, and limit categories; it performs no retrieval.
- `yamalgam-parser/src/event.rs` owns lossless event-to-owned conversion.
  `yamalgam-parser/src/resolve.rs` retains the resolver trait and structured
  consumer-facing errors.
- `yamalgam-resolve/src/resource.rs` and `pointer.rs` own fragmentless identity,
  base joining, safe rendering, and RFC 6901 decoding.
- `yamalgam-resolve/src/retriever/` owns the synchronous trait, scheme router,
  memory backend, capability-rooted file backend, and Librebar worker.
- `yamalgam-resolve/src/document.rs` owns the neutral metadata-preserving YAML
  tree and event conversion.
- `yamalgam-resolve/src/session.rs` owns caches, canonical aliases, cycle state,
  budgets, recursion, and report mutation.
- `yamalgam-resolve/src/ref_resolver.rs`, `include.rs`, and `anchors.rs` own the
  three transformation-specific algorithms.
- `yamalgam-resolve/src/report.rs` owns deterministic graph and provenance
  records; `error.rs` owns retriever-internal failures.
- `yamalgam-resolve/src/builtin.rs` is the only adapter from the shared session
  engine to `yamalgam_parser::Resolver`.
- `yamalgam-query/src/resolution.rs` owns `OwnedNode`/`YamlVal` conversion and
  session handles; `runtime_resolution.rs` preserves typed failures across jaq.
- `yamalgam/src/lib.rs` and `commands/query.rs` own only CLI arguments,
  validation, root/base selection, and retriever/session wiring.
- `docs/yg/` owns copy-paste user guidance; the query spec and implementation
  notes own normative syntax and coverage status.

---

### Task 1: Replace boolean resolution switches with accepted policy modes and limits

**Files:**

- Modify `crates/yamalgam-core/src/loader.rs`.
- Modify `crates/yamalgam-core/src/lib.rs` exports.
- Modify `crates/yamalgam-parser/src/resolve.rs` for the mode-aware no-op check.

**Interfaces:**

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResolutionMode {
    #[default]
    Disabled,
    Explicit,
    Automatic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionCapability {
    Include,
    Ref,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolutionLimits {
    pub max_depth: Option<usize>,
    pub max_edges: Option<usize>,
    pub max_materialized_bytes: Option<usize>,
}

pub struct ResolutionPolicy {
    pub include: IncludePolicy,
    pub refs: RefPolicy,
    pub files: FilePolicy,
    pub limits: ResolutionLimits,
}

pub struct IncludePolicy {
    pub mode: ResolutionMode,
}

pub struct FilePolicy {
    pub root: Option<PathBuf>,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub follow_symlinks: bool,
}

pub struct RefPolicy {
    pub mode: ResolutionMode,
    pub allow_schemes: Vec<String>,
    pub allow_hosts: Vec<String>,
    pub timeout: Duration,
}
```

- [x] **Step 1: Write policy-mode and preset tests**

Add exact tests for these contracts:

```rust
#[test]
fn resolution_presets_match_adr_0016() {
    let cases = [
        (LoaderConfig::moderate(), Some(10), Some(256), Some(256 * 1024 * 1024)),
        (LoaderConfig::strict(), Some(5), Some(32), Some(10 * 1024 * 1024)),
        (LoaderConfig::trusted(), Some(64), Some(4_096), None),
        (LoaderConfig::unchecked(), None, None, None),
    ];
    for (config, depth, edges, bytes) in cases {
        assert_eq!(config.resolution.limits.max_depth, depth);
        assert_eq!(config.resolution.limits.max_edges, edges);
        assert_eq!(config.resolution.limits.max_materialized_bytes, bytes);
        assert!(!config.resolution.is_enabled());
    }
}

#[test]
fn explicit_and_automatic_modes_are_distinct() {
    let mut policy = ResolutionPolicy::moderate();
    policy.include.mode = ResolutionMode::Explicit;
    policy.refs.mode = ResolutionMode::Automatic;
    assert!(policy.allows_explicit(ResolutionCapability::Include));
    assert!(!policy.resolves_automatically(ResolutionCapability::Include));
    assert!(policy.resolves_automatically(ResolutionCapability::Ref));
}
```

Add `ResolutionDepth`, `ResolutionEdges`, and `MaterializedBytes` to
`ResourceLimit`. For each one, test the category, observed count, maximum, and
display text.

- [x] **Step 2: Run the focused tests to verify RED**

```bash
cargo nextest run -p yamalgam-core -E 'test(resolution_presets_match_adr_0016) | test(explicit_and_automatic_modes_are_distinct)'
```

Expected: compilation fails because the mode, capability, and resolution-limit
types do not exist.

- [x] **Step 3: Implement the policy types and presets**

`ResolutionPolicy::{moderate,strict,trusted,unchecked}` must return disabled
capabilities with the accepted limits. `ResolutionPolicy::disabled()` remains
as a compatibility alias for `moderate()`. `LoaderConfig` presets select the
matching policy constructor. Remove `IncludePolicy::{enabled,max_depth,
max_total_bytes}` and `RefPolicy::enabled`; update existing core/parser tests
to use `ResolutionMode` without adding behavior yet.

- [x] **Step 4: Run core and parser policy tests to verify GREEN**

```bash
cargo nextest run -p yamalgam-core -p yamalgam-parser -E 'test(resolution) | test(noop_resolver) | test(enabled_policy)'
```

- [x] **Step 5: Commit the focused policy change**

```bash
git add crates/yamalgam-core crates/yamalgam-parser
git commit -m "feat(resolve): define resolution modes and budgets"
```

---

### Task 2: Create `yamalgam-resolve` and upgrade Librebar to 0.6.0

**Files:**

- Create `crates/yamalgam-resolve/Cargo.toml`.
- Create `crates/yamalgam-resolve/src/lib.rs`.
- Modify `crates/yamalgam/Cargo.toml`.
- Modify `Cargo.lock` through Cargo.

**Dependency boundary:**

```toml
[dependencies]
cap-std = "4"
globset = { version = "0.4", default-features = false }
librebar = { version = "0.6.0", default-features = false, features = ["http"] }
percent-encoding = "2.3"
thiserror = "2.0"
tokio = { version = "1.52", features = ["rt", "sync"] }
url = "2.5"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-parser = { path = "../yamalgam-parser" }
yamalgam-scanner = { path = "../yamalgam-scanner" }
```

- [x] **Step 1: Add the crate manifest and documented empty library root**

Create the manifest with the dependency table above. Create `src/lib.rs` with
crate-level documentation describing the parser-above/query-below boundary and
`#![deny(unsafe_code)]`. Do not declare placeholder public types; subsequent
tasks add each real module with its tests.

- [x] **Step 2: Update the CLI manifest**

Update the CLI's Librebar dependency from 0.4 to 0.6.0. Retain the explicit
`cli`, `config`, `logging`, and `diagnostics` features. Add
`yamalgam-resolve` as a CLI dependency. Do not enable Librebar cookies, HTTP
cache, update, or default features.

- [x] **Step 3: Update the lockfile deliberately**

```bash
cargo update -p librebar --precise 0.6.0
```

Inspect the complete dependency diff. Confirm that Librebar's HTTP feature is
the only reason Hyper, Tokio, or TLS packages enter the shipped graph.

- [x] **Step 4: Compile both dependency surfaces**

```bash
cargo check -p yamalgam-resolve -p yamalgam
```

Expected: both packages compile against Librebar 0.6.0. Fix any Librebar API
breakage only in the CLI integration that uses the changed API; do not enable
additional Librebar features.

- [x] **Step 5: Verify the dependency boundary**

```bash
cargo check -p yamalgam-resolve
cargo tree -p yamalgam -e features -i librebar
just deny
```

- [x] **Step 6: Commit the crate and dependency foundation**

```bash
git add Cargo.lock crates/yamalgam/Cargo.toml crates/yamalgam-resolve
git commit -m "feat(resolve): add resolver crate and Librebar HTTP"
```

---

### Task 3: Implement canonical resource identities and URI-reference parsing

**Files:**

- Create `crates/yamalgam-resolve/src/resource.rs`.
- Create `crates/yamalgam-resolve/src/pointer.rs`.
- Modify `crates/yamalgam-resolve/src/lib.rs`.
- Test in `crates/yamalgam-resolve/tests/resource_id.rs`.

**Interfaces:**

```rust
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ResourceId {
    File(PathBuf),
    Uri(Url),
}

pub struct ResolvedReference {
    pub resource: ResourceId,
    pub fragment: Fragment,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Fragment {
    Root,
    Pointer(JsonPointer),
}
```

`ResourceId` is always fragmentless. `Uri` covers `memory`, `http`, `https`,
and application-defined schemes. File identities retain `PathBuf`. This avoids
silent corruption of non-UTF-8 Unix names.

- [x] **Step 1: Write table-driven resource tests**

Cover:

- relative file references against a containing file.
- `file:` URI references.
- HTTP relative references and normalized hosts.
- no fragment and empty `#` selecting the root.
- percent-decoding before RFC 6901 `~0`/`~1` decoding.
- invalid percent escapes, invalid `~` escapes, fragmentless-ID invariants,
  and references without a base.
- sanitization removing HTTP userinfo and query without changing the internal
  cache identity.

Use exact cases including:

```rust
assert_eq!(
    base.resolve("../shared.yaml#/a~1b/~0name")?.fragment.pointer_tokens(),
    ["a/b", "~name"]
);
assert_eq!(base.resolve("#")?.fragment, Fragment::Root);
```

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test resource_id
```

- [x] **Step 3: Implement resolution without ambient I/O**

Use `Url::join` for hierarchical URI references and `percent_encoding` for
fragment decoding. Implement file-relative joining lexically; do not
canonicalize or open a path here. Reject a fragment in `ResourceId::new` and
return it separately from `resolve`. Make `Display` and `Debug` safe by
default: omit URI userinfo and query values. Expose full parsed components
through accessors used by policy and retrieval, not a full-identity formatter.
The cache key still retains the complete fragmentless URL.

- [x] **Step 4: Verify GREEN and Clippy**

```bash
cargo nextest run -p yamalgam-resolve --test resource_id
cargo clippy -p yamalgam-resolve --all-targets -- -D warnings
```

- [x] **Step 5: Commit resource identity**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): canonicalize resource references"
```

---

### Task 4: Define retriever, provenance, integrity, error, and report contracts

**Files:**

- Create `crates/yamalgam-resolve/src/error.rs`.
- Create `crates/yamalgam-resolve/src/retriever/mod.rs`.
- Create `crates/yamalgam-resolve/src/report.rs`.
- Modify `crates/yamalgam-resolve/src/lib.rs`.
- Modify `crates/yamalgam-parser/src/resolve.rs`.
- Test in `crates/yamalgam-resolve/tests/report.rs` and parser unit tests.

**Interfaces:**

```rust
pub trait ResourceRetriever: Send + Sync {
    fn retrieve(
        &self,
        request: &ResourceRequest,
    ) -> Result<RetrievedResource, RetrieveError>;

    fn discover(
        &self,
        request: &DiscoveryRequest,
    ) -> Result<Vec<ResourceId>, RetrieveError> {
        Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
    }
}

pub struct ResourceRequest {
    pub id: ResourceId,
    pub max_bytes: Option<usize>,
}

pub struct DiscoveryRequest {
    pub base: ResourceId,
    pub selector: DiscoverySelector,
    pub max_matches: Option<usize>,
}

pub enum DiscoverySelector {
    Glob { pattern: String },
    Directory {
        path: String,
        recursive: bool,
        matches: Vec<String>,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RetrieveError {
    #[error("no retriever is registered for scheme {0}")]
    UnsupportedScheme(String),
    #[error("resource discovery is unsupported from {0}")]
    DiscoveryUnsupported(ResourceId),
    #[error("resource denied by policy: {id}")]
    PolicyDenied { id: ResourceId },
    #[error("resource not found: {0}")]
    NotFound(ResourceId),
    #[error("resource is not a regular file: {0}")]
    NotRegularFile(ResourceId),
    #[error("symbolic link denied: {0}")]
    SymlinkDenied(ResourceId),
    #[error("resource escapes the configured root: {0}")]
    OutsideRoot(ResourceId),
    #[error("resource size {observed} exceeds maximum of {maximum}: {id}")]
    TooLarge { id: ResourceId, observed: usize, maximum: usize },
    #[error("matched resource count {observed} exceeds maximum of {maximum}")]
    TooManyMatches { observed: usize, maximum: usize },
    #[error("HTTP status {status} for {id}")]
    HttpStatus { id: ResourceId, status: u16 },
    #[error("HTTP request timed out for {0}")]
    Timeout(ResourceId),
    #[error("I/O failed for {id}: {source}")]
    Io { id: ResourceId, #[source] source: std::io::Error },
    #[error("HTTP transport failed for {id}: {source}")]
    Transport {
        id: ResourceId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("HTTP worker stopped before completing {0}")]
    WorkerStopped(ResourceId),
}

pub struct RetrievedResource {
    pub id: ResourceId,
    pub bytes: Vec<u8>,
    pub provenance: ResourceProvenance,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceProvenance {
    pub validators: Vec<ResourceValidator>,
    pub integrity: Option<ContentIntegrity>,
}

pub enum ResourceValidator {
    Etag(String),
    LastModified(String),
    Opaque { name: String, value: String },
}

pub struct ContentIntegrity {
    pub algorithm: String,
    pub encoded_value: String,
}

pub struct ResolutionReport {
    resources: Vec<ResourceRecord>,
    edges: Vec<ResolutionEdge>,
}

pub struct ResourceRecord {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub fetched_bytes: usize,
    pub cache_hits: usize,
    pub provenance: ResourceProvenance,
}

pub enum ResourceKind {
    Memory,
    File,
    Http,
    Https,
    Other(String),
}

pub struct ResolutionEdge {
    pub parent: ResourceId,
    pub child: Option<ResourceId>,
    pub capability: ResolutionCapability,
    pub source_span: Span,
    pub requested: String,
    pub fragment: Fragment,
    pub matches: Vec<ResourceId>,
    pub cache_hit: bool,
    pub fetched_bytes: usize,
    pub materialized_bytes: usize,
    pub outcome: ResolutionOutcome,
}

pub enum ResolutionOutcome {
    Resolved,
    Failed(ResolutionErrorCategory),
}

pub enum ResolutionErrorCategory {
    MalformedDirective,
    IncompatibleContext,
    DisabledCapability,
    PolicyDenied,
    MissingBase,
    InvalidResource,
    Retrieval,
    InvalidUtf8,
    InvalidYaml,
    DocumentCount,
    MissingFragment,
    NonSelfContainedFragment,
    Conflict,
    AggregateType,
    Cycle,
    Limit,
    ResourceOverflow,
}
```

`ContentIntegrity` stores an algorithm identifier and encoded value but M11
does not compute, require, or verify it. Reports retain full IDs internally;
their `Display`/diagnostic rendering uses sanitized IDs.

- [x] **Step 1: Write structured-contract tests**

Test that:

- a custom retriever can return bytes, validators, and an integrity value.
- resource and edge order is insertion/traversal order.
- success and terminal failure edges retain source span, requested text,
  fragment, matches, cache status, fetched bytes, materialized bytes, and typed
  outcome.
- rendered reports omit HTTP userinfo, query strings, bodies, and validator
  values.
- `ResolveError::clone_structured` preserves every new built-in resolution
  category and the original display prefix.
- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test report
cargo nextest run -p yamalgam-parser -E 'test(resolve_error)'
```

- [x] **Step 3: Implement typed errors and deterministic reports**

Add a typed built-in `ResolveError::Resolution` variant in the parser. It
carries category, capability, directive span, sanitized target, reference
chain, and optional source. Keep the existing `Include`, `Ref`, and `Custom`
variants for third-party resolver compatibility. Report failures use the same
category enum rather than storing only rendered text.

Expose `ResolutionReport::{resources,edges}` as slices, plus safe rendering.
Never expose a response body through `Debug` or `Display`; custom `Debug` for
`RetrievedResource` prints `bytes_len` only.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-parser -p yamalgam-resolve
```

- [x] **Step 5: Commit contracts**

```bash
git add crates/yamalgam-parser crates/yamalgam-resolve
git commit -m "feat(resolve): define retriever and report contracts"
```

---

### Task 5: Implement retriever routing and the in-memory backend

**Files:**

- Create `crates/yamalgam-resolve/src/retriever/memory.rs`.
- Create `crates/yamalgam-resolve/src/retriever/router.rs`.
- Modify `crates/yamalgam-resolve/src/retriever/mod.rs`.
- Test in `crates/yamalgam-resolve/tests/memory_retriever.rs`.

**Interfaces:**

```rust
#[derive(Default)]
pub struct MemoryRetriever {
    resources: BTreeMap<ResourceId, RetrievedResource>,
}

#[derive(Default)]
pub struct RetrieverRouter {
    by_scheme: BTreeMap<String, Box<dyn ResourceRetriever>>,
}

impl RetrieverRouter {
    pub fn register(
        &mut self,
        scheme: impl Into<String>,
        retriever: impl ResourceRetriever + 'static,
    ) -> Result<(), RetrieveError>;
}
```

- [x] **Step 1: Write memory and routing tests**

Prove exact retrieval, byte ceilings, and missing resources. Test duplicate
scheme registration, unsupported schemes, and `memory:` relative bases. A
denied or unsupported request must never call a spy retriever.

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test memory_retriever
```

- [x] **Step 3: Implement the backends**

Enforce `ResourceRequest::max_bytes` in every backend even when the session has
already checked a budget. Provide blanket `ResourceRetriever` implementations
for `Box<T>` and `Arc<T>` so sessions can share a router without sharing
mutable resolution state.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test memory_retriever
```

- [x] **Step 5: Commit routing and memory retrieval**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): route in-memory resources"
```

---

### Task 6: Implement the capability-rooted file backend and deterministic discovery

**Files:**

- Create `crates/yamalgam-resolve/src/retriever/file.rs`.
- Modify `crates/yamalgam-resolve/src/retriever/mod.rs`.
- Test in `crates/yamalgam-resolve/tests/file_retriever.rs`.

**Interface:**

```rust
pub struct FileRetriever {
    root_path: PathBuf,
    root: cap_std::fs::Dir,
    allow: GlobSet,
    deny: GlobSet,
    follow_symlinks: bool,
}

impl FileRetriever {
    pub fn open(policy: &FilePolicy) -> Result<Self, RetrieveError>;
}
```

- [x] **Step 1: Write sandbox tests before implementation**

Use `tempfile` and cover:

- ordinary relative files and extensionless files.
- absolute targets and `..` traversal rejected before open.
- a root input or `file:` target outside the configured root.
- deny-before-allow behavior with overlapping `globset` patterns.
- directories, Unix sockets/FIFOs on Unix, and other non-regular targets.
- final-component and intermediate symlinks rejected by default.
- enabled symlinks that remain inside the capability root.
- enabled relative and absolute symlinks attempting to escape the root.
- glob and directory discovery, recursive directory mode, default YAML match
  patterns, and zero matches.
- ordering by normalized root-relative path bytes, including deliberately
  reversed creation order.
- non-UTF-8 Unix filenames: retrieval identity remains lossless; a derived
  `named` key later rejects names that cannot become YAML strings.
- discovered symlink files that match a selector fail when symlinks are off;
  recursive walks skip, but never follow, unrelated symlink directories.
- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test file_retriever
```

- [x] **Step 3: Implement capability-relative access**

Open ambient authority exactly once in `FileRetriever::open`. Every later
metadata or directory operation uses the held `cap_std::fs::Dir`. The same is
true for canonicalization and reads. They may also use an entry opened from
that directory. Reject symlinks by
checking every component with capability-relative `symlink_metadata` when
`follow_symlinks` is false. When enabled, use `Dir::canonicalize` and require
the returned relative canonical path to remain inside the root before reading.
`cap-std` itself performs beneath-root path resolution; do not re-open the
canonical absolute path through `std::fs`.

Return the canonical `ResourceId` in `RetrievedResource`. The session maps the
lexical request ID to that canonical ID. Alternate relative spellings then
share one raw-document cache entry. Enabled in-root symlinks also share its
cycle identity.

Compile globs with `GlobBuilder::literal_separator(true)`. Match normalized
root-relative paths, apply deny first, then require an allow match only when
the allow set is non-empty. Walk directories through capability-relative
`read_dir`; never use ambient `walkdir` or `std::fs::read_dir`.
Stop discovery as soon as `max_matches + 1` approved candidates are found.
Return `RetrieveError::TooManyMatches`. The session supplies its remaining
edge budget as `DiscoveryRequest::max_matches`.

- [x] **Step 4: Verify GREEN and the dependency policy**

```bash
cargo nextest run -p yamalgam-resolve --test file_retriever
cargo clippy -p yamalgam-resolve --all-targets -- -D warnings
just deny
```

- [x] **Step 5: Commit the file backend**

```bash
git add crates/yamalgam-resolve Cargo.lock
git commit -m "feat(resolve): sandbox filesystem retrieval"
```

---

### Task 7: Implement Librebar HTTP through a blocking worker

**Files:**

- Create `crates/yamalgam-resolve/src/retriever/http.rs`.
- Modify `crates/yamalgam-resolve/src/retriever/mod.rs`.
- Test in `crates/yamalgam-resolve/tests/http_retriever.rs`.

**Interface and worker shape:**

```rust
pub struct LibrebarRetriever {
    allowed_schemes: BTreeSet<String>,
    allowed_hosts: BTreeSet<String>,
    work: Option<SyncSender<HttpWork>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

struct HttpWork {
    url: String,
    response: SyncSender<Result<RetrievedResource, RetrieveError>>,
}
```

- [x] **Step 1: Adapt Librebar's local test server and write HTTP tests**

Keep the server inside the test process. Cover:

- successful HTTP YAML and validator extraction.
- exact case-normalized host and scheme allowlists.
- rejected userinfo, missing host, disallowed scheme, and disallowed host before
  the spy server accepts a connection.
- a 3xx response returned as an error without following its `Location`.
- 4xx/5xx status errors.
- whole-operation timeout.
- decoded body limit.
- invalid UTF-8 left for the session's typed text error.
- clean worker shutdown, failed runtime construction, worker panic/channel
  closure, and repeated requests through one worker.
- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test http_retriever
```

- [x] **Step 3: Implement the private runtime adapter**

Follow Librebar 0.6.0's `src/otel/blocking_hyper.rs` ownership pattern. Use a
bounded work channel and readiness handshake. Run one current-thread Tokio
runtime on a named worker. Give each request its own synchronous response
channel. Build the client exactly as follows:

```rust
let client = librebar::http::HttpClient::builder("yamalgam", env!("CARGO_PKG_VERSION"))
    .max_redirects(0)
    .retry_policy(librebar::http::RetryPolicy::none())
    .max_response_size(max_response_size)
    .timeout(timeout)
    .build()?;
```

Set `max_response_size` from the active
`LoaderConfig::limits.max_input_bytes`. Use Librebar's documented zero value
only for an unchecked loader. The session still checks the returned length
before parsing and materialization accounting.

Do not enable cookie or cache features. Check `is_success`, capture opaque ETag
and Last-Modified values into `ResourceValidator`, then consume `into_parts`.
Custom `Debug` must not render bodies or validator contents.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test http_retriever
cargo clippy -p yamalgam-resolve --all-targets -- -D warnings
```

- [x] **Step 5: Commit HTTP retrieval**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): retrieve HTTP refs with Librebar"
```

---

### Task 8: Build a neutral owned YAML document model above parser events

**Files:**

- Modify `crates/yamalgam-parser/src/event.rs`.
- Create `crates/yamalgam-resolve/src/document.rs`.
- Modify `crates/yamalgam-resolve/src/lib.rs`.
- Test in `crates/yamalgam-resolve/tests/document.rs`.

**Interfaces:**

```rust
pub struct OwnedDocument {
    pub root: OwnedNode,
}

pub struct OwnedNode {
    pub origin: ResourceId,
    pub span: Span,
    pub anchor: Option<String>,
    pub tag: Option<String>,
    pub kind: OwnedKind,
}

pub enum OwnedKind {
    Scalar(OwnedScalar),
    Alias { name: String },
    Sequence { style: CollectionStyle, items: Vec<OwnedNode> },
    Mapping { style: CollectionStyle, entries: Vec<(OwnedNode, OwnedNode)> },
}

pub struct OwnedScalar {
    pub value: String,
    pub style: ScalarStyle,
    pub chomp: Chomp,
    pub flags: AtomFlags,
}
```

- [x] **Step 1: Write event/document round-trip tests**

Parse representative scalars and flow/block collections. Include arbitrary
mapping keys, tags, anchors, aliases, styles, chomping, and flags. Also cover
empty documents, comments, and multi-document streams. Assert semantic event
equivalence after `events -> OwnedDocument -> events`. Imported strings must be
`'static`; no slice may borrow retrieved input.

Add malformed-event tests for truncated collections, mapping parity, multiple
roots, and stream events in a node.

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test document
```

- [x] **Step 3: Implement owned conversion iteratively where depth is unbounded**

Add `Event::into_owned(self) -> Event<'static>` in the parser. This avoids
repeated Cow conversion in consumers. `OwnedDocument` parsing consumes
semantic events and retains node presentation metadata and origin. Keep
structural events separately only for root passthrough/CST replay; imported
semantic documents do not need comments in their replacement event stream.

Implement iterative `OwnedNode::materialized_weight` with checked arithmetic.
Count each node allocation, scalar/tag/anchor/alias bytes, collection storage,
and mapping pair storage. Return a typed resource-overflow error instead of
wrapping.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-parser
cargo nextest run -p yamalgam-resolve --test document
```

- [x] **Step 5: Commit the owned model**

```bash
git add crates/yamalgam-parser crates/yamalgam-resolve
git commit -m "feat(resolve): own YAML documents for substitution"
```

---

### Task 9: Implement shared resolution-session caching, cycles, budgets, and reports

**Files:**

- Create `crates/yamalgam-resolve/src/session.rs`.
- Modify `crates/yamalgam-resolve/src/report.rs`.
- Modify `crates/yamalgam-resolve/src/lib.rs`.
- Test in `crates/yamalgam-resolve/tests/session.rs`.

**Interfaces:**

```rust
pub struct ResolutionSession<R> {
    retriever: R,
    config: LoaderConfig,
    cache: HashMap<ResourceId, Arc<OwnedDocument>>,
    aliases: HashMap<ResourceId, ResourceId>,
    active: Vec<ResolutionTarget>,
    edges: usize,
    materialized_bytes: usize,
    report: ResolutionReport,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ResolutionTarget {
    resource: ResourceId,
    fragment: Fragment,
}

pub enum ResolutionTrigger {
    Automatic,
    Explicit(ResolutionCapability),
}

impl<R: ResourceRetriever> ResolutionSession<R> {
    pub fn new(retriever: R, config: LoaderConfig) -> Self;
    pub fn prime_root(&mut self, id: ResourceId, document: OwnedDocument);
    pub fn resolve_root(
        &mut self,
        document: OwnedDocument,
    ) -> Result<OwnedDocument, ResolveError>;
    pub fn resolve_explicit(
        &mut self,
        capability: ResolutionCapability,
        node: OwnedNode,
    ) -> Result<OwnedNode, ResolveError>;
    pub fn report(&self) -> &ResolutionReport;
    pub fn into_report(self) -> ResolutionReport;
    pub fn reset_for_next_root(&mut self) -> ResolutionReport;
}
```

- [x] **Step 1: Write session-invariant tests with a spy retriever**

Prove:

- disabled policy performs zero retrieve/discover calls.
- raw fragmentless documents are fetched and parsed once.
- cache hits still charge a new edge and a new materialization.
- fetched-byte reporting distinguishes first fetch from cache reuse.
- active cycle keys are `(ResourceId, Fragment)`; separate fragments of one
  resource are allowed while a return to the same target fails with the full
  chain.
- root raw documents are primed before resolution, enabling same-document refs.
- retrieved streams with zero or multiple documents fail before selection.
- invalid UTF-8 and invalid YAML remain distinct structured failures.
- depth/edge/materialized-byte exact boundaries pass and boundary+1 fails with
  category, observed, maximum, source site, and report edge outcome.
- counter overflow is structured and never wraps.
- explicit capability recursion resolves the same capability; the other
  capability resolves only in `Automatic` mode.
- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test session
```

- [x] **Step 3: Implement the session state machine**

Cache parsed unresolved documents. Record an edge before handling each target.
Check its edge and depth budgets. Retrieve the raw document or clone it from
cache. Select or aggregate the target, check self-containment, and namespace
anchors. Resolve the selected subtree under `ResolutionTrigger`. Charge its
materialized weight before marking the report edge successful. Every early
return marks that same edge with the typed terminal outcome.

Use RAII or one cleanup function so the active cycle stack is popped on every
success/error path. Never hold a cache or report mutable borrow while calling
the retriever or recursing.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test session
```

- [x] **Step 5: Commit the session engine**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): share caches budgets and reports"
```

---

### Task 10: Implement sole-key `$ref` and RFC 6901 selection

**Files:**

- Create `crates/yamalgam-resolve/src/ref_resolver.rs`.
- Modify `crates/yamalgam-resolve/src/pointer.rs`.
- Modify `crates/yamalgam-resolve/src/session.rs`.
- Test in `crates/yamalgam-resolve/tests/ref_resolution.rs`.
- [x] **Step 1: Write the complete `$ref` behavior matrix**

Cover:

- fragmentless and empty-fragment whole-document substitution.
- mapping-key and sequence-index RFC 6901 selection.
- escaped `/`, escaped `~`, and percent-encoded UTF-8 tokens.
- native null/bool/integer/float/string/sequence/mapping targets.
- same-document, relative file, absolute file URI, memory URI, HTTP, HTTPS,
  and nested relative targets.
- sole-key detection with block and flow mappings.
- rejection of siblings, non-string values, malformed pointers, missing keys,
  out-of-range/leading-zero/`-` sequence indices, and non-string YAML mapping
  keys.
- selection before recursion: unresolved directives outside the selected
  fragment must not fetch or fail.
- cross-resource and same-resource fragment cycles.

Use this exact sibling regression:

```yaml
$ref: ./shared.yaml#/defaults
local: must-not-be-silently-merged
```

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test ref_resolution
```

- [x] **Step 3: Implement replacement and selection**

Recognize `$ref` only when ref resolution should run. If any `$ref` key is
present in a mapping being resolved, require exactly one entry and a YAML
string value. Resolve the URI reference from the directive node's `origin`,
not the root input. Select from the unresolved cached tree before recursively
resolving the selected clone.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test ref_resolution
```

- [x] **Step 5: Commit `$ref`**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): replace URI refs with YAML nodes"
```

---

### Task 11: Implement scalar and structured `!include` aggregation

**Files:**

- Create `crates/yamalgam-resolve/src/include.rs`.
- Modify `crates/yamalgam-resolve/src/session.rs`.
- Test in `crates/yamalgam-resolve/tests/include_resolution.rs`.

**Parsed option types:**

```rust
enum IncludeSelector {
    Path(String),
    Glob(String),
    Dir(String),
}

struct IncludeSpec {
    selector: IncludeSelector,
    format: IncludeFormat,
    mode: IncludeMode,
    recursive: bool,
    matches: Vec<String>,
    key: IncludeKey,
    on_conflict: ConflictPolicy,
}
```

- [x] **Step 1: Write directive-shape tests**

Cover scalar shorthand and structured mappings with exactly one of `path`,
`glob`, or `dir`. Reject unknown or duplicate options. Reject wrong scalar
types, selector conflicts, and invalid enums. Reject `recursive`/`match`
outside `dir`, `key` outside `named`, and `on_conflict` outside mapping
`merge`.

- [x] **Step 2: Write every mode and context test**

Cover:

- `node`: exactly one YAML or UTF-8 text resource.
- `list`: ordered YAML roots or text strings.
- `named`: `stem` and extensionless `/`-normalized `relative` keys, duplicate
  names, and non-UTF-8 derived names.
- `merge`: homogeneous mappings with `error`/`first`/`last`, homogeneous
  sequence concatenation, mixed roots, and scalar roots.
- `splice`: sequence-item context only and sequence roots only.
- `<<: !include`: included mappings merge into the containing mapping while
  explicit parent entries win regardless of order.
- zero matches, one/many path mismatches, invalid UTF-8 text, invalid YAML,
  multiple YAML documents, directory recursion, and deterministic reversed
  creation order.
- HTTP/HTTPS include rejection even when HTTP refs are enabled.
- per-match edge/byte/report accounting.
- [x] **Step 3: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test include_resolution
```

- [x] **Step 4: Implement context-aware aggregation**

Parse options without serializing the node. Resolve discovered resources in
the retriever's deterministic order. Apply `on_conflict` only among included
roots. For `<<: !include`, remove the merge entry after materialization and add
explicit containing entries last so they always replace included values.
For `splice`, replace the directive item with the selected sequence items.

Charge each matched root independently. Then charge synthesized list/named keys
and container storage. Aggregate wrappers cannot evade the byte budget.

- [x] **Step 5: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test include_resolution
```

- [x] **Step 6: Commit includes**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): aggregate structured includes"
```

---

### Task 12: Namespace imported anchors and reject non-self-contained fragments

**Files:**

- Create `crates/yamalgam-resolve/src/anchors.rs`.
- Modify `crates/yamalgam-resolve/src/session.rs`.
- Test in `crates/yamalgam-resolve/tests/anchors.rs`.
- [x] **Step 1: Write anchor and alias tests**

Cover duplicate source anchor names from two imports. Repeat one cached
fragment at two insertion sites. Test aliases wholly inside a selected subtree
and aliases that depend on an outside anchor. Include aliases inside keys,
nested imports, and anchor-like text that must not change.

Assert names are stable across runs and unique per materialization edge. Use a
YAML-valid prefix such as `yg_r{resource_ordinal}_e{edge_ordinal}_` and verify
all alias references are rewritten with the corresponding definition.

- [x] **Step 2: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve --test anchors
```

- [x] **Step 3: Implement two-pass subtree analysis**

First collect anchors defined in the selected subtree and aliases used there.
Reject `used - defined`. Then allocate deterministic names from report resource
ordinal plus edge ordinal and rewrite definitions and references. Do not use a
hash digest or process-random state in emitted names.

- [x] **Step 4: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve --test anchors
```

- [x] **Step 5: Commit anchor handling**

```bash
git add crates/yamalgam-resolve
git commit -m "feat(resolve): namespace imported YAML anchors"
```

---

### Task 13: Implement `BuiltinResolver` over the existing event middleware

**Files:**

- Create `crates/yamalgam-resolve/src/builtin.rs`.
- Modify `crates/yamalgam-resolve/src/lib.rs`.
- Add dev-dependencies in `crates/yamalgam-compose/Cargo.toml`,
  `crates/yamalgam-serde/Cargo.toml`, and `crates/yamalgam-cst/Cargo.toml`.
- Test in existing consumer test modules or new focused integration tests.

**Interface:**

```rust
pub struct BuiltinResolver<R> {
    session: ResolutionSession<R>,
    root: ResourceId,
    state: ResolverState,
    completed_reports: Vec<ResolutionReport>,
}

impl<'input, R: ResourceRetriever> Resolver<'input> for BuiltinResolver<R> {
    fn on_event_with_config(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError>;
}
```

Expose `BuiltinResolver::reports(&self) -> &[ResolutionReport]` and
`BuiltinResolver::into_reports(self) -> Vec<ResolutionReport>`. At each
top-level `DocumentEnd`, call `session.reset_for_next_root()`. Move its result
into `completed_reports`. The next document gets a fresh cache, cycle stack,
budgets, and report. It retains the same retriever.

- [x] **Step 1: Write middleware tests**

Prove that the resolver:

- buffers one root document at a time, primes it, resolves it, and emits owned
  replacement events.
- supports multi-document top-level streams with reset session state/report per
  root document.
- uses zero-width substitution spans at each directive site.
- stops permanently after the first terminal error, matching
  `ResolvedEvents`.
- passes all events through unchanged with no owned conversion or retrieval
  when both capabilities are disabled.
- [x] **Step 2: Write cross-consumer integration tests**

Run the same memory-backed `$ref` and `!include` fixtures through:

- `yamalgam-compose::Composer::with_resolver_and_config`.
- `yamalgam_serde::Deserializer::with_resolver_and_config`.
- `yamalgam_cst::parse_to_cst_with_resolver_and_config`.

Assert equivalent resolved values. For CST, assert `cst.to_text() == input`.
Injected semantic nodes must be synthetic. The original directive text remains
recoverable from gap coverage.

- [x] **Step 3: Verify RED**

```bash
cargo nextest run -p yamalgam-resolve -p yamalgam-compose -p yamalgam-serde -p yamalgam-cst -E 'test(builtin_resolution)'
```

- [x] **Step 4: Implement the adapter**

Use the neutral document/session APIs; do not duplicate `$ref` or include
logic. Replay borrowed input events on disabled policy. When resolution is
enabled, retain original root structural events for CST coverage but emit
resolved semantic nodes as owned events. Imported nodes use synthetic
zero-width spans at the directive site while the report retains original
resource spans.

- [x] **Step 5: Verify GREEN**

```bash
cargo nextest run -p yamalgam-resolve -p yamalgam-compose -p yamalgam-serde -p yamalgam-cst
```

- [x] **Step 6: Commit consumer integration**

```bash
git add crates/yamalgam-resolve crates/yamalgam-compose crates/yamalgam-serde crates/yamalgam-cst
git commit -m "feat(resolve): integrate built-in event resolution"
```

---

### Task 14: Preserve resource origins and expose automatic/explicit query resolution

**Files:**

- Modify `crates/yamalgam-query/Cargo.toml`.
- Create `crates/yamalgam-query/src/resolution.rs`.
- Create `crates/yamalgam-query/src/runtime_resolution.rs`.
- Modify `crates/yamalgam-query/src/lib.rs`.
- Modify `crates/yamalgam-query/src/build.rs`.
- Modify `crates/yamalgam-query/src/value.rs`.
- Modify `crates/yamalgam-query/src/engine.rs`.
- Modify `crates/yamalgam-query/src/funs.rs`.
- Modify `crates/yamalgam-query/src/error.rs`.
- Test in `crates/yamalgam-query/tests/resolution.rs` and existing engine tests.

**Interfaces:**

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceOrigin {
    pub resource: ResourceId,
    #[doc(hidden)]
    pub session: ResolutionSessionKey,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResolutionSessionKey(u64);

pub type ResolutionHandle = Rc<RefCell<dyn QueryResolver>>;

pub trait QueryResolver {
    fn resolve_value(
        &mut self,
        capability: ResolutionCapability,
        value: YamlVal,
    ) -> Result<YamlVal, ResolveError>;
    fn report(&self) -> &ResolutionReport;
}

pub struct ResolvedInput {
    pub value: YamlVal,
    pub resolution: ResolutionHandle,
}
```

- [x] **Step 1: Write owned-node/query-value adapter tests**

Round-trip every YAML data type. Preserve tags, anchors, aliases, style, raw
scalar text, chomp, flags, mapping order, and arbitrary keys. Preserve each
node's `ResourceOrigin`. Constructed strings that resemble booleans, numbers,
or null must remain strings. Imported nested nodes retain their parsed resource
rather than the root input resource.

- [x] **Step 2: Write policy/filter tests**

Cover:

- automatic resolution before filter evaluation.
- `include` on tagged scalar/structured directives under explicit include
  policy.
- `ref` on sole-key mappings under explicit ref policy.
- disabled filter errors without I/O.
- explicit same-capability recursion and automatic-only cross-capability
  recursion.
- relative filters using selected node origin.
- constructed directives using the active `--base-uri` equivalent and failing
  without one.
- slurped values from multiple input sessions selecting the correct opaque
  session key.
- report access after success and failure.
- typed runtime resolution errors surviving the jaq error boundary.
- [x] **Step 3: Verify RED**

```bash
cargo nextest run -p yamalgam-query --test resolution
```

- [x] **Step 4: Implement the query adapter and session registry**

Add `Meta::origin`. Refactor scalar construction from `build.rs` into a shared
crate-private helper so `OwnedNode -> YamlVal` follows the same schema logic.
Implement `YamlVal -> OwnedNode` with canonical scalar presentation when a
constructed value lacks raw metadata.

`RunCtx` owns an opaque session registry and stages the active session with
`InputMeta`. Native filters select a session from the value's origin first,
then the active input/base session. Keep resolution sessions out of value
equality, ordering, hashing, and emission.

Allocate `ResolutionSessionKey` from a checked run-local counter starting at
one. Store `HashMap<ResolutionSessionKey, ResolutionHandle>` in `RunCtx`; a
counter overflow returns `YgError::ResourceOverflow` and never reuses a key.

Register native `include` and `ref` filters. Mirror `runtime_limits`. Place the
typed `ResolveError` and report handle in a one-shot side channel. Then return
a stable jaq text error. Recover the typed values during engine normalization.
Add structured `YgError::{InputResolution,RuntimeResolution}` variants with
preserved rendered context and sources.

- [x] **Step 5: Preserve the disabled fast path**

Existing `parse_str_with_config` must continue through `NoopResolver` with no
resolver crate allocation when policy is disabled. Add a spy/allocation-neutral
regression that disabled query parsing never calls a retriever or creates an
HTTP worker.

- [x] **Step 6: Verify GREEN**

```bash
cargo nextest run -p yamalgam-query
```

- [x] **Step 7: Commit query integration**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): resolve external YAML resources"
```

---

### Task 15: Wire the accepted resolution policy into `yg`

**Files:**

- Modify `crates/yamalgam/src/lib.rs`.
- Modify `crates/yamalgam/src/commands/query.rs`.
- Modify `crates/yamalgam-query/src/error.rs` to add filename/document context
  without erasing structured resolution errors.
- Test in `crates/yamalgam/tests/yg.rs` and focused unit tests in
  `crates/yamalgam/src/commands/query.rs`.

**CLI fields:**

```rust
#[arg(long, conflicts_with = "resolve_includes")]
pub allow_includes: bool,

#[arg(long, conflicts_with = "allow_includes")]
pub resolve_includes: bool,

#[arg(long, value_delimiter = ',', conflicts_with = "resolve_refs")]
pub allow_refs: Vec<RefSchemeArg>,

#[arg(long, value_delimiter = ',', conflicts_with = "allow_refs")]
pub resolve_refs: Vec<RefSchemeArg>,

#[arg(long = "allow-ref-host")]
pub allow_ref_hosts: Vec<String>,

#[arg(long, value_name = "DIR")]
pub resolution_root: Option<Utf8PathBuf>,

#[arg(long, value_name = "URI")]
pub base_uri: Option<String>,
```

- [x] **Step 1: Write argument and validation tests**

Assert exact help text and conflicts. Reject duplicate or unsupported schemes.
Reject HTTP/HTTPS without a host and host entries without HTTP/HTTPS. Reject
fragments in `--base-uri` and file bases outside `--resolution-root`. Reject
relative filesystem access from stdin without a root/base. Malformed hosts and
URIs also fail. Normalize scheme and host case once.

- [x] **Step 2: Write filesystem CLI behavior tests**

Use temp fixtures for automatic and explicit include/ref flags. Cover
`--resolution-root`, per-file bases, and stdin with `--base-uri`. Cover
multiple roots, slurp, raw input, and null input. Exercise in-place output and
both YAML and JSON output. Assert typed exit status 5. Default invocations must
never retrieve.

- [x] **Step 3: Write local HTTP CLI tests**

Use an in-process server for HTTP success and nested relative URL refs. Test
host denial, redirect refusal, and non-success status. Test timeout, body
limit, invalid YAML, and invalid UTF-8. Do not make an internet request.

- [x] **Step 4: Verify RED**

```bash
cargo nextest run -p yamalgam --test yg -E 'test(resolve) | test(include) | test(ref)'
```

- [x] **Step 5: Implement thin CLI wiring**

Replace `const fn loader_config` with a fallible policy builder. Build one
shared retriever router per root input. Add memory only for library callers.
Add file retrieval when file/include access is enabled. Add one Librebar worker
when HTTP/HTTPS refs are enabled. Create one `ResolutionSession` per top-level
document while sharing the router. Store its handle on `InputRecord` so
explicit filters continue the same report and cache.

The initial file's directory is the default capability root. An explicit
`--resolution-root` overrides it and requires the initial file/base to be
contained. Standard input needs `--base-uri` for relative refs and
`--resolution-root` for includes. Top-level URL-shaped FILE arguments remain
ordinary paths and fail as input reads.

- [x] **Step 6: Verify GREEN**

```bash
cargo nextest run -p yamalgam --test yg
cargo nextest run -p yamalgam --test cli
```

- [x] **Step 7: Commit CLI behavior**

```bash
git add crates/yamalgam crates/yamalgam-query
git commit -m "feat(yg): expose controlled resource resolution"
```

---

### Task 16: Document the M11 truth for end users and maintainers

**Files:**

- Modify `docs/yg/querying.md`.
- Modify `docs/yg/reference.md`.
- Modify `docs/yg/yaml-features.md`.
- Modify `docs/yg/troubleshooting.md`.
- Modify `docs/yg/README.md` if navigation changes.
- Modify `docs/spec/yg-query-language-spec-draft-01.md` section 7.
- Modify `docs/spec/yg-spec-draft-01-implementation-notes.md`.
- [x] **Step 1: Add executable guide examples**

Document and exercise:

```bash
yg --resolve-refs=file,https --allow-ref-host=example.test \
  --resolution-root=. '.' app.yaml

yg --allow-includes 'include' app.yaml
yg --allow-refs=file 'ref' app.yaml
```

Use local fixtures in `scripts/check-yg-doc-examples`; do not depend on public
network access. Add examples for whole-document `$ref` and RFC 6901 fragments.
Cover mapping merge, sequence splice, glob/list/named/merge, and raw text. Show
base and root selection.

- [x] **Step 2: Document security and non-goals plainly**

State that network access is opt-in by scheme and host. Redirects, cookies,
auth, persistent cache, and HTTP includes are unavailable. Symlinks default to
off. `$ref` siblings are errors. Retrieved YAML must be one document. Top-level
URLs are not inputs. Reports and lockfiles have no CLI command in M11.

- [x] **Step 3: Update spec implementation markers**

Replace only now-implemented resolution notes. Keep deferred lockfiles,
redirect-hop validation, HTTP includes, auth, parameters, and dependency CLI
work explicitly deferred. Add requirement-to-test traceability markers for the
new flags, filters, resolution semantics, report, and security boundary.

- [x] **Step 4: Run all documentation gates**

```bash
just docs-check
just mdlint docs/yg docs/spec/yg-query-language-spec-draft-01.md docs/spec/yg-spec-draft-01-implementation-notes.md
```

- [x] **Step 5: Commit end-user documentation**

```bash
git add docs
git commit -m "docs(yg): document external resource resolution"
```

---

### Task 17: Run full verification, independent review, and final reconciliation

**Files:**

- Modify only files required by confirmed review findings.
- Do not add release notes or increment versions unless separately requested.
- [ ] **Step 1: Run focused resolution suites with all features**

```bash
cargo nextest run -p yamalgam-core -p yamalgam-parser -p yamalgam-resolve -p yamalgam-compose -p yamalgam-serde -p yamalgam-cst -p yamalgam-query -p yamalgam --all-features
```

- [ ] **Step 2: Re-run YAML compliance with resolution disabled**

```bash
just test-compliance
```

Expected: the existing compliance baseline is unchanged because external
resolution remains disabled by default.

- [ ] **Step 3: Run dependency, unsafe, and documentation gates**

```bash
just deny
just unsafe-census
just docs-check
```

Inspect the unsafe census delta from Librebar HTTP/TLS dependencies. No new
unsafe code may appear in Yamalgam crates.

- [ ] **Step 4: Run the complete repository gate**

```bash
just check
git --no-pager diff --check
```

- [ ] **Step 5: Request independent architecture/security and Rust reviews**

Review explicitly for:

- URI/base/fragment correctness and selection-before-recursion.
- sandbox escape, symlink, glob, HTTP allowlist, redirect, credential, and
  report-redaction boundaries.
- cache/cycle/accounting/report consistency on every error path.
- owned event lifetimes, anchor namespace correctness, CST coverage, and query
  origin/session routing.
- exact policy mode semantics and structured errors across jaq/CLI boundaries.

Classify every review item as confirmed, false positive, or deferred-by-design.
Fix every confirmed issue with a failing regression first.

- [ ] **Step 6: Re-run affected focused tests and `just check` after review**

```bash
just check
git --no-pager diff --check
git status --short
```

- [ ] **Step 7: Commit confirmed review fixes**

```bash
git add crates docs record Cargo.toml Cargo.lock
git commit -m "fix(resolve): address M11 integration review"
```

If review requires no edits, do not create an empty commit. Record the exact
verified HEAD, commands, and results in the handoff instead.

## Final acceptance checklist

- [ ] File, memory, HTTP, and custom retrievers share one synchronous trait.
- [ ] Librebar is exactly 0.6.0 and HTTP redirects/cookies/cache/auth are off.
- [ ] Disabled, explicit, and automatic modes match ADR-0016 at every entry
  point.
- [ ] `$ref` implements sole-key replacement, whole-document roots, and RFC
  6901 fragments for file and URL resources.
- [ ] `!include` implements node, mapping merge, sequence splice, glob,
  directory, list, named, merge, and UTF-8 text contracts.
- [ ] Filesystem operations remain capability-relative; traversal and symlink
  escape tests pass on macOS and CI platforms.
- [ ] Every session shares raw parse cache, cycle stack, budgets, and report;
  cache hits remain chargeable.
- [ ] Anchor/alias behavior is deterministic and fragment self-containment is
  enforced.
- [ ] Composer, serde, CST, query, and CLI integration tests pass.
- [ ] Query values retain resource origin and explicit filters reuse the
  correct session after slurp/multiple inputs.
- [ ] Reports exist on success and failure, preserve integrity/provenance seams,
  and render no sensitive URL/validator/body data.
- [ ] End-user docs state the exact M11 security boundary and deferred work.
- [ ] `just test-compliance`, `just docs-check`, `just unsafe-census`, and
  `just check` pass at the final HEAD.
