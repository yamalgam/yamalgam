# Query Builder Subtree Height Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the query builder's repeated heap-backed depth walks while preserving exact depth enforcement for ordinary nodes and materialized aliases.

**Architecture:** Make the private builder carry a `subtree_height` beside each completed `YamlVal`. Compute scalar height as zero and collection height as one plus the maximum child height. Cache the same pair in the anchor table so an alias can validate `placement_depth + target_height` without traversing the cloned target. Keep `YamlVal::check_depth` unchanged for runtime-created values and other non-parser consumers.

**Tech Stack:** Rust 2024, `yamalgam-core::ResourceLimits`, `cargo nextest`, Just.

## Global Constraints

- Preserve all public query APIs, `YamlVal` representation, metadata, anchor scoping, and error variants/messages.
- Preserve depth semantics: depth zero is the document root and a node exactly at `max_depth` is accepted.
- Validate materialized aliases at their placement depth, including aliases to anchored collections that already contain aliases.
- Use checked arithmetic for both child depth and subtree height.
- Do not add dependencies or edit immutable audit source artifacts.
- Follow RED -> GREEN with focused tests, then run the full query suite and `just check`.

---

### Task 1: Lock the no-rewalk and alias-boundary contracts

**Files:**

Modify `crates/yamalgam-query/src/value.rs` test-only support around `YamlVal::check_depth`.

Modify `crates/yamalgam-query/src/build.rs` unit tests.

Modify `crates/yamalgam-query/tests/value.rs` alias-depth integration tests.

- [ ] **Step 1: Add test-only depth-walk observation**

Add a thread-local counter under `#[cfg(test)]` in `value.rs`. Increment it at the entry to `YamlVal::check_depth`, and expose crate-private reset/read helpers only to unit tests. Thread-local state keeps parallel nextest cases isolated and adds no production code.

- [ ] **Step 2: Add a failing builder no-rewalk regression**

In `build.rs`, parse a nested document containing anchored collections and aliases, then assert that builder parsing invoked `YamlVal::check_depth` zero times. The current implementation must fail because `Builder::finish` invokes the whole-tree walk for every completed node.

- [ ] **Step 3: Add exact alias-expanded depth boundary coverage**

Extend `tests/value.rs` so the existing nested-anchor document is rejected at `max_depth = 3` and accepted at `max_depth = 4`. Assert the accepted alias metadata and resulting value shape remain intact.

Add exact-boundary cases for anchored empty sequences and mappings aliased at depth three; anchored mappings whose deepest descendant is respectively a collection key and a collection value; a deep duplicate mapping value overwritten by a shallow value; and same-name anchor redefinition from a deep value to a scalar under `max_anchor_count = 1`, followed by a second distinct anchor that must fail the count limit. These prove empty collection height is zero, both mapping sides contribute, overwritten entries do not contribute, and anchor redefinition replaces both value and height.

- [ ] **Step 4: Run focused tests to verify RED**

Run:

```bash
cargo nextest run -p yamalgam-query -E 'test(builder_does_not_rewalk_completed_values) | test(builder_enforces_resulting_alias_value_depth)'
```

Expected: `builder_does_not_rewalk_completed_values` fails with a positive depth-walk count; the existing semantic rejection remains green.

---

### Task 2: Carry subtree height through the private builder

**Files:**

Modify `crates/yamalgam-query/src/build.rs:94-345`.

- [ ] **Step 1: Introduce a private completed-node carrier**

Add a private `BuiltNode { value: YamlVal, subtree_height: usize }`. Change the anchor table to `HashMap<String, BuiltNode>` and make `Builder::node` and `Builder::finish` return `BuiltNode`. At the `Documents` boundary, return only `BuiltNode::value` so the public iterator item remains unchanged.

- [ ] **Step 2: Compute collection heights during the existing recursive build**

For a scalar, pass height zero to `finish`. For sequences, unpack each child `BuiltNode`, insert only its value, and retain the maximum child height. Compute the collection height with `checked_add(1)`; empty collections have height zero because they contain no deeper node.

For mappings, keep a private `Vec<(key_height, value_height)>` aligned with the surviving `IndexMap` entry order. Before insertion, locate an equal existing key. A new key appends both heights; a duplicate retains the original key height, replaces only the value height, and preserves the existing map replacement semantics. Compute the maximum from this final aligned height vector, never from children that were overwritten.

Use checked addition when deriving `child_depth`. Map overflow to the existing `BuildError::Limit` class with stable depth-oriented text.

- [ ] **Step 3: Validate aliases from cached target height**

Clone the anchored `BuiltNode`, compute `depth.checked_add(subtree_height)`, and pass that deepest materialized depth through `ResourceLimits::check_depth`. Then update the cloned root metadata exactly as today (`anchor = None`, `alias_of = name`) and return the unchanged cached height with the value.

Before storing an anchor, compute the prospective distinct count with checked arithmetic as `anchors.len() + usize::from(!anchors.contains_key(name))`, validate that count, then insert or replace the completed `BuiltNode`. This preserves same-name redefinition while replacing both value and cached height. Preserve document-boundary clearing of both fields.

- [ ] **Step 4: Remove builder whole-tree checks**

Delete the `node.check_depth(...)` call from `Builder::finish`. Do not remove or weaken `YamlVal::check_depth`; engine, construction, emission, and conversion paths still require it.

- [ ] **Step 5: Run focused tests to verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-query -E 'test(builder_does_not_rewalk_completed_values) | test(builder_enforces_resulting_alias_value_depth)'
```

Expected: both tests pass, including exact acceptance at depth four and zero builder depth walks.

---

### Task 3: Verify compatibility and close the finding

**Files:**

Modify only regressions required by confirmed failures.

- [ ] **Step 1: Run the complete query suite**

```bash
cargo nextest run -p yamalgam-query
```

- [ ] **Step 2: Run repository gates**

```bash
just check
git --no-pager diff --check
```

- [ ] **Step 3: Request independent code review**

Review depth arithmetic, empty collections, mapping keys and values, duplicate-key replacement, same-name anchor redefinition, nested anchored aliases, metadata compatibility, anchor-count failure ordering, and the test-only observation mechanism. Address every confirmed issue and rerun affected gates.

- [ ] **Step 4: Commit code, then append the immutable ledger**

Create a signed focused code commit. Record its exact full SHA in a new bottom entry of `record/audits/2026-08-04-16-m10-post-remediation/actions-taken.md`, update counters from fixed 12/open 15 to fixed 13/open 14, independently review the append, and create a separate signed ledger commit.
