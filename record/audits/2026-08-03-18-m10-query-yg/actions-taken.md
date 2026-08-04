---
audit: .
last_updated: 2026-08-03
status:
  fixed: 19
  mitigated: 1
  accepted: 0
  disputed: 0
  deferred: 0
  open: 16
---

# Actions Taken: M10 surface — the yamalgam Query crate and the yg binary, as introduced across main...m10 Yg Cli (21 commits, ~5k lines of new src, ~1.3k lines of tests)

Summary of remediation status for the [2026-08-03 M10 surface — the yamalgam-query crate and the yg binary, as introduced across main...m10-yg-cli (21 commits, ~5k lines of new src, ~1.3k lines of tests) audit](README.md).

---

## 2026-08-03 — Harden YAML emission and mutation metadata

**Disposition:** fixed
**Addresses:** [flow-context-plain-scalar-injection](README.md#flow-context-plain-scalar-injection), [alias-metadata-survives-mutation-dropping-edits](README.md#alias-metadata-survives-mutation-dropping-edits), [emitter-quoting-ignores-active-schema](README.md#emitter-quoting-ignores-active-schema), [json-nonstring-keys-collide](README.md#json-nonstring-keys-collide), [plain-scalar-control-characters](README.md#plain-scalar-control-characters), [stale-raw-metadata-after-string-splice](README.md#stale-raw-metadata-after-string-splice)
**Commit:** 1cf84ed8f45530e2e76a490b1bc8506c4011d3b6
**Author:** Clay Loveless and Codex

Made scalar quoting aware of flow context and the active YAML schema, escaped control characters and apostrophes correctly, and invalidated anchor, alias, and raw-scalar metadata whenever a value is mutated. JSON emission now rejects mapping keys that collide after string conversion instead of silently overwriting a value. Regression tests cover each malformed-output and metadata-loss path.

---

## 2026-08-03 — Make in-place editing staged and conservative

**Disposition:** fixed
**Addresses:** [in-place-empty-output-truncates-file](README.md#in-place-empty-output-truncates-file), [in-place-replaces-symlinks-and-drops-ownership](README.md#in-place-replaces-symlinks-and-drops-ownership), [in-place-multi-file-partial-application](README.md#in-place-multi-file-partial-application), [slurp-silently-per-file-under-in-place](README.md#slurp-silently-per-file-under-in-place), [in-place-with-from-file-rejected-by-clap](README.md#in-place-with-from-file-rejected-by-clap)
**Commit:** 1cf84ed8f45530e2e76a490b1bc8506c4011d3b6
**Author:** Clay Loveless and Codex

Changed `yg -i` to render every input into a sibling temporary file before replacing any source file, and to refuse replacement when a filter produces no output. It now rejects symlink paths, warns when atomic replacement will split a hard-link set, preserves Unix ownership and permissions, rejects incompatible global modes, and accepts filters supplied through `-f`. Focused CLI tests verify that parse and transform failures leave every source unchanged.

---

## 2026-08-03 — Enforce resource limits through evaluation and emission

**Disposition:** fixed
**Addresses:** [yaml-bomb-materialized-by-emitters](README.md#yaml-bomb-materialized-by-emitters), [filter-constructed-values-recurse-without-depth-bound](README.md#filter-constructed-values-recurse-without-depth-bound), [map-index-allocates-directly-from-untrusted-index](README.md#map-index-allocates-directly-from-untrusted-index), [fromjson-ignores-configured-limits](README.md#fromjson-ignores-configured-limits)
**Commit:** 1cf84ed8f45530e2e76a490b1bc8506c4011d3b6
**Author:** Clay Loveless and Codex

Extended loader limits with collection and output ceilings, activated those limits during jaq evaluation, and preflighted YAML and JSON emission for depth, scalar size, collection size, alias materialization, and output bytes. Large sequence indices and repeated strings are rejected before allocation, `fromjson` uses the JSON parser under the active limits, and deeply nested values are destroyed iteratively to avoid stack overflow.

---

## 2026-08-03 — Stabilize query process behavior

**Disposition:** fixed
**Addresses:** [null-input-still-blocks-on-stdin](README.md#null-input-still-blocks-on-stdin), [stdout-epipe-becomes-runtime-error](README.md#stdout-epipe-becomes-runtime-error), [halt-prints-a-spurious-error-line](README.md#halt-prints-a-spurious-error-line), [runtime-errors-drop-document-and-file-identity](README.md#runtime-errors-drop-document-and-file-identity)
**Commit:** 1cf84ed8f45530e2e76a490b1bc8506c4011d3b6
**Author:** Clay Loveless and Codex

Deferred input loading so `-n` can run without draining standard input while keeping real inputs available to `input` and `inputs`. Broken pipes now terminate silently with success, `halt` uses its requested status without a spurious diagnostic, and runtime failures include the active filename and document index. CLI tests exercise open stdin, closed stdout, halt status, and multi-file error identity.

---

## 2026-08-03 — Correct query-path exit classifications

**Disposition:** mitigated
**Addresses:** [exit-code-mapping-diverges-from-spec](README.md#exit-code-mapping-diverges-from-spec)
**Commit:** 1cf84ed8f45530e2e76a490b1bc8506c4011d3b6
**Author:** Clay Loveless and Codex

Split unreadable inputs from parse/runtime failures so they exit 2, and changed `halt` status conversion to POSIX byte truncation so `halt(-1)` exits 255. The query path now matches the documented classifications, but configuration loading and logging initialization still return through `anyhow`; those pre-pipeline failures can therefore exit 1 and remain to be corrected.
