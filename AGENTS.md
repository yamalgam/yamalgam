# yamalgam

## Workspace Layout

This is a Cargo workspace. All crates live under `crates/` (plus `xtask/`):

| Crate | Purpose |
|-------|---------|
| `yamalgam` | CLI binary (thin shell) |
| `yamalgam-core` | Shared library (config, types, logic) |
| `yamalgam-scanner` | YAML scanner (ported from libfyaml 0.9.5, now independent) |
| `yamalgam-parser` | StAX-style pull parser over the scanner's token stream |
| `yamalgam-compare` | YAML Test Suite compliance and cross-implementation comparison |
| `xtask` | Dev automation (completions, man pages, benchmarks) |

Configuration files live in `config/` with `.toml.example` and `.yaml.example` templates.
## Commands

Use `just` for all dev tasks:

```
just check          # fmt + clippy + deny + test + doc-test (run before pushing)
just test           # cargo nextest run
just clippy         # lint with pinned toolchain
just fmt            # cargo fmt --all
just deny           # security/license audit
just fix            # auto-fix clippy warnings
just bench          # run all benchmarks
just release-check  # pre-release validation
just outdated       # check for outdated dependencies
just upgrade        # update deps in Cargo.toml and Cargo.lock
```


**Tests use `cargo nextest run`**, not `cargo test`. Doc tests are separate: `cargo test --doc`.

## Rust Conventions

- **Edition 2024**, MSRV **1.88.0**, toolchain pinned in `rust-toolchain.toml`
- `unsafe_code = "deny"` — no unsafe unless explicitly allowed with a `// SAFETY:` comment
- Clippy `all` = warn, `nursery` = warn — treat warnings as errors in CI
- Use `anyhow::Result` in the binary, `thiserror` for library error types
- Shared logic belongs in `yamalgam-core`; the CLI crate handles argument parsing and I/O


**IMPORTANT — THIN CLI, FAT CORE.** Feature logic and new dependencies belong in `yamalgam-core`, not `yamalgam`. The CLI crate is a thin shell: argument parsing, I/O, and wiring. This keeps the core testable without subprocess gymnastics and leaves the door open for other frontends (WASM, Tauri, etc).
## Adding CLI Commands

1. Create `crates/yamalgam/src/commands/your_cmd.rs`
2. Add the variant to `Commands` enum in `crates/yamalgam/src/lib.rs`
3. Wire it up in `match cli.command` in `main.rs`
4. Add integration tests in `crates/yamalgam/tests/`


## Scanner Testing

The scanner is tested at two levels:

### 1. Unit tests (fast, run first)
```
cargo nextest run -p yamalgam-scanner
```
178 scanner unit tests in `crates/yamalgam-scanner/tests/scanner.rs`. Run these after any scanner change — they catch regressions fast.

### 2. YAML Test Suite compliance
```
just test-compliance
```
Runs 351 YAML Test Suite cases through the yamalgam parser and compares event output against the expected `tree` field in each test file. No external dependencies — pure Rust.

To check a specific test case: `cargo nextest run -p yamalgam-compare --test compliance -E 'test(TEST_ID)' --success-output=immediate`

To see what a test expects: `cat vendor/yaml-test-suite/TEST_ID.yaml`

### 3. Full check (before pushing)
```
just check    # fmt + clippy + deny + nextest + doc-test
```

## Do Not

- Commit anything in `target/`
- Add dependencies without checking `deny.toml` license policy (`just deny`)
- Skip `--all-targets --all-features` when running clippy
- Use `cargo test` instead of `cargo nextest run`
- Run raw cargo commands when a `just` recipe exists
