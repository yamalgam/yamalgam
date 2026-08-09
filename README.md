# yamalgam

[![CI](https://github.com/claylo/yamalgam/actions/workflows/ci.yml/badge.svg)](https://github.com/claylo/yamalgam/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/yamalgam.svg)](https://crates.io/crates/yamalgam)
[![docs.rs](https://docs.rs/yamalgam/badge.svg)](https://docs.rs/yamalgam)
[![MSRV](https://img.shields.io/badge/MSRV-1.89.0-blue.svg)](https://github.com/claylo/yamalgam)

A pure Rust YAML toolkit: scanner, event parser, lossless CST, DOM, and a
streaming serde deserializer — one pipeline, four consumers.

> **Pre-release.** We haven't published to crates.io yet; APIs change
> without deprecation cycles. The `yg` CLI is available from this workspace.

## Architecture

```text
YAML bytes → Scanner (tokens) → Parser (events) → Resolver middleware
                                                        │
                    ┌───────────┬───────────────┬───────┴───────┐
                    │           │               │               │
              Streaming     Value (DOM)       CST          SAX/Callbacks
              serde Deser   (lossy)        (lossless)      (zero alloc)
```

| Crate | Role |
|-------|------|
| `yamalgam` | `yg` CLI + re-exported public API. |
| `yamalgam-query` | Metadata-preserving YAML value model, jaq engine, YAML/JSON emitters. |
| `yamalgam-core` | `Value` DOM, tag resolution, loader config, diagnostics. |
| `yamalgam-scanner` | Bytes → tokens. |
| `yamalgam-parser` | Tokens → events (full-fidelity: comments, indicators, spans). |
| `yamalgam-compose` | Events → `Value` documents. |
| `yamalgam-cst` | Events → lossless concrete syntax tree (`to_text()` round-trips). |
| `yamalgam-serde` | Streaming serde `Deserializer` (no intermediate DOM). |

## Status

- YAML 1.2 core schema; event compliance 350/351 against the
  [YAML Test Suite](https://github.com/yaml/yaml-test-suite).
- CST round-trip fidelity 343/351 (byte-for-byte `parse → to_text`)
- Streaming serde deserializer with anchor/alias replay, multi-document
  iteration, structured errors with source spans, and behavioral parity
  with serde_yaml (divergences documented in tests).
- Composer and serde pipelines agree on the entire test suite.
- Fastest of 7 benchmarked peers on small/medium inputs.
- `yg` query engine with 230 passing jq 1.8 manual examples and an exact,
  reasoned allowlist for every divergence or scope exclusion.

## yg

`yg` is **not jq for YAML**. It is a YAML processor whose expression language
is jq. You get jq-familiar composition over YAML's real data model. Tags,
anchors, aliases, styles, schemas, merge keys, insertion order, and non-string
mapping keys stay available to the query.

Install the workspace binary:

```bash
cargo install --path crates/yamalgam
```

```bash
yg . config.yml                                # identity / pretty-print
yg '.services.api.image' compose.yml           # jq-familiar field access
yg 'keys' config.yml                           # insertion order; keys_sorted opts in to sorting
yg -j '.jobs.build' .github/workflows/ci.yml   # deliberate JSON escape hatch
yg --schema 1.1 '.enabled | type' legacy.yml   # schema-aware resolution
yg -i '.image.tag = "2026.08.02"' deploy.yml   # atomic in-place mutation
```

YAML-native filters make the representation queryable instead of incidental:
`tag`, `anchor`, `style`, `raw`, `kind`, `schema`, `aliases`,
and `orphan_anchors`. Mutation filters such as `set_tag`, `set_anchor`,
`set_style`, and `merge` operate on the same value model as jq expressions.

> **Current `-i` caveat:** edits are atomic and preserve supported node styles,
> but they reserialize the document and therefore **lose comments and exact
> formatting**. Milestone 13 adds CST-spliced editing.

Start with the [`yg` user guide](docs/yg/README.md). See the
[draft-01 implementation notes](docs/spec/yg-spec-draft-01-implementation-notes.md)
for compatibility counts and deliberate deviations.

## Using the library (pre-release)

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Server {
    host: String,
    port: u16,
}

let server: Server = yamalgam::from_str("host: localhost\nport: 8080")?;
```

Multi-document streams:

```rust
let docs: Vec<yamalgam::Value> = yamalgam::Deserializer::from_str("---\na: 1\n---\nb: 2")
    .documents()
    .collect::<Result<_, _>>()?;
```

## Development

```bash
just check   # fmt + clippy + deny + nextest + doc-tests
just bench   # scanner/parser self-benches, comparative, CLI
just unsafe-census # compiler-derived census for the shipped yg graph
```

The roadmap lives in [`record/superpowers/plans/README.md`](record/superpowers/plans/README.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
