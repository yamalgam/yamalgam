---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex M11 design]
---

# 0016: Use three resolution modes with explicit security boundaries

## Context and Problem Statement

The current include and reference policies use an `enabled` Boolean. M11 needs
two distinct choices. One grants permission. The other controls automatic
behavior. A query filter may resolve one node. A loader may resolve every
matching directive.

How should callers enable resolution without turning permission into ambient
filesystem or network access?

## Decision Drivers

- Keep all external loading disabled by default.
- Separate explicit query actions from automatic loading.
- Apply one filesystem policy to includes and file references.
- Require positive network scheme and host allowlists.
- Bound recursion and substitutions.
- Bound the bytes added to output.
- Bound bytes retained by the session cache and filesystem discovery work.
- Make cache hits obey the same logical budgets as fresh retrievals.

## Considered Options

- Keep one enabled Boolean for each capability.
- Use one global automatic-resolution switch.
- Add separate permission and automatic Booleans.
- Use one three-state mode for includes and another for references.

## Decision Outcome

Chosen option: use one three-state mode for each capability.

`Disabled` rejects the action. `Explicit` permits the matching query filter.
`Automatic` resolves matching directives during ordinary loading. An explicit
action recurses through the same capability. It does not grant the other
capability. That capability runs only when its mode is automatic.

All `LoaderConfig` presets keep both modes disabled. Their resolution limits
are:

- Moderate: depth 10, 256 edges, 256 MiB materialized, 256 MiB fetched, and
  100,000 discovery visits.
- Strict: depth 5, 32 edges, 10 MiB materialized, 10 MiB fetched, and 10,000
  discovery visits.
- Trusted: depth 64, 4,096 edges, unlimited materialized and fetched bytes,
  and 1,000,000 discovery visits.
- Unchecked: unlimited depth, edges, bytes, and discovery visits.

Fetched bytes and discovery visits are cumulative across one resolution
session. Discovery counts every opened directory and enumerated entry,
including non-matches. Glob traversal begins at its static directory prefix
and descends only as deeply as the pattern requires unless it contains `**`.

The file policy uses one opened root-directory capability. Includes use
relative paths. File references use relative paths or `file:` URIs. Deny rules
run before allow rules. Targets must be regular files. The default rejects
symlinks. An enabled symlink must still resolve inside the root.

HTTP and HTTPS references require an allowed scheme. They also require an
allowed host. M11 rejects URL user information. It disables redirects, cookies,
ambient authentication, and persistent caching. Redirects stay off until
Yamalgam can validate every hop.

The CLI maps `--allow-*` to explicit mode. It maps `--resolve-*` to automatic
mode. The two forms conflict for the same capability. Standard input and
anonymous values need an explicit root or base URI. Without one, they cannot
use relative resources.

### Consequences

- Good, because permission no longer implies automatic expansion.
- Good, because scripts can audit or select one directive with a query filter.
- Good, because disabled presets perform no external I/O.
- Good, because one session enforces limits across cached and fresh resources.
- Bad, because resolution configuration gains more fields and CLI conflicts.
- Bad, because callers must provide both schemes and hosts for network access.
- Bad, because M11 cannot follow redirects, even between allowed hosts.

### Confirmation

- Disabled, explicit, and automatic modes have separate integration tests.
- Explicit resolution never enables the other capability by accident.
- Each preset enforces its depth, edge, and byte limits.
- Fetched-byte and discovery-visit limits are cumulative across one session.
- File traversal, symlink escape, and network allowlist tests fail closed.
- Disabled policy performs no file or HTTP retrieval.

## Pros and Cons of the Options

### Keep enabled Booleans

- Good, because the current configuration shape stays small.
- Bad, because permission and automatic behavior remain ambiguous.

### Use separate permission and automatic Booleans

- Good, because both ideas become expressible.
- Bad, because invalid combinations require extra validation.

### Use three-state modes

- Good, because every state has one meaning.
- Good, because CLI flags map directly to policy.
- Bad, because the existing public policy structs change before 1.0.

## More Information

- [M11 external resource resolution design](../superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md).
- [ADR-0006: LoaderConfig for resource limits and security policy](0006-loaderconfig-for-resource-limits-and-security-policy.md).
- [ADR-0011: Reject unhandled resolution policy](0011-reject-unhandled-resolution-policy.md).
