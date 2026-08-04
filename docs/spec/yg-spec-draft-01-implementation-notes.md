# yg Draft 01 Implementation Notes

Milestone 10 implements Phases 1 and 2 of
[`yg-query-language-spec-draft-01.md`](yg-query-language-spec-draft-01.md).
This document records deliberate deviations and implementation-driven changes
for the draft-02 revision. It does not edit or reinterpret draft 01.
Bracketed `r[...]` links identify Tracey requirement IDs in draft 01.

## Compatibility baseline

The compatibility harness vendors the jq 1.8 manual at commit
`603db3f57741d217ba651e61086b550a72148b83` and executes every example triple.
Of 251 examples, 230 pass, 13 are expected failures, and 8 are skipped. Rules
match exact program-and-input pairs, require a reason, reject stale entries, and
fail on both unexpected failures and unexpected passes
([`r[phases.core]`](yg-query-language-spec-draft-01.md#phase-1-core-expression-engine)).

The 13 expected failures comprise four numeric-backend limitations; seven
intentional YAML semantic differences involving `type`, key order, and
order-sensitive mapping equality; and two jaq 3 differences involving
multi-path sequence deletion and unmatched optional regex captures. The eight
skips cover source-location/environment globals, jaq's missing `?//` grammar,
and jq streaming filters.

## 1. jaq 3 crate layout

Draft 01 names `jaq-syn` and `jaq-core`. jaq 3 consolidates parsing,
compilation, and execution in `jaq-core`; the implementation therefore uses
`jaq-core` and `jaq-std` 3.x. It does not use `jaq-json`, because `YamlVal`
implements jaq's value traits directly. This preserves the requirement to
adopt rather than reimplement jq's grammar
([`r[overview.jaq-foundation]`](yg-query-language-spec-draft-01.md#1-overview),
[`r[phases.core]`](yg-query-language-spec-draft-01.md#phase-1-core-expression-engine)).

## 2. Strict comparison is deferred

The `===` and `!==` tokens cannot be added without changing jaq's grammar.
Milestone 10 does not ship a strict-comparison operator; it exposes `tag`,
`anchor`, `style`, and `raw` for explicit representation checks instead. Draft
02 should specify a filter such as `strict_eq(f; g)` or depend on an upstream
jaq grammar extension
([`r[comparison.strict]`](yg-query-language-spec-draft-01.md#32-strict-comparison--)).

## 3. Alias expansion is `explode_aliases`

jq already defines `explode` as string-to-codepoint conversion, and the manual
compatibility contract requires that name to retain jq behavior. Recursive
alias materialization therefore ships as `explode_aliases`; JSON output performs
the equivalent metadata-dropping materialization internally. Draft 02 should
rename the YAML operation
([`r[ext.explode]`](yg-query-language-spec-draft-01.md#67-anchoralias-operations),
[`r[cli.json-output]`](yg-query-language-spec-draft-01.md#83-output-flags)).

## 4. Chomping is deferred

The query value builder receives scalar style from parser events, but chomping
is scanner/CST presentation data. The `chomping` filter remains deferred to the
CST phase, consistent with its absence from the Phase 1 filter list
([`r[ext.chomping]`](yg-query-language-spec-draft-01.md#61-node-metadata-filters),
[`r[phases.cst]`](yg-query-language-spec-draft-01.md#phase-3-cst-round-trip)).

## 5. Numeric storage is `i64` plus `f64`

`YamlVal` matches yamalgam's current resolved value model: integers are `i64`
and floats are IEEE 754 binary64. Checked integer arithmetic promotes to `f64`
on overflow. Four jq-manual cases remain expected failures because arbitrary
precision and original decimal literal spelling are not preserved
([`r[datamodel.types.int]`](yg-query-language-spec-draft-01.md#21-resolved-value-types),
[`r[comparison.resolved.numeric]`](yg-query-language-spec-draft-01.md#31-resolved-comparison--)).

## 6. Mapping ordering follows mapping equality

Mapping equality is insertion-order-sensitive, as draft 01 requires. Mapping
ordering is also insertion-order-sensitive so `Ord == Equal` remains consistent
with `Eq`; sorting and deduplication would otherwise disagree. One jq-manual
object-equality example is expected to differ
([`r[comparison.resolved.collection]`](yg-query-language-spec-draft-01.md#31-resolved-comparison--),
[`r[comparison.ordering]`](yg-query-language-spec-draft-01.md#33-ordering----)).

## 7. Tags have deterministic shorthand

Parser-resolved core tags are reported as `!!name`. Local tags and other URIs
retain their stored spelling. This resolves draft 01's examples, which mix core
shorthand and full URI forms, into one deterministic rule
([`r[ext.tag]`](yg-query-language-spec-draft-01.md#61-node-metadata-filters)).

## 8. Document schema pragmas are deferred

Milestone 10 ships explicit `--schema` selection and the `schema`, `resolve`,
and `resolve_as` filters. It does not inspect the `# yg-schema:` document
pragma, which was not part of Phases 1 or 2
([`r[datamodel.schema.document-declared]`](yg-query-language-spec-draft-01.md#23-document-declared-schema),
[`r[cli.schema-flag]`](yg-query-language-spec-draft-01.md#84-schema-and-security-flags)).

## 9. Exit code 4 represents no output

With `-e`, `yg` returns 4 when a successful filter emits no values. This follows
jq and does not conflict with draft 01's defined 0, 1, 2, 3, and 5 classes
([`r[cli.exit]`](yg-query-language-spec-draft-01.md#85-exit-codes)).

## 10. External-resolution flags wait for M11

`--allow-includes` and `--allow-refs` would be dead controls before the resolver
filters exist, so they are deferred with external resolution. `--trusted` and
`--strict` ship now because they select existing loader resource policies
([`r[cli.allow-includes]`](yg-query-language-spec-draft-01.md#84-schema-and-security-flags),
[`r[cli.allow-refs]`](yg-query-language-spec-draft-01.md#84-schema-and-security-flags),
[`r[phases.resolution]`](yg-query-language-spec-draft-01.md#phase-4-external-resolution)).

## 11. Library paths remain deferred; variable arguments ship

`-L` remains outside the M10 CLI surface. `--arg NAME VALUE` and
`--argjson NAME JSON` now bind data through jaq globals rather than requiring
shell interpolation into filter source. The JSON form uses strict JSON parsing
under the active resource limits
([`r[cli]`](yg-query-language-spec-draft-01.md#8-cli-interface),
[`r[overview.jaq-foundation]`](yg-query-language-spec-draft-01.md#1-overview)).

## 12. Regex syntax is the regex-bites subset

jaq-std 3.0.1 implements regex filters with `regex-bites`, not jq's Oniguruma
engine. Common jq patterns, named captures, and flags work, but lookaround,
backreferences, atomic groups, and possessive quantifiers are unsupported.
Draft 02 should replace the draft-01 "PCRE-compatible" promise with this
finite-automata constraint
([`r[filters.regex]`](yg-query-language-spec-draft-01.md#45-regular-expressions)).

## 13. Normal file input is lazy per file, not per document

Normal multi-file queries read, parse, evaluate, and release one file at a
time. `-s` intentionally materializes the unified input sequence, and `-i`
stages every rendered file before replacement. Within one file, the scanner's
borrowed-input architecture still requires the complete decoded file and the
query builder currently constructs all of its documents before evaluation;
an unterminated standard-input stream therefore does not produce incremental
documents. Full document streaming requires an incremental scanner input
source and remains a post-M10 pipeline change.

## 14. `resolved_type` aliases `type`

The builder resolves scalar text under the active schema before query
evaluation, so M10 retains no unresolved value state for `resolved_type` to
inspect. It therefore aliases `type`; after `resolve_as`, both report the type
of the newly resolved value. Draft 02 should either remove the redundant filter
or define a distinct result that the value model can represent
([`r[ext.resolved-type]`](yg-query-language-spec-draft-01.md#62-type-introspection)).

## Additional implementation-driven changes

- Librebar 0.4 reserves `-C` for changing directory and exposes the canonical
  `--color auto|always|never` control. `yg` uses that shared interface instead
  of draft 01's `-C`/`-M` color pair
  ([`r[cli.color]`](yg-query-language-spec-draft-01.md#83-output-flags)).
- `-i` writes a same-directory temporary file, syncs it, preserves permissions,
  and atomically replaces each source file. It carries supported node metadata
  through reserialization, but comments and exact formatting are lost until
  CST-spliced editing lands. `-i` is exclusive with `-n`, `-s`, and JSON output
  (both `-j` and `--format json`): null/slurp modes have no coherent per-file
  replacement semantics, and JSON output would silently change a YAML file's
  format
  ([`r[cli.inplace]`](yg-query-language-spec-draft-01.md#83-output-flags),
  [`r[output.yaml.style-preservation]`](yg-query-language-spec-draft-01.md#91-yaml-output-default)).
