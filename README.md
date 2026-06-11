# yamalgam

[![CI](https://github.com/claylo/yamalgam/actions/workflows/ci.yml/badge.svg)](https://github.com/claylo/yamalgam/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/yamalgam.svg)](https://crates.io/crates/yamalgam)
[![docs.rs](https://docs.rs/yamalgam/badge.svg)](https://docs.rs/yamalgam)
[![MSRV](https://img.shields.io/badge/MSRV-1.89.0-blue.svg)](https://github.com/claylo/yamalgam)

A pure Rust YAML toolkit: scanner, event parser, lossless CST, DOM, and a
streaming serde deserializer — one pipeline, four consumers.

> **Pre-release.** Nothing is published to crates.io yet; APIs change
> without deprecation cycles. The `yg` CLI (jq-style queries for YAML)
> lands in Milestone 10.

## Architecture

```
YAML bytes → Scanner (tokens) → Parser (events) → Resolver middleware
                                                        │
                    ┌───────────┬───────────────┬───────┴───────┐
                    │           │               │               │
              Streaming     Value (DOM)       CST          SAX/Callbacks
              serde Deser   (lossy)        (lossless)      (zero alloc)
```

| Crate | Role |
|-------|------|
| `yamalgam` | CLI (becomes `yg`) + re-exported public API |
| `yamalgam-core` | `Value` DOM, tag resolution, loader config, diagnostics |
| `yamalgam-scanner` | Bytes → tokens |
| `yamalgam-parser` | Tokens → events (full-fidelity: comments, indicators, spans) |
| `yamalgam-compose` | Events → `Value` documents |
| `yamalgam-cst` | Events → lossless concrete syntax tree (`to_text()` round-trips) |
| `yamalgam-serde` | Streaming serde `Deserializer` (no intermediate DOM) |

## Status

- YAML 1.2 core schema; event compliance 350/351 against the
  [YAML Test Suite](https://github.com/yaml/yaml-test-suite)
- CST round-trip fidelity 343/351 (byte-for-byte `parse → to_text`)
- Streaming serde deserializer with anchor/alias replay, multi-document
  iteration, structured errors with source spans, and behavioral parity
  with serde_yaml (divergences documented in tests)
- Composer and serde pipelines agree on the entire test suite
- Fastest of 8 benchmarked peers on small/medium inputs

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
```

The roadmap lives in [`docs/plans/README.md`](docs/plans/README.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
