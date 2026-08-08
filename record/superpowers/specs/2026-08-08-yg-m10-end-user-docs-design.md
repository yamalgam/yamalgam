# `yg` M10 End-User Documentation

**Date:** 2026-08-08
**Status:** Accepted

## Overview

We will add task-based guides for the `yg` command as it works in Milestone 10.
The guides will cover install, queries, YAML features, in-place edits, config,
and errors. They will not promise M11 or M13 behavior.

## Context

The root README gives `yg` a short introduction and a few examples. The query
spec and its M10 notes define the larger contract. Neither is a practical user
guide. Today, users must piece together workflows from CLI help, specs, tests,
and source code.

M10 is pre-release software. It has a broad query surface, but users need to
know its limits:

- `-i` replaces files atomically after it writes new YAML. This loses comments
  and exact layout until M13 adds concrete syntax tree (CST)-based editing that
  preserves the original YAML text.
- External include and reference resolution remains deferred.
- jq support is broad, but YAML rules and the chosen regex engine differ in
  known ways.
- Users install from the workspace until release packages are ready.

The guides must make those limits clear without letting future work bury the
happy path.

## Approach

We will add a small Markdown documentation suite:

```text
docs/
|-- README.md
`-- yg/
    |-- README.md
    |-- querying.md
    |-- yaml-features.md
    |-- editing.md
    |-- reference.md
    `-- troubleshooting.md
```

Each file has one job:

- `docs/README.md` separates user guides from the bundled YAML specs.
- `docs/yg/README.md` installs the workspace binary and reaches a useful first
  query in under five minutes.
- `querying.md` covers jq-style filters, input modes, variables, filter files,
  document streams, schemas, and output controls.
- `yaml-features.md` covers tags, anchors, aliases, styles, non-string keys,
  key order, and the native read and write filters.
- `editing.md` covers atomic in-place updates, safe use, saved metadata, and
  M10 fidelity limits.
- `reference.md` lists CLI options, native filters, config, environment
  variables, and exit codes.
- `troubleshooting.md` covers logs, resource profiles, common errors, and the
  `doctor` command.

The root README will keep its short product pitch and link to the new starting
point. The YAML specs under `docs/spec/` will not move or change.

The source-of-truth order is:

1. Current CLI behavior and `yg --help` output.
2. CLI and query tests.
3. Public source types and the native filter list.
4. The [Draft-01 M10 implementation notes](../../../docs/spec/yg-spec-draft-01-implementation-notes.md)
   for known limits.
5. The [Draft-01 query language specification](../../../docs/spec/yg-query-language-spec-draft-01.md)
   only where tests or code confirm it.

We will run every command example against the current binary. We will show
output when it helps users spot success. Checks will cover Markdown, offline
links, Bito readability, and focused CLI tests.

## Alternatives considered

### One large `docs/yg.md`

This would be fast to write. Install steps, recipes, reference data, and fixes
would all compete in one long page. It would become harder to scan as M11
through M13 land.

### A generated documentation site

A site with generated CLI reference would improve navigation and reduce drift.
It would also add deploy, theme, and build work before the CLI is stable. We
will revisit it near M13.

### Specification-only documentation

The spec helps with language design and jq parity work. It is the wrong entry
point for task-focused users. Keeping it as the only reference would preserve
the current gap.

## Consequences

- Users gain a direct path from installation to useful queries.
- YAML-native behavior becomes clear without reading code or audit records.
- M10 limitations become explicit, especially the fidelity cost of `-i`.
- Multiple focused files cost more to maintain than one page.
- Hand-written CLI reference can drift. Examples and options must be checked
  against the binary when the CLI changes.
- A docs site and generated reference remain deferred until the CLI is closer
  to release.

## Related decisions

No ADRs. This design groups user guides around the accepted M10 command. It
does not change the product or set a lasting technical constraint.
