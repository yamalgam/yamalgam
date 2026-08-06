# Stack-Safe CST Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close `cst-recursive-traversals-have-no-depth-bound` by making
ordinary CST construction moderate, enforcing collection depth inside the
builder, and removing recursive traversal and destruction from public CST
values.

**Architecture:** `parse_to_cst` and `CstBuilder::new` select
`LoaderConfig::default()`, while config-aware and explicitly unchecked variants
preserve caller control. `CstBuilder` counts only mapping and sequence nesting
and converts local limit failures into the existing lossless error-node form.
The three whole-tree operations and `Drop` use iterative worklists, so publicly
constructed trees are stack-safe independently of builder policy.

**Tech Stack:** Rust 2024, `LoaderConfig`, `ResourceLimits`, `cargo nextest`,
`just`.

## Global Constraints

- Work directly on branch `m10-yg-cli`; do not create a worktree.
- Add no dependency and do not change any project version.
- Keep `Parser::new` unlimited for low-level compatibility.
- Preserve the public `CstNode`, `CstElement`, and `CstToken` representations.
- Preserve the lossless contract: every construction path returns a CST whose
  `to_text()` reproduces the complete input, including on a local depth error.
- Do not add structured CST diagnostics in this action; that is the separate
  `cst-discards-parse-error` finding.
- Follow strict red-green-refactor and stage only the named files.
- Do not edit immutable audit artifacts. Update only the existing append-only
  `actions-taken.md` after the implementation commit exists.
- Leave the six untracked files under
  `record/audits/2026-08-03-21-m10-fresh/` untouched.

## File Map

- Modify `crates/yamalgam-cst/src/lib.rs`: safe constructors, builder-local
  collection depth, iterative operations, iterative drop, and regression tests.
- Append
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`: third
  ledger action and updated status counts.

---

### Task 1: Apply safe CST construction defaults

**Files:**

- Modify: `crates/yamalgam-cst/src/lib.rs`
- Test: `crates/yamalgam-cst/src/lib.rs`

**Interfaces:**

- Consumes: `LoaderConfig::default()`, `LoaderConfig::unchecked()`, and
  `Parser::with_config`.
- Produces: `parse_to_cst_with_config`, `parse_to_cst_unchecked`,
  `CstBuilder::new_with_config`, and `CstBuilder::new_unchecked` without
  removing the existing const constructor contract.
- Preserves: `parse_to_cst` and `CstBuilder::new` names, lossless recovery, and
  unlimited `Parser::new` behavior.

**Steps:**

- [ ] **Step 1: Add the failing safe-default test**

Add iterative test helpers:

```rust
fn nested_flow_sequence(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

fn contains_node_kind(root: &CstNode<'_>, kind: NodeKind) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.kind == kind {
            return true;
        }
        pending.extend(node.children.iter().rev().filter_map(|child| match child {
            CstElement::Node(child) => Some(child),
            CstElement::Token(_) => None,
        }));
    }
    false
}
```

Add:

```rust
#[test]
fn ordinary_cst_entry_point_uses_moderate_limits() {
    let input = nested_flow_sequence(257);
    let cst = parse_to_cst(&input);

    assert_eq!(cst.to_text(), input);
    assert!(contains_node_kind(&cst, NodeKind::Error));
}
```

- [ ] **Step 2: Run the safe-default test and verify RED**

```bash
cargo nextest run -p yamalgam-cst \
  -E 'test(ordinary_cst_entry_point_uses_moderate_limits)' \
  --success-output=immediate
```

Expected: FAIL because `parse_to_cst` currently uses unlimited `Parser::new`
and builds the complete tree without an error node.

- [ ] **Step 3: Add safe and configured constructors**

Import `LoaderConfig` and `ResourceLimits`. Extend `CstBuilder` with a cloned
limit policy and a collection-depth counter:

```rust
pub struct CstBuilder<'input> {
    source: &'input str,
    stack: Vec<CstNode<'input>>,
    last_offset: usize,
    limits: ResourceLimits,
    collection_depth: usize,
}
```

Preserve `CstBuilder::new` as a const API. `LoaderConfig::default()` delegates
to the const `moderate()` preset, so use that preset directly in its struct
literal. Add the config-aware constructor separately:

```rust
#[must_use]
pub const fn new(source: &'input str) -> Self {
    Self {
        source,
        stack: Vec::new(),
        last_offset: 0,
        limits: LoaderConfig::moderate().limits,
        collection_depth: 0,
    }
}

#[must_use]
pub fn new_with_config(source: &'input str, config: &LoaderConfig) -> Self {
    Self {
        source,
        stack: Vec::new(),
        last_offset: 0,
        limits: config.limits.clone(),
        collection_depth: 0,
    }
}
```

Route the ordinary convenience API through one config-aware path:

```rust
pub fn parse_to_cst(input: &str) -> CstNode<'_> {
    parse_to_cst_with_config(input, &LoaderConfig::default())
}

pub fn parse_to_cst_with_config<'input>(
    input: &'input str,
    config: &LoaderConfig,
) -> CstNode<'input> {
    let parser = Parser::with_config(input, config);
    CstBuilder::new_with_config(input, config).build(parser)
}
```

Document ordinary constructors as moderate and the configured constructors as
caller-policy paths.

- [ ] **Step 4: Run the safe-default test and verify GREEN**

Run the Step 2 command again.

Expected: the ordinary path returns a lossless recovery tree with an error
node.

- [ ] **Step 5: Add the failing explicit-unchecked test**

Add:

```rust
#[test]
fn unchecked_cst_entry_points_preserve_unlimited_behavior() {
    let input = nested_flow_sequence(257);

    let parsed = parse_to_cst_unchecked(&input);
    assert_eq!(parsed.to_text(), input);
    assert!(!contains_node_kind(&parsed, NodeKind::Error));

    let built = CstBuilder::new_unchecked(&input).build(Parser::new(&input));
    assert_eq!(built.to_text(), input);
    assert!(!contains_node_kind(&built, NodeKind::Error));
}
```

- [ ] **Step 6: Run the unchecked test and verify RED**

```bash
cargo nextest run -p yamalgam-cst \
  -E 'test(unchecked_cst_entry_points_preserve_unlimited_behavior)' \
  --success-output=immediate
```

Expected: compilation fails because `parse_to_cst_unchecked` and
`CstBuilder::new_unchecked` do not exist.

- [ ] **Step 7: Implement the explicit unchecked constructors**

Preserve const construction for the builder:

```rust
#[must_use]
pub const fn new_unchecked(source: &'input str) -> Self {
    Self {
        source,
        stack: Vec::new(),
        last_offset: 0,
        limits: LoaderConfig::unchecked().limits,
        collection_depth: 0,
    }
}
```

Add the free trusted-input path:

```rust
pub fn parse_to_cst_unchecked(input: &str) -> CstNode<'_> {
    parse_to_cst_with_config(input, &LoaderConfig::unchecked())
}
```

Document both unchecked constructors as trusted-input-only.

- [ ] **Step 8: Run both constructor-policy tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-cst -E \
  'test(ordinary_cst_entry_point_uses_moderate_limits) + test(unchecked_cst_entry_points_preserve_unlimited_behavior)' \
  --success-output=immediate
```

Expected: both tests pass.

---

### Task 2: Enforce collection depth inside `CstBuilder`

**Files:**

- Modify: `crates/yamalgam-cst/src/lib.rs`
- Test: `crates/yamalgam-cst/src/lib.rs`

**Interfaces:**

- Consumes: `ResourceLimits::check_depth`.
- Produces: builder-local protection for caller-supplied, unlimited parser
  event streams.
- Preserves: stream, document, mapping-entry, and sequence-entry wrappers do
  not count as YAML collection depth.

**Steps:**

- [ ] **Step 1: Add the failing custom-builder boundary test**

Add:

```rust
fn depth_config(max_depth: usize) -> LoaderConfig {
    LoaderConfig {
        limits: ResourceLimits {
            max_depth: Some(max_depth),
            ..ResourceLimits::none()
        },
        ..LoaderConfig::unchecked()
    }
}

#[test]
fn cst_builder_enforces_collection_depth_at_the_exact_boundary() {
    let config = depth_config(1);

    let exact_input = "[0]";
    let exact = CstBuilder::new_with_config(exact_input, &config)
        .build(Parser::new(exact_input));
    assert_eq!(exact.to_text(), exact_input);
    assert!(!contains_node_kind(&exact, NodeKind::Error));

    let over_input = "[[0]]";
    let over = CstBuilder::new_with_config(over_input, &config)
        .build(Parser::new(over_input));
    assert_eq!(over.to_text(), over_input);
    assert!(contains_node_kind(&over, NodeKind::Error));
}
```

- [ ] **Step 2: Run the custom-builder test and verify RED**

```bash
cargo nextest run -p yamalgam-cst \
  -E 'test(cst_builder_enforces_collection_depth_at_the_exact_boundary)' \
  --success-output=immediate
```

Expected: FAIL because an unchecked `Parser::new` event stream bypasses the
stored builder limits and the depth-two input has no error node.

- [ ] **Step 3: Make event processing stop on a local depth failure**

Change `process_event` to return `bool`. In `build`, break when it returns
`false`; retain the existing parse-error branch.

Add an `open_collection` helper that checks before changing `last_offset` or
opening a node:

```rust
fn open_collection(&mut self, kind: NodeKind, span: Span) -> bool {
    let Some(depth) = self.collection_depth.checked_add(1) else {
        self.handle_error(ParseError::LimitExceeded("depth overflow".into()));
        return false;
    };
    if let Err(message) = self.limits.check_depth(depth) {
        self.handle_error(ParseError::LimitExceeded(message));
        return false;
    }
    self.insert_whitespace_before(span);
    self.open_node(kind, span);
    self.collection_depth = depth;
    true
}
```

Use it for `SequenceStart` and `MappingStart`; return `false` immediately on
failure. Return `true` after all other successfully handled events. For the
existing implicit-empty-scalar early return, return `true`.

Centralize depth decrement in `close_node`: when a mapping or sequence is
successfully attached to its parent, decrement `collection_depth` with
`saturating_sub(1)`. This keeps normal and recovery closure paths consistent.
Do not count stream, document, or entry nodes.

On a local failure, `handle_error` appends the untouched source from
`last_offset` through end of input and sets `last_offset` to the input length.
Do not change its diagnostic-discard behavior in this action.

- [ ] **Step 4: Run both construction tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-cst -E \
  'test(ordinary_cst_entry_point_uses_moderate_limits) + test(cst_builder_enforces_collection_depth_at_the_exact_boundary)' \
  --success-output=immediate
```

Expected: both tests pass and every returned tree remains lossless.

---

### Task 3: Make public CST operations and destruction stack-safe

**Files:**

- Modify: `crates/yamalgam-cst/src/lib.rs`
- Test: `crates/yamalgam-cst/src/lib.rs`

**Interfaces:**

- Preserves: `to_text`, `node_at_offset`, `leaf_count`, `Display`, and normal
  drop behavior.
- Produces: no recursive call chain proportional to public `CstNode` depth.

**Steps:**

- [ ] **Step 1: Add an isolated deep-traversal regression**

Add a test that re-executes itself as a child process. The child keeps a pre-fix
stack overflow from aborting the parent test runner. It deliberately forgets
the tree after exercising traversal so recursive drop cannot mask the traversal
checkpoint:

```rust
const DEEP_CST_TRAVERSAL_WORKER: &str = "YAMALGAM_DEEP_CST_TRAVERSAL_WORKER";

fn deeply_nested_public_cst(depth: usize) -> CstNode<'static> {
    let span = Span {
        start: yamalgam_core::Mark::default(),
        end: yamalgam_core::Mark {
            offset: 1,
            ..Default::default()
        },
    };
    let mut node = CstNode {
        kind: NodeKind::Document,
        span,
        children: vec![CstElement::Token(CstToken {
            kind: TokenKind::Scalar,
            text: Cow::Borrowed("x"),
            span,
        })],
    };
    for _ in 0..depth {
        node = CstNode {
            kind: NodeKind::Sequence,
            span,
            children: vec![CstElement::Node(node)],
        };
    }
    node
}

fn assert_deep_worker_succeeds(environment: &str, test_name: &str) {
    let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .env(environment, "1")
        .args(["--exact", test_name, "--nocapture"])
        .output()
        .expect("run deep CST worker");
    assert!(
        output.status.success(),
        "deep CST worker failed: status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn deep_public_cst_traversals_are_stack_safe() {
    if std::env::var_os(DEEP_CST_TRAVERSAL_WORKER).is_some() {
        let node = deeply_nested_public_cst(100_000);
        assert_eq!(node.to_text(), "x");
        assert_eq!(node.to_string(), "x");
        assert_eq!(node.leaf_count(), 1);
        let deepest = node.node_at_offset(0).expect("offset in tree");
        assert!(matches!(deepest.children.as_slice(), [CstElement::Token(_)]));
        std::mem::forget(node);
        return;
    }

    assert_deep_worker_succeeds(
        DEEP_CST_TRAVERSAL_WORKER,
        "tests::deep_public_cst_traversals_are_stack_safe",
    );
}
```

- [ ] **Step 2: Run the deep-traversal test and verify RED**

```bash
cargo nextest run -p yamalgam-cst \
  -E 'test(deep_public_cst_traversals_are_stack_safe)' \
  --success-output=immediate
```

Expected: FAIL in the parent because the child process aborts on a recursive
traversal stack overflow.

- [ ] **Step 3: Replace recursive traversal with iteration**

Implement `to_text` and `leaf_count` with a `Vec<&CstElement>` LIFO worklist.
Seed and extend the worklist in reverse child order so popping preserves source
order. Remove `collect_text`.

Implement `node_at_offset` as an iterative descent. After rejecting an offset
outside the root span, repeatedly select the first direct child node containing
the offset. Return the current node when no child contains it. This preserves
the existing first-child depth-first result for overlapping spans.

- [ ] **Step 4: Run the deep-traversal test and verify GREEN**

Run the Step 2 command again.

Expected: the child completes text reconstruction, formatting, lookup, and
leaf counting at 100,000 nodes. `std::mem::forget` keeps recursive destruction
out of this checkpoint.

- [ ] **Step 5: Add the isolated deep-drop test**

Add a second worker mode and test:

```rust
const DEEP_CST_DROP_WORKER: &str = "YAMALGAM_DEEP_CST_DROP_WORKER";

#[test]
fn deep_public_cst_drop_is_stack_safe() {
    if std::env::var_os(DEEP_CST_DROP_WORKER).is_some() {
        drop(deeply_nested_public_cst(100_000));
        return;
    }

    assert_deep_worker_succeeds(
        DEEP_CST_DROP_WORKER,
        "tests::deep_public_cst_drop_is_stack_safe",
    );
}
```

- [ ] **Step 6: Run the deep-drop test and verify RED**

```bash
cargo nextest run -p yamalgam-cst \
  -E 'test(deep_public_cst_drop_is_stack_safe)' \
  --success-output=immediate
```

Expected: FAIL in the parent because the child process aborts during recursive
destruction.

- [ ] **Step 7: Add iterative destruction**

Implement safe custom drop:

```rust
impl Drop for CstNode<'_> {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        pending.append(&mut self.children);
        while let Some(element) = pending.pop() {
            if let CstElement::Node(mut node) = element {
                pending.append(&mut node.children);
            }
        }
    }
}
```

Each descendant node drops only after its children have been moved to the
worklist, so the compiler's ordinary safe drop glue never receives a deep
nested chain. Do not add `unsafe` or change public field types.

- [ ] **Step 8: Run both deep-tree tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-cst -E \
  'test(deep_public_cst_traversals_are_stack_safe) + test(deep_public_cst_drop_is_stack_safe)' \
  --success-output=immediate
```

Expected: both isolated children complete successfully at 100,000 nodes.

---

### Task 4: Verify and commit the CST implementation

**Files:**

- Modify: `crates/yamalgam-cst/src/lib.rs`

**Steps:**

- [ ] **Step 1: Run focused and crate gates**

```bash
just fmt
cargo nextest run -p yamalgam-cst
cargo test --doc -p yamalgam-cst
just clippy
git --no-pager diff --check
```

Expected: the CST suite, doctests, warning-denied workspace clippy, and diff
checks all pass.

- [ ] **Step 2: Obtain independent code review**

Review the complete uncommitted CST diff against this plan and the accepted
design. Fix every Critical or Important issue and reasonable Minor issue, then
rerun Step 1 and obtain PASS.

- [ ] **Step 3: Commit only the implementation**

```bash
git add crates/yamalgam-cst/src/lib.rs
git --no-pager diff --cached --check
git --no-pager diff --cached --name-only
git commit -m "fix(cst): bound construction and remove recursion"
git rev-parse HEAD
git --no-pager log --show-signature -1 --format=fuller
```

Expected: exactly the CST source is committed and signature output contains
`Good signature`. Record the full implementation SHA.

---

### Task 5: Append the CST action to the audit ledger

**Files:**

- Modify:
  `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`

**Interfaces:**

- Consumes: the real full implementation SHA from Task 4.
- Produces: status `fixed: 3`, `open: 24`, plus one new bottom entry.

**Steps:**

- [ ] **Step 1: Capture and verify the implementation SHA**

```bash
implementation_sha="$(git rev-parse HEAD)"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: the SHA names the CST code commit and signature output contains
`Good signature`.

- [ ] **Step 2: Update counts and append the action**

Change only the frontmatter counters to `fixed: 3` and `open: 24`. Append this
entry after the existing final `---`, writing `implementation_sha` after
`**Commit:**`:

```markdown
## 2026-08-06 — CST construction and traversal bounded

**Disposition:** fixed
**Addresses:** [cst-recursive-traversals-have-no-depth-bound](README.md#cst-recursive-traversals-have-no-depth-bound)
**Commit:** ${implementation_sha}
**Author:** Codex

Ordinary CST convenience and builder constructors now apply
`LoaderConfig::default()`, while configured and explicitly unchecked variants
preserve caller control. The builder enforces collection depth locally even
for caller-supplied unlimited parser events and preserves the complete source
inside the existing recovery tree on failure.

Whole-tree text reconstruction, offset lookup, leaf counting, formatting, and
destruction now use iterative traversal. Regression tests cover moderate and
unchecked construction, the exact custom depth boundary, lossless local
recovery, and a 100,000-node publicly constructed CST in an isolated process.

---
```

- [ ] **Step 3: Validate and commit the append-only update**

```bash
/Users/clay/.cargo/bin/rumdl check \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
implementation_sha="$(sed -n 's/^\*\*Commit:\*\* //p' \
  record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md | tail -1)"
test "${#implementation_sha}" -eq 40
git cat-file -e "${implementation_sha}^{commit}"
git --no-pager diff --check
git add record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md
git --no-pager diff --cached --name-only
git commit -m "docs(audit): record stack-safe CST remediation"
git --no-pager log --show-signature -1 --format=fuller
```

Expected: only the ledger is committed, its new entry retains the code SHA,
and the ledger commit has a good signature.
