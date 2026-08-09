---
audit: .
last_updated: 2026-08-03
status:
  fixed: 34
  mitigated: 1
  accepted: 1
  disputed: 0
  deferred: 0
  open: 0
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

---

## 2026-08-03 — Complete process exit classification

**Disposition:** fixed
**Addresses:** [exit-code-mapping-diverges-from-spec](README.md#exit-code-mapping-diverges-from-spec)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Superseded the earlier mitigation by mapping configuration and argument failures to exit 2 and initialization or runtime failures to exit 5 at the binary boundary. The CLI now renders those errors consistently without falling through `anyhow`'s generic exit 1 path, and the invalid-configuration integration test asserts the documented status.

---

## 2026-08-03 — Add data-safe filter parameters and sandbox policy

**Disposition:** fixed
**Addresses:** [no-argument-binding-forces-filter-concatenation](README.md#no-argument-binding-forces-filter-concatenation), [env-filter-registered-unconditionally](README.md#env-filter-registered-unconditionally)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Added `--arg` and `--argjson` bindings that pass values through jaq globals instead of interpolating untrusted data into filter source. `--argjson` uses strict JSON parsing under the active resource limits. The public compile policy can now omit the `env` filter for sandboxed embedders while preserving the existing trusted default.

---

## 2026-08-03 — Stabilize the public value and evaluation contracts

**Disposition:** fixed
**Addresses:** [no-public-read-access-to-yamlval](README.md#no-public-read-access-to-yamlval), [input-meta-refcell-races-with-input-lookahead](README.md#input-meta-refcell-races-with-input-lookahead), [integer-only-indexing-diverges-from-equal-float-values](README.md#integer-only-indexing-diverges-from-equal-float-values), [run-filter-signature-overconstrains-callers](README.md#run-filter-signature-overconstrains-callers)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Exposed typed scalar, sequence, and mapping accessors without committing callers to the internal `Data` layout. Input metadata is staged and activated only when evaluation begins the corresponding record, so `input` lookahead cannot replace the current filename or document identity. A shared index conversion accepts finite integral floats wherever equal integer indexes are valid, and `run_filter` now accepts borrowed iterators plus typed errors without a `'static` requirement.

---

## 2026-08-03 — Retain the single-threaded value representation

**Disposition:** accepted
**Addresses:** [rc-makes-value-model-non-send](README.md#rc-makes-value-model-non-send)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

The project retains `Rc` throughout `YamlVal` because jaq evaluation is single-threaded and cheap clones are central to the hot path. Public type documentation now makes the resulting `!Send` and `!Sync` contract explicit. Parallel evaluation would require an intentional value-boundary redesign rather than silently paying atomic reference-counting costs in the current engine.

---

## 2026-08-03 — Remove emitter allocation hot spots

**Disposition:** fixed
**Addresses:** [yaml-inline-emitter-string-per-node](README.md#yaml-inline-emitter-string-per-node), [needs-quotes-copies-every-scalar-it-inspects](README.md#needs-quotes-copies-every-scalar-it-inspects), [emitter-anchor-table-is-a-linear-scanned-vec](README.md#emitter-anchor-table-is-a-linear-scanned-vec)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Flow emission now writes recursively into one output buffer, indentation reuses a context cache, and scalar quote checks ask the active resolver directly instead of allocating a temporary string value. Anchor membership uses hash-based sets with shared `Rc<str>` names, while ordered audit output retains insertion order through `IndexSet`.

---

## 2026-08-03 — Bound normal query input materialization per file

**Disposition:** mitigated
**Addresses:** [query-cli-materializes-all-inputs-before-filtering](README.md#query-cli-materializes-all-inputs-before-filtering)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Normal multi-file queries now load, evaluate, and release one file at a time instead of materializing the entire input set before evaluation. `-s` remains intentionally eager and `-i` still stages all rendered files for atomic replacement. The scanner and builder continue to buffer a complete file and its documents, so unbounded standard-input streams do not yet emit incrementally; the implementation notes record that residual limitation and the required post-M10 pipeline work.

---

## 2026-08-03 — Close the documented conformance gaps

**Disposition:** fixed
**Addresses:** [alias-audit-skips-mapping-keys](README.md#alias-audit-skips-mapping-keys), [regex-is-not-pcre](README.md#regex-is-not-pcre)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

Alias inventory now traverses mapping keys and records key positions distinctly from value paths, preventing false orphan reports. The implementation notes now identify the finite-automata `regex-bites` syntax, enumerate the unsupported PCRE constructs, and direct draft 02 to replace the broader compatibility claim.

---

## 2026-08-03 — Align release and dependency policy with the actual jaq API

**Disposition:** fixed
**Addresses:** [release-check-runs-cargo-deny-without-config](README.md#release-check-runs-cargo-deny-without-config), [jaq-trait-coupling-outruns-version-constraint](README.md#jaq-trait-coupling-outruns-version-constraint), [unused-dev-dep-pretty-assertions](README.md#unused-dev-dep-pretty-assertions)
**Commit:** a3a473909b28f3d5aa5cc2f473cf1866a7e12795
**Author:** Clay Loveless and Codex

The release gate now runs cargo-deny with all features and the repository's `.config/deny.toml` policy. The query crate pins the jaq core and standard-library versions whose traits it implements directly, and removes its unused `pretty_assertions` development dependency. The canonical deny and full check gates pass with the resulting lockfile.
