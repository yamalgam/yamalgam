---
audit: 2026-08-08-15-m10-fresh
last_updated: 2026-08-08
status:
  fixed: 5
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 0
---

# Actions Taken: Full Repository Static Audit

Remediation status for the [2026-08-08 full repository static audit](README.md).

---

## 2026-08-08 — Reject invalid tree scalar styles

**Disposition:** fixed
**Addresses:** [compare-tree-parser-panics-on-nonascii-style](README.md#compare-tree-parser-panics-on-nonascii-style)
**Commit:** `b8c97e6752bcdbb0c525facbf4a80c3b35778ffc`
**Author:** Codex

`parse_val` now accepts only the five ASCII style indicators defined by the
YAML Test Suite tree format. Unsupported and multibyte indicators return no
event instead of reaching a byte-one string slice. A regression covers the
previous `parse_tree("=VALé")` panic, while the existing tests retain coverage
for every accepted style.

Verification: focused regression passed; `just check` passed with 1,929 tests,
Clippy with warnings denied, Cargo Deny, formatting, and doc tests. Independent
code review returned PASS.

---

## 2026-08-08 — Guard explicit indentation boundaries

**Disposition:** fixed
**Addresses:** [test-suite-extractor-slices-inside-unicode-indentation](README.md#test-suite-extractor-slices-inside-unicode-indentation)
**Commit:** `a2850ffebcee0dbadaa9e3be0d303792ae177af1`
**Author:** Codex

Explicit block-scalar indentation now uses checked string slicing. A malformed
line whose configured byte offset falls inside multibyte whitespace terminates
that block instead of panicking; blank lines and inferred indentation retain
their existing behavior. A regression exercises the exact Unicode-boundary
failure mode.

Verification: focused regression passed; `just check` passed with 1,930 tests,
Clippy with warnings denied, Cargo Deny, formatting, and doc tests. Independent
code review returned PASS.

---

## 2026-08-08 — Enforce dependency policy for the fuzz graph

**Disposition:** fixed
**Addresses:** [fuzz-workspace-escapes-dependency-policy](README.md#fuzz-workspace-escapes-dependency-policy)
**Commit:** `9d008a116bdd59b92baaf39393b2c8c8c3e56f48`
**Author:** Codex

The excluded fuzz package now declares the project license, and Cargo Deny
allows NCSA only for `libfuzzer-sys`, whose expression combines that license
with MIT or Apache-2.0. The canonical local, release, and CI gates now check the
root and fuzz graphs with their lockfiles enforced. Regenerating the fuzz lock
cut it from 776 to 254 lines and retained a current 28-package graph.

Verification: both locked Cargo Deny graphs passed; `cargo audit --file
fuzz/Cargo.lock`, nightly all-target fuzz-package check, locked metadata, and
Actionlint passed. `just check` passed with 1,930 tests, Clippy with warnings
denied, formatting, both dependency graphs, and doc tests. Independent code
review returned PASS.

---

## 2026-08-08 — Preserve structured resource-limit failures

**Disposition:** fixed
**Addresses:** [resource-limit-checks-return-unstructured-strings](README.md#resource-limit-checks-return-unstructured-strings)
**Commit:** `543b5380ced8633dbc4983b5108d4326df0b8064`
**Author:** Codex

Resource-limit checks now return a public `LimitExceeded` payload containing
the resource category, observed value, and configured maximum. Scanner,
parser, composer, serde, CST, query, and CLI boundaries preserve that payload;
internal accounting overflow remains a distinct error. ADR-0014 records the
error contract, including preservation of existing rendered context and
prefixes.

Verification: exact-payload regressions cover all nine resource categories and
their public propagation paths, including jaq runtime operations and nested
scanner/parser context. `just check` passed with 1,932 tests, Clippy with
warnings denied, both Cargo Deny graphs, formatting, and doc tests. Independent
code review returned PASS.

---

## 2026-08-08 — Preserve scalar presentation metadata

**Disposition:** fixed
**Addresses:** [scanner-never-populates-atom-presentation-metadata](README.md#scanner-never-populates-atom-presentation-metadata)
**Commit:** `b9d2407d0ae9c0ecd47b213be5989f880c888d1e`
**Author:** Codex

The scanner now records block chomping modes and derives scalar content and
scan-context flags, including safe direct-output eligibility. Parser events and
query values preserve that metadata through their public boundaries. The YAML
emitter consumes block chomp and direct-output metadata while normalizing stale
public metadata whenever preserving it would change the scalar value. ADR-0002's
confirmation now points to the deterministic regressions that enforce this
contract.

Verification: regressions cover strip, clip, and keep chomping; empty and
line-break-only block scalars; whitespace and escape flags; parser and query
propagation; direct-output styles; and semantic emit/reparse equality. `just
check` passed with 1,939 tests, Clippy with warnings denied, both Cargo Deny
graphs, formatting, and doc tests. ADR lint passed. Independent code review
returned PASS.
