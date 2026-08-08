---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex post-remediation audit]
---

# 0013: Retire the yamlstar benchmark peer until its Rust binding uses a maintained loader

## Context and Problem Statement

Yamalgam's comparative benchmark has seven ordinary Rust peers and one optional
`yamlstar` peer. The optional peer needs a separately installed
`libyamlstar` shared library, so it is absent from normal benchmark runs.
Enabling all workspace features still compiles its Rust binding.

The actively maintained `yamlstar` 0.1.18 binding depends on `dlopen` 0.1.8.
That crate unconditionally depends on `dlopen_derive` 0.1.4, even though
yamlstar uses the symbol API rather than its derive macros. RustSec classifies
both loader crates as unmaintained, reports no patched version, and recommends
moving to `dlopen2`. Yamalgam must accept RUSTSEC-2023-0051 just to compile this
peer. The peer is absent from the default performance baseline.

Should Yamalgam keep ownership of this optional comparison?

## Decision Drivers

- Keep all-feature builds free of accepted RustSec advisories.
- Keep benchmark peers reproducible on ordinary development and CI hosts.
- Avoid maintaining a fork or local FFI binding solely for one benchmark peer.
- Preserve varied peer coverage across event, DOM, and serde-style YAML APIs.
- Leave a clear condition for restoring yamlstar after its binding changes.

## Considered Options

- Keep yamlstar and continue accepting RUSTSEC-2023-0051.
- Fork yamlstar or replace its loader locally with `dlopen2`.
- Retire yamlstar from Yamalgam's benchmark matrix until the published binding
  uses a maintained dynamic loader.

## Decision Outcome

Chosen option: retire the yamlstar benchmark peer.

Remove its optional dependency and feature, its three benchmark cases, the
thin peer wrapper, and the RustSec ignore. The ordinary seven-peer comparative
matrix remains unchanged. Restore yamlstar only after a published release no
longer resolves `dlopen` or `dlopen_derive`. Its shared library must also be easy
to install on benchmark hosts.

Yamalgam will not fork the binding. YAMLStar is an active project. A parallel
FFI implementation would create ownership and correctness work outside
Yamalgam's product surface. That burden is not justified by a feature-gated
benchmark which is absent from the default baseline.

### Consequences

- Good, because all-feature dependency checks need no advisory exception.
- Good, because ordinary and all-feature builds use the same seven benchmark
  peers.
- Good, because Yamalgam does not acquire an FFI fork or loader abstraction.
- Bad, because the benchmark matrix no longer includes YAMLStar's
  cross-language shared-library implementation.
- Bad, because historical eight-peer results are not directly comparable with
  new seven-peer runs.

### Confirmation

- `cargo tree -i dlopen_derive` must report that the package is absent.
- `cargo audit` and `cargo deny --all-features check` must report no accepted
  advisory for RUSTSEC-2023-0051.
- The comparative benchmark must compile and run with its seven retained peers.
- The workspace all-feature gate must remain green.

## More Information

- [RustSec RUSTSEC-2023-0051](https://rustsec.org/advisories/RUSTSEC-2023-0051.html)
- [YAMLStar Rust binding](https://github.com/yaml/yamlstar/tree/main/rust)
- [Post-remediation audit finding](../audits/2026-08-04-16-m10-post-remediation/README.md#yamlstar-pulls-unmaintained-dlopen-derive)
