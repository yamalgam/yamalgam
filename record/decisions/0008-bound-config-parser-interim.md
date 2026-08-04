---
status: accepted
date: 2026-08-03
decision-makers: [Clay Loveless]
consulted: [Codex M10 audit]
---

# 0008: Bound the foreign config parser interim to yamalgam publication

## Context and Problem Statement

Librebar uses `serde-saphyr` to read YAML config files. `yg` finds project
config before it reads query input. A repository can therefore supply YAML to
this parser during startup.

This stack handles config only. It cannot change query limits. Still, it copies
a core yamalgam feature and adds two new parser crates to `yg`.

How do we control this interim until yamalgam can replace it? The target design
already uses a pluggable Librebar backend. We need to give the interim a clear
end point.

## Decision Drivers

- The swap needs a published yamalgam crate after M10.
- Project config may come from an untrusted repository.
- Parser updates on this path need review.
- Config cannot weaken query limits. It only sets two logging fields.
- Removing YAML configuration would break an existing CLI contract.
- Only `Cargo.lock` controls Librebar's two-hop version ranges.

## Considered Options

- Swap Librebar to yamalgam immediately.
- Remove YAML as a supported configuration format until the swap.
- Keep the current backend as a version-pinned interim tied to publication.
- Accept the current backend indefinitely and let routine updates move it.

## Decision Outcome

Chosen option: "Keep a version-pinned interim tied to publication." This keeps
the current CLI and gives the extra parser stack a firm end condition. The
yamalgam crates.io release will trigger the Librebar backend work.

Until then, we keep `serde-saphyr` 1.0.0 and `granit-parser` 1.0.0 in the
lockfile. Just update recipes restore both versions after a broad update. A
version change now needs an explicit edit and review.

### Consequences

- Good, because the planned backend swap remains the only destination.
- Good, because routine update jobs cannot move the parser versions in secret.
- Good, because the existing TOML, JSON, YAML, and YML config contract remains
  intact.
- Bad, because `yg` still ships 507 KiB of duplicate parser code.
- Bad, because parser fixes now need a manual pin review.

### Confirmation

- Cargo tree must show both 1.0.0 crates under Librebar.
- Every mutating Just update must restore both exact versions.
- After yamalgam ships, add the Librebar backend and supersede this ADR.
- Then remove both pins and verify both crates leave the shipped `yg` graph.

## Pros and Cons of the Options

### Swap immediately

- Good, because it removes the duplicate parser now.
- Bad, because the cross-repository work is blocked on publication.
- Bad, because doing it in M10 adds dependency-cycle pressure.

### Remove YAML configuration temporarily

- Good, because the foreign parser leaves the startup path immediately.
- Bad, because it breaks documented config discovery for a short-term issue.

### Keep a bounded, pinned interim

- Good, because it keeps compatibility and sets a clear end point.
- Bad, because it carries known duplication until the publication milestone.

### Accept the backend indefinitely

- Good, because it needs no coordination with Librebar.
- Bad, because `yg` would always ship another YAML parser for its own config.
- Bad, because yamalgam would lose the planned dogfooding path.

## More Information

- [M10 audit finding](../audits/2026-08-03-21-m10-fresh/README.md#yg-ships-competing-yaml-parser-for-its-own-config)
- [M10 query CLI implementation plan](../../docs/plans/2026-06-10-m10-yg-cli-implementation-plan.md)
- [Release and crates.io setup](../../.github/docs/releases.md)
