---
audit: 2026-08-09-1537-m11-post-remediation
last_updated: 2026-08-09
status:
  fixed: 3
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 8
---

# Actions Taken: M11 External Resolution Changes from c85f7e0 through 246f707

Summary of remediation status for the
[2026-08-09 M11 external-resolution changes from c85f7e0 through 246f707 audit](README.md).

---

## 2026-08-09 — Unify Resolver Policy Boundaries

**Disposition:** fixed
**Addresses:**
[relative-includes-can-retrieve-non-file-schemes](README.md#relative-includes-can-retrieve-non-file-schemes),
[explicit-only-sessions-skip-root-anchor-reservation](README.md#explicit-only-sessions-skip-root-anchor-reservation),
[builtin-resolver-mixes-caller-and-session-configs](README.md#builtin-resolver-mixes-caller-and-session-configs)
**Commit:** 949ee80fce7d50ce14cd1974181cd6aca6aaedb8
**Author:** Codex

`ResolutionSession` now rejects every non-file include identity before
retrieval or discovery, and `prime_root` reserves anchors from the complete
root before an explicit subtree can import content. `BuiltinResolver` also
rejects any downstream `LoaderConfig` that differs from the configuration
owned by its session, before buffering begins.

Regression coverage exercises non-file relative includes, explicit-only
anchor collisions, and mode, root, host, and limit configuration mismatches.
Memory-backed consumer fixtures now use file identities, preserving embedded
retrieval without weakening the include identity boundary.
