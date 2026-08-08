# Scanner fail:true Rejection Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix all 11 EVENT_UNEXPECTED compliance failures — `fail:true` inputs the scanner incorrectly accepts.

**Architecture:** Add document-state tracking fields to Scanner, validate directives/tags/structure during tokenization. All fixes in `yamalgam-scanner` crate — no parser changes.

**Tech Stack:** Rust, yamalgam-scanner, cargo nextest

---

## New Scanner State Fields

All fixes share these additions to `Scanner<'input>`:

```rust
/// Whether we're in the directive prologue (stream start, or after `...`).
/// Directives (`%YAML`, `%TAG`) are only valid here.
in_directive_prologue: bool,           // init: true
/// Whether any directive was seen in the current prologue.
seen_directive: bool,                  // init: false
/// Whether `%YAML` was seen in the current prologue (for duplicate detection).
seen_yaml_directive: bool,             // init: false
/// Line of most recent `---` (for same-line block collection rejection).
document_start_line: Option<u32>,      // init: None
/// Root-level content already complete (standalone scalar or flow collection at indent -1).
root_token_emitted: bool,              // init: false
/// Named tag handles registered via `%TAG` in current directive prologue.
tag_handles: Vec<String>,              // init: empty
```

### State Transitions

| Event | `in_directive_prologue` | `seen_directive` | `seen_yaml_directive` | `document_start_line` | `root_token_emitted` | `tag_handles` |
|-------|------------------------|------------------|-----------------------|-----------------------|---------------------|---------------|
| `---` | false | false | false | Some(line) | false | — (keep) |
| `...` | true | false | false | None | false | clear |
| `%YAML`/`%TAG` | — | true | (YAML only) | — | — | (TAG: push) |
| Content start | false | — | — | — | — | — |

---

## Task 1: Add scanner fields + directive validation (SF5V, 9MMA, 9HCY, EB22, RHX7)

**Files:**
- Modify: `crates/yamalgam-scanner/src/scanner.rs`
- Test: `crates/yamalgam-scanner/tests/scanner.rs`

**5 fixes:**
1. `SF5V` — duplicate `%YAML`: in `fetch_version_directive`, check `seen_yaml_directive`
2. `9MMA` — directive-only stream: at EOF, if `seen_directive && in_directive_prologue` → error
3. `9HCY` — directive without `...`: at `%` col 0, if `!in_directive_prologue` → error
4. `EB22` — same as 9HCY
5. `RHX7` — same as 9HCY

**Validation points:**
- `%` at col 0 in `fetch_next_token`: check `in_directive_prologue`
- `fetch_version_directive`: check `seen_yaml_directive`
- EOF in `fetch_next_token`: check `seen_directive && in_directive_prologue`
- `fetch_document_indicator(DocumentStart)`: reset state
- `fetch_document_indicator(DocumentEnd)`: reset state
- Content fallthrough in `fetch_next_token`: set `in_directive_prologue = false`

## Task 2: Block collection on `---` line (9KBC, CXX2)

**Files:** same as Task 1

In `roll_indent`, when pushing a new indent level, check:
```rust
if self.document_start_line == Some(self.reader.mark().line) {
    // Block collections cannot start on the `---` line (YAML §7.3.2)
    self.error = Some(ScanError { message: "block collection on document start line" });
    return;
}
```

## Task 3: Multiline flow key in block context (C2SP)

**Files:** same as Task 1

In `fetch_flow_collection_end`, when `flow_level` reaches 0, update the outer simple key's `end_line`:
```rust
if self.flow_level == 0 {
    if let Some(sk) = self.simple_keys.last_mut().filter(|s| s.flow_level == 0) {
        sk.end_line = end.line;
    }
}
```
The existing multiline key check in `fetch_value` then catches `[23\n]: 42`.

## Task 4: Tag handle scope across documents (QLJ7)

**Files:** same as Task 1

In `fetch_tag`, after scanning the tag text, extract handle portion:
- `!<uri>` → verbatim, skip validation
- `!suffix` or bare `!` → primary handle, always valid
- `!!suffix` → secondary handle, always valid
- `!name!suffix` → named handle, check `tag_handles` contains `!name!`

In `fetch_tag_directive`, extract handle and push to `tag_handles`.
On `...`, clear `tag_handles`.
On `---`, clear `tag_handles` (handles from previous doc's prologue are consumed, new doc needs fresh ones — but handles declared before THIS `---` are valid for this doc, so clear AFTER the doc they apply to ends).

Wait — `%TAG` directives come BEFORE `---`. The handles are valid for the document that follows. On `---`, keep the handles (they apply to this doc). On `...`, clear them (doc is over). On the NEXT `---` after content, the handles from the previous prologue were already consumed. Actually:

- `%TAG !p! ...` → push `!p!` to tag_handles
- `---` → handles are now active for this document (keep them)
- content uses `!p!A` → valid
- `---` (new doc) → clear tag_handles (previous doc's handles expire)

So: clear `tag_handles` on `---` BEFORE registering any new handles. But `%TAG` comes before `---`. The flow is:
1. `%TAG` → push handle
2. `---` → DON'T clear (handles apply to this doc)
3. content
4. `---` (new doc) → clear handles
5. `!p!B` → handle not found → error

Actually the simplest: clear `tag_handles` on BOTH `---` and `...`, but process `%TAG` ADDS after the clear. Since `%TAG` always comes before `---`, the sequence is: `%TAG`(push) → `---`(clear) → content. That would clear the just-registered handles!

Correct approach: `---` should NOT clear handles. Only `...` should clear them. And at `---`, if there were NO new `%TAG` directives (i.e., no prologue before this `---`), the handles from the previous doc are invalid.

Better: track handles separately for "pending" (from current prologue) and "active" (for current document). Or simpler: clear on `...` and also clear when `in_directive_prologue` transitions to false via content (implicit doc start) — because implicit docs have no directives.

Simplest correct approach:
- `tag_handles` accumulates handles from `%TAG` directives
- On `---`: if `in_directive_prologue` was true (we were in prologue), keep handles (they apply to this doc). Reset `in_directive_prologue` to false.
- On `...`: clear `tag_handles`, set `in_directive_prologue` to true
- On implicit content start (no `---`): clear `tag_handles` (no directives for implicit doc)

Wait, but what about multiple `---` without `...`?
```yaml
%TAG !p! tag:one/
--- !p!A
a: b
--- !p!B     ← should FAIL (no %TAG before this ---)
c: d
```

When second `---` is seen:
- `in_directive_prologue` is false (we're in a document)
- So this `---` starts a new document
- We should clear `tag_handles` because no new `%TAG` was issued

So: on `---`, clear `tag_handles` UNLESS we're in the directive prologue (where `%TAG` was just registered).

```rust
// In DocumentStart handler:
if !self.in_directive_prologue {
    // New document without directive prologue — no tag handles carry over
    self.tag_handles.clear();
}
// Always reset these:
self.in_directive_prologue = false;
self.seen_directive = false;
self.seen_yaml_directive = false;
```

For QLJ7: `%TAG !prefix! ...\n--- !prefix!A\n...\n--- !prefix!B\n...`
Wait, QLJ7 doesn't have `...` between docs. It has: `%TAG ... --- ... --- ... ---`
1. `%TAG !prefix! ...` → tag_handles = ["!prefix!"]
2. `---` → in_directive_prologue was true → keep handles. in_directive_prologue=false
3. `!prefix!A` → handle found → OK
4. `---` → in_directive_prologue was false → clear handles. in_directive_prologue=false
5. `!prefix!B` → handle NOT found → ERROR ✓

## Task 5: Root-level extra content (BS4K, KS4U)

**Files:** same as Task 1

Set `root_token_emitted = true` when:
1. In `fetch_flow_collection_end`: flow_level reaches 0, indent == -1, and no `:` follows
2. In `purge_stale_simple_keys`: when purging a non-required key at flow_level==0 with queue token at indent==-1 (standalone scalar at root)

Check in `fetch_next_token` after col-0 block:
```rust
if self.root_token_emitted && self.indent == -1 && self.flow_level == 0 {
    return Some(Err(ScanError { message: "extra content after document root node" }));
}
```

Reset `root_token_emitted` on `---`, `...`, and when `roll_indent` pushes indent from -1.

---

## Verification

After all fixes:
```
cargo nextest run -p yamalgam-scanner
cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | grep -oE "^    (PASS|UNEXPECTED|MISMATCH|EXPECTED)" | sort | uniq -c | sort -rn
```

Target: 0 EVENT_UNEXPECTED (down from 11), 0 regressions in existing PASS tests.
