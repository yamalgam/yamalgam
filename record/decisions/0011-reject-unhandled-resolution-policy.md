---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex post-remediation audit]
---

# 0011: Reject unhandled resolution policy on no-op paths

## Context and Problem Statement

`LoaderConfig::resolution` publicly lets callers enable `!include` and `$ref`
policy. Built-in resolvers remain scheduled for M11. Before resolver middleware
reached every consumer, high-level configuration APIs accepted enabled policy
and then parsed normally without resolving anything or reporting that the
requested capability was absent.

ADR-0010 now gives every consumer a config-aware `ResolvedEvents` adapter.
Custom resolvers can read and enforce the selected policy, but ordinary loading
paths still use `NoopResolver`. What should happen when enabled external
resolution reaches that no-op path?

## Decision Drivers

- Enabled security policy must not be silently ignored.
- Built-in filesystem and network resolution must wait for complete M11
  enforcement of roots, allowlists, cycles, budgets, and timeouts.
- Explicit custom resolver entry points must remain usable with caller-selected
  policy.
- Parser and scanner APIs must remain policy-neutral because they produce the
  event stream that a later resolver consumes.
- Existing consumer error and CST recovery APIs should remain source compatible.

## Considered Options

- Implement partial built-in include and reference resolution now.
- Reject any enabled policy before parsing, including custom resolver paths.
- Hide or remove the public resolution fields until M11.
- Make the no-op resolver reject enabled policy while explicit custom resolvers
  retain responsibility for enforcement.

## Decision Outcome

Chosen option: make `NoopResolver` reject enabled include or reference policy.

The failure is a structured `ResolveError::UnsupportedResolutionPolicy` that
records both enabled capability flags. `ResolvedEvents` stops after a resolver
failure so one unsupported request produces one diagnostic. Upstream parser
errors retain their existing recovery behavior.

Every ordinary composer, serde, CST, and query loading path already uses
`NoopResolver`, so the adapter enforces the boundary consistently. Explicit
resolver-aware entry points accept enabled policy and delegate its enforcement
to the supplied resolver, as established by ADR-0010.

Low-level scanner and parser constructors continue to read only resource
limits. This is intentional: they do not perform the resolution stage, and
their event stream may be passed to a resolver that does.

The legacy `CstReport` stores `ParseError` diagnostics. It maps this one
no-op-policy failure to a matching structured
`ParseError::UnsupportedResolutionPolicy` instead of changing the report's
public type or panicking. The resolved CST report retains the original
`ResolveError`.

### Consequences

- Good, because enabled external resolution can no longer succeed silently on
  ordinary loading paths.
- Good, because no partial filesystem or network resolver bypasses the policy
  fields reserved for M11.
- Good, because applications can implement and test custom policy-aware
  resolvers now.
- Good, because existing high-level entry-point signatures remain unchanged.
- Bad, because custom resolver entry points trust the supplied resolver to
  enforce policy correctly.
- Bad, because the compatibility CST report mirrors one resolver error variant
  in `ParseError` until its diagnostic surface is unified.

### Confirmation

- Enabling include only, refs only, or both produces one typed no-op error.
- Composer, serde, CST, and query ordinary configured paths expose that error.
- A custom resolver observes enabled policy and continues successfully.
- Disabled policy preserves every existing default and unchecked path.
- CST recovery still reproduces every input byte.

## More Information

- [ADR-0006](../../docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)
- [ADR-0010](0010-use-stateful-event-resolver-adapters.md)
- [Post-remediation audit finding](../audits/2026-08-04-16-m10-post-remediation/README.md#loader-resolution-policy-is-entirely-unwired)
