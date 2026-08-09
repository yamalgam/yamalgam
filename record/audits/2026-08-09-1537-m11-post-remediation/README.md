---
audit_date: 2026-08-09
commit: 246f707890c9bc42bd41b5af128905be104bf354
scope: M11 external-resolution changes from c85f7e0 through 246f707
findings:
  critical: 0
  significant: 6
  moderate: 5
  advisory: 0
  note: 0
---

# Audit: M11 Post-Remediation Review

The first remediation pass closes all 20 findings from the M11 final review,
but this fresh audit found 11 remaining boundary gaps. **The Resolution Policy
and Graph Integrity Surface** still has inconsistent entry-point policy and
anchor handling. **The Session Budgets and Discovery Surface** bounds output
better than it bounds retained memory and filesystem work. **The Query Runtime
and Reporting Surface** preserves typed errors on its primary paths but still
has recursive unchecked conversions and incomplete report retention. M11 is
close; these are the remaining shipping blockers for hostile or very large
inputs.

Tool coverage was complete through the canonical repository gates: `just
check`, `just deny`, and `just unsafe-census` all passed at the audited commit.
The Crustoleum wrapper's Geiger pass could not attribute benchmark-only
`gungraun` code because that registry source was unavailable; the shipped
Yamalgam crates remained covered by the repository unsafe census.

## The Resolution Policy and Graph Integrity Surface

*The central resolver is disciplined, but three public entry paths do not share
one authoritative policy and anchor boundary.*

### Relative Includes Can Retrieve Non-File Schemes

**significant** · `crates/yamalgam-resolve/src/session.rs:1277-1292` · effort:
small

The M11 contract limits includes to filesystem resources. Policy enforcement
rejects HTTP and HTTPS, but a relative include under a memory or custom-scheme
document inherits that scheme and returns through the early-success branch.
The registered backend then receives it without the reference scheme allowlist.

```rust crates/yamalgam-resolve/src/session.rs:1277-1292
        if capability == ResolutionCapability::Include
            && matches!(reference.resource.scheme(), "http" | "https")
        {
            return Err(self.error(
                ResolutionErrorCategory::PolicyDenied,
                capability,
                span,
                target_name(reference),
                Some(Box::new(RetrieveError::PolicyDenied {
                    id: reference.resource.clone(),
                })),
            ));
        }
        if capability != ResolutionCapability::Ref || reference.resource == *parent {
            return Ok(());
        }
```

> A custom retriever gives me a second transport. I only need a relative
> include under that origin; the file-only contract is never checked.

**Remediation:** Require every resolved include target and discovery base to
use the file scheme. Replace memory-backed include success fixtures with
filesystem fixtures or explicit rejection tests.

### Explicit-Only Sessions Skip Root Anchor Reservation

**significant** · `crates/yamalgam-resolve/src/session.rs:99-120` · effort:
small

Automatic resolution reserves anchors from the complete root before importing
subtrees. Explicit-only query sessions call `prime_root` instead, which caches
the document without reserving its anchors. A later explicit import can choose
a namespace that collides with a sibling root anchor and rebind a following
alias.

```rust crates/yamalgam-resolve/src/session.rs:99-120
    /// Seed the raw-document cache with the containing root document.
    pub fn prime_root(&mut self, id: ResourceId, document: OwnedDocument) {
        if self.cache.insert(id.clone(), Arc::new(document)).is_none() {
            self.report.record_resource(ResourceRecord {
                kind: resource_kind(&id),
                id,
                fetched_bytes: 0,
                cache_hits: 0,
                provenance: ResourceProvenance::default(),
            });
        }
    }

    /// Resolve automatic directives in one root document.
    ///
    /// # Errors
    ///
    /// Returns a structured [`ResolveError`] on the first failed directive.
    pub fn resolve_root(&mut self, document: OwnedDocument) -> Result<OwnedDocument, ResolveError> {
        anchors::reserve(&document.root, &mut self.reserved_anchors);
        let origin = document.root.origin.clone();
        self.prime_root(origin, document.clone());
```

**Remediation:** Reserve `document.root` inside `prime_root` and remove the
duplicate automatic-path reservation. Add an explicit-only subtree regression
with a predicted generated name already present in a root sibling.

### Builtin Resolver Mixes Caller and Session Configs

**significant** · `crates/yamalgam-resolve/src/builtin.rs:58-90` · effort:
medium

`on_event_with_config` supplies the downstream consumer's `LoaderConfig`, while
the retained `ResolutionSession` owns the independently supplied constructor
configuration. The caller configuration decides whether buffering runs; the
session configuration controls modes, schemes, hosts, roots, parsing, and
limits. A permissive constructor policy can therefore resolve directives even
when the active consumer policy is stricter.

```rust crates/yamalgam-resolve/src/builtin.rs:58-90
    fn process<'input>(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        if !automatically_resolves(config) {
            return Ok(yamalgam_parser::resolve::smallvec![event]);
        }
        if matches!(self.state, ResolverState::Failed) {
            return Ok(ResolvedEventBuf::new());
        }
        if matches!(event, Event::StreamStart | Event::StreamEnd) {
            return Ok(yamalgam_parser::resolve::smallvec![event]);
        }

        let is_document_end = matches!(event, Event::DocumentEnd { .. });
        let owned = event.into_owned();
        match &mut self.state {
            ResolverState::Idle => self.state = ResolverState::Buffering(vec![owned]),
            ResolverState::Buffering(events) => events.push(owned),
            ResolverState::Failed => unreachable!("failed state returned above"),
        }
        if !is_document_end {
            return Ok(ResolvedEventBuf::new());
        }

        let ResolverState::Buffering(events) = mem::replace(&mut self.state, ResolverState::Idle)
        else {
            unreachable!("document end always has a buffer")
        };
        let result = OwnedDocument::from_events(self.root.clone(), events.iter().cloned())
            .map_err(document_error)
            .and_then(|document| self.session.resolve_root(document));
```

**Remediation:** Make one configuration authoritative. Reject a downstream
configuration that differs from the session before buffering, or redesign the
adapter so one shared configuration constructs both components. Cover mode,
root, host, and limit mismatches.

*Verdict: Identity classification and backend policy are solid on the main
path. M11 still needs one include transport rule, one root-seeding path, and one
authoritative loader configuration across every adapter.*

## The Session Budgets and Discovery Surface

*Edge and materialization limits are enforced, but retained cache memory and
filesystem traversal work can grow beyond those counters.*

### Resolution Cache Bypasses Session Byte Budget

**significant** · `crates/yamalgam-resolve/src/session.rs:870-899` · effort:
medium

Each unique resource may consume the complete per-input ceiling. Its bytes then
remain in `raw_cache` alongside parsed or text representations, without a
session-total charge. A reference can select a tiny fragment from a large
resource, so `max_materialized_bytes` stays low while retained memory grows. At
moderate defaults, 256 edges at 256 MiB each permit roughly 64 GiB of raw cache
before parsed data.

```rust crates/yamalgam-resolve/src/session.rs:870-899
        let retrieved = self
            .retriever
            .retrieve(&ResourceRequest {
                id: requested.clone(),
                max_bytes: self.config.limits.max_input_bytes,
            })
            .map_err(|source| {
                self.error(
                    retrieve_category(&source),
                    capability,
                    span,
                    requested.to_string(),
                    Some(Box::new(source)),
                )
            })?;
        let fetched_bytes = retrieved.bytes.len();
        let canonical = retrieved.id;
        if &canonical != requested {
            self.aliases.insert(requested.clone(), canonical.clone());
        }
        self.report.record_resource(ResourceRecord {
            id: canonical.clone(),
            kind: resource_kind(&canonical),
            fetched_bytes,
            cache_hits: 0,
            provenance: retrieved.provenance,
        });
        let bytes: Arc<[u8]> = Arc::from(retrieved.bytes);
        self.raw_cache.insert(canonical.clone(), Arc::clone(&bytes));
        Ok((bytes, canonical, false, fetched_bytes))
```

> I do not need a large output. I reference one small fragment from each large
> resource and let the cache retain everything the materialization budget does
> not see.

**Remediation:** Add a session-wide fetched or cache byte limit and charge it
before parsing. Drop raw bytes after building a typed cache where possible, or
conservatively account raw, text, and parsed representations. Test many large
resources exposing small fragments.

### Glob Discovery Scans Entire Base

**moderate** · `crates/yamalgam-resolve/src/retriever/file.rs:373-388` · effort:
medium

Every glob starts at the containing directory with recursion enabled, even when
the pattern cannot match below that directory. `max_matches` counts approved
matches, not visited directories or non-matching entries, and each visited
directory is fully collected and sorted. A no-match pattern can therefore scan
and buffer an arbitrarily large authorized tree without spending edge budget.

```rust crates/yamalgam-resolve/src/retriever/file.rs:373-388
        match &request.selector {
            DiscoverySelector::Glob { pattern } => {
                reject_escaping_pattern(pattern, &request.base)?;
                let glob = compile_globs(std::slice::from_ref(pattern))?;
                let matcher = |candidate: &Path, _name: &OsStr| {
                    candidate
                        .strip_prefix(parent)
                        .is_ok_and(|relative| glob.is_match(relative))
                };
                let mut state = WalkState {
                    recursive: true,
                    matcher: &matcher,
                    maximum: request.max_matches,
                    matches: &mut matches,
                };
                self.walk(&canonical_parent, parent, &mut state)?;
```

**Remediation:** Derive the static directory prefix and required recursion
depth from the glob. Add a discovery visit budget for directories and entries,
including non-matches, and enforce it while iterating.

### Symlink Aliases Consume Match Limit Before Deduplication

**moderate** · `crates/yamalgam-resolve/src/retriever/file.rs:179-197` · effort:
small

When symlink following is enabled, one canonical file can be reached through
its real directory and multiple aliases. Every occurrence increments the
vector length before final sorting and deduplication. A ceiling of one can reject
two aliases to one resource even though the returned set would contain one
identity.

```rust crates/yamalgam-resolve/src/retriever/file.rs:179-197
    fn record_match(
        &self,
        requested: &Path,
        canonical: PathBuf,
        maximum: Option<usize>,
        matches: &mut Vec<PathBuf>,
    ) -> Result<(), RetrieveError> {
        self.approved(requested)?;
        self.approved(&canonical)?;
        matches.push(canonical);
        if let Some(maximum) = maximum
            && matches.len() > maximum
        {
            return Err(RetrieveError::TooManyMatches {
                observed: maximum.saturating_add(1),
                maximum,
            });
        }
        Ok(())
```

Related to [Glob Discovery Scans Entire Base](#glob-discovery-scans-entire-base).

**Remediation:** Track canonical matches during traversal and apply
`max_matches` only to newly inserted identities. Continue checking every alias
path independently against policy.

*Verdict: The filesystem backend is race-safe and deterministic, but its
resource ceilings need to count the work and retained state that produce the
result, not only the final unique edges.*

## The Query Runtime and Reporting Surface

*Typed errors and reports survive the main paths, but deep unchecked values and
several post-resolution branches still escape those guarantees.*

### Caught Resolution Errors Accumulate Until Run End

**moderate** · `crates/yamalgam-query/src/runtime_resolution.rs:142-185` ·
effort: medium

Each runtime resolution error is inserted into a thread-local token map. When
jaq catches the error and returns a fallback, the token-bearing value never
reaches `take_error`; the structured error remains until the guard drops. A
filter can retain one error per caught attempt for the life of a long run.

```rust crates/yamalgam-query/src/runtime_resolution.rs:142-185
pub struct Guard {
    pending: HashMap<u64, ResolveError>,
}

pub fn activate() -> Guard {
    ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_add(1)));
    Guard {
        pending: PENDING_ERRORS.with(|errors| errors.replace(HashMap::new())),
    }
}

pub fn jaq_error(error: ResolveError) -> jaq_core::Error<YamlVal> {
    let message = error.to_string();
    let mut value = YamlVal::str(format!("external resolution failed: {message}"));
    let token = ACTIVE_GUARDS.with(|guards| guards.get() > 0).then(|| {
        NEXT_ERROR_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .ok()
    });
    let token = token.flatten();
    if let Some(token) = token {
        value.runtime_error_token = Some(token);
        PENDING_ERRORS.with(|errors| {
            errors.borrow_mut().insert(token, error);
        });
    }
    jaq_core::Error::new(value)
}

pub fn take_error(value: &YamlVal) -> Option<ResolveError> {
    let token = value.runtime_error_token?;
    PENDING_ERRORS.with(|errors| errors.borrow_mut().remove(&token))
}

impl Drop for Guard {
    fn drop(&mut self) {
        PENDING_ERRORS.with(|errors| {
            errors.replace(std::mem::take(&mut self.pending));
        });
        ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_sub(1)));
    }
}
```

**Remediation:** Prefer a hidden reference-counted error payload on `YamlVal`
that is excluded from equality and hashing. If the side channel remains, bound
it and add catch-aware reclamation. Cover repeated caught failures with a
test-only pending count.

### Post-Resolution Conversion Drops Session Report

**moderate** · `crates/yamalgam-query/src/resolution.rs:119-137` · effort:
medium

Resolver failures now retain their report, but `value_from_owned_node` can fail
after successful resolution and drops the still-live resolver through `?`.
The multi-document path repeats this conversion and can also lose reports from
earlier documents.

```rust crates/yamalgam-query/src/resolution.rs:119-137
    let document = OwnedDocument::from_events(root.clone(), events)
        .map_err(|error| YgError::InputParse(error.to_string()))?;
    let mut resolver = SessionQueryResolver::new(retriever, config.clone(), Some(root));
    let document = match resolver.resolve_input(document) {
        Ok(document) => document,
        Err(error) => {
            let message = error.to_string();
            return Err(YgError::input_resolution(
                error,
                message,
                vec![resolver.into_report()],
            ));
        }
    };
    let value = value_from_owned_node(document.root, ResolutionSessionKey::UNREGISTERED, config)?;
    Ok(ResolvedInput {
        value,
        resolution: Rc::new(RefCell::new(resolver)),
    })
```

**Remediation:** Preserve reports on every error after session creation. Add a
post-resolution input error wrapper carrying the structured conversion error
and completed/current reports in both single- and multi-document paths.

### Direct Include Preflight Can Omit Failed Report Edge

**moderate** · `crates/yamalgam-resolve/src/session.rs:408-435` · effort:
small

Path includes do not create an edge until after target-resource preflight. A
relative path on an anonymous origin produces `ResourceError::MissingBase`,
which this branch wraps as `InvalidResource`, before any edge exists. The error
reaches the caller without a source edge in the promised failure report.

```rust crates/yamalgam-resolve/src/session.rs:408-435
        let first_edge = self.report.edges().len();
        let collection_edge = match &spec.selector {
            IncludeSelector::Path(_) => None,
            IncludeSelector::Glob(pattern) | IncludeSelector::Dir(pattern) => {
                Some(self.report.record_edge(ResolutionEdge {
                    parent: directive.origin.clone(),
                    child: None,
                    capability: ResolutionCapability::Include,
                    source_span: directive.span,
                    requested: pattern.clone(),
                    fragment: Fragment::Root,
                    matches: Vec::new(),
                    cache_hit: false,
                    fetched_bytes: 0,
                    materialized_bytes: 0,
                    outcome: ResolutionOutcome::Resolved,
                }))
            }
        };
        let result = self.resolve_include_inner(directive, spec, context, trigger, collection_edge);
        if let Err(error) = &result {
            let failed_edge = collection_edge.or_else(|| {
                (self.report.edges().len() > first_edge).then(|| self.report.edges().len() - 1)
            });
            if let Some(failed_edge) = failed_edge {
                self.report.edge_mut(failed_edge).outcome =
                    ResolutionOutcome::Failed(error_category(error));
            }
```

**Remediation:** Record a sanitized path edge before preflight and update that
exact edge on every failure. Use the shared requested-reference helper so the
stable `MissingBase` category is preserved.

### Resolved Owned Tree Conversion Remains Recursive

**significant** · `crates/yamalgam-query/src/resolution.rs:323-342` · effort:
medium

External YAML reaches `owned_to_value` after resolution. When `max_depth` is
`None`, the only guard protects `usize` arithmetic; input depth maps directly
to Rust call-stack depth. Unchecked loading can therefore abort the process
instead of returning a typed error.

```rust crates/yamalgam-query/src/resolution.rs:323-342
        OwnedKind::Sequence { style, items } => {
            let child_depth = checked_child_depth(depth)?;
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(owned_to_value(item, session, config, anchors, child_depth)?);
            }
            (
                Data::Seq(Rc::new(YgSeq(values))),
                NodeStyle::Collection(style),
                None,
                None,
            )
        }
        OwnedKind::Mapping { style, entries } => {
            let child_depth = checked_child_depth(depth)?;
            let mut mapping = YgMap::default();
            for (key, value) in entries {
                let key = owned_to_value(key, session, config, anchors, child_depth)?;
                let value = owned_to_value(value, session, config, anchors, child_depth)?;
                mapping.0.insert(key, value);
```

Related to [Query Session Assignment Recurses Without a Bound](#query-session-assignment-recurses-without-a-bound).

**Remediation:** Convert `OwnedNode` to `YamlVal` with an explicit work stack,
or impose an immutable implementation safety ceiling in unchecked mode.
Preserve anchor visibility, mapping order, metadata, and typed depth failures.

### Query Session Assignment Recurses Without a Bound

**significant** · `crates/yamalgam-query/src/resolution.rs:248-268` · effort:
medium

`register_resolved_input` calls `assign_session` without a depth argument or
limit. A deeply nested programmatic value, or a value produced under unchecked
loading, drives unbounded recursion during session annotation. Explicit
resolution repeats the traversal for every returned value.

```rust crates/yamalgam-query/src/resolution.rs:248-268
pub(crate) fn assign_session(value: &mut YamlVal, session: ResolutionSessionKey) {
    if let Some(metadata) = &mut value.meta
        && let Some(origin) = &mut Rc::make_mut(metadata).origin
    {
        origin.session = session;
    }
    match &mut value.data {
        Data::Seq(items) => {
            for item in &mut Rc::make_mut(items).0 {
                assign_session(item, session);
            }
        }
        Data::Map(mapping) => {
            let entries = std::mem::take(&mut Rc::make_mut(mapping).0);
            for (mut key, mut value) in entries {
                assign_session(&mut key, session);
                assign_session(&mut value, session);
                Rc::make_mut(mapping).0.insert(key, value);
            }
        }
        Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_) => {}
```

Related to [Resolved Owned Tree Conversion Remains Recursive](#resolved-owned-tree-conversion-remains-recursive).

**Remediation:** Rewrite session assignment as an explicit work-stack
transformation while preserving `Rc` copy-on-write behavior, mapping keys,
values, and metadata.

*Verdict: The typed runtime and reporting architecture is worth keeping. The
remaining work is mechanical but mandatory: make every deep traversal
stack-safe, every caught error bounded, and every post-session failure retain
its report.*

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---------|---------|----------|--------|--------|
| **Resolution Policy and Graph Integrity** | | | | |
| [relative-includes-can-retrieve-non-file-schemes](#relative-includes-can-retrieve-non-file-schemes) | significant | `crates/yamalgam-resolve/src/session.rs:1277-1292` | small | |
| [explicit-only-sessions-skip-root-anchor-reservation](#explicit-only-sessions-skip-root-anchor-reservation) | significant | `crates/yamalgam-resolve/src/session.rs:99-120` | small | |
| [builtin-resolver-mixes-caller-and-session-configs](#builtin-resolver-mixes-caller-and-session-configs) | significant | `crates/yamalgam-resolve/src/builtin.rs:58-90` | medium | |
| **Session Budgets and Discovery** | | | | |
| [resolution-cache-bypasses-session-byte-budget](#resolution-cache-bypasses-session-byte-budget) | significant | `crates/yamalgam-resolve/src/session.rs:870-899` | medium | |
| [glob-discovery-scans-entire-base](#glob-discovery-scans-entire-base) | moderate | `crates/yamalgam-resolve/src/retriever/file.rs:373-388` | medium | related: aliases |
| [symlink-aliases-consume-match-limit-before-deduplication](#symlink-aliases-consume-match-limit-before-deduplication) | moderate | `crates/yamalgam-resolve/src/retriever/file.rs:179-197` | small | related: glob traversal |
| **Query Runtime and Reporting** | | | | |
| [caught-resolution-errors-accumulate-until-run-end](#caught-resolution-errors-accumulate-until-run-end) | moderate | `crates/yamalgam-query/src/runtime_resolution.rs:142-185` | medium | |
| [post-resolution-conversion-drops-session-report](#post-resolution-conversion-drops-session-report) | moderate | `crates/yamalgam-query/src/resolution.rs:119-137` | medium | |
| [direct-include-preflight-can-omit-the-failed-report-edge](#direct-include-preflight-can-omit-failed-report-edge) | moderate | `crates/yamalgam-resolve/src/session.rs:408-435` | small | |
| [resolved-owned-tree-conversion-remains-recursive](#resolved-owned-tree-conversion-remains-recursive) | significant | `crates/yamalgam-query/src/resolution.rs:323-342` | medium | related: session assignment |
| [query-session-assignment-recurses-without-a-bound](#query-session-assignment-recurses-without-a-bound) | significant | `crates/yamalgam-query/src/resolution.rs:248-268` | medium | related: owned conversion |

<sub>
Generated 2026-08-09 at commit 246f707.
Intermediate artifacts: recon.yaml, findings.yaml.
</sub>
