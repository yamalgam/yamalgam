---
audit: 2026-08-09-13-m11-final-review
last_updated: 2026-08-09
status:
  fixed: 2
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 18
---

# Actions Taken: M11 External Resource Resolution

Summary of remediation status for the
[2026-08-09 M11 external-resource-resolution audit](README.md).

---

## 2026-08-09 — close URI selector dispatch gaps

**Disposition:** fixed

**Addresses:**
[scheme-relative-ref-becomes-local-file](README.md#scheme-relative-ref-becomes-local-file),
[include-uri-bypasses-relative-selector-contract](README.md#include-uri-bypasses-relative-selector-contract)

**Commit:** `88ab70284a75e95b5c0b0eafed4d04d7e5c205a1`

**Author:** Codex

URI-base resolution now recognizes RFC 3986 network-path references before
platform path dispatch, while retaining the existing absolute-file behavior.
`!include` validation rejects all absolute URI selectors before retrieval or
discovery. Regression coverage exercises scheme inheritance and file, memory,
custom, and HTTP include schemes.
