# Bounded Composer Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close `composer-default-path-has-unbounded-recursion` by making
ordinary composer APIs use moderate limits and enforcing depth and collection
limits inside the composer.

**Architecture:** Keep `Parser::new` and `Scanner::new` unchanged. High-level
composer entry points and `Composer::new` select `LoaderConfig::default()`,
while new `*_unchecked` APIs preserve the former unlimited behavior. The
composer checks depth and collection counts locally so caller-supplied event
streams cannot bypass the policy.

**Tech Stack:** Rust 2024, `yamalgam-core::LoaderConfig`,
`yamalgam-parser`, `cargo nextest`, `just`.

## Global Constraints

- Work directly on branch `m10-yg-cli`; do not create a worktree.
- Add no dependency and do not change any project version.
- Keep `Scanner::new` and `Parser::new` unlimited for compatibility.
- Preserve the ordered `Value` and `Mapping` public models.
- Use `ComposeError::LimitExceeded` for composer-local limit failures.
- Follow strict red-green-refactor: no production edit before its focused test
  fails for the expected reason.
- Do not edit the source audit artifacts. Only append to
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md` after the
  implementation commit exists.
- Stage only named files. Leave the six untracked files under
  `record/audits/2026-08-03-21-m10-fresh/` untouched.

## File map

- Modify `crates/yamalgam-compose/src/lib.rs`: constructor policy, explicit
  unchecked APIs, local depth tracking, collection checks, and unit tests.
- Create
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`: first
  append-only ledger entry after the code commit.

---

### Task 1: Bound composer loading

**Files:**

- Modify: `crates/yamalgam-compose/src/lib.rs`
- Test: `crates/yamalgam-compose/src/lib.rs`

**Interfaces:**

- Consumes: `LoaderConfig::default()`, `LoaderConfig::unchecked()`,
  `ResourceLimits::check_depth`, and `ResourceLimits::check_collection_size`.
- Produces: `Composer::from_str_unchecked`,
  `Composer::from_str_with_tag_resolver_unchecked`,
  `Composer::with_resolver_unchecked`, `Composer::new_unchecked`,
  `from_str_unchecked`, and `from_str_single_unchecked`.
- Preserves: `from_str_with_config`, `from_str_single_with_config`, and
  `Composer::new_with_config` for caller-selected limits.

**Steps:**

- [ ] **Step 1: Add the safe-default regression test**

Add these helpers and tests to the existing `tests` module:

```rust
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

#[test]
fn ordinary_string_entry_points_use_moderate_limits() {
    let input = nested_flow_sequence(257);

    assert_limit_error(Composer::from_str(&input));
    assert_limit_error(Composer::with_resolver(&input, NoopResolver));
    assert_limit_error(Composer::from_str_with_tag_resolver(
        &input,
        Yaml12TagResolver,
    ));

    let parser = Parser::new(&input);
    let events = ResolvedEvents::new(
        Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
        NoopResolver,
    );
    assert_limit_error(Composer::new(events).compose_stream());
}

```

- [ ] **Step 2: Run the constructor tests and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-compose -E \
  'test(ordinary_string_entry_points_use_moderate_limits)'
```

Expected: FAIL because the current ordinary APIs accept depth 257.

- [ ] **Step 3: Implement the safe constructor policy**

Import `LoaderConfig` with the existing core imports:

```rust
use yamalgam_core::{LoaderConfig, Mapping, ResourceLimits, Value};
```

Update the existing rustdoc for `Composer::from_str`,
`Composer::from_str_with_tag_resolver`, `Composer::with_resolver`, and
`Composer::new` to state that each uses `LoaderConfig::default()`. Remove the
stale `Composer::new` claim that it uses no resource limits.

Make ordinary constructors select the default profile. Preserve the former
paths under explicit names:

```rust
pub fn from_str(input: &'input str) -> Result<Vec<Value>, ComposeError> {
    Self::with_resolver(input, NoopResolver)
}

pub fn from_str_with_tag_resolver(
    input: &'input str,
    tag_resolver: impl TagResolver + 'static,
) -> Result<Vec<Value>, ComposeError> {
    Self::from_str_with_tag_resolver_and_config(
        input,
        tag_resolver,
        &LoaderConfig::default(),
    )
}

```

The private tag-resolver helper must configure both parser and composer, then
replace only the composer's tag resolver:

```rust
fn from_str_with_tag_resolver_and_config(
    input: &'input str,
    tag_resolver: impl TagResolver + 'static,
    config: &LoaderConfig,
) -> Result<Vec<Value>, ComposeError> {
    let parser = Parser::with_config(input, config);
    let events = ResolvedEvents::new(
        Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
        NoopResolver,
    );
    let mut composer = Composer::new_with_config(events, config);
    composer.tag_resolver = Box::new(tag_resolver);
    composer.compose_stream()
}
```

Use one config-aware implementation for custom event resolvers:

```rust
pub fn with_resolver(input: &'input str, resolver: R) -> Result<Vec<Value>, ComposeError> {
    Self::with_resolver_and_config(input, resolver, &LoaderConfig::default())
}

fn with_resolver_and_config(
    input: &'input str,
    resolver: R,
    config: &LoaderConfig,
) -> Result<Vec<Value>, ComposeError> {
    let parser = Parser::with_config(input, config);
    let events = ResolvedEvents::new(
        Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
        resolver,
    );
    let mut composer = Composer::new_with_config(events, config);
    composer.compose_stream()
}
```

Make arbitrary-event construction moderate by default:

```rust
pub fn new(events: I) -> Self {
    Self::new_with_config(events, &LoaderConfig::default())
}

```

Do not add unchecked APIs in this cycle; their test comes next.

- [ ] **Step 4: Run the safe-default test and verify GREEN**

Run the Step 2 command again.

Expected: the ordinary entry points return a structured resource-limit error.

- [ ] **Step 5: Add the explicit-unchecked regression test**

Add the second test only after the safe-default test is green:

```rust
#[test]
fn unchecked_string_entry_points_preserve_unlimited_behavior() {
    let input = nested_flow_sequence(257);

    assert!(Composer::from_str_unchecked(&input).is_ok());
    assert!(Composer::with_resolver_unchecked(&input, NoopResolver).is_ok());
    assert!(
        Composer::from_str_with_tag_resolver_unchecked(&input, Yaml12TagResolver).is_ok()
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
```

- [ ] **Step 6: Run the unchecked test and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-compose \
  -E 'test(unchecked_string_entry_points_preserve_unlimited_behavior)'
```

Expected: compilation fails because the explicit unchecked APIs do not exist.

- [ ] **Step 7: Implement the explicit unchecked APIs**

Add associated unchecked constructors beside their safe counterparts:

```rust
/// Parse and compose all documents with no resource limits.
///
/// Use only for trusted input. Prefer [`Self::from_str`] for ordinary input.
pub fn from_str_unchecked(input: &'input str) -> Result<Vec<Value>, ComposeError> {
    Self::with_resolver_unchecked(input, NoopResolver)
}

/// Parse and compose all documents with a custom tag resolver and no limits.
///
/// Use only for trusted input. Prefer [`Self::from_str_with_tag_resolver`]
/// for ordinary input.
pub fn from_str_with_tag_resolver_unchecked(
    input: &'input str,
    tag_resolver: impl TagResolver + 'static,
) -> Result<Vec<Value>, ComposeError> {
    Self::from_str_with_tag_resolver_and_config(
        input,
        tag_resolver,
        &LoaderConfig::unchecked(),
    )
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

/// Create a composer from an event iterator with no resource limits.
///
/// Use only for trusted events. Prefer [`Self::new`] for ordinary input.
pub fn new_unchecked(events: I) -> Self {
    Self::new_with_config(events, &LoaderConfig::unchecked())
}
```

Add free unchecked helpers beside the existing free functions:

```rust
/// Parse a YAML string into documents with no resource limits.
///
/// Use only for trusted input. Prefer [`from_str`] for ordinary input.
pub fn from_str_unchecked(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str_unchecked(input)
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
```

- [ ] **Step 8: Run both constructor tests and verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-compose -E \
  'test(ordinary_string_entry_points_use_moderate_limits) + test(unchecked_string_entry_points_preserve_unlimited_behavior)'
```

Expected: both tests pass. The default paths return a structured limit error;
all explicit unchecked paths accept the same input.

- [ ] **Step 9: Add a failing composer-local depth test**

Add a test that deliberately uses an unchecked upstream parser:

```rust
#[test]
fn composer_enforces_depth_for_unchecked_event_sources() {
    let input = "[[0]]";
    let config = LoaderConfig {
        limits: ResourceLimits {
            max_depth: Some(1),
            ..ResourceLimits::none()
        },
        ..LoaderConfig::unchecked()
    };
    let parser = Parser::new(input);
    let events = ResolvedEvents::new(
        Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
        NoopResolver,
    );
    let mut composer = Composer::new_with_config(events, &config);

    assert!(matches!(
        composer.compose_stream(),
        Err(ComposeError::LimitExceeded(_))
    ));
}
```

- [ ] **Step 10: Run the local depth test and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-compose \
  -E 'test(composer_enforces_depth_for_unchecked_event_sources)'
```

Expected: FAIL because `Composer::new_with_config` currently trusts the
upstream parser and composes `[[0]]` successfully.

- [ ] **Step 11: Thread checked depth through composition**

Change the root calls to `self.compose_node(0)`. Change the recursive method to
accept and validate depth:

```rust
fn compose_node(&mut self, depth: usize) -> Result<Value, ComposeError> {
    self.config
        .check_depth(depth)
        .map_err(ComposeError::LimitExceeded)?;
    let event = self
        .next_event()?
        .ok_or_else(|| ComposeError::UnexpectedEvent("unexpected end of events".into()))?;
    // existing match follows
}
```

Before each sequence item or mapping key/value descent, derive child depth with
checked arithmetic:

```rust
fn child_depth(depth: usize) -> Result<usize, ComposeError> {
    depth
        .checked_add(1)
        .ok_or_else(|| ComposeError::LimitExceeded("depth overflow".into()))
}
```

Call `self.compose_node(child_depth(depth)?)` for sequence items, mapping keys,
and mapping values. Do not compute child depth for an empty collection.

- [ ] **Step 12: Run the local depth and constructor tests and verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-compose -E \
  'test(composer_enforces_depth_for_unchecked_event_sources) + test(ordinary_string_entry_points_use_moderate_limits) + test(unchecked_string_entry_points_preserve_unlimited_behavior)'
```

Expected: all three tests pass.

- [ ] **Step 13: Add failing sequence, mapping, and merge-size tests**

Add a test helper that supplies an unchecked parser to a configured composer:

```rust
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
```

- [ ] **Step 14: Run the collection tests and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-compose -E \
  'test(composer_enforces_sequence_size_for_unchecked_event_sources) + test(composer_counts_duplicate_mapping_entries_toward_the_limit) + test(composer_limits_materialized_merge_entries)'
```

Expected: all three tests fail because the composer does not call
`check_collection_size`.

- [ ] **Step 15: Enforce collection sizes before unbounded accumulation**

Add a helper that checks arithmetic overflow and the configured cap:

```rust
fn next_collection_size(&self, current: usize) -> Result<usize, ComposeError> {
    let next = current
        .checked_add(1)
        .ok_or_else(|| ComposeError::LimitExceeded("collection size overflow".into()))?;
    self.config
        .check_collection_size(next)
        .map_err(ComposeError::LimitExceeded)?;
    Ok(next)
}
```

For a sequence, call `self.next_collection_size(items.len())?` once immediately
before composing each item. For a mapping, maintain a raw `pair_count` and
assign `pair_count = self.next_collection_size(pair_count)?` once per pair,
immediately before composing its key. The subsequent value belongs to the same
pair and must not increment the counter. This makes duplicate keys consume the
syntactic entry budget without an off-by-one allowance.

In `collect_merge_pairs`, use checked addition plus
`config.check_collection_size(next)` before every `pairs.push`. While building
the final merged `Mapping`, call `check_collection_size(merged.len())` after
each insertion. This covers combined merge sources even when each source map
is individually below the cap.

- [ ] **Step 16: Run all focused composer tests and verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-compose
```

Expected: all `yamalgam-compose` tests pass.

- [ ] **Step 17: Format and run the reviewer gates**

Run:

```bash
just fmt
cargo nextest run -p yamalgam-compose
just clippy
git --no-pager diff --check
```

Expected: formatting is stable, the complete composer suite passes, clippy
passes with warnings denied, and Git reports no whitespace errors.

- [ ] **Step 18: Commit the implementation**

Stage only the composer source and inspect the staged patch:

```bash
git add crates/yamalgam-compose/src/lib.rs
git --no-pager diff --cached --check
git --no-pager diff --cached --name-only
git commit -m "fix(compose): bound default document loading"
```

Run:

```bash
git --no-pager rev-parse HEAD
git --no-pager log --show-signature -1 --format=fuller
```

Record the 40-character implementation SHA. The signature output must contain
`Good signature` before writing the ledger entry.

---

### Task 2: Record the completed audit action

**Files:**

- Create:
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`

**Interfaces:**

- Consumes: the real full SHA from Task 1.
- Produces: the first append-only ledger entry for the 27-finding audit.

**Steps:**

- [ ] **Step 1: Capture the implementation SHA in this shell**

Run this at the start of Task 2 even if Task 1 ran in the same shell:

```bash
implementation_sha="$(git rev-parse HEAD)"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: `implementation_sha` is the 40-character Task 1 code commit and the
signature output contains `Good signature`.

- [ ] **Step 2: Create the ledger with the implementation SHA**

Create the file only after Task 1 commits successfully. Use this exact shape
and write the value of `implementation_sha` after `**Commit:**`:

```markdown
---
audit: 2026-08-04-16-m10-post-remediation
last_updated: 2026-08-06
status:
  fixed: 1
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 26
---

# Actions Taken: Full Repository Post-Remediation Audit

Summary of remediation status for the
[2026-08-04 full repository post-remediation audit](README.md).

---

## 2026-08-06 — Default composition bounded

**Disposition:** fixed
**Addresses:** [composer-default-path-has-unbounded-recursion](README.md#composer-default-path-has-unbounded-recursion)
**Commit:** ${implementation_sha}
**Author:** Codex

Ordinary composer string and event-stream constructors now apply
`LoaderConfig::default()`. Explicitly named unchecked entry points preserve
trusted-input compatibility without making unlimited loading the default.

The composer also enforces depth and collection limits locally, including raw
mapping pairs and materialized merge entries, so caller-supplied event streams
cannot bypass parser-side controls. Regression tests cover default and
unchecked APIs, local depth rejection, duplicate mapping entries, sequences,
and merged mappings.

---
```

- [ ] **Step 3: Validate the ledger and repository state**

Run:

```bash
rg -n 'pending' \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
implementation_sha="$(sed -n 's/^\*\*Commit:\*\* //p' \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md | tail -1)"
test "${#implementation_sha}" -eq 40
git --no-pager cat-file -e "${implementation_sha}^{commit}"
git --no-pager diff --check
git --no-pager status --short
```

Expected: the placeholder scan returns no matches, the implementation commit
resolves, no whitespace errors exist, and only the ledger plus the six known
old-audit files are untracked.

- [ ] **Step 4: Commit the append-only ledger entry**

```bash
git add record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
git --no-pager diff --cached --check
git --no-pager diff --cached --name-only
git commit -m "docs(audit): record bounded composer remediation"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: the ledger commit succeeds and signature output contains
`Good signature`. Do not replace the implementation SHA with the ledger commit
SHA.
