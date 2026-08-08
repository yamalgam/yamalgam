---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex fresh audit remediation]
---

# 0014: Preserve structured resource-limit failures

## Context and Problem Statement

`ResourceLimits` enforces nine independent limits. Its public check methods
return `String`, even though every failure begins with three structured facts:
the resource category, the observed value, and the configured maximum.
Scanner, parser, composer, serde, CST, and query layers then store or wrap that
text. Callers must parse prose to identify the breached limit.

Arithmetic overflow is a different failure. It has no valid observed value or
configured maximum. Treating it as a configured limit breach would require
inventing data.

How should limit failures cross crate boundaries without changing their useful
human-readable diagnostics?

## Decision Drivers

- Let callers match a limit category without parsing text.
- Preserve the observed and configured values.
- Keep existing display messages stable.
- Keep the shared taxonomy in `yamalgam-core`.
- Do not fabricate structured values for arithmetic overflow.
- Render to text only where a foreign interface accepts text alone.

## Considered Options

- Keep returning strings and add message-parsing helpers.
- Return one struct with a string category.
- Return a typed category plus observed and maximum values.
- Add separate error variants for each of the nine limits.

## Decision Outcome

Chosen option: `yamalgam-core` defines a non-exhaustive `ResourceLimit` enum and
a `LimitExceeded` value containing `limit`, `observed`, and `maximum`.
`ResourceLimits::check_*` returns that value.

First-party public error types preserve `LimitExceeded` as a payload and error
source. Scanner keeps its existing message while carrying the value in the
`ScanErrorKind::LimitExceeded` variant. Parser, resolver, composer, serde, CST,
and query errors keep the structured value through their public boundaries.
Query orchestration errors also retain their fully wrapped display message so
scanner labels and per-document context remain unchanged.

Arithmetic overflow uses distinct `ResourceOverflow` variants in layers that
can encounter it. These variants retain a static resource description. They do
not pretend that an overflow was a configured maximum breach.

Adapters for jaq and other text-only interfaces call `to_string()` at that
foreign boundary. The structured error remains available from Yamalgam's
first-party build, parse, compose, deserialize, and emit APIs.

### Consequences

- Good, because callers can branch on `ResourceLimit` and inspect both values.
- Good, because `Display` output remains compatible with current diagnostics.
- Good, because new limit categories can be added to the non-exhaustive enum.
- Good, because overflow and configured policy failures are no longer
  conflated.
- Bad, because this changes public enum payloads before the 1.0 release.
- Bad, because foreign text-only interfaces still erase the type at their
  boundary.

### Confirmation

- Every check method returns the category, observed value, and maximum.
- Scanner, parser, composer, serde, CST, query build, query runtime, and query
  emit tests match structured payloads and their exact values.
- Existing display assertions continue to pass.
- The full repository gate remains green.

## More Information

- [ADR-0006](../../docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)
- [Fresh audit finding](../audits/2026-08-08-15-m10-fresh/README.md#resource-limit-checks-return-unstructured-strings)
