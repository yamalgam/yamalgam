---
audit: 2026-08-11-07-m12-full-repo
last_updated: 2026-08-11
status:
  fixed: 2
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 18
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
