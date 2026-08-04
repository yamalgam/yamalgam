---
audit: 2026-08-03-21-m10-fresh
last_updated: 2026-08-03
status:
  fixed: 7
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 40
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

---

## 2026-08-03 — Runtime value display bounded

**Disposition:** fixed
**Addresses:** [explode-aliases-unbounded-expansion](README.md#explode-aliases-unbounded-expansion)
**Commit:** 133bf2cb994f50558919505d1452d895554637f3
**Author:** Codex

Superseded the earlier mitigation by routing `Display for YamlVal` through a
bounded buffer using the active runtime limits. Error formatting now stops
walking once `max_output_bytes` is reached and substitutes a truncation marker
instead of descending beyond `max_depth`; unlimited profiles retain the prior
behavior.

Regression tests exercise both the logical alias-tree output cap and recursive
depth truncation. Together with commit `e95ce8b`, this closes both amplification
paths recorded by the finding.

---

## 2026-08-03 — Scalar text allocation shared

**Disposition:** fixed
**Addresses:**
[build-raw-metadata-duplicates-every-scalar](README.md#build-raw-metadata-duplicates-every-scalar),
[resolver-allocates-throwaway-string-per-scalar](README.md#resolver-allocates-throwaway-string-per-scalar)
**Commit:** f236453974b937c6fb15e7a12774ba6f209db3f8
**Author:** Codex

The builder now allocates scalar source text once as `Rc<str>` and shares that
allocation between `Data::Str` and `Meta::raw`. Typed scalars retain the single
raw-text allocation while storing their resolved non-string value.

Plain and explicitly typed scalar paths consult the allocation-free
`resolves_as_string` predicate before full resolution, avoiding the temporary
owned `Value::String` that was immediately copied and dropped. A pointer
identity regression test covers plain, quoted, explicitly string-tagged, and
failed typed-tag string paths.

---

## 2026-08-03 — Extended control characters escaped

**Disposition:** fixed
**Addresses:**
[emitter-omits-c1-control-escapes](README.md#emitter-omits-c1-control-escapes)
**Commit:** 26c243fcf8c045277e4dbb43c28034c8e93d96b2
**Author:** Codex

YAML double-quoted output now escapes DEL and the C1 control range as `\xNN`,
and classifies U+2028/U+2029 for emission as `\L`/`\P`. JSON output mirrors the
extended-control handling with `\u00NN` escapes.

Regression tests cover both emitters at the control-range boundaries, NEL,
and the Unicode line and paragraph separators. The full `yamalgam-query` suite
and clippy gate remain green.

---

## 2026-08-03 — YAML document markers quoted

**Disposition:** fixed
**Addresses:**
[emitter-plain-document-end-marker](README.md#emitter-plain-document-end-marker)
**Commit:** ba9243024ad96c1d35d7a8aadf25ffedf09e728e
**Author:** Codex

The centralized scalar classifier now requires quoting for exact `...` and
`---` strings and marker prefixes followed by whitespace. Root scalar output
can no longer be reinterpreted as a YAML document boundary.

A focused regression test asserts quoted output and parse/emit round trips for
both document markers and their followed-by-text forms. All 94 query tests and
the clippy gate pass.

---

## 2026-08-03 — Runtime value growth bounded

**Disposition:** fixed
**Addresses:**
[growth-operators-bypass-runtime-limits](README.md#growth-operators-bypass-runtime-limits)
**Commit:** ac28cf65681e061f5adbc672ff6fa49992b9908a
**Author:** Codex

String concatenation and sequence/mapping addition now check their prospective
sizes before allocating or extending. Fallible mapping and `map_values`
constructors enforce the same active collection cap incrementally.

For jaq's infallible array-construction contract, the runtime now stops at the
first over-limit item, records a scoped pending error, and makes the engine
return that violation before accepting output. Tests cover all five growth
paths under deliberately small limits; all 96 query tests and clippy pass.
