---
audit_date: 2026-08-09
project: Yamalgam
commit: 56b8b3ec9358dd3cac11a6cd22748d94c981ae59
scope: M11 external-resolution changes from c85f7e0 through 56b8b3e
auditor: Codex
findings:
  critical: 0
  significant: 10
  moderate: 9
  advisory: 1
  note: 0
---

# Audit: M11 External Resource Resolution

M11 establishes the right architecture: Yamalgam owns parsing and resolution,
retrievers are pluggable, filesystem access is capability-rooted, and reports
are structured. **The Resource Identity and Filesystem Retrieval surfaces**
contain policy-dispatch, race, recursion, and determinism gaps. **The YAML
Anchor, Query Runtime, and Reporting surfaces** contain integrity, session,
control-flow, and diagnostic-safety defects. **The Resolution Efficiency and
Dependency Hygiene surfaces** add two scaling costs and one unused dependency.
The design should stand, but the ten significant findings need remediation
before M11 is treated as complete.

## Resource Identity and Policy Dispatch

*Canonical identities are the security hinge between URI resolution and capability-rooted files, but two dispatch paths cross that boundary incorrectly and the public representation does not enforce its own invariant.*

### Scheme Relative Ref Becomes Local File

**significant** · `crates/yamalgam-resolve/src/resource.rs:275-282` · effort: small

RFC 3986 network-path references such as //other.example/data.yaml are
relative references that inherit the containing URL's scheme. Url::parse
without a base rejects that form, then Path::is_absolute accepts it on
Unix. Remotely supplied YAML can therefore move an HTTP ref onto the file
capability instead of the host allowlist path.

```rust crates/yamalgam-resolve/src/resource.rs:275-282
    if let Ok(url) = Url::parse(reference) {
        return ResourceId::from_url(url);
    }

    let path = Path::new(reference);
    if path.is_absolute() {
        return Ok(ResourceId::from_file(path));
    }
```

> **Attacker perspective:** I supply a network-path reference under a URI base. The resolver dispatches it to the filesystem backend before the HTTP policy can evaluate the host.

**Remediation:** Resolve all non-absolute references against a URI base with Url::join
before applying platform path semantics. Interpret absolute paths only
for file bases and cover network-path references under mixed policies.

<div>&hairsp;</div>

### Include URI Bypasses Relative Selector Contract

**moderate** · `crates/yamalgam-resolve/src/include.rs:252-259` · effort: small

Values such as file:///root/data.yaml and memory://fixture/data.yaml
have no leading platform root component and pass directive validation.
Resource resolution subsequently recognizes them as absolute identities,
violating the accepted relative-only include contract.

```rust crates/yamalgam-resolve/src/include.rs:252-259
    if Path::new(value)
        .components()
        .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        return Err(SessionError::MalformedInclude(
            "include selector must be relative",
        ));
    }
```

**Remediation:** Reject every absolute URI reference in path, glob, and directory include
selectors before resource resolution. Retain normal platform-relative
path syntax and add file, memory, and custom-scheme regressions.

<div>&hairsp;</div>

### Resource ID Public Variants Bypass Canonical Invariants

**moderate** · `crates/yamalgam-resolve/src/resource.rs:16-24` · effort: medium

The type promises a normalized, fragmentless identity, but callers can
construct ResourceId::File with dot segments or ResourceId::Uri with a
fragment. Cache keys, cycle keys, and policy checks can then observe two
representations for one resource or accept a fragmented fetch identity.

```rust crates/yamalgam-resolve/src/resource.rs:16-24
#[non_exhaustive]
pub enum ResourceId {
    /// A non-retrievable root with no relative external-resource base.
    Anonymous,
    /// A filesystem resource. Paths retain their platform-native encoding.
    File(PathBuf),
    /// A URL resource, including memory, HTTP, and application-defined schemes.
    Uri(Url),
}
```

**Remediation:** Make the representation private and expose only checked constructors and
accessors. Preserve redacted Display and Debug behavior and add compile-time
and runtime invariant coverage.

<div>&hairsp;</div>

*Verdict: The policy model is sound after an identity is classified. Classification itself needs to be closed: URI bases must use URI-reference rules before path rules, includes must reject every absolute URI form, and callers must not be able to construct noncanonical identities.*

<div>&nbsp;</div>

## Filesystem Retrieval and Discovery

*Capability rooting prevents ambient escape, but pathname reopens, alias policy checks, recursive traversal, and backend ordering still leave correctness and resource-safety gaps.*

### File Policy Checks Race Path Reopen

**significant** · `crates/yamalgam-resolve/src/retriever/file.rs:219-247` · effort: medium

Policy, symlink, type, and size checks occur before root.read reopens the
path. Concurrent replacement can change the checked entry or target; file
growth can also force allocation of the complete oversized body before
TooLarge is returned. Root confinement remains intact, but in-root deny
policy and bounded-memory guarantees are not stable across the reopen.

```rust crates/yamalgam-resolve/src/retriever/file.rs:219-247
        let metadata = self
            .root
            .metadata(&canonical)
            .map_err(|source| map_io(id.clone(), source))?;
        if !metadata.is_file() {
            return Err(RetrieveError::NotRegularFile(id));
        }
        if let Some(maximum) = request.max_bytes
            && metadata.len() > maximum as u64
        {
            return Err(RetrieveError::TooLarge {
                id,
                observed: usize::try_from(metadata.len()).unwrap_or(usize::MAX),
                maximum,
            });
        }
        let bytes = self
            .root
            .read(&canonical)
            .map_err(|source| map_io(id.clone(), source))?;
        if let Some(maximum) = request.max_bytes
            && bytes.len() > maximum
        {
            return Err(RetrieveError::TooLarge {
                id,
                observed: bytes.len(),
                maximum,
            });
        }
```

> **Attacker perspective:** I replace an allowed in-root pathname after validation but before the second open, or grow the file after its size check. The resolver consumes content that was never validated as the same object.

**Remediation:** Open one stable handle with the configured symlink behavior, validate the
opened file, and read through a maximum-plus-one bounded reader. Avoid a
second pathname open after validation.

<div>&hairsp;</div>

### Symlink Discovery Skips Alias Policy

**significant** · `crates/yamalgam-resolve/src/retriever/file.rs:181-191` · effort: small

With symlink following enabled, a denied alias path can canonicalize into
an allowed target. Direct retrieval checks both forms, but discovery checks
only the canonical result, so deny rules such as alias/** can be bypassed.

```rust crates/yamalgam-resolve/src/retriever/file.rs:181-191
                let canonical = self.canonical_existing(&scan_path)?;
                let metadata = self
                    .root
                    .metadata(&canonical)
                    .map_err(|source| map_io(self.id_for(&canonical), source))?;
                if metadata.is_dir() {
                    if state.recursive {
                        self.walk(&canonical, &match_path, state)?;
                    }
                } else if metadata.is_file() && (state.matcher)(&match_path, &name) {
                    self.record_match(canonical, state.maximum, state.matches)?;
```

> **Attacker perspective:** I place a denied alias in an allowed discovery tree and point it at an allowed canonical target. Discovery evaluates the target policy and forgets the denied name used to reach it.

**Remediation:** Apply deny-before-allow policy to both requested alias paths and canonical
targets for symlinked files and directories.

<div>&hairsp;</div>

### Recursive File Discovery Has No Depth Bound

**moderate** · `crates/yamalgam-resolve/src/retriever/file.rs:181-200` · effort: medium

Glob discovery always recurses and directory mode may recurse once per
filesystem level. The edge ceiling counts matched files rather than visited
directories, so a deeply nested tree with no matches can bypass that budget
and overflow the process stack.

```rust crates/yamalgam-resolve/src/retriever/file.rs:181-200
                let canonical = self.canonical_existing(&scan_path)?;
                let metadata = self
                    .root
                    .metadata(&canonical)
                    .map_err(|source| map_io(self.id_for(&canonical), source))?;
                if metadata.is_dir() {
                    if state.recursive {
                        self.walk(&canonical, &match_path, state)?;
                    }
                } else if metadata.is_file() && (state.matcher)(&match_path, &name) {
                    self.record_match(canonical, state.maximum, state.matches)?;
                } else if (state.matcher)(&match_path, &name) {
                    return Err(RetrieveError::NotRegularFile(self.id_for(&canonical)));
                }
                continue;
            }

            if file_type.is_dir() {
                if state.recursive {
                    self.walk(&scan_path, &match_path, state)?;
```

**Remediation:** Replace recursive descent with an explicit work stack while preserving
deterministic match paths and symlink cycle detection.

<div>&hairsp;</div>

### Symlink Aliases Make Glob Results Enumeration Dependent

**moderate** · `crates/yamalgam-resolve/src/retriever/file.rs:158-188` · effort: medium

Two aliases may point to one canonical directory while only one alias path
matches the glob. The first read_dir entry marks the canonical path visited;
the other alias is skipped. Final-result sorting cannot restore omitted
traversal, so results vary with filesystem enumeration order.

```rust crates/yamalgam-resolve/src/retriever/file.rs:158-188
        if !state.visited.insert(scan_dir.to_path_buf()) {
            return Ok(());
        }
        let entries = self
            .root
            .read_dir(capability_path(scan_dir))
            .map_err(|source| map_io(self.id_for(scan_dir), source))?;
        for entry in entries {
            let entry = entry.map_err(|source| map_io(self.id_for(scan_dir), source))?;
            let name = entry.file_name();
            let scan_path = scan_dir.join(&name);
            let match_path = match_dir.join(&name);
            let file_type = entry
                .file_type()
                .map_err(|source| map_io(self.id_for(&scan_path), source))?;

            if file_type.is_symlink() {
                if !self.follow_symlinks {
                    if (state.matcher)(&match_path, &name) {
                        return Err(RetrieveError::SymlinkDenied(self.id_for(&scan_path)));
                    }
                    continue;
                }
                let canonical = self.canonical_existing(&scan_path)?;
                let metadata = self
                    .root
                    .metadata(&canonical)
                    .map_err(|source| map_io(self.id_for(&canonical), source))?;
                if metadata.is_dir() {
                    if state.recursive {
                        self.walk(&canonical, &match_path, state)?;
```

**Remediation:** Sort entries before traversal and separate active-cycle detection from
alias-specific match traversal so each distinct match path is evaluated.

<div>&hairsp;</div>

### Discovery Results Do Not Enforce Deterministic Order

**moderate** · `crates/yamalgam-resolve/src/session.rs:575-590` · effort: small

FileRetriever sorts and deduplicates, but ResourceRetriever is pluggable and
the shared session accepts any returned Vec. A custom backend can therefore
make list, named, merge, and report order nondeterministic or repeat one
identity, contradicting the resolver-wide deterministic contract.

```rust crates/yamalgam-resolve/src/session.rs:575-590
        self.retriever
            .discover(&DiscoveryRequest {
                base: directive.origin.clone(),
                selector,
                max_matches,
            })
            .map_err(|source| {
                self.error(
                    retrieve_category(&source),
                    ResolutionCapability::Include,
                    directive.span,
                    directive.origin.to_string(),
                    Some(Box::new(source)),
                )
            })
    }
```

**Remediation:** Normalize discovery results at the shared session boundary with a stable
ResourceId ordering and deduplication, while keeping backend-specific path
ordering equivalent to normalized root-relative bytes.

<div>&hairsp;</div>

*Verdict: The filesystem backend needs to become descriptor-centered and iterative. Requested and canonical paths must both pass policy, traversal order must be fixed before descent, alias-specific paths must not be collapsed, and shared-session discovery must normalize custom retrievers.*

<div>&nbsp;</div>

## YAML Anchor and Alias Integrity

*Imported fragments are renamed, but validation ignores YAML event order and prefix allocation ignores anchors already present in the output document.*

### Imported Anchor Prefix Can Collide With Root

**significant** · `crates/yamalgam-resolve/src/session.rs:1055-1056` · effort: medium

A root anchor can already have the generated name for an imported anchor.
YAML aliases bind to the most recent preceding definition, so an alias after
substitution may silently switch from the root anchor to the imported node.

```rust crates/yamalgam-resolve/src/session.rs:1055-1056
        let prefix = format!("yg_r{resource_ordinal}_e{edge_ordinal}_");
        anchors::namespace(&mut selected, &prefix).map_err(|source| {
```

**Remediation:** Inventory every anchor in the complete output root and prior imports. Select
a deterministic prefix whose rewritten names do not intersect that set, then
reserve the imported names.

<div>&hairsp;</div>

### Fragment Self Containment Ignores Anchor Binding Order

**moderate** · `crates/yamalgam-resolve/src/anchors.rs:8-33` · effort: small

YAML aliases bind to a preceding anchor, not any same-named anchor anywhere
in the subtree. A fragment beginning with *a and defining &a later is accepted
even though the selected alias depended on an external definition.

```rust crates/yamalgam-resolve/src/anchors.rs:8-33
pub fn namespace(node: &mut OwnedNode, prefix: &str) -> Result<(), SessionError> {
    let mut defined = HashSet::new();
    let mut used = HashSet::new();
    let mut pending = vec![&*node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &node.anchor {
            defined.insert(anchor.clone());
        }
        match &node.kind {
            OwnedKind::Alias { name } => {
                used.insert(name.clone());
            }
            OwnedKind::Sequence { items, .. } => pending.extend(items),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }

    if let Some(external) = used.iter().filter(|name| !defined.contains(*name)).min() {
        return Err(SessionError::NonSelfContainedAlias(external.clone()));
    }
```

**Remediation:** Traverse in emitted YAML event order, tracking anchors visible at each alias.
Reject a use without an earlier in-subtree definition while preserving legal
redefinition behavior.

<div>&hairsp;</div>

*Verdict: Anchor isolation needs document-wide state. Validate aliases in event order, reserve every existing output anchor, and choose deterministic collision-free import prefixes before rewriting a selected subtree.*

<div>&nbsp;</div>

## Session Identity and Query Runtime

*The query bridge carries typed resolver state through jaq, but run-local keys, slurp construction, caught errors, and recursive conversion do not preserve the promised session and control-flow semantics.*

### Run Local Resolution Keys Collide Across Contexts

**significant** · `crates/yamalgam-query/src/engine.rs:142-153` · effort: small

ResourceOrigin contains only the numeric key, not context identity. A YamlVal
retained from context A can therefore select an unrelated resolver registered
under the same key in context B, resolving against the wrong base and policy.

```rust crates/yamalgam-query/src/engine.rs:142-153
    pub fn register_resolved_input(
        &self,
        mut input: ResolvedInput,
    ) -> Result<(YamlVal, ResolutionSessionKey), YgError> {
        let raw = self.next_resolution_key.get();
        let next = raw.checked_add(1).ok_or_else(|| {
            YgError::resource_overflow("resolution session key", "resolution session key overflow")
        })?;
        let key = ResolutionSessionKey(raw);
        crate::resolution::assign_session(&mut input.value, key);
        self.resolutions.borrow_mut().insert(key, input.resolution);
        self.next_resolution_key.set(next);
```

**Remediation:** Allocate nonreused process-wide session keys with checked overflow, or include
a generative context identity in the opaque key. Add a cross-context value
transfer regression.

<div>&hairsp;</div>

### Slurp Drops Resolution Base

**moderate** · `crates/yamalgam/src/commands/query.rs:605-625` · effort: small

Source nodes retain their origins, but a filter that constructs a new $ref or
!include from a slurped input must fall back to the active input session. The
aggregate record clears that session even when --base-uri supplies the intended
base, producing MissingBase.

```rust crates/yamalgam/src/commands/query.rs:605-625
    if args.slurp {
        let values = records
            .into_iter()
            .map(|record| record.value)
            .collect::<Result<Vec<_>, _>>()?;
        let value = if args.raw_input {
            YamlVal::str(
                values
                    .iter()
                    .filter_map(YamlVal::as_str_content)
                    .collect::<String>(),
            )
        } else {
            YamlVal::seq(values)
        };
        return Ok(vec![InputRecord {
            metadata: InputMeta::default(),
            value: Ok(value),
            resolution: None,
        }]);
    }
```

**Remediation:** Register the slurped aggregate with one constructed-value resolution session
derived from the explicit base and active policy. Preserve per-node source
origins inside the sequence.

<div>&hairsp;</div>

### Caught Resolution Error Remains Terminal

**moderate** · `crates/yamalgam-query/src/runtime_resolution.rs:140-149` · effort: medium

try ref catch "fallback" lets jaq consume the native error and produce a
fallback, but jaq_error has already stored ResolveError in thread-local state.
The engine observes the successful output, drains the slot, and returns
RuntimeResolution anyway, diverging from specified jq try/catch behavior.

```rust crates/yamalgam-query/src/runtime_resolution.rs:140-149
pub fn jaq_error(error: ResolveError) -> jaq_core::Error<YamlVal> {
    let message = error.to_string();
    PENDING_ERROR.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending.is_none() {
            *pending = Some(error);
        }
    });
    jaq_core::Error::str(format_args!("external resolution failed: {message}"))
}
```

**Remediation:** Attach a typed token to the jaq error value and recover the ResolveError only
when that error propagates out of jaq. Clear tokens for errors consumed by catch.

<div>&hairsp;</div>

### Query Resolution Conversion Recurses Without Configured Bound

**significant** · `crates/yamalgam-query/src/resolution.rs:388-411` · effort: medium

User-constructed YamlVal depth drives Rust recursion before the session's
iterative traversal. LoaderConfig depth limits are not consulted, and unchecked
mode has no implementation safety ceiling, so sufficiently deep values can
overflow the process stack instead of returning a typed limit error.

```rust crates/yamalgam-query/src/resolution.rs:388-411
        Data::Seq(values) => {
            let child_depth = checked_resolution_depth(depth, capability)?;
            let items = values
                .iter()
                .cloned()
                .map(|value| value_to_owned(value, fallback, capability, child_depth))
                .collect::<Result<Vec<_>, _>>()?;
            OwnedKind::Sequence {
                style: collection_style(metadata.style),
                items,
            }
        }
        Data::Map(mapping) => {
            let child_depth = checked_resolution_depth(depth, capability)?;
            let entries = mapping
                .0
                .iter()
                .map(|(key, value)| {
                    Ok((
                        value_to_owned(key.clone(), fallback, capability, child_depth)?,
                        value_to_owned(value.clone(), fallback, capability, child_depth)?,
                    ))
                })
                .collect::<Result<Vec<_>, ResolveError>>()?;
```

**Remediation:** Convert query values iteratively, or enforce the active configured depth before
every recursive descent plus a nonconfigurable implementation safety ceiling.
Preserve structured limit and overflow categories.

<div>&hairsp;</div>

*Verdict: Session identity must be globally unambiguous, constructed slurp values need an explicit retained session, typed error payloads must follow jaq propagation rather than a run-wide flag, and query-to-owned conversion must enforce real depth safety.*

<div>&nbsp;</div>

## Resolution Reporting

*The report model contains the right fields, but convenience APIs discard failure reports, raw request text can expose secrets, and collection match sets are never populated.*

### Automatic Resolution Failure Discards Report

**significant** · `crates/yamalgam-query/src/resolution.rs:111-115` · effort: medium

ResolutionSession retains failure edges internally, but resolve_document and
resolve_documents return only YgError. The local resolver is dropped on failure;
a later document failure also drops reports from already completed top-level
documents, so callers cannot satisfy the public success-and-failure contract.

```rust crates/yamalgam-query/src/resolution.rs:111-115
    let mut resolver = SessionQueryResolver::new(retriever, config.clone(), Some(root));
    let document = resolver.resolve_input(document).map_err(|error| {
        let message = error.to_string();
        YgError::input_resolution(error, message)
    })?;
```

**Remediation:** Carry the failed session report, and prior completed-document reports for stream
loading, in the structured InputResolution error without changing Display text.

<div>&hairsp;</div>

### Resolution Report Retains Raw Secret Reference

**significant** · `crates/yamalgam-resolve/src/report.rs:118-134` · effort: small

ResourceId Display and Debug redact userinfo and query values, but
ResolutionEdge.requested exposes the original directive verbatim. Limit and
overflow errors also copy the raw request into ResolveError.target. Allowed query
tokens and denied userinfo therefore survive in public structured diagnostics.

```rust crates/yamalgam-resolve/src/report.rs:118-134
/// One attempted parent-to-child resolution edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionEdge {
    /// Containing resource.
    pub parent: ResourceId,
    /// Resolved child identity, absent for failures before identity creation.
    pub child: Option<ResourceId>,
    /// Include or reference capability.
    pub capability: ResolutionCapability,
    /// Directive location in the containing resource.
    pub source_span: Span,
    /// Original directive reference retained as structured data.
    pub requested: String,
    /// Selected document fragment.
    pub fragment: Fragment,
    /// Deterministically ordered collection matches.
    pub matches: Vec<ResourceId>,
```

> **Attacker perspective:** I cause resolution to fail after a credential-bearing URI is requested. The public report or error can then preserve that credential in logs and diagnostics.

**Remediation:** Store a sanitized public reference that removes userinfo and query values before
recording and before using it as an error target. Keep raw text private only if
internal diagnostics require it and make every public Debug path redaction-safe.

<div>&hairsp;</div>

### Collection Report Never Records Matches

**moderate** · `crates/yamalgam-resolve/src/session.rs:599-611` · effort: medium

The model promises ordered collection matches, but the session creates only
per-resource fetch edges and initializes every matches field empty. Callers cannot
reconstruct which files belonged to one directive or attribute aggregate failures.

```rust crates/yamalgam-resolve/src/session.rs:599-611
        let edge_index = self.report.record_edge(ResolutionEdge {
            parent: directive.origin.clone(),
            child: Some(resource.clone()),
            capability: ResolutionCapability::Include,
            source_span: directive.span,
            requested: requested.clone(),
            fragment: Fragment::Root,
            matches: Vec::new(),
            cache_hit: false,
            fetched_bytes: 0,
            materialized_bytes: 0,
            outcome: ResolutionOutcome::Resolved,
        });
```

**Remediation:** Record one directive-level collection edge containing the normalized match set and
terminal outcome, then retain per-resource fetch/materialization edges beneath it.

<div>&hairsp;</div>

*Verdict: Reports need to be treated as first-class outputs on every path. Failure errors should carry accumulated reports, public requested references must be sanitized before storage, and collection directives need their own match-bearing edge.*

<div>&nbsp;</div>

## Resolution Efficiency

*Two implementation choices amplify memory or CPU with document size: source identities clone owned buffers per node and mapping merges scan all prior keys.*

### Owned Nodes Clone Resource Identities Per Node

**significant** · `crates/yamalgam-resolve/src/document.rs:204-208` · effort: medium

ResourceId owns PathBuf or Url directly. Parsing clones it into every OwnedNode,
document construction clones it into every node and subtree selection clones it
again. Query-to-owned conversion also clones metadata origins. Large documents
therefore allocate and copy the same identity proportional to node count before
payload work begins; owned-to-query conversion moves those identities.

```rust crates/yamalgam-resolve/src/document.rs:204-208
                    attach_node(
                        OwnedNode {
                            origin: origin.clone(),
                            span,
                            anchor: cow_option(anchor),
```

**Remediation:** Give ResourceId a private shared immutable representation so ordinary Clone is
constant-time while preserving equality, ordering, hashing, and redaction.

<div>&hairsp;</div>

### Include Mapping Merge Is Quadratic

**significant** · `crates/yamalgam-resolve/src/include.rs:498-511` · effort: medium

Merging N distinct keys performs O(N squared) semantic comparisons. Scalar
comparisons also allocate resolved Value strings, and explicit keys repeat the
same full-vector scan through retain. Large generated mappings can consume
disproportionate CPU within otherwise valid byte and edge budgets.

```rust crates/yamalgam-resolve/src/include.rs:498-511
                if let Some(position) = merged
                    .iter()
                    .position(|(key, _)| semantic_key_eq(key, &entry.0, tag_resolution))
                {
                    match spec.on_conflict {
                        ConflictPolicy::Error => return Err(SessionError::IncludeConflict),
                        ConflictPolicy::First => {}
                        ConflictPolicy::Last => merged[position] = entry,
                    }
                } else {
                    merged.push(entry);
                }
            }
        }
```

**Remediation:** Add an equality-safe semantic-key fingerprint index that maps hashes to candidate
positions, verifies collisions with semantic_key_eq, and preserves insertion and
conflict-policy order.

<div>&hairsp;</div>

*Verdict: These are not architectural defects. A private shared ResourceId representation and an equality-safe semantic-key index remove the scaling costs without changing behavior.*

<div>&nbsp;</div>

## Dependency Hygiene

*The new resolver crate has one direct workspace dependency that is not used.*

### Resolver Directly Depends On Unused Scanner

**advisory** · `crates/yamalgam-resolve/Cargo.toml:21` · effort: trivial

yamalgam-resolve imports parser types but no scanner symbols. The extra direct
edge overstates coupling and expands feature and dependency maintenance work.

```toml crates/yamalgam-resolve/Cargo.toml:21
yamalgam-scanner = { path = "../yamalgam-scanner" }
```

**Remediation:** Remove the direct dependency and verify the resolver crate plus workspace feature
matrix still build through the parser's legitimate scanner edge.

<div>&hairsp;</div>

*Verdict: The scanner is correctly reached through the parser and query layers; yamalgam-resolve should not declare it directly.*

<div>&nbsp;</div>

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---------|---------|----------|--------|--------|
| **Resource Identity and Policy Dispatch** | | | | |
| [scheme-relative-ref-becomes-local-file](#scheme-relative-ref-becomes-local-file) | significant | `crates/yamalgam-resolve/src/resource.rs:275-282` | small |  |
| [include-uri-bypasses-relative-selector-contract](#include-uri-bypasses-relative-selector-contract) | moderate | `crates/yamalgam-resolve/src/include.rs:252-259` | small |  |
| [resource-id-public-variants-bypass-canonical-invariants](#resource-id-public-variants-bypass-canonical-invariants) | moderate | `crates/yamalgam-resolve/src/resource.rs:16-24` | medium |  |
| **Filesystem Retrieval and Discovery** | | | | |
| [file-policy-checks-race-path-reopen](#file-policy-checks-race-path-reopen) | significant | `crates/yamalgam-resolve/src/retriever/file.rs:219-247` | medium |  |
| [symlink-discovery-skips-alias-policy](#symlink-discovery-skips-alias-policy) | significant | `crates/yamalgam-resolve/src/retriever/file.rs:181-191` | small |  |
| [recursive-file-discovery-has-no-depth-bound](#recursive-file-discovery-has-no-depth-bound) | moderate | `crates/yamalgam-resolve/src/retriever/file.rs:181-200` | medium |  |
| [symlink-aliases-make-glob-results-enumeration-dependent](#symlink-aliases-make-glob-results-enumeration-dependent) | moderate | `crates/yamalgam-resolve/src/retriever/file.rs:158-188` | medium |  |
| [discovery-results-do-not-enforce-deterministic-order](#discovery-results-do-not-enforce-deterministic-order) | moderate | `crates/yamalgam-resolve/src/session.rs:575-590` | small |  |
| **YAML Anchor and Alias Integrity** | | | | |
| [imported-anchor-prefix-can-collide-with-root](#imported-anchor-prefix-can-collide-with-root) | significant | `crates/yamalgam-resolve/src/session.rs:1055-1056` | medium |  |
| [fragment-self-containment-ignores-anchor-binding-order](#fragment-self-containment-ignores-anchor-binding-order) | moderate | `crates/yamalgam-resolve/src/anchors.rs:8-33` | small |  |
| **Session Identity and Query Runtime** | | | | |
| [run-local-resolution-keys-collide-across-contexts](#run-local-resolution-keys-collide-across-contexts) | significant | `crates/yamalgam-query/src/engine.rs:142-153` | small |  |
| [slurp-drops-resolution-base](#slurp-drops-resolution-base) | moderate | `crates/yamalgam/src/commands/query.rs:605-625` | small |  |
| [caught-resolution-error-remains-terminal](#caught-resolution-error-remains-terminal) | moderate | `crates/yamalgam-query/src/runtime_resolution.rs:140-149` | medium |  |
| [query-resolution-conversion-recurses-without-configured-bound](#query-resolution-conversion-recurses-without-configured-bound) | significant | `crates/yamalgam-query/src/resolution.rs:388-411` | medium |  |
| **Resolution Reporting** | | | | |
| [automatic-resolution-failure-discards-report](#automatic-resolution-failure-discards-report) | significant | `crates/yamalgam-query/src/resolution.rs:111-115` | medium |  |
| [resolution-report-retains-raw-secret-reference](#resolution-report-retains-raw-secret-reference) | significant | `crates/yamalgam-resolve/src/report.rs:118-134` | small |  |
| [collection-report-never-records-matches](#collection-report-never-records-matches) | moderate | `crates/yamalgam-resolve/src/session.rs:599-611` | medium |  |
| **Resolution Efficiency** | | | | |
| [owned-nodes-clone-resource-identities-per-node](#owned-nodes-clone-resource-identities-per-node) | significant | `crates/yamalgam-resolve/src/document.rs:204-208` | medium |  |
| [include-mapping-merge-is-quadratic](#include-mapping-merge-is-quadratic) | significant | `crates/yamalgam-resolve/src/include.rs:498-511` | medium |  |
| **Dependency Hygiene** | | | | |
| [resolver-directly-depends-on-unused-scanner](#resolver-directly-depends-on-unused-scanner) | advisory | `crates/yamalgam-resolve/Cargo.toml:21` | trivial |  |

<sub>
Generated 2026-08-09 at commit 56b8b3e.
Intermediate artifacts: recon.yaml, findings.yaml.
</sub>
