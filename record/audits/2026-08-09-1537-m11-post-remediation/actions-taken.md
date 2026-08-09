---
audit: 2026-08-09-1537-m11-post-remediation
last_updated: 2026-08-09
status:
  fixed: 11
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 0
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

---

## 2026-08-09 — Preserve Reports and Caught Errors

**Disposition:** fixed
**Addresses:**
[caught-resolution-errors-accumulate-until-run-end](README.md#caught-resolution-errors-accumulate-until-run-end),
[post-resolution-conversion-drops-session-report](README.md#post-resolution-conversion-drops-session-report),
[direct-include-preflight-can-omit-the-failed-report-edge](README.md#direct-include-preflight-can-omit-failed-report-edge)
**Commit:** d40ded1b0f145718873899a01c1d7ab21931c294
**Author:** Codex

Explicit query failures now carry their typed resolver error in an immutable,
reference-counted `YamlVal` payload that is excluded from value semantics.
Caught failures are released with their jaq error value instead of accumulating
in a run-scoped token map.

The query adapter now wraps post-resolution conversion failures with their
structured source and all completed/current resolution reports. It also checks
the converted value's expanded depth, including alias expansion. Direct path
includes reserve a sanitized report edge before resource preflight, so every
failure has a terminal edge outcome. ADR 0019 and the M11 design and plan now
record these report-lifecycle guarantees.

---

## 2026-08-09 — Make Query Resolution Walks Stack Safe

**Disposition:** fixed
**Addresses:**
[resolved-owned-tree-conversion-remains-recursive](README.md#resolved-owned-tree-conversion-remains-recursive),
[query-session-assignment-recurses-without-a-bound](README.md#query-session-assignment-recurses-without-a-bound)
**Commit:** 3647cb1a742e19ef79d7f77d3000cd2ead12b608
**Author:** Codex

Owned-tree to query-value conversion and query-session assignment now rebuild
sequences and mappings through explicit work stacks. The conversion preserves
anchor visibility, mapping order, presentation metadata, resource origins, and
typed depth failures; assignment preserves mapping keys and `Rc` copy-on-write
isolation.

Regressions exercise successful unchecked conversion, depth-limit cleanup, and
session assignment at 20,000 nested sequence levels on a 128 KiB thread stack.
A focused mapping regression verifies key order, origin updates, and isolation
of a shared pre-assignment clone. The M11 design and implementation plan now
state the stack-safety guarantee.
