# Handoff: Audit Remediation Complete — M9 Integration Tests Next

**Date:** 2026-06-10
**Branch:** `main` (clean, all merged)
**State:** Green — 1290 tests pass, `just check` clean, all 13 findings from the 2026-04-08 cased audit closed.

## Where things stand

Five PRs merged today (#102-#106) closed out the entire audit backlog:

- **#102** — cd.yml was invalid YAML since #92 (unindented heredoc body terminated the
  `run:` block scalar). Every push showed a phantom failed run. Fixed by indenting the
  heredoc body to the block-scalar indent. Needs backport to the claylo-rs template.
- **#103 (tier 1)** — serde depth underflow fixed with `checked_sub` + regression test
  (reachable from safe Rust via a contract-violating `MapAccess` caller); `lto = "thin"`
  on release; figment `yaml` feature dropped (`serde_yaml`/`unsafe-libyaml` are OUT of
  Cargo.lock — YAML config files are now skipped in discovery and rejected via
  `ConfigError::UnsupportedFormat` when passed with `--config`); 11 unused deps removed;
  `Display`/`Error` for `Diagnostic`; CST whitespace clamp; stale M8/M9 roadmap fixed.
- **#104 (tier 2)** — `#[inline]` on 9 Reader hot methods; `consume_peeked()`/
  `peeked_token()` replaced 49 parser `.expect("peeked")` panic sites with error returns;
  serde `deserialize_any` peek-clone eliminated; `Resolver::on_event` returns
  `ResolvedEventBuf` (= `SmallVec<[Event; 1]>`, ADR-0007) — passthrough is allocation-free.
- **#105** — `ScanErrorKind` (#[non_exhaustive], 12 family variants) on every `ScanError`,
  all 48 construction sites classified, `ScanError::new(kind, msg)`, 12 kind-assertion
  integration tests. Kind flows through `ParseError::Scan` to the whole pipeline.
- **#106** — `Parser<'input, I = Scanner<'input>>`: token source devirtualized via default
  type parameter (zero changes needed outside parser.rs); `new()`/`from_tokens()` now
  `const fn`. `Event::span()` accessor added. Spans threaded through all yamalgam-serde
  error sites; `Display` renders `(line L, column C)` one-indexed.
  `from_str_with_limits` deleted (superseded by `from_str_with_config`).

## Decisions made

- **Delete, don't deprecate.** Pre-crates.io there are no users; superseded APIs get
  removed outright (Clay, removing `from_str_with_limits`).
- **ScanErrorKind is 12 family variants, not 48 message-shaped ones** — callers match
  families; `#[non_exhaustive]` allows splitting later.
- **Parser generic via default type param** — `impl<'input> Parser<'input>` resolves to
  the Scanner default everywhere except the struct definition, so the "ripples through
  four crates" fear evaporated.
- **erased-serde error flattening handled in two stages** — position is rendered into
  `Display` now (survives the string boundary); restoring the structured error through
  `from_str` is a queued follow-up (stash pattern), see Landmines.
- **YAML config support intentionally removed** until yamalgam can parse its own config
  (figment-provider feature). A YAML tool whose config can't be YAML is funny-sad but
  beats shipping deprecated serde_yaml.

## What's next

1. **M9 integration tests** (Tasks 14-16 of
   `docs/plans/2026-03-09-m9-serde-implementation-plan.md`):
   - Crawl real-world YAML fixtures (yamllint, yamlfmt, yq, prettier) into
     `crates/yamalgam-serde/tests/fixtures/`.
   - Port serde_yaml compatibility tests for behavioral parity.
   - Add serde round-trip to the compliance harness (`from_str::<Value>()` vs Composer) —
     requires `impl Deserialize for Value` in yamalgam-core (core already depends on serde).
2. **erased-serde error stash** — restore structured errors through `from_str`. Wrap the
   ~30 `serde::Deserializer` trait methods with a `map_err` that stashes the original
   error in the `Deserializer` and forwards a rendered placeholder; `from_str` retrieves
   the stash on failure. `Error` is not `Clone` (`ResolveError` holds `io::Error`), so
   the stash takes ownership.
3. **M10** — yg CLI + jaq query engine (spec: `docs/spec/yg-query-language-spec-draft-01.md`).
   The CLI is still the claylo-rs template stub (doctor/info) — M10 gives it its purpose.
4. Smaller queued items: 12 CST flow close-token allowlist cases, rust-yaml anomalously
   fast on large-input benchmarks (investigate), claylo-rs template backports (cd.yml
   heredoc + artifact SHAs).

## Landmines

- **erased-serde flattens structured errors.** `from_str` → `erased_serde::deserialize` →
  error becomes `Error::Custom(String)`. Position survives in the message; the structured
  `Span`/variant only via direct `Deserializer` usage until the stash lands.
- **clippy nursery is merciless.** New code will hit `missing_const_for_fn` (even in test
  helpers), `or_fun_call` (use `ok_or_else`), `option_if_let_else` (use `map_or`). Budget
  one extra `just check` round per PR.
- **Plain `grep` is broken in Claude Code sessions here** (shell snapshot shadows it with
  a function referencing unbound `ZSH_VERSION`) — use `/usr/bin/grep`.
- **MEMORY.md was over its 200-line load limit for months** — sessions silently lost the
  bottom 40 lines including the CRITICAL scanner error-path gotcha. Restructured 2026-06-10
  to a 47-line index + topic files (`milestone-history.md`, `scanner-parser-gotchas.md`).
- **Workflow hard stop:** after staging + writing commit.txt, STOP — no further tree edits
  until Clay confirms the merge landed. Unstaged ride-alongs broke a `gsm && git up` today.
- **Benchmarks workflow on main** was still in_progress when last checked this morning —
  verify it's green before trusting CI benchmark trends.
