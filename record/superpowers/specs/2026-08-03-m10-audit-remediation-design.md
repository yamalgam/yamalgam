# M10 Audit Remediation Design

## Objective

Disposition all 47 findings in `record/audits/2026-08-03-21-m10-fresh/`
without weakening the audit record, mixing revisions, or closing findings on
assertion alone. Behavioral findings receive focused regression tests and
minimal code changes. Non-code dispositions require evidence and, for accepted
or deferred risk, Clay's explicit decision.

## Baseline and integrity

The campaign starts on branch `m10-yg-cli` at
`b96a66abf1495c84d2c23ced276678bd25527167`. The audit's `recon.yaml` names
that same commit. `README.md`, `report.html`, `findings.yaml`, `recon.yaml`,
`reviewer-verdicts.yaml`, and `assets/` remain immutable throughout
remediation. Only `actions-taken.md` may be created or appended inside the
audit directory.

Before each action, recheck the repository root, branch, HEAD, worktree, and
audit directory. User-owned changes are preserved. Remediation occurs on the
existing branch; no worktree is created.

## Remediation order

Work risk-first and sequentially:

1. Significant findings, beginning with `emitter-replays-tags-verbatim`.
2. Moderate findings, ordered so shared primitives land before their callers.
3. Advisory findings that have a concrete, proportionate implementation.
4. Notes and policy findings, using documentation, evidence-backed dispute, or
   an explicit accepted/deferred decision as appropriate.

One action may address multiple slugs only when the same code change resolves
the same underlying mechanism. Findings are not bundled merely because they
share a file or narrative surface.

## Action cycle

Each behavioral action uses this cycle:

1. Read the finding's complete concern, evidence, location, and remediation.
2. Identify the observable break and the focused existing test target.
3. Add one minimal regression test and run it with `cargo nextest` to observe
   the expected failure.
4. Implement the smallest change that makes the test pass, without unrelated
   refactoring or new dependencies.
5. Run the focused test target, then the affected crate's tests and clippy.
6. Review the diff and create a conventional code-fix commit.
7. Create or append `actions-taken.md`, recording the code-fix SHA, exact
   finding slug, disposition, author, and concise rationale.
8. Commit that ledger update separately. This avoids impossible self-referential
   commit metadata and keeps the code action independently reviewable.

Documentation and configuration findings use the same cycle except where no
behavioral test is meaningful. In those cases, the action records the focused
validation command or source evidence used instead.

## First action: tag emission boundary

`emitter-replays-tags-verbatim` has two inputs to one unsafe sink: decoded tag
metadata from parsed YAML and arbitrary strings supplied to `set_tag`. The fix
will define one representation-aware tag-safety rule inside
`yamalgam-query`, apply it before metadata mutation, and make the YAML emitter
refuse to replay an invalid tag. Existing valid local tags, core tags, and full
tag URIs must retain their current output.

Regression coverage will prove both boundaries:

- parsing `k: !x%0Ainjected%3A%20true v` and emitting it cannot introduce an
  `injected` mapping key;
- `set_tag` rejects a tag containing whitespace, line breaks, control
  characters, flow indicators, or malformed verbatim delimiters;
- existing `!custom` and `!!binary` round trips remain unchanged.

Rejecting an unsafe tag is preferred over inventing a second URI codec in the
query crate. Parsed malicious metadata is omitted at emission; filter-provided
invalid metadata is rejected with a jaq error. This closes the structure
injection path while preserving valid tags and avoiding a new dependency.

## Commit and ledger policy

Every implemented action receives a focused conventional commit. Every ledger
entry names that real code-fix SHA. The subsequent ledger-only commit is not
used as the action's `Commit` value. No entry uses `pending`, a working-tree
description, or a fabricated future SHA.

The ledger front matter always reflects the latest disposition for each slug,
and `open` is recomputed as
`47 - (fixed + mitigated + accepted + disputed + deferred)`. Past entries are
never edited; a superseding result is appended as a new entry.

## Verification and completion

Focused checks run after every action. `just check` runs after each coherent
surface and again before campaign completion. The audit artifact hashes are
rechecked before final status is reported.

When the ledger reaches zero open findings, remediation is not considered the
final assessment. A brand-new Cased audit must be run against the
post-remediation commit, in a new audit directory, with fresh reconnaissance,
analysis, evidence review, schema validation, and report assembly.
