---
audit: 2026-08-03-21-m10-fresh
last_updated: 2026-08-04
status:
  fixed: 23
  mitigated: 0
  accepted: 1
  disputed: 0
  deferred: 1
  open: 22
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

---

## 2026-08-03 — `tostring` failures propagated

**Disposition:** fixed
**Addresses:**
[into-string-substitutes-error-text-as-data](README.md#into-string-substitutes-error-text-as-data)
**Commit:** 9de9e1726507a664f9e5709a0f7a5bd5b618c5c0
**Author:** Codex

`ValT::into_string` now renders with the active invocation's limits and records
conversion failures in the scoped runtime error channel. The engine rejects
that pending error before visiting output, so error text can no longer flow
through a successful filter as ordinary string data.

Regression tests cover duplicate JSON key conversion and an invocation-specific
depth cap, assert that no output reaches the visitor, and verify a runtime error
is returned. All 98 query tests and clippy pass.

---

## 2026-08-03 — Input error classes preserved

**Disposition:** fixed
**Addresses:**
[input-error-class-flattened-through-string](README.md#input-error-class-flattened-through-string)
**Commit:** 80fe53bbc81781ec3a3f50ec84990a29b592f876
**Author:** Codex

The jaq input adapter now pairs its required string error with the original
typed `YgError` and restores that variant when the outer driver or native
`input`/`inputs` path surfaces the matching failure. `InputRead` therefore
retains its distinct CLI exit class instead of becoming parse/runtime failure.

The generic `From<String> for YgError` conversion was removed so future call
sites must choose a class explicitly. Tests cover driver and native-input
paths plus a caught input error followed by an unrelated runtime error. All
100 query tests and clippy pass.

---

## 2026-08-03 — Resulting value depth enforced

**Disposition:** fixed
**Addresses:**
[value-tree-recursion-has-no-depth-bound](README.md#value-tree-recursion-has-no-depth-bound)
**Commit:** 288d13136722771894dace707ba6ae29eb434605
**Author:** Codex

An iterative `YamlVal` worklist now validates resulting tree depth. The builder
applies it before anchor insertion, so syntactically shallow alias chains
cannot create deeper values than the configured limit.

Infallible jaq array construction records over-depth failures and substitutes
a safe value before recursive operations can continue; mapping construction
and every engine output are checked as well. Tests cover alias-expanded depth,
deep filter construction before comparison, and visitor-boundary validation.
All 103 query tests and clippy pass.

---

## 2026-08-03 — Strict depth CLI contract aligned

**Disposition:** fixed
**Addresses:**
[value-tree-recursion-has-no-depth-bound](README.md#value-tree-recursion-has-no-depth-bound)
**Commit:** b212751d5417339f308cf1ce5f747b90e3ad1cfc
**Author:** Codex

Updated the strict-mode CLI regression contract to require rejection when a
filter constructs an over-depth value and later discards it. This pins the
construction-time guard added by commit `288d131` and confirms that no partial
placeholder reaches stdout.

---

## 2026-08-03 — CLI output buffered

**Disposition:** fixed
**Addresses:**
[cli-output-is-unbuffered](README.md#cli-output-is-unbuffered)
**Commit:** 6be2063a16b0d4fc43354b3f58866ed4aa011005
**Author:** Codex

Standard output now passes through `BufWriter` rather than relying on the
line-buffered stdout lock. In-place output uses the same buffering around the
temporary file and explicitly flushes before `sync_all`, preserving the
durability boundary before replacement.

All 40 `yg` CLI integration tests and the workspace Clippy gate pass.

---

## 2026-08-03 — JSON emitter allocations reduced

**Disposition:** fixed
**Addresses:**
[json-emitter-allocates-indent-and-triple-allocates-keys](README.md#json-emitter-allocates-indent-and-triple-allocates-keys)
**Commit:** ca7c2e8f28ae20d39449974af5266479bd682984
**Author:** Codex

JSON indentation, quoted strings, integers, and floats now append directly to
the final output buffer instead of allocating intermediate strings. Duplicate
object-name detection stores borrowed `Rc<str>` key text through `Cow<str>`;
only non-string YAML keys allocate their required JSON name.

The five focused JSON emitter tests, all 103 `yamalgam-query` tests, and the
workspace Clippy gate pass.

---

## 2026-08-03 — Uncolored YAML scalar copies removed

**Disposition:** fixed
**Addresses:**
[yaml-emitter-paint-copies-every-scalar-when-color-is-off](README.md#yaml-emitter-paint-copies-every-scalar-when-color-is-off)
**Commit:** 4246b2821012818c593616e24d3bd40b37f3137c
**Author:** Codex

Rendered YAML scalar text now flows through `Cow<str>`. With color disabled,
plain strings and literal lines remain borrowed and already-formatted scalar
text is reused; the paint step no longer allocates or copies it. ANSI-enabled
output still creates the required wrapped value.

All 14 focused YAML emitter tests, the CLI color regression, all 103
`yamalgam-query` tests, and the workspace Clippy gate pass.

---

## 2026-08-03 — YAML boolean extraction distinguished

**Disposition:** fixed
**Addresses:**
[as-bool-inherent-shadows-jaq-truthiness](README.md#as-bool-inherent-shadows-jaq-truthiness)
**Commit:** b330d75ab54e5c8105b378dc3dff77723a6be39f
**Author:** Codex

The resolved YAML boolean extractor is now `as_boolean`, leaving natural
`.as_bool()` syntax to select `jaq_core::ValT` truthiness when that trait is in
scope. The `jaq_std::ValT::as_f64` implementation delegates to the inherent
numeric accessor rather than maintaining a second match.

A compile-failing RED test captured the original method shadow. Focused trait
and accessor regressions, all 103 `yamalgam-query` tests, and the workspace
Clippy gate pass after the rename.

---

## 2026-08-03 — Value storage representation sealed

**Disposition:** fixed
**Addresses:**
[internal-value-representation-is-public](README.md#internal-value-representation-is-public)
**Commit:** 7e9d425c13bf2cccbfa27f0e7bee575371ee2cef
**Author:** Codex

The `value` module is now crate-private, so `Data`, `YgSeq`, `YgMap`, and their
storage choices cannot be named downstream. The supported `YamlVal`, `Meta`,
and `NodeStyle` API remains available through the crate-root re-exports.

A compile-fail doctest was RED while `value::Data` remained importable and now
pins the sealed boundary. The doctest, all 103 `yamalgam-query` tests, and the
workspace Clippy gate pass.

---

## 2026-08-03 — Query execution state borrowed

**Disposition:** fixed
**Addresses:**
[run-records-static-bound-forces-clones](README.md#run-records-static-bound-forces-clones)
**Commit:** 3dddc26af8b0d3399d4b253a9892caa381e44b10
**Author:** Codex

`run_records` now accepts a borrowed `RunCtx` and an input iterator with its
actual lifetime. Deferred inputs borrow the file list, arguments, and loader
configuration instead of owning clones.

`cmd_query` constructs one run context and reuses it for normal output and
every in-place file, removing the per-file context allocation and repeated
configuration and variable clones. All 40 `yg` CLI integration tests and the
workspace Clippy gate pass.

---

## 2026-08-03 — Query output options typed

**Disposition:** fixed
**Addresses:**
[three-bool-parameters-at-command-boundary](README.md#three-bool-parameters-at-command-boundary)
**Commit:** 34dd75cd5502bcdd483dd7b5472e132c47019066
**Author:** Codex

The query command boundary now accepts one named `RunOptions` value containing
an `OutputMode`. JSON and YAML are distinct variants, and only the YAML variant
can carry color state; quiet remains a named field.

The typed output mode is resolved once in `main` and threaded through in-place
and record execution, removing the repeated positional boolean pairs and the
`color && !json_output` invariant. All 40 `yg` CLI integration tests and the
workspace Clippy gate pass.

---

## 2026-08-03 — Librebar features explicitly scoped

**Disposition:** fixed
**Addresses:**
[librebar-default-features-not-disabled](README.md#librebar-default-features-not-disabled)
**Commit:** 57e207a2122d71b8982e6deb945d24c770033f48
**Author:** Codex

The librebar dependency now disables default features and explicitly enables
only the four modules yamalgam uses: `cli`, `config`, `logging`, and
`diagnostics`. The resolved graph no longer enables librebar `default`,
`cache`, or `crash`; its unused `base64` and `tempfile` edges left the lockfile
entry.

All 86 `yamalgam` package tests and the workspace Clippy and Cargo-deny gates
pass. Cargo-deny reports only its existing policy warnings.

---

## 2026-08-03 — Config parser swap bounded by publication

**Disposition:** deferred
**Addresses:**
[yg-ships-competing-yaml-parser-for-its-own-config](README.md#yg-ships-competing-yaml-parser-for-its-own-config)
**Commit:** 05587d99111d54aaa1d7940d8a7cac1a74e43d32
**Author:** Codex

ADR 0008 accepts the existing Librebar YAML backend only as an interim and ties
its removal to the post-M10 yamalgam publication milestone. The chosen
pluggable-backend swap remains unchanged; this remediation does not invent a
second cross-repository integration.

Mutating Just dependency updates now restore `serde-saphyr` and
`granit-parser` to the audited 1.0.0 versions. Moving either pin requires an
explicit edit and review. The ADR passes bito completeness and readability at
grade 9.8, and Just and Cargo dry-runs confirm the update guard and pins.

---

## 2026-08-04 — Jaq time cost explicitly accepted

**Disposition:** accepted
**Addresses:**
[jaq-std-time-feature-bundles-tzdb](README.md#jaq-std-time-feature-bundles-tzdb)
**Commit:** 3d6eab9be30e026275a05726147429907846961b
**Author:** Codex

Disabling `jaq-std`'s `time` feature made `yamalgam-query` fail to compile:
jaq-std 3.0.1 exposes its aggregate `funs()` registry only when all six
supplementary features are active, and its non-time registries are private.
Using only `base_funs()` would remove the format, log, math, and regex filters
that yg already exposes and tests.

The manifest now disables Cargo defaults and names the complete six-feature
set explicitly. ADR 0009 records the measured `jiff` cost and the revisit
condition rather than forking or copying upstream internals. All 103 query
tests and the workspace Clippy and Cargo-deny gates pass; Cargo-deny reports
only its existing policy warnings.

---

## 2026-08-04 — Native JSON parsing enforced

**Disposition:** fixed
**Addresses:**
[serde-json-redundant-in-yaml-native-engine](README.md#serde-json-redundant-in-yaml-native-engine)
**Commit:** c3f59d353bc26bf83e42232c8cbc67f12d4e8f7c
**Author:** Codex

`fromjson` now validates one strict JSON value without constructing a second
DOM, then materializes it through yamalgam's scanner, parser, and event builder
under `TagResolution::Json`. Depth, decoded scalar and key sizes, and
per-collection entry counts are checked before the next child is built.

The native scanner now decodes JSON-compatible UTF-16 surrogate pairs, and
JSON number resolution preserves the former `i64`/finite-`f64` boundary.
`serde_json` moved to dev-dependencies as a differential oracle; the normal
`yamalgam-query` graph no longer contains it, and yg no longer enables its
`preserve_order` feature. All 361 scanner, core, and query tests pass,
including comparison across more than 500 jq-manual JSON texts, along with the
workspace Clippy and Cargo-deny gates. Cargo-deny reports only its existing
policy warnings.

---

## 2026-08-04 — Runtime constructor limits pinned

**Disposition:** fixed
**Addresses:**
[runtime-limits-cover-three-call-sites](README.md#runtime-limits-cover-three-call-sites)
**Commit:** 94c7bb0eb645313ae1145fb2960a8e6aaf9591ca
**Author:** Codex

The growth-limit implementation already extended active collection and scalar
budgets beyond the three original call sites: fallible constructors now return
errors directly, while jaq's infallible sequence constructor records its first
violation in scoped runtime state. The engine drains that pending error before
accepting output and after result exhaustion.

A focused contract test now proves `[range(4)]` under a three-item limit
returns a collection-size error and invokes the output visitor zero times, so
the internally truncated placeholder cannot escape as successful data. All
111 `yamalgam-query` tests and the workspace Clippy gate pass.

---

## 2026-08-04 — Environment access made optional

**Disposition:** fixed
**Addresses:**
[env-access-always-compiled-in](README.md#env-access-always-compiled-in)
**Commit:** 5ce4cad5a95d30dfe8e6f89dbac89ed15c27414d
**Author:** Codex

The CLI now exposes `--no-env`, which selects the query engine's existing
`CompileOptions::sandboxed()` capability policy before compiling the filter.
Default invocations retain jq-compatible `env` behavior; restricted
invocations omit the native entirely and fail at filter compilation rather
than reading process state.

The end-to-end test sets a sentinel environment variable, proves default
access, then proves `--no-env` exits with the filter-compile code without
printing the sentinel. All 87 `yamalgam` package tests and the workspace
Clippy gate pass.

---

## 2026-08-04 — Halt exit-code wraparound removed

**Disposition:** fixed
**Addresses:**
[halt-exit-code-truncated-to-u8](README.md#halt-exit-code-truncated-to-u8)
**Commit:** ab1ce0ee026afdfa8377876f0e81d2876df70cbc
**Author:** Codex

Halt status conversion now preserves values in the POSIX byte range and maps
every other out-of-range code to generic failure 255. This retains the
existing `halt(-1)` behavior while preventing nonzero requests such as
`halt(256)` from modulo-wrapping to silent shell success.

The CLI regression test pins `halt`, `halt(-1)`, and `halt(256)`, including
their intentionally empty diagnostics. All 87 `yamalgam` package tests and the
workspace Clippy gate pass.

---

## 2026-08-04 — Builder invariants typed separately

**Disposition:** fixed
**Addresses:**
[build-invariant-errors-labeled-as-limit](README.md#build-invariant-errors-labeled-as-limit)
**Commit:** 9def9495dc887f8775b4fbbbc1f769902d79064e
**Author:** Codex

`BuildError::Invariant` now represents an unexpected event within a node or a
truncated parser event stream. `BuildError::Limit` is again reserved for
configured resource-limit breaches, so library callers can distinguish a
document that needs a different profile from a parser/builder contract bug.

A focused variant test pins the invariant diagnostic and classification. All
112 `yamalgam-query` tests and the workspace Clippy gate pass.
