# Architectural Decision Records

This directory holds the project's Architectural Decision Records (ADRs). Each
file records one important technical choice. Give each new ADR the next
four-digit number, then add it to this list.

| Status | ADR | Date |
|:------:|-----|------|
| ✅ accepted | [0001: Port libfyaml's state machine, redesign data structures in idiomatic Rust](0001-port-libfyaml-state-machine-redesign-data-structures.md) | 2026-03-06 |
| ✅ accepted | [0002: Preserve libfyaml's atom concept to enable CST round-trip fidelity](0002-preserve-atom-concept-for-cst-fidelity.md) | 2026-03-06 |
| ⚠️ deprecated | [0003: Use a comparison MCP server to verify scanner correctness against libfyaml](0003-comparison-mcp-server-for-correctness-verification.md) | 2026-03-06 |
| ✅ accepted | [0004: Support UTF-8, UTF-16, and UTF-32 encoding from day one](0004-utf8-16-32-encoding-support-from-day-one.md) | 2026-03-06 |
| ✅ accepted | [0005: Parser event model — flat events with inline directives](0005-parser-event-model-with-inline-directives.md) | 2026-03-07 |
| ✅ accepted | [0006: LoaderConfig for resource limits, resolution policy, and security boundaries](0006-loaderconfig-for-resource-limits-and-security-policy.md) | 2026-03-07 |
| 🔁 superseded | [0007: Resolver trait for composable event-stream middleware](0007-resolver-trait-for-event-stream-middleware.md) | 2026-03-08 |
| ✅ accepted | [0008: Bound the foreign config parser interim to yamalgam publication](0008-bound-config-parser-interim.md) | 2026-08-03 |
| ✅ accepted | [0009: Retain jaq time until native registries are composable](0009-retain-jaq-time-until-registries-compose.md) | 2026-08-03 |
| ✅ accepted | [0010: Use stateful event resolver adapters across consumers](0010-use-stateful-event-resolver-adapters.md) | 2026-08-08 |
| ✅ accepted | [0011: Reject unhandled resolution policy on no-op paths](0011-reject-unhandled-resolution-policy.md) | 2026-08-08 |
| ✅ accepted | [0012: Specialize parser-backed resolver upstreams](0012-specialize-parser-backed-resolver-upstreams.md) | 2026-08-08 |
| ✅ accepted | [0013: Retire the yamlstar benchmark peer until its Rust binding uses a maintained loader](0013-retire-yamlstar-benchmark-peer.md) | 2026-08-08 |
| ✅ accepted | [0014: Preserve structured resource-limit failures](0014-preserve-structured-resource-limit-failures.md) | 2026-08-08 |
| ✅ accepted | [0015: Use pluggable retrievers within shared resolution sessions](0015-pluggable-retrievers-with-shared-resolution-sessions.md) | 2026-08-08 |
| ✅ accepted | [0016: Use three resolution modes with explicit security boundaries](0016-use-three-mode-resolution-policy.md) | 2026-08-08 |
| ✅ accepted | [0017: Replace sole-key URI references with RFC 6901 targets](0017-use-sole-key-uri-references.md) | 2026-08-08 |
| ✅ accepted | [0018: Use structured includes for deterministic typed aggregation](0018-use-structured-include-aggregation.md) | 2026-08-08 |
| ✅ accepted | [0019: Record resolution reports and defer integrity lockfiles](0019-record-resolution-reports-and-integrity-metadata.md) | 2026-08-08 |

## Status labels

- ✅ `accepted`: current
- 🟡 `proposed`: under review
- ❌ `rejected`: considered but not adopted
- ⚠️ `deprecated`: no longer used
- 🔁 `superseded`: replaced by a later ADR, but kept as part of the project record
