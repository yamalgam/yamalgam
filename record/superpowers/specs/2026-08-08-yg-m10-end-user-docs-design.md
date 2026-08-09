# `yg` M10 End-User Documentation

**Date:** 2026-08-08
**Status:** Implemented

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
- Reusable filter programs need a clear file name convention that does not
  imply exact jq behavior or collide with an existing file type.

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

Reusable filter programs will use `.ygq`, short for "yg query." The documented
form is `yg -f program.ygq input.yaml`. The `-f` option will remain extension
agnostic for jq parity, pipes, and temporary files.

Markdown examples will use jq as the fence language and `.ygq` as the file
name passed through `mdcode` metadata:

````markdown
```jq file=examples/select-image.ygq
.services.api.image
```
````

This gives rendered docs jq syntax highlighting while `mdcode` extracts the
canonical file name. Named shell blocks will run the extracted filters and YAML
fixtures in a temporary directory. `mdcode` is a contributor check, not a
runtime dependency for `yg` users.

Editors should associate `*.ygq` with jq syntax support. We will not create a
Tree-sitter grammar while `yg` uses jq grammar through jaq and adds behavior
through native filters. A distinct grammar becomes valid only if the accepted
query syntax diverges from jq.

The source-of-truth order is:

1. Current CLI behavior and `yg --help` output.
2. CLI and query tests.
3. Public source types and the native filter list.
4. The [Draft-01 M10 implementation notes](../../../docs/spec/yg-spec-draft-01-implementation-notes.md)
   for known limits.
5. The [Draft-01 query language specification](../../../docs/spec/yg-query-language-spec-draft-01.md)
   only where tests or code confirm it.

We will run every command example against the current binary. We will show
output when it helps users spot success. Checks will cover `mdcode` examples,
Markdown, offline links, Bito readability, and focused CLI tests.

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

### Other filter file extensions

`.yg` is short but has an established ransomware search collision. `.jq` gets
good tool support but implies exact jq behavior. `.jqy` belongs to JSONiq.
`.ygq` states that the file is a `yg` query without claiming a new grammar.

### A custom Tree-sitter grammar

M10 adds YAML-aware values and native filters, not query syntax. A custom
grammar would duplicate jq grammar, drift from the jaq parser, and add a new
maintenance surface without improving M10 correctness.

### Separate example fixtures or doctests

Keeping `.ygq` fixtures beside the guides or copying examples into Rust or
shell tests would avoid `mdcode` metadata and another contributor tool. It
would also create a second copy of each example that could drift from the
published docs. `mdcode` keeps the fenced example canonical and extracts it
for named shell checks, so we accept the contributor-only dependency.

## Consequences

- Users gain a direct path from installation to useful queries.
- YAML-native behavior becomes clear without reading code or audit records.
- M10 limitations become explicit, especially the fidelity cost of `-i`.
- Multiple focused files cost more to maintain than one page.
- Hand-written CLI reference can drift. Examples and options must be checked
  against the binary when the CLI changes.
- `.ygq` files need one-time jq file association in editors that do not know
  the extension.
- `mdcode` adds a contributor tool, but turns copy-paste examples into checks
  that can fail when behavior changes.
- A docs site and generated reference remain deferred until the CLI is closer
  to release.

## Related decisions

No ADRs. This design groups user guides around the accepted M10 command. It
does not change the product or set a lasting technical constraint.
