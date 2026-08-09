# Safe Serde Defaults Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close `serde-default-disables-loader-limits` by applying moderate
limits to ordinary serde string, reader, and document-stream entry points while
preserving explicit unchecked APIs for trusted input.

**Architecture:** `Deserializer::from_str` delegates to
`from_str_with_config` with `LoaderConfig::default()`. A new
`Deserializer::from_str_unchecked` delegates with `LoaderConfig::unchecked()`.
Free string and reader functions select those constructors through one private
single-document helper, so error restoration and end-of-document checks remain
identical.

**Tech Stack:** Rust 2024, serde, erased-serde, `LoaderConfig`,
`cargo nextest`, `just`.

## Global Constraints

- Work directly on branch `m10-yg-cli`; do not create a worktree.
- Add no dependency and do not change any project version.
- Keep `Parser::new` and `Scanner::new` unlimited for compatibility.
- Preserve `from_str_with_config` and its caller-selected policy.
- Do not alter `from_reader` I/O error classification; that is the separate
  `from-reader-erases-io-errors` finding.
- Follow strict red-green-refactor and stage only named files.
- Do not edit immutable audit artifacts. Update only the existing append-only
  `actions-taken.md` after the implementation commit exists.
- Leave the six untracked files under
  `record/audits/2026-08-03-21-m10-fresh/` untouched.

## File map

- Modify `crates/yamalgam-serde/src/de.rs`: safe and unchecked deserializer
  constructors.
- Modify `crates/yamalgam-serde/src/lib.rs`: shared single-document driver and
  free unchecked string/reader APIs.
- Modify `crates/yamalgam-serde/tests/config.rs`: default and unchecked depth
  regression coverage.
- Append
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`: second
  ledger action and updated status counts.

---

### Task 1: Apply safe serde defaults

**Files:**

- Modify: `crates/yamalgam-serde/src/de.rs`
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Test: `crates/yamalgam-serde/tests/config.rs`

**Interfaces:**

- Consumes: `LoaderConfig::default()`, `LoaderConfig::unchecked()`, and the
  existing `Deserializer::from_str_with_config`.
- Produces: `Deserializer::from_str_unchecked`, `from_str_unchecked`, and
  `from_reader_unchecked`.
- Preserves: `from_str_with_config`, `from_reader` I/O behavior, structured
  stashed errors, and multi-document iteration.

**Steps:**

- [ ] **Step 1: Add the failing safe-default test**

Extend `crates/yamalgam-serde/tests/config.rs` imports:

```rust
use std::io::Cursor;

use yamalgam_parser::ParseError;
use yamalgam_serde::{
    Deserializer, Error, from_reader, from_str, from_str_with_config,
};
```

Add these helpers and the safe-default test:

```rust
fn nested_flow_sequence(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

fn assert_limit_error<T>(result: Result<T, Error>) {
    let err = match result {
        Ok(_) => panic!("expected a resource-limit error"),
        Err(error) => error,
    };
    match err {
        Error::LimitExceeded(_) | Error::Parse(ParseError::LimitExceeded(_)) => {}
        Error::Parse(ParseError::Scan(error))
            if error.message.starts_with("depth ")
                && error.message.contains("exceeds maximum") => {}
        other => panic!("expected a resource-limit error, got {other:?}"),
    }
}

#[test]
fn ordinary_serde_entry_points_use_moderate_limits() {
    let input = nested_flow_sequence(257);

    assert_limit_error(from_str::<serde_json::Value>(&input));

    let result = Deserializer::from_str(&input)
        .documents::<serde_json::Value>()
        .next()
        .expect("one document result");
    assert_limit_error(result);

    assert_limit_error(from_reader::<_, serde_json::Value>(Cursor::new(
        input.as_bytes(),
    )));
}
```

- [ ] **Step 2: Run the safe-default test and verify RED**

```bash
cargo nextest run -p yamalgam-serde --test config \
  -E 'test(ordinary_serde_entry_points_use_moderate_limits)' \
  --success-output=immediate
```

Expected: FAIL because all three ordinary entry points currently accept depth
257 under `ResourceLimits::none()`.

- [ ] **Step 3: Make `Deserializer::from_str` moderate**

Replace the duplicated unchecked constructor body in `de.rs`:

```rust
/// Create a `Deserializer` with [`LoaderConfig::default`] resource limits.
#[must_use]
#[allow(clippy::should_implement_trait)]
pub fn from_str(input: &'input str) -> Self {
    Self::from_str_with_config(input, &LoaderConfig::default())
}
```

Do not change `from_str_with_config`. It remains the single constructor that
builds the parser, copies limits, and selects tag resolution.

- [ ] **Step 4: Run the safe-default test and verify GREEN**

Run the Step 2 command again.

Expected: the free string function, documents iterator, and reader function
all return a structured parser or resource-limit error.

- [ ] **Step 5: Share the free single-document driver**

Move the existing erased-serde and end-check logic into a private helper in
`lib.rs`:

```rust
fn deserialize_one<'de, T: serde::Deserialize<'de>>(
    mut de: Deserializer<'de>,
) -> Result<T, Error> {
    let result = {
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut de);
        erased_serde::deserialize::<T>(&mut erased)
    };
    match result {
        Ok(value) => {
            de.clear_stashed();
            de.check_end()?;
            Ok(value)
        }
        Err(error) => Err(de.restore_stashed(Error::from(error))),
    }
}
```

Route both existing string functions through it:

```rust
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    deserialize_one(Deserializer::from_str(input))
}

pub fn from_str_with_config<'de, T: serde::Deserialize<'de>>(
    input: &'de str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<T, Error> {
    deserialize_one(Deserializer::from_str_with_config(input, config))
}
```

`from_reader` continues to read into `String` and call the now-safe
`from_str(&buf)`. Do not change its I/O `Error::Custom` mapping in this action.

Update the existing `from_str` rustdoc to state that it applies moderate
resource limits and link [`from_str_unchecked`]. Update `from_reader` rustdoc
the same way and link [`from_reader_unchecked`]. Keep their current `# Errors`
sections intact.

- [ ] **Step 6: Verify the shared-driver refactor remains GREEN**

Run the Step 2 command again.

Expected: the free string function, documents iterator, and reader function
still return the same structured resource-limit errors after the refactor.

- [ ] **Step 7: Add the failing explicit-unchecked test**

Extend the serde imports with `from_reader_unchecked` and
`from_str_unchecked`, then add:

```rust
#[test]
fn unchecked_serde_entry_points_preserve_unlimited_behavior() {
    let input = nested_flow_sequence(257);

    assert!(from_str_unchecked::<serde_json::Value>(&input).is_ok());
    assert!(
        Deserializer::from_str_unchecked(&input)
            .documents::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .is_ok()
    );
    assert!(
        from_reader_unchecked::<_, serde_json::Value>(Cursor::new(input.as_bytes())).is_ok()
    );
}
```

- [ ] **Step 8: Run the unchecked test and verify RED**

```bash
cargo nextest run -p yamalgam-serde --test config \
  -E 'test(unchecked_serde_entry_points_preserve_unlimited_behavior)'
```

Expected: compilation fails because the three unchecked APIs do not exist.

- [ ] **Step 9: Implement the explicit unchecked APIs**

Add the associated constructor in `de.rs`:

```rust
/// Create a `Deserializer` with no resource limits.
///
/// Use only for trusted input. Prefer [`Self::from_str`] for ordinary input.
#[must_use]
pub fn from_str_unchecked(input: &'input str) -> Self {
    Self::from_str_with_config(input, &LoaderConfig::unchecked())
}
```

Add the free string helper in `lib.rs`:

```rust
/// Deserialize a single trusted YAML document with no resource limits.
///
/// Prefer [`from_str`] for ordinary input.
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, or multiple documents.
pub fn from_str_unchecked<'de, T: serde::Deserialize<'de>>(
    input: &'de str,
) -> Result<T, Error> {
    deserialize_one(Deserializer::from_str_unchecked(input))
}
```

Add the reader helper beside `from_reader` without changing I/O error
classification:

```rust
/// Deserialize trusted YAML from a reader with no resource limits.
///
/// Prefer [`from_reader`] for ordinary input.
///
/// # Errors
///
/// Returns [`Error`] on I/O failure, parse failure, type mismatch, or multiple
/// documents.
pub fn from_reader_unchecked<R: std::io::Read, T: serde::de::DeserializeOwned>(
    mut reader: R,
) -> Result<T, Error> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|error| Error::Custom(format!("I/O error: {error}")))?;
    from_str_unchecked(&buf)
}
```

- [ ] **Step 10: Run both default-policy tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-serde --test config -E \
  'test(ordinary_serde_entry_points_use_moderate_limits) + test(unchecked_serde_entry_points_preserve_unlimited_behavior)'
```

Expected: both tests pass.

- [ ] **Step 11: Run the crate and reviewer gates**

```bash
just fmt
cargo nextest run -p yamalgam-serde
cargo test --doc -p yamalgam-serde
just clippy
git --no-pager diff --check
```

Expected: the serde suite, doctests, warning-denied workspace clippy, and diff
checks all pass.

- [ ] **Step 12: Commit the implementation**

```bash
git add crates/yamalgam-serde/src/de.rs \
  crates/yamalgam-serde/src/lib.rs \
  crates/yamalgam-serde/tests/config.rs
git --no-pager diff --cached --check
git --no-pager diff --cached --name-only
git commit -m "fix(serde): apply safe loader defaults"
git --no-pager rev-parse HEAD
git --no-pager log --show-signature -1 --format=fuller
```

Expected: exactly the three serde files are committed and the signature output
contains `Good signature`. Record the full implementation SHA.

---

### Task 2: Append the serde action to the audit ledger

**Files:**

- Modify:
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`

**Interfaces:**

- Consumes: the real full implementation SHA from Task 1.
- Produces: status `fixed: 2`, `open: 25`, plus one new bottom entry.

**Steps:**

- [ ] **Step 1: Capture and verify the implementation SHA**

```bash
implementation_sha="$(git rev-parse HEAD)"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: the SHA names the serde code commit and signature output contains
`Good signature`.

- [ ] **Step 2: Update counts and append the action**

Change only the frontmatter counters to `fixed: 2` and `open: 25`. Append this
entry after the existing final `---`, writing the value of
`implementation_sha` after `**Commit:**`:

```markdown
## 2026-08-06 — Serde defaults bounded

**Disposition:** fixed
**Addresses:** [serde-default-disables-loader-limits](README.md#serde-default-disables-loader-limits)
**Commit:** ${implementation_sha}
**Author:** Codex

Ordinary serde string, reader, and document-stream entry points now apply
`LoaderConfig::default()`. Explicit unchecked entry points preserve the prior
unlimited behavior for trusted input.

The free functions share one single-document driver, so structured error
restoration and end-of-document checks remain consistent across default,
configured, and unchecked paths. Regression tests cover default rejection and
unchecked acceptance beyond the moderate depth limit.

---
```

- [ ] **Step 3: Validate and commit the append-only update**

```bash
/Users/clay/.cargo/bin/rumdl check \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
implementation_sha="$(sed -n 's/^\*\*Commit:\*\* //p' \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md | tail -1)"
test "${#implementation_sha}" -eq 40
git --no-pager cat-file -e "${implementation_sha}^{commit}"
git --no-pager diff --check
git add record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
git --no-pager diff --cached --name-only
git commit -m "docs(audit): record safe serde defaults"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: only the ledger is committed, its new entry retains the code SHA,
and the ledger commit has a good signature.
