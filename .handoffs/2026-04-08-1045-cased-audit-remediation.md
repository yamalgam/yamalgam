# Handoff: Addressing cased Audit Findings

**Date:** 2026-04-08
**Branch:** `main`
**State:** Yellow — 1270+ tests pass, `just check` clean. 29 audit findings documented, none critical.

## Where things stand

A full cased + crustoleum audit was run against all 9 workspace crates at commit `6116052`. Report, structured findings, and recon artifacts are at `record/audits/2026-04-08-full-workspace/`. The HTML report (`report.html`) has interactive navigation and sparklines. All 29 findings were verified by a reviewer agent — zero disputed. Tool output (clippy, audit, deny, machete, geiger, udeps) is cached in `.crustoleum/`.

## Decisions made

- **Skipped safety-auditor and concurrency agents** — no `unsafe` code (all crates `#![deny(unsafe_code)]`), no async, no threads.
- **29 findings organized into 6 narratives** — Untrusted Input, Error Architecture, Panic Discipline, Hot Path, Supply Chain, API Design.

## What's next

Findings are ordered by impact-to-effort ratio. The first three are trivial one-session fixes.

### Tier 1: Trivial effort, high impact

1. **Fix serde depth underflows** — `crates/yamalgam-serde/src/de.rs:748` (`skip_value`), `:888` (`SeqAccess::drain`), `:934` (`MapAccess::drain`). Replace `depth -= 1` with `checked_sub(1)` + error return. Debug panics / release infinite loops on malformed event streams.
2. **Add `lto = "thin"` to `[profile.release]`** — `Cargo.toml:57`. Bench profile has LTO, release doesn't. Benchmarks misrepresent release performance. One line.
3. **Drop figment `yaml` feature** — `crates/yamalgam-core/Cargo.toml:37`. Change to `features = ["toml", "json"]`. Removes deprecated `serde_yaml` + `unsafe-libyaml` from the entire dep tree.
4. **Remove 7 unused deps** — `thiserror` in scanner/compare/serde, `serde_json`+`yamalgam-core` in compare, `serde`+`yamalgam-core` in bench. Also `pretty_assertions` in 4 dev-deps.
5. **Implement Display + Error for Diagnostic** — `crates/yamalgam-core/src/diagnostic.rs:49`. Two trait impls, unblocks `?` propagation through `anyhow`/`Box<dyn Error>`.
6. **Clamp CST whitespace indexing** — `crates/yamalgam-cst/src/lib.rs:488`. Add `.min(self.source.len())` matching the pattern in `source_text()`.

### Tier 2: Small effort, moderate impact

7. **Add `#[inline]` to Reader hot functions** — `reader.rs:40-90` (`peek`, `peek_at`, `advance`, `mark`, `is_eof`). ~10 annotations.
8. **Add `consume_peeked()` to Parser** — Eliminates 40+ `.expect("peeked")` panic sites in `parser.rs`. One helper + mechanical call-site updates.
9. **Eliminate serde peek-clone pattern** — `de.rs:375`. Extract discriminant from peek, drop borrow, then `next_event()`. Removes one Event clone per YAML value.
10. **Adopt SmallVec for Resolver::on_event** — `resolve.rs:93`. ADR-0007 specified this but it wasn't implemented. Eliminates 1 Vec alloc per event on passthrough path.

### Tier 3: Medium effort, architectural improvement

11. **Add ScanErrorKind enum** — `scanner.rs:34`. 51 error sites need variant classification. Unblocks typed error handling through the entire pipeline. Biggest single error-architecture improvement.
12. **Make Parser generic over token iterator** — `parser.rs:72`. Replace `Box<dyn Iterator>` with generic `I: Iterator`. Enables Scanner::next() inlining. Ripples through compose, serde, cst, compare.
13. **Thread spans to serde Error construction** — `de.rs` 21 call sites pass `span: None`. Event already carries span; needs threading through.

## Landmines

- **`.crustoleum/` is untracked.** Contains tool output from this audit (645KB geiger.txt). Either gitignore it or delete it after addressing findings.
- **`record/audits/` is untracked.** The full audit directory needs to be committed or gitignored. Currently sitting in the working tree.
- **Unused dep removal may break doc-tests.** `cargo-udeps` can't detect doc-test-only usage. Test with `cargo nextest run` AND `cargo test --doc` after removing deps.
- **Parser generic refactor (Tier 3 #12) is the biggest API change.** Every consumer of `Parser` needs updating. Consider doing it on a feature branch with a design doc.
- **`from_str_with_limits` coexists with `from_str_with_config`** — audit flagged the ownership inconsistency (`ResourceLimits` by value vs `&LoaderConfig` by reference). Deriving `Copy` on `ResourceLimits` is the quick fix. Long-term: deprecate the limits variant.
- **MEMORY.md is over 200 lines** (truncated on load). If updating, move detail to topic files.
