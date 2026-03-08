# Code Review Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Address all 7 findings from the Codex-53 code review (`docs/code-reviews/codex-53-high-03-08-2026.md`).

**Architecture:** Six independent workstreams — compliance harness improvements (findings 1+2), composer limit enforcement (finding 3), config-aware convenience APIs (finding 4), merge-key error handling (finding 5), C harness hardening (finding 6), and tag comparison mode (finding 7). Each is a separate commit.

**Tech Stack:** Rust (yamalgam-compare, yamalgam-parser, yamalgam-core), C (tools/fyaml-tokenize), `cargo nextest`, `datatest-stable`.

---

### Task 1: Compliance harness assertions with allowlist (Finding #1)

**Files:**
- Modify: `crates/yamalgam-compare/tests/compliance.rs`

**Context:** Currently both `compliance_test` and `event_compliance_test` return `Ok(())` for every category. The fix adds assertions that fail on `MISMATCH`, `UNEXPECTED`, `EVENT_MISMATCH`, `EVENT_UNEXPECTED` — with an explicit allowlist for known/understood gaps.

**Known allowlist entries (from MEMORY.md):**
- Token MISMATCH: `M7A3` (C scanner bug), `BD7L` (cosmetic, both fail:true)
- Token EXPECTED: 15 cases — all `fail:true`, we correctly reject, C incorrectly accepts
- Event EXPECTED: `CFD4`, `DK95`
- Token UNEXPECTED: 0, Event UNEXPECTED: 0

**Step 1: Add allowlists and assertions to `compliance_test`**

At the top of the file, add:

```rust
/// Token-level allowlist: test IDs where known divergence is acceptable.
/// MISMATCH: M7A3 (C scanner bug), BD7L (cosmetic fail:true diff).
/// EXPECTED: C accepts, we correctly reject (all fail:true cases).
const TOKEN_MISMATCH_ALLOWLIST: &[&str] = &["M7A3", "BD7L"];
// EXPECTED (C succeeds, Rust errors) — these are all fail:true inputs
// where our stricter rejection is correct. No allowlist needed: EXPECTED is fine.
// UNEXPECTED (Rust succeeds, C errors) — these are regressions. Always fail.
```

In `compliance_test`, change the match arms:

```rust
fn compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing code through `let result = ...` ...

    match &result {
        CompareResult::Match { token_count } => {
            eprintln!("PASS: {id} ({token_count} tokens matched)");
        }
        CompareResult::CSuccessRustError { rust_error, c_token_count } => {
            // EXPECTED: we're stricter than C. Fine for fail:true cases.
            eprintln!("EXPECTED: {id} (C produced {c_token_count} tokens, Rust: {rust_error})");
        }
        CompareResult::BothErrorMatch => {
            eprintln!("PASS: {id} (both errored, matching)");
        }
        CompareResult::BothErrorMismatch { .. } => {
            eprintln!("PASS: {id} (both errored)");
        }
        CompareResult::RustSuccessCError { c_error, rust_token_count } => {
            // UNEXPECTED: our scanner is too permissive. Always a regression.
            panic!("UNEXPECTED: {id} (Rust produced {rust_token_count} tokens, C: {c_error})");
        }
        CompareResult::TokenMismatch { index, c_token, rust_token, .. } => {
            if TOKEN_MISMATCH_ALLOWLIST.contains(&id.as_str()) {
                eprintln!(
                    "MISMATCH (allowlisted): {id} at index {index} (C: {:?}, Rust: {:?})",
                    c_token.kind, rust_token.kind
                );
            } else {
                panic!(
                    "MISMATCH: {id} at index {index} (C: {:?}, Rust: {:?})",
                    c_token.kind, rust_token.kind
                );
            }
        }
    }

    Ok(())
}
```

**Step 2: Add assertions to `event_compliance_test`**

Same pattern:

```rust
/// Event-level allowlist for known divergence.
const EVENT_MISMATCH_ALLOWLIST: &[&str] = &[];
// EVENT_EXPECTED: CFD4, DK95 — we're stricter. Fine.
// EVENT_UNEXPECTED: always a regression.
```

```rust
fn event_compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing code through `let result = ...` ...

    match &result {
        CompareEventResult::Match { event_count } => {
            eprintln!("EVENT_PASS: {id} ({event_count} events matched)");
        }
        CompareEventResult::CSuccessRustError { rust_error, c_event_count } => {
            eprintln!("EVENT_EXPECTED: {id} (C produced {c_event_count} events, Rust: {rust_error})");
        }
        CompareEventResult::BothErrorMatch => {
            eprintln!("EVENT_PASS: {id} (both errored, matching)");
        }
        CompareEventResult::BothErrorMismatch { .. } => {
            eprintln!("EVENT_PASS: {id} (both errored)");
        }
        CompareEventResult::RustSuccessCError { c_error, rust_event_count } => {
            panic!("EVENT_UNEXPECTED: {id} (Rust produced {rust_event_count} events, C: {c_error})");
        }
        CompareEventResult::EventMismatch { index, c_event, rust_event, .. } => {
            if EVENT_MISMATCH_ALLOWLIST.contains(&id.as_str()) {
                eprintln!(
                    "EVENT_MISMATCH (allowlisted): {id} at index {index} (C: {:?}, Rust: {:?})",
                    c_event.kind, rust_event.kind
                );
            } else {
                panic!(
                    "EVENT_MISMATCH: {id} at index {index} (C: {:?}, Rust: {:?})",
                    c_event.kind, rust_event.kind
                );
            }
        }
    }

    Ok(())
}
```

**Step 3: Run compliance tests**

Run: `cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | tail -20`

Expected: All 702 tests pass (349 EVENT_PASS + 334 PASS + allowlisted + EXPECTED categories).

**Step 4: Commit**

```
fix(compliance): fail on MISMATCH and UNEXPECTED with explicit allowlist
```

---

### Task 2: Multi-case test file extraction (Finding #2)

**Files:**
- Modify: `crates/yamalgam-compare/src/c_baseline.rs`
- Modify: `crates/yamalgam-compare/tests/compliance.rs`

**Context:** 6 test files contain multiple cases (14 extra cases total). Currently only the first `yaml:` block is extracted. The fix extracts all cases and assigns `FILE#N` IDs (e.g., `2G84#0`, `2G84#1`). The multi-case files are: 2G84 (2), 9MQT (2), SM9W (2), DK95 (3), MUS6 (3), Y79Y (8).

**Step 1: Add `extract_all_yaml_inputs` to `c_baseline.rs`**

Add a new function alongside the existing `extract_yaml_input`:

```rust
/// Extract all YAML inputs from a YAML Test Suite file.
///
/// Multi-case files contain a YAML array where each element has a `yaml:` field.
/// Returns a vec of `(case_index, yaml_input)` tuples. Single-case files return
/// one entry with index 0.
pub fn extract_all_yaml_inputs(content: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut current_case = 0;
    let mut in_yaml_block = false;
    let mut indent: Option<usize> = None;
    let mut yaml_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        // New array element at column 0 (starts with "- ") — flush previous case.
        if !in_yaml_block
            && line.starts_with("- ")
            && !yaml_lines.is_empty()
        {
            if let Some(input) = finalize_yaml_lines(&yaml_lines) {
                results.push((current_case, input));
            }
            current_case += 1;
            yaml_lines.clear();
            indent = None;
        }

        if in_yaml_block {
            if let Some(min_indent) = indent {
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();

                if !line.trim().is_empty() && current_indent < min_indent {
                    in_yaml_block = false;
                    // Don't consume this line — check if it starts a new yaml: block below.
                } else if line.len() >= min_indent {
                    yaml_lines.push(&line[min_indent..]);
                    continue;
                } else if line.trim().is_empty() {
                    yaml_lines.push("");
                    continue;
                } else {
                    in_yaml_block = false;
                }
            } else if !line.trim().is_empty() {
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();
                indent = Some(current_indent);
                yaml_lines.push(&line[current_indent..]);
                continue;
            }
        }

        if line.trim_start().starts_with("yaml:") {
            let after_key = line.trim_start().strip_prefix("yaml:").unwrap().trim();
            if after_key.is_empty()
                || after_key == "|"
                || after_key == "|2"
                || after_key == "|-"
            {
                in_yaml_block = true;
                indent = None;
            } else {
                // Inline yaml value.
                yaml_lines.clear();
                yaml_lines.push(after_key);
                in_yaml_block = false;
            }
        }
    }

    // Flush final case.
    if let Some(input) = finalize_yaml_lines(&yaml_lines) {
        results.push((current_case, input));
    }

    // If only one case, return index 0 (no sub-indexing needed).
    results
}

/// Convert accumulated yaml lines into a finished string with marker replacement.
fn finalize_yaml_lines(yaml_lines: &[&str]) -> Option<String> {
    if yaml_lines.is_empty() {
        return None;
    }

    let mut result = yaml_lines.join("\n");

    // Convert YAML Test Suite visual markers to actual characters.
    if result.contains('\u{2014}') {
        result = result.replace('\u{2014}', "");
    }
    if result.contains('\u{2423}') {
        result = result.replace('\u{2423}', " ");
    }
    if result.contains('\u{00BB}') {
        result = result.replace('\u{00BB}', "\t");
    }
    if result.contains('\u{21B5}') {
        result = result.replace('\u{21B5}', "");
    }
    if result.contains('\u{220E}') {
        result = result.replace('\u{220E}', "");
        if result.ends_with('\n') {
            result.pop();
        }
    }

    Some(result)
}
```

**Step 2: Update `generate` to use all cases**

In `c_baseline.rs`, the `generate` function currently calls `extract_yaml_input` (single). Change it to use `extract_all_yaml_inputs`:

```rust
for entry in &entries {
    let path = entry.path();
    let id = path.file_stem().unwrap().to_string_lossy().to_string();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let cases = extract_all_yaml_inputs(&content);
    if cases.len() == 1 {
        test_cases.push((id, cases[0].1.clone().into_bytes()));
    } else {
        for (idx, yaml) in &cases {
            test_cases.push((format!("{id}#{idx}"), yaml.clone().into_bytes()));
        }
    }
}
```

**Step 3: Update compliance tests to iterate all cases**

In `compliance.rs`, update both test functions to handle multi-case files. The `datatest_stable` harness iterates files, so the test function needs to extract all cases internally:

```rust
fn compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let file_id = test_id(path);
    let cases = c_baseline::extract_all_yaml_inputs(&content);

    if cases.is_empty() {
        eprintln!("SKIP: no yaml input found in {}", path.display());
        return Ok(());
    }

    for (idx, yaml_input) in &cases {
        let id = if cases.len() == 1 {
            file_id.clone()
        } else {
            format!("{file_id}#{idx}")
        };

        let cached = token_cache().as_ref().and_then(|cache| cache.get(&id));
        let result = compare_input_cached(cached, yaml_input.as_bytes());

        // ... match arms with assertions (same as Task 1) ...
    }

    Ok(())
}
```

Same pattern for `event_compliance_test`.

**Step 4: Regenerate C baselines**

Run: `just c-baseline`

This regenerates `target/c-baseline-tokens.json` and `target/c-baseline-events.json` with per-case IDs for multi-case files.

**Step 5: Run compliance tests**

Run: `cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | grep -oE "^    (PASS|UNEXPECTED|MISMATCH|EXPECTED|EVENT_PASS|EVENT_UNEXPECTED|EVENT_MISMATCH|EVENT_EXPECTED)" | sort | uniq -c | sort -rn`

Expected: ~14 more test cases now exercised. Totals should increase by ~14 each for tokens and events.

**Step 6: Commit**

```
fix(compliance): extract all cases from multi-case test suite files
```

---

### Task 3: Enforce alias/anchor/merge limits in Composer (Finding #3)

**Files:**
- Modify: `crates/yamalgam-core/src/loader.rs` (add check helpers)
- Modify: `crates/yamalgam-parser/src/compose.rs` (wire limits, add config field)
- Modify: `crates/yamalgam-parser/src/lib.rs` (config-aware convenience APIs)

**Context:** `max_alias_expansions`, `max_anchor_count`, `max_merge_depth` are declared in `ResourceLimits` but not enforced. The Composer needs to track counters and check them.

**Step 1: Add check helpers to `ResourceLimits`**

In `crates/yamalgam-core/src/loader.rs`, after `check_input_size`:

```rust
/// Check whether `count` is within [`max_alias_expansions`](Self::max_alias_expansions).
pub fn check_alias_expansions(&self, count: usize) -> Result<(), String> {
    check_limit(count, self.max_alias_expansions, "alias expansions")
}

/// Check whether `count` is within [`max_anchor_count`](Self::max_anchor_count).
pub fn check_anchor_count(&self, count: usize) -> Result<(), String> {
    check_limit(count, self.max_anchor_count, "anchor count")
}

/// Check whether `depth` is within [`max_merge_depth`](Self::max_merge_depth).
pub fn check_merge_depth(&self, depth: usize) -> Result<(), String> {
    check_limit(depth, self.max_merge_depth, "merge depth")
}
```

**Step 2: Add `LimitExceeded` variant to `ComposeError`**

In `compose.rs`:

```rust
pub enum ComposeError {
    Resolve(ResolveError),
    UndefinedAlias(String),
    UnexpectedEvent(String),
    /// A resource limit was exceeded.
    LimitExceeded(String),
}
```

Update `Display` and `Error` impls accordingly.

**Step 3: Add config and counters to `Composer`**

```rust
pub struct Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    events: std::iter::Peekable<I>,
    anchors: HashMap<String, Value>,
    config: ResourceLimits,
    alias_expansion_count: usize,
}
```

Update `Composer::new` to accept `ResourceLimits`:

```rust
pub fn new(events: I) -> Self {
    Self {
        events: events.peekable(),
        anchors: HashMap::new(),
        config: ResourceLimits::none(),
        alias_expansion_count: 0,
    }
}

pub fn new_with_config(events: I, config: &yamalgam_core::LoaderConfig) -> Self {
    Self {
        events: events.peekable(),
        anchors: HashMap::new(),
        config: config.limits.clone(),
        alias_expansion_count: 0,
    }
}
```

**Step 4: Enforce anchor count on insertion**

In `compose_node`, after each `self.anchors.insert(...)`:

```rust
if let Some(name) = anchor {
    self.anchors.insert(name.into_owned(), resolved.clone());
    if let Err(msg) = self.config.check_anchor_count(self.anchors.len()) {
        return Err(ComposeError::LimitExceeded(msg));
    }
}
```

Apply to all three insert sites (scalar, sequence, mapping).

**Step 5: Enforce alias expansion count**

In the `Event::Alias` arm:

```rust
Event::Alias { name, .. } => {
    self.alias_expansion_count += 1;
    if let Err(msg) = self.config.check_alias_expansions(self.alias_expansion_count) {
        return Err(ComposeError::LimitExceeded(msg));
    }
    let name_str = name.into_owned();
    self.anchors
        .get(&name_str)
        .cloned()
        .ok_or(ComposeError::UndefinedAlias(name_str))
}
```

**Step 6: Enforce merge depth**

Change `collect_merge_pairs` to track depth:

```rust
fn collect_merge_pairs(
    val: &Value,
    pairs: &mut Vec<(Value, Value)>,
    depth: usize,
    max_depth: Option<usize>,
) -> Result<(), ComposeError> {
    if let Some(max) = max_depth {
        if depth > max {
            return Err(ComposeError::LimitExceeded(
                format!("merge depth {depth} exceeds maximum of {max}"),
            ));
        }
    }
    match val {
        Value::Mapping(m) => {
            for (k, v) in m.iter() {
                pairs.push((k.clone(), v.clone()));
            }
        }
        Value::Sequence(seq) => {
            for item in seq {
                collect_merge_pairs(item, pairs, depth + 1, max_depth)?;
            }
        }
        _ => {
            // Non-mapping merge value — error per spec.
            return Err(ComposeError::UnexpectedEvent(
                "merge key (<<) value must be a mapping or sequence of mappings".into(),
            ));
        }
    }
    Ok(())
}
```

Update the call site:

```rust
if is_merge_key(&key) {
    collect_merge_pairs(&val, &mut merge_pairs, 0, self.config.max_merge_depth)?;
} else {
    map.insert(key, val);
}
```

**NOTE:** This also implements Finding #5 — non-mapping merge values now error instead of silently dropping.

**Step 7: Add tests**

In `compose.rs` tests:

```rust
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
    let result = Composer::from_str("<<: 1");
    assert!(matches!(result, Err(ComposeError::UnexpectedEvent(_))));
}
```

**Step 8: Run tests**

Run: `just test`

Expected: All existing tests pass, new limit tests pass.

**Step 9: Commit**

```
feat(composer): enforce alias/anchor/merge limits and reject invalid merge values
```

---

### Task 4: Config-aware convenience APIs (Finding #4)

**Files:**
- Modify: `crates/yamalgam-parser/src/compose.rs`
- Modify: `crates/yamalgam-parser/src/lib.rs`

**Step 1: Add `from_str_with_config` to Composer**

```rust
impl<'input> Composer<'input, ResolvedEvents<'input, NoopResolver>> {
    // existing from_str ...

    /// Parse and compose all documents with resource limits.
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
}
```

**Step 2: Add config-aware free functions to `lib.rs`**

```rust
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
```

**Step 3: Export from lib.rs**

Ensure `from_str_with_config` and `from_str_single_with_config` are accessible.

**Step 4: Run tests**

Run: `just test`

**Step 5: Commit**

```
feat(parser): add config-aware from_str_with_config convenience APIs
```

---

### Task 5: C batch harness bounds check (Finding #6)

**Files:**
- Modify: `tools/fyaml-tokenize/main.c`

**Step 1: Replace `atol` with bounded parsing**

In `main.c` around line 385, replace:

```c
size_t input_len = (size_t)atol(len_buf);
```

with:

```c
char *endptr;
errno = 0;
unsigned long long raw_len = strtoull(len_buf, &endptr, 10);
if (errno != 0 || endptr == len_buf || raw_len > (256ULL * 1024 * 1024)) {
    fprintf(stdout, "{\"error\":\"invalid or excessive frame length\"}\n---END\n");
    fflush(stdout);
    continue;
}
size_t input_len = (size_t)raw_len;
```

Add `#include <errno.h>` if not already present.

**Step 2: Rebuild C harness**

Run: `cd tools/fyaml-tokenize && make clean && make && cd ../..`

**Step 3: Regenerate baselines**

Run: `just c-baseline`

**Step 4: Run compliance tests to verify nothing broke**

Run: `cargo nextest run -p yamalgam-compare --test compliance`

**Step 5: Commit**

```
fix(c-harness): validate batch frame length with bounds check
```

---

### Task 6: Optional strict tag comparison (Finding #7)

**Files:**
- Modify: `crates/yamalgam-compare/src/compare.rs`

**Context:** Tag comparison is intentionally skipped because libfyaml resolves to URIs while yamalgam keeps shorthand. This is important for the editable workflow. Adding an opt-in strict mode preserves existing behavior while enabling tag testing when we want it.

**Step 1: Add `events_match_strict` function**

```rust
/// Compare two events including tags (strict mode).
///
/// Use when both sides produce tags in the same format.
pub fn events_match_strict(a: &EventSnapshot, b: &EventSnapshot) -> bool {
    a.kind == b.kind
        && a.value == b.value
        && a.anchor == b.anchor
        && a.implicit == b.implicit
        && a.tag == b.tag
}
```

**Step 2: Add `compare_event_streams_strict` function**

Clone `compare_event_streams` but call `events_match_strict` instead. Accept a `strict: bool` parameter or make it a separate function.

```rust
/// Compare event streams with optional tag comparison.
pub fn compare_event_streams_with_tags(
    c_events: &[EventSnapshot],
    rust_events: &[EventSnapshot],
) -> CompareEventResult {
    // Same logic as compare_event_streams but uses events_match_strict
    // ...
}
```

**Step 3: Commit**

```
feat(compare): add strict event comparison mode with tag matching
```

---

## Execution Order

Tasks 1+2 are compliance harness (do together).
Task 3+4+5 touch composer/parser (do together — Task 5 merged into Task 3).
Task 5 (C harness) is independent.
Task 6 (tag comparison) is independent.

Recommended order: **1 → 2 → 3+4 → 5 → 6**

Each task is one commit. Run `just check` after all tasks.
