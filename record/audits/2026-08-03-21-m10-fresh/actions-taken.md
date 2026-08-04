---
audit: 2026-08-03-21-m10-fresh
last_updated: 2026-08-03
status:
  fixed: 1
  mitigated: 1
  accepted: 0
  disputed: 0
  deferred: 0
  open: 45
---

# Actions Taken: M10 milestone surface — 24 commits on branch m10 Yg Cli vs main (merge Base 6e3a646): the new yamalgam Query crate, the yg CLI, and M10's yamalgam Core tag Resolution additions

Summary of remediation status for the [2026-08-03 M10 milestone surface — 24 commits on branch m10-yg-cli vs main (merge-base 6e3a646): the new yamalgam-query crate, the yg CLI, and M10's yamalgam-core tag-resolution additions audit](README.md).

---

## 2026-08-03 — Unsafe YAML tag replay rejected

**Disposition:** fixed
**Addresses:** [emitter-replays-tags-verbatim](README.md#emitter-replays-tags-verbatim)
**Commit:** b7282b6c1d7f23199b756739d1cb18a576abba17
**Author:** Codex

Added a representation-aware tag validator in
`crates/yamalgam-query/src/value.rs` and applied it at both metadata
boundaries. `set_tag` now rejects tag strings that cannot be emitted safely,
while the YAML emitter omits unsafe tag metadata decoded from hostile input
instead of replaying it as YAML structure.

Regression tests cover the percent-decoded newline injection path and invalid
`set_tag` arguments. Existing local and core tag round trips remain covered by
the emitter suite.

---

## 2026-08-03 — Alias materialization bounded

**Disposition:** mitigated
**Addresses:** [explode-aliases-unbounded-expansion](README.md#explode-aliases-unbounded-expansion)
**Commit:** e95ce8b19b087d84a08ba63d77d587e799600608
**Author:** Codex

Threaded the active `ResourceLimits` into `explode_aliases` and made expansion
fallible. A per-invocation budget now enforces logical alias expansions, total
materialized collection entries, and recursion depth before copying the value
tree. Focused tests exercise all three limits, and the complete
`yamalgam-query` suite remains green.

The finding is mitigated rather than fixed because `Display for YamlVal` still
walks the logical alias tree without a rendering budget. A follow-up action
must bound that error-formatting path before this finding can be closed.
