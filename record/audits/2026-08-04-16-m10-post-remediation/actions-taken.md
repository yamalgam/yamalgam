---
audit: 2026-08-04-16-m10-post-remediation
last_updated: 2026-08-08
status:
  fixed: 3
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 24
---

# Actions Taken: Full Repository Post-Remediation Audit

Summary of remediation status for the
[2026-08-04 full repository post-remediation audit](README.md).

---

## 2026-08-06 — Default composition bounded

**Disposition:** fixed
**Addresses:** [composer-default-path-has-unbounded-recursion](README.md#composer-default-path-has-unbounded-recursion)
**Commit:** 809f867b90dac17d34f43d30309331fbafa4abb0
**Author:** Codex

Ordinary composer string and event-stream constructors now apply
`LoaderConfig::default()`. Explicitly named unchecked entry points preserve
trusted-input compatibility without making unlimited loading the default.

The composer also enforces depth and collection limits locally, including raw
mapping pairs and materialized merge entries, so caller-supplied event streams
cannot bypass parser-side controls. Regression tests cover default and
unchecked APIs, exact custom-depth behavior, duplicate mapping entries,
sequences, and merged mappings.

---

## 2026-08-06 — Serde defaults bounded

**Disposition:** fixed
**Addresses:** [serde-default-disables-loader-limits](README.md#serde-default-disables-loader-limits)
**Commit:** 0c18a346c54ac3a001762562301ef1adfe48c0a2
**Author:** Codex

Ordinary serde string, reader, and document-stream entry points now apply
`LoaderConfig::default()`. Explicit unchecked entry points preserve the prior
unlimited behavior for trusted input.

The free functions share one single-document driver, so structured error
restoration and end-of-document checks remain consistent across default,
configured, and unchecked paths. Regression tests cover default rejection and
unchecked acceptance beyond the moderate depth limit.

---

## 2026-08-08 — CST construction and traversal bounded

**Disposition:** fixed
**Addresses:** [cst-recursive-traversals-have-no-depth-bound](README.md#cst-recursive-traversals-have-no-depth-bound)
**Commit:** 5a71b87c590c1fbdd72dbe233d9965959f1f0c78
**Author:** Codex

Ordinary CST convenience and builder constructors now apply moderate resource
limits, while configured and explicitly unchecked variants preserve caller
control. The builder enforces collection depth locally even for caller-supplied
unlimited parser events and preserves the complete source in the existing
recovery tree on failure.

Whole-tree text reconstruction, offset lookup, leaf counting, formatting, and
destruction now use iterative traversal. Regression tests cover moderate and
unchecked construction, the exact custom depth boundary, lossless local
recovery, and a 100,000-node publicly constructed CST in isolated processes.

---
