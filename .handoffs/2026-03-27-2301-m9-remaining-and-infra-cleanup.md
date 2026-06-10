# Handoff: M9 Remaining Work + Infrastructure Cleanup

**Date:** 2026-03-27
**Branch:** `main` (clean, all merged)
**State:** Yellow — 1270 tests pass, `just check` clean. CI hyperfine benchmark still failing (see Landmines). `docs/plans/README.md` shows M8/M9 as "Not started" — stale.

## Where things stand

M8 (CST) and M9 (serde Deserializer) core implementation is merged on main (PRs #80-#83). Template updated to claylo-rs 1.1.0 (PR #92). Benchmark infrastructure overhauled: KDL replaced with TOML + `.rs.tmpl` templates (PR #91), gungraun enabled as dev-dep, artifact action SHAs fixed. M9 integration tests (Tasks 14-16) are the remaining work before M9 is complete.

## Decisions made

- **KDL dropped for benchmark codegen.** Replaced with `benchmarks.toml` + `divan_benchmarks.rs.tmpl` / `gungraun_benchmarks.rs.tmpl`. Generator uses `str::replace`, no template engine. Three-layer strategy (ADR-0008) preserved.
- **Gungraun enabled as dev-dep** to keep CI pipeline warm. Catches infra issues early (proved its value this session — bad SHAs were hidden when job was disabled).
- **claylo-rs 1.1.0 applied.** MSRV bumped to 1.89.0, clap to 4.6 (HelpShort pattern), `Bash(*)` permissions with deny rules in `.claude/settings.json`.
- **Merge conflict resolution strategy:** keep yamalgam-specific infrastructure (scanner testing, compliance, comparative benchmarks, fuzz targets, serde crate) over template defaults.

## What's next

1. **Fix `docs/plans/README.md`** — M8 and M9 status still says "Not started." Update with merged PR numbers and test counts.
2. **M9 integration tests (Tasks 14-16):**
   - Crawl real-world YAML fixtures from yamllint, yamlfmt, yq, prettier. Store in `crates/yamalgam-serde/tests/fixtures/`.
   - Port serde_yaml compatibility tests to verify behavioral parity.
   - Add serde round-trip to compliance harness (`from_str::<Value>()` vs Composer). Requires `impl Deserialize for Value` in `yamalgam-core`.
3. **Investigate CI hyperfine failure** — `command not found` despite binstall step. Likely stale cache from prior runs. May need cache bust or step reordering.
4. **Fix bad SHAs upstream in claylo-rs template** — `upload-artifact` and `download-artifact` SHAs were wrong. Corrected here, needs backport to template.
5. **After M9:** M10 (yg CLI + Query Engine), 12 CST flow close token allowlist fixes, performance tuning.

## Landmines

- **CI hyperfine job still failing on main.** `scripts/bench-cli.sh: line 62: hyperfine: command not found`. Step order was fixed (rust-cache before setup-cargo-tools) but may need a cache bust. The `setup-cargo-tools` composite action at `.github/actions/setup-cargo-tools/action.yml` caches `~/.cargo/bin` — a stale cache hit could skip the binstall step. Try manually re-running the workflow or check if `actions/cache` restore-keys are too broad.
- **`Deserialize for Value` doesn't exist yet.** Compliance round-trip test (Task 15) needs it. Implement in `yamalgam-core` since it already depends on serde.
- **`from_str_with_limits` coexists with `from_str_with_config`.** Consider deprecating the limits variant after config API stabilizes.
- **`docs/plans/README.md` is stale.** M8 and M9 show "Not started" — will confuse any agent that reads it for context.
- **MEMORY.md is over 200 lines** (truncated on load). Move detail to topic files if updating.
