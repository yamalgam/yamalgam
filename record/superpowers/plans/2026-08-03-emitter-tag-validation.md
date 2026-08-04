# YAML Tag Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve `emitter-replays-tags-verbatim` by rejecting unsafe tag
metadata at mutation time and refusing to replay unsafe parsed metadata.

**Architecture:** Add one representation-aware `is_valid_tag` predicate to
`yamalgam-query`'s value module. `YamlVal::set_tag` uses it as an input
boundary; the YAML emitter uses the same predicate as a final output boundary.
Invalid parsed tags are omitted rather than serialized, while invalid filter
arguments return a jaq error.

**Tech Stack:** Rust 2024, `yamalgam-query`, jaq, cargo-nextest, Just.

## Global Constraints

- Keep the CLI thin; all behavior remains in `yamalgam-query`.
- Add no dependency and do not change project versions.
- Preserve valid local, core, and verbatim tags.
- Do not edit immutable files in
  `record/audits/2026-08-03-21-m10-fresh/`.
- Create the code-fix commit before creating the append-only ledger entry.
- Use the existing branch; do not create a worktree.

---

### Task 1: Reject unsafe tags at both query boundaries

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/emit/yaml.rs`
- Test: `crates/yamalgam-query/tests/emit.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Interfaces:**

- Consumes: `YamlVal::set_tag`, YAML `Context::properties`, and the existing
  `safe_tag(&str) -> Option<String>` formatter.
- Produces: `pub(crate) fn is_valid_tag(&str) -> bool`, used by mutation and
  emission boundaries.

**Implementation steps:**

- [ ] **Step 1: Add the failing emitter regression test**

Append this test beside `yaml_reemits_anchors_aliases_and_tags` in
`crates/yamalgam-query/tests/emit.rs`:

```rust
#[test]
fn yaml_omits_tags_that_cannot_be_reemitted_safely() {
    let value = parse_str("k: !x%0Ainjected%3A%20true v\n")
        .unwrap()
        .remove(0);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()).unwrap(),
        "k: v\n"
    );
}
```

The production change this test catches is removal or bypass of output-boundary
tag validation, which would reintroduce an attacker-controlled mapping key.

- [ ] **Step 2: Run the emitter test and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-query --test emit \
  -E 'test(yaml_omits_tags_that_cannot_be_reemitted_safely)' \
  --success-output=immediate
```

Expected: FAIL because the current emitter returns
`"k: !x\ninjected: true v\n"` rather than `"k: v\n"`.

- [ ] **Step 3: Add the failing mutation-boundary regression test**

Append this test beside `set_anchor_rejects_invalid_names` in
`crates/yamalgam-query/tests/engine.rs`:

```rust
#[test]
fn set_tag_rejects_tags_that_cannot_be_emitted_safely() {
    for filter_source in [
        r#"set_tag("not valid")"#,
        r#"set_tag("!x\ninjected: true")"#,
        r#"set_tag("!x,y")"#,
        r#"set_tag("!<broken")"#,
    ] {
        let filter = engine::compile_filter(filter_source, &[]).unwrap();
        let context = RunCtx::new(LoaderConfig::moderate());
        let documents = yamalgam_query::parse_str("scalar\n").unwrap();
        let result = engine::run_filter(
            &filter,
            &context,
            documents.into_iter().map(Ok::<_, String>),
            |_| Ok::<_, yamalgam_query::YgError>(()),
        );

        assert!(result.is_err(), "filter unexpectedly succeeded: {filter_source}");
    }
}
```

The production change this test catches is removal or bypass of input-boundary
validation in `set_tag`.

- [ ] **Step 4: Run the mutation test and verify RED**

Run:

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(set_tag_rejects_tags_that_cannot_be_emitted_safely)' \
  --success-output=immediate
```

Expected: FAIL because every invalid string is currently stored as metadata.

- [ ] **Step 5: Add the shared tag predicate**

Add this predicate near the other value-formatting helpers in
`crates/yamalgam-query/src/value.rs`:

```rust
pub(crate) fn is_valid_tag(tag: &str) -> bool {
    fn is_uri_char(character: char) -> bool {
        character.is_ascii_alphanumeric()
            || "-#;/?:@&=+$,_.!~*'()[]%".contains(character)
            || (!character.is_ascii()
                && !character.is_control()
                && !character.is_whitespace())
    }

    fn is_tag_char(character: char) -> bool {
        is_uri_char(character) && !"!,[]{}".contains(character)
    }

    if let Some(verbatim) = tag.strip_prefix("!<") {
        let Some(uri) = verbatim.strip_suffix('>') else {
            return false;
        };
        return !uri.is_empty() && uri.chars().all(is_uri_char);
    }

    if let Some(core) = tag.strip_prefix("!!") {
        return !core.is_empty() && core.chars().all(is_tag_char);
    }

    if let Some(local) = tag.strip_prefix('!') {
        return local.chars().all(is_tag_char);
    }

    !tag.is_empty() && tag.chars().all(is_uri_char)
}
```

This mirrors YAML's distinction between `ns-uri-char` inside verbatim tags and
the stricter `ns-tag-char` used by shorthand tags. Raw whitespace, controls,
flow indicators in shorthand tags, and verbatim delimiters are rejected.

- [ ] **Step 6: Enforce the predicate in `set_tag`**

Import no new types. Change the `Data::Str` match in `YamlVal::set_tag` to:

```rust
Data::Str(tag) if is_valid_tag(&tag) => Some(tag.strip_prefix("!!").map_or_else(
    || tag.clone(),
    |suffix| Rc::from(format!("tag:yaml.org,2002:{suffix}")),
)),
Data::Str(tag) => {
    return Err(jaq_core::Error::str(format_args!(
        "invalid YAML tag {tag:?}"
    )));
}
```

- [ ] **Step 7: Enforce the predicate in the emitter**

Add `is_valid_tag` to the `crate::value` import list in
`crates/yamalgam-query/src/emit/yaml.rs`, then begin `safe_tag` with:

```rust
if !is_valid_tag(tag) {
    return None;
}
```

Keep the existing output formatting for valid `!!`, `!<...>`, local, and bare
URI forms.

- [ ] **Step 8: Run both focused tests and verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-query \
  -E 'test(yaml_omits_tags_that_cannot_be_reemitted_safely) \
      | test(set_tag_rejects_tags_that_cannot_be_emitted_safely)' \
  --success-output=immediate
```

Expected: 2 tests pass.

- [ ] **Step 9: Run the affected crate test suite**

Run:

```bash
cargo nextest run -p yamalgam-query
```

Expected: all `yamalgam-query` tests pass.

- [ ] **Step 10: Run formatting and clippy**

Run:

```bash
just fmt
just clippy
```

Expected: formatting makes no unrelated changes and clippy exits successfully
with warnings denied.

- [ ] **Step 11: Review and commit the code action**

Run:

```bash
git --no-pager diff --check
git --no-pager diff --stat
git --no-pager diff
git add crates/yamalgam-query/src/value.rs \
  crates/yamalgam-query/src/emit/yaml.rs \
  crates/yamalgam-query/tests/emit.rs \
  crates/yamalgam-query/tests/engine.rs
git commit -m "fix(query): reject unsafe YAML tags"
```

Expected: one focused code commit containing two regression tests and the
shared validation rule.

---

### Task 2: Record the completed action in the append-only ledger

**Files:**

- Create: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Interfaces:**

- Consumes: the exact 40-character SHA produced by Task 1 and the immutable
  audit title and slug.
- Produces: a Cased remediation ledger with `fixed: 1` and `open: 46`.

**Ledger steps:**

- [ ] **Step 1: Capture and verify the code-fix SHA**

Run:

```bash
git --no-pager show -s --format='%H%n%s' HEAD
```

Expected: the full SHA followed by
`fix(query): reject unsafe YAML tags`.

- [ ] **Step 2: Create the ledger with the verified SHA**

Create `actions-taken.md` using the audit briefing's exact front matter and
title. Set `last_updated` to `2026-08-03`, `fixed` to `1`, every other closed
disposition to `0`, and `open` to `46`. Add one entry titled
`Unsafe YAML tag replay rejected` with:

- `Disposition: fixed`
- `Addresses: [emitter-replays-tags-verbatim](README.md#emitter-replays-tags-verbatim)`
- `Commit:` followed by the exact SHA printed in Step 1
- `Author: Codex`
- a concise description naming the shared predicate, the `set_tag` rejection,
  the emitter fallback, and the two regression tests

**Validation and commit steps:**

- [ ] **Step 3: Validate the ledger and audit immutability**

Run:

```bash
sed -n '1,220p' record/audits/2026-08-03-21-m10-fresh/actions-taken.md
shasum record/audits/2026-08-03-21-m10-fresh/README.md \
  record/audits/2026-08-03-21-m10-fresh/report.html \
  record/audits/2026-08-03-21-m10-fresh/findings.yaml \
  record/audits/2026-08-03-21-m10-fresh/recon.yaml
git --no-pager diff --check
```

Expected: the four immutable hashes remain respectively
`6279c46a43719466f800c3620ea18d504937862c`,
`f767b1ecf889877078b172497c429d6a1cc71bb5`,
`86e17628a6558af3d65aaa866382dcea8a3d200f`, and
`74e0cc74ba003b44bd9babd68863714c9ddb781e`.

- [ ] **Step 4: Commit the ledger entry separately**

Run:

```bash
git add record/audits/2026-08-03-21-m10-fresh/actions-taken.md
git commit -m "docs(audit): record unsafe tag remediation"
```

Expected: a ledger-only commit whose entry points back to Task 1's code-fix
commit.
