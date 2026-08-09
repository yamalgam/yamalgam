---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex post-remediation audit]
---

# 0012: Specialize parser-backed resolver upstreams

## Context and Problem Statement

`ResolvedEvents` is generic over its resolver. However, it stores its upstream
event iterator as `Box<dyn Iterator>`. The normal composer, serde, CST, and
query entry points all create the same parser adapter. They box it at once.
Serde later boxes the full resolved stream at its public boundary. It also
boxes its tag resolver.

The post-remediation audit did not treat virtual calls alone as proof that a
refactor was needed. It required a profile of real workloads first. Three
100-sample Divan runs compare the current boxed upstream with a typed prototype.
Both paths have the same behavior. The inputs are a Kubernetes-style document
and a generated 1,000-record document.

All three runs favor the typed upstream:

| Surface | Small improvement | Medium improvement |
|---------|------------------:|-------------------:|
| resolved events | 4.5–6.0% | 2.5–3.8% |
| end-to-end composer | 3.9–5.1% | 2.4–4.4% |

Concrete and dynamic tag-resolution medians were alike in two runs. The third
run differed only at nanosecond scale and favored the concrete path. There was
no stable cost. Serde's erased end-to-end baseline also remains in place. No
equivalent typed serde path was measured. The audit profile record retains all
commands and medians.

How should Yamalgam remove this cost without adding needless public types?

## Decision Drivers

- Fix only dispatch costs shown in real workloads.
- Keep source compatibility for callers that name the existing
  two-parameter `ResolvedEvents<'input, R>` type.
- Keep boxed paths for mixed event sources.
- Keep serde's erased public boundary and boxed tag resolver until a profile
  shows a reason to change either one.
- Use one map function for all parser-backed constructors in each crate.

## Considered Options

- Accept the measured dispatch as a code-size trade-off.
- Make serde, composer, and every resolver consumer generic.
- Add a generic upstream iterator parameter to `ResolvedEvents` with the
  existing boxed iterator as its default.
- Add separate boxed and generic resolver adapter structs.

## Decision Outcome

Chosen option: add a generic upstream iterator parameter to `ResolvedEvents`,
defaulted to its current boxed iterator type.

Existing `ResolvedEvents<'input, R>` type annotations still select the boxed
form. Existing constructors still accept boxed iterators. Type inference lets
parser-backed callers pass their concrete adapter instead. Each consumer crate
uses one named function to map parse results. This gives repeated constructors
one shared upstream type instead of a new closure type at each call site.

Serde continues to box the full resolved iterator in `Deserializer`. This
preserves its stable, non-generic public type and its code-size boundary.
`TagResolver` also stays boxed because its dispatch had no repeatable cost.
Custom callers with mixed iterator types can still choose the default boxed
`ResolvedEvents` form.

The permanent benchmark will compare the production boxed and typed paths. It
is evidence and a regression tool, not a CI threshold. Wall-clock gates would
be unstable across development machines.

### Consequences

- Good, because normal load paths lose one measured vtable call per event.
- Good, because composition gains 2.4–5.1% with no change in behavior.
- Good, because old two-part type names and boxed construction remain valid.
- Good, because the benchmark can detect loss of the typed path. It does this
  without a noisy CI speed gate.
- Bad, because `ResolvedEvents` gains a third public type parameter. This may
  produce more copies of the iterator code.
- Bad, because serde still makes one dynamic iterator call per event.

### Confirmation

- An explicitly typed `ResolvedEvents<'_, NoopResolver>` still accepts a boxed
  parser adapter.
- A parser adapter passed directly has a concrete upstream type and no heap box.
- Default, configured, unchecked, and custom resolver paths keep their behavior
  in composer, serde, CST, and query.
- The permanent dispatch benchmark compiles and reports boxed, typed, composer,
  serde, and tag-resolution workloads.
- Focused suites and the full repository gate remain green.

## More Information

- [ADR-0010](0010-use-stateful-event-resolver-adapters.md)
- [Dispatch profile](../audits/2026-08-04-16-m10-post-remediation/type-erasure-profile.md)
- [Post-remediation audit finding](../audits/2026-08-04-16-m10-post-remediation/README.md#parser-consumers-erase-concrete-hot-path-types)
