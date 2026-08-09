---
audit: 2026-08-09-1537-m11-post-remediation
last_updated: 2026-08-09
status:
  fixed: 6
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 5
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

---

## 2026-08-09 — Bound Session Cache and Discovery Work

**Disposition:** fixed
**Addresses:**
[resolution-cache-bypasses-session-byte-budget](README.md#resolution-cache-bypasses-session-byte-budget),
[glob-discovery-scans-entire-base](README.md#glob-discovery-scans-entire-base),
[symlink-aliases-consume-match-limit-before-deduplication](README.md#symlink-aliases-consume-match-limit-before-deduplication)
**Commit:** 003c97c373b5737f974ebc2b26cf9192904ec0da
**Author:** Codex

Resolution sessions now enforce cumulative fetched-byte and discovery-visit
budgets across both cached and newly retrieved resources. Discovery begins at
the glob's static path prefix, bounds directory and entry visits, and reports
the actual work performed to the session.

File discovery policy-checks every alias path while counting canonicalized,
unique resources toward the match ceiling. Regression coverage exercises
cross-resource byte exhaustion, cumulative discovery work, bounded static
prefix traversal, and duplicate symlink aliases. ADR 0016 and the M11 design
and implementation plan now document the two session-wide budgets.
