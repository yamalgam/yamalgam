---
audit: 2026-08-11-07-m12-full-repo
last_updated: 2026-08-11
status:
  fixed: 5
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 15
---

# Actions Taken: M12 full-repository audit

Remediation status for the [2026-08-11 M12 full-repository audit](README.md).

---

## 2026-08-11 — Preserve leading document-marker semantics

**Disposition:** fixed  
**Addresses:** [leading-document-marker-shifts-lsp-document-indexes](README.md#leading-document-marker-shifts-lsp-document-indexes)  
**Commit:** 1b3b14b726f37941c203c8847c026475aac38a12  
**Author:** Codex

`DocumentState` now derives a byte offset's semantic document index from the
recovering CST's `Document` nodes. Completion, hover, definition, and syntax
diagnostics all use that shared boundary, so an initial `---` remains part of
document zero while later markers advance normally. Four consumer-level
regressions cover the boundary; `just test-lsp` passes 18 tests.

---

## 2026-08-11 — Resolve sequence-item schema context

**Disposition:** fixed  
**Addresses:** [sequence-item-schema-context-is-unreachable](README.md#sequence-item-schema-context-is-unreachable)  
**Commit:** 69ea147b3c89544a8153d034c03abfb01e906fe8  
**Author:** Codex

Completion and hover now derive JSON Pointer paths from recovering CST
ancestry. Block sequence entries use their semantic sibling index; flow
sequences count their direct comma separators. The fallback probe stays inside
the current line, preserving partial `- ` recovery without inheriting the
previous mapping entry. Block completion, flow completion, and sequence-value
hover regressions pass with the full 21-test LSP suite and workspace Clippy.

---

## 2026-08-11 — Align Ropey line boundaries with LSP

**Disposition:** fixed

**Addresses:** [ropey-unicode-lines-desynchronize-lsp-positions](README.md#ropey-unicode-lines-desynchronize-lsp-positions)

**Commit:** 53da91348fd819edbf437d1287d35048a24db1ec

**Author:** Codex

The CST now disables Ropey's Unicode-line feature while retaining CR-line and
SIMD support. LSP UTF-16 conversions therefore treat U+2028 and U+2029 as
content while honoring CR, LF, and CRLF as line boundaries. A bidirectional
regression passes with all 54 CST tests, all 21 LSP tests, and workspace Clippy.

---

## 2026-08-11 — Render schema examples in hover

**Disposition:** fixed

**Addresses:** [documented-example-hover-is-not-implemented](README.md#documented-example-hover-is-not-implemented)

**Commit:** ef26ca4b44d7b9515ff444c421a4aa626ee4a1a1

**Author:** Codex

Schema navigation now collects the first eight unique JSON Schema examples in
deterministic traversal order, including examples contributed by combinators.
Hover renders those compact JSON values as plain text, so an example-only
schema produces useful output. The regression passes with all 41 schema tests,
all 22 LSP tests, and workspace Clippy.

---

## 2026-08-11 — Make LSP request cancellation responsive

**Disposition:** fixed

**Addresses:** [lsp-cancellation-cannot-interrupt-work-and-retains-request-ids](README.md#lsp-cancellation-cannot-interrupt-work-and-retains-request-ids)

**Commit:** 8f5e87ff44ed9d539dd5ca3f29a2fd982a06897c

**Author:** Codex

The receiver loop now dispatches request work to a bounded four-worker queue
over immutable document snapshots. Active request generations carry
cooperative cancellation flags; cancellation responds immediately, suppresses
stale completion, and removes the exact generation so reused IDs remain clean.
Unknown and late cancellation IDs are ignored. The lifecycle and deterministic
held-work regressions pass with all 23 LSP tests and workspace Clippy.

---
