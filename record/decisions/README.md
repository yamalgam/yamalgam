# Architectural Decision Records

ADRs 0001–0007 predate the repository's move from `docs/` to `record/`. This
index keeps the full sequence visible while new decisions remain with the
evolution record.

| Status | ADR | Date |
|:------:|-----|------|
| ✅ | [0001: Port libfyaml's state machine, redesign data structures in idiomatic Rust](../../docs/decisions/0001-port-libfyaml-state-machine-redesign-data-structures.md) | 2026-03-06 |
| ✅ | [0002: Preserve libfyaml's atom concept to enable CST round-trip fidelity](../../docs/decisions/0002-preserve-atom-concept-for-cst-fidelity.md) | 2026-03-06 |
| ⚠️ | [0003: Use a comparison MCP server to verify scanner correctness against libfyaml](../../docs/decisions/0003-comparison-mcp-server-for-correctness-verification.md) | 2026-03-06 |
| ✅ | [0004: Support UTF-8, UTF-16, and UTF-32 encoding from day one](../../docs/decisions/0004-utf8-16-32-encoding-support-from-day-one.md) | 2026-03-06 |
| ✅ | [0005: Parser event model — flat events with inline directives](../../docs/decisions/0005-parser-event-model-with-inline-directives.md) | 2026-03-07 |
| ✅ | [0006: LoaderConfig for resource limits, resolution policy, and security boundaries](../../docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md) | 2026-03-07 |
| ↪️ | [0007: Resolver trait for composable event-stream middleware](../../docs/decisions/0007-resolver-trait-for-event-stream-middleware.md) | 2026-03-08 |
| ✅ | [0008: Bound the foreign config parser interim to yamalgam publication](0008-bound-config-parser-interim.md) | 2026-08-03 |
| ✅ | [0009: Retain jaq time until native registries are composable](0009-retain-jaq-time-until-registries-compose.md) | 2026-08-03 |
| ✅ | [0010: Use stateful event resolver adapters across consumers](0010-use-stateful-event-resolver-adapters.md) | 2026-08-08 |
