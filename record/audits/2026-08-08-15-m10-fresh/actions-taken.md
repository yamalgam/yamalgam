---
audit: 2026-08-08-15-m10-fresh
last_updated: 2026-08-08
status:
  fixed: 1
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 4
---

# Actions Taken: Full Repository Static Audit

Remediation status for the [2026-08-08 full repository static audit](README.md).

---

## 2026-08-08 — Reject invalid tree scalar styles

**Disposition:** fixed
**Addresses:** [compare-tree-parser-panics-on-nonascii-style](README.md#compare-tree-parser-panics-on-nonascii-style)
**Commit:** `b8c97e6752bcdbb0c525facbf4a80c3b35778ffc`
**Author:** Codex

`parse_val` now accepts only the five ASCII style indicators defined by the
YAML Test Suite tree format. Unsupported and multibyte indicators return no
event instead of reaching a byte-one string slice. A regression covers the
previous `parse_tree("=VALé")` panic, while the existing tests retain coverage
for every accepted style.

Verification: focused regression passed; `just check` passed with 1,929 tests,
Clippy with warnings denied, Cargo Deny, formatting, and doc tests. Independent
code review returned PASS.
