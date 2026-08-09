# yg Query Language Specification

**Status:** Draft
**Date:** 2026-03-08
**Author:** Clay Loveless
**Engine:** yamalgam

---

## 1. Overview

r[overview]

yg is a YAML-native query and transformation tool. Its expression language is jq-familiar: it reuses jq's grammar (via the jaq parser/evaluator crates) but operates on YAML values with YAML semantics. Where YAML's data model diverges from JSON's, yg follows YAML. Where YAML has capabilities JSON lacks, yg extends the filter vocabulary.

r[overview.positioning]
yg is NOT "jq for YAML." yg is a YAML processor whose expression language is jq. The distinction determines every semantic decision in this specification.

r[overview.jaq-foundation]
yg MUST use jaq-syn and jaq-core as its expression parser and evaluator engine. yg MUST NOT reimplement the jq expression grammar. YAML-specific behavior is achieved through a custom `ValT` implementation and registered native filters, not through parser modifications.

r[overview.compat-promise]
yg does not promise jq compatibility. yg promises jq *familiarity*. Where YAML semantics demand different behavior from JSON semantics, yg follows YAML and documents the divergence. Users who need exact jq JSON semantics SHOULD pipe through `yg -j | jq`.

### 1.1 Milestone 11 Implementation Status

This draft describes the target language. Milestone 11 implements Phases 1, 2,
and 4 with two language-level deferrals:

- [`r[comparison.strict]`](#32-strict-comparison--) is deferred pending the
  draft-02 decision on extending jaq without forking its grammar.
- [`r[ext.chomping]`](#61-node-metadata-filters) is deferred to the Phase 3
  CST milestone, where the required presentation metadata is available.

External resolution now ships with local `!include`, local and HTTP `$ref`,
explicit and automatic policy modes, filesystem sandboxing, cycle and resource
budgets, and in-memory resolution reports. Redirect-hop validation, HTTP
includes, authentication, parameters, dependency commands, and lockfiles
remain deferred.

The [draft-01 implementation notes](yg-spec-draft-01-implementation-notes.md)
record the complete rationale and every other implementation-driven change.

---

## 2. Data Model

r[datamodel]

yg operates on YAML values, not JSON values. The data model is richer than JSON's.

### 2.1 Value Types

r[datamodel.types]

Every yg value is one of the following kinds:

r[datamodel.types.null]
**Null** — the absence of a value. Produced by resolving the plain scalars `null`, `Null`, `NULL`, or `~` under the core schema, or by explicit `!!null` tag.

r[datamodel.types.bool]
**Boolean** — `true` or `false`. Under the YAML 1.2 core schema, ONLY `true`, `True`, `TRUE`, `false`, `False`, `FALSE` resolve to booleans. Under no circumstances do `yes`, `no`, `on`, `off`, `y`, `n` resolve to booleans in the default schema.

r[datamodel.types.int]
**Integer** — an arbitrary-precision integer. Produced by resolving plain scalars matching integer patterns under the core schema (decimal, hex `0x`, octal `0o`, binary `0b`), or by explicit `!!int` tag.

r[datamodel.types.float]
**Float** — an IEEE 754 double-precision floating point number, plus `.inf`, `-.inf`, and `.nan`. Produced by resolving plain scalars matching float patterns under the core schema, or by explicit `!!float` tag.

r[datamodel.types.str]
**String** — a Unicode string. Any quoted scalar, or any plain scalar that does not resolve to another type under the active schema. This is the fallback type.

r[datamodel.types.seq]
**Sequence** — an ordered list of values. Corresponds to jq's array type.

r[datamodel.types.mapping]
**Mapping** — an ordered collection of key-value pairs. Keys may be any value type (not restricted to strings as in JSON). Corresponds to jq's object type, but with preserved insertion order and non-string key support.

### 2.2 Node Metadata

r[datamodel.metadata]

Unlike JSON values, YAML nodes carry metadata that yg preserves and exposes.

r[datamodel.metadata.tag]
Every node MAY have a **tag** — a string indicating the node's type. Explicit tags like `!!str`, `!!int`, `!!binary`, or custom tags like `!include` are preserved. Nodes without explicit tags have an implicit tag determined by schema resolution.

r[datamodel.metadata.anchor]
Every node MAY have an **anchor** — a named reference point. Anchor names are preserved as strings. Alias nodes reference anchors.

r[datamodel.metadata.style]
Every scalar node has a **style**: `plain`, `single-quoted`, `double-quoted`, `literal` (`|`), or `folded` (`>`). Block scalars additionally carry a chomping indicator (`clip`, `strip`, `keep`). Collection nodes have a style: `block` or `flow`.

r[datamodel.metadata.comment]
Nodes MAY have associated **comments**: leading comments (above/before the node), inline comments (same line, after the value), and trailing comments (below the node, before the next sibling). Comment access requires CST-level parsing and is gated on the CST milestone.

### 2.3 Schema Resolution

r[datamodel.schema]

Schema resolution determines how plain scalars are interpreted as typed values.

r[datamodel.schema.default]
The default schema is **YAML 1.2 Core**. Plain scalars are resolved according to the YAML 1.2 core schema rules: `null`/`true`/`false` keywords, integer patterns, float patterns, everything else is a string.

r[datamodel.schema.failsafe]
The **failsafe** schema (`--schema=failsafe`) treats all plain scalars as strings. No implicit typing. This is the StrictYAML equivalent. Use this when you want to eliminate all type coercion surprises.

r[datamodel.schema.json]
The **JSON** schema (`--schema=json`) matches JSON's type rules exactly. Useful when processing YAML that was machine-generated from JSON.

r[datamodel.schema.yaml11]
The **YAML 1.1** schema (`--schema=1.1`) applies legacy YAML 1.1 boolean rules (yes/no/on/off/y/n etc. are booleans). This exists for backward compatibility with documents authored against YAML 1.1 parsers. It SHOULD emit a warning.

r[datamodel.schema.document-declared]
A document MAY declare its schema compliance via a `%YAML` directive or a `# yg-schema:` comment pragma. When present, yg MUST use the declared schema for that document unless overridden by a CLI flag.

r[datamodel.schema.document-declared.pragma]
The pragma format is: `# yg-schema: <schema-name>` appearing as a comment before the first document content. Valid schema names are `core`, `failsafe`, `json`, `1.1`. This pragma is yg-specific and not part of the YAML specification.

---

## 3. Comparison Semantics

r[comparison]

yg provides two comparison modes, reflecting the fundamental tension between YAML's resolved values and its raw representation.

### 3.1 Resolved Comparison (`==`, `!=`)

r[comparison.resolved]

The `==` operator compares **resolved values** after schema resolution.

r[comparison.resolved.type-coercion]
Values of different resolved types are never equal under `==`. The integer `1` and the string `"1"` are not equal. There is no implicit type coercion. This diverges from PHP's `==` deliberately — yg's `==` is type-safe after resolution.

r[comparison.resolved.numeric]
Integer and float values compare by numeric value. `1 == 1.0` is `true`. This matches jq behavior.

r[comparison.resolved.string]
Strings compare by Unicode content, regardless of original quoting style. A single-quoted `'hello'` and a double-quoted `"hello"` and a plain `hello` (resolved as string) are all equal under `==`.

r[comparison.resolved.null]
`null == null` is `true`. `null == false` is `false`. Null is its own type.

r[comparison.resolved.collection]
Sequences compare element-by-element. Mappings compare by key-value pair equality, **order-sensitive**. Two mappings with the same entries in different order are NOT equal under `==`. This diverges from jq where object comparison is order-insensitive.

### 3.2 Strict Comparison (`===`, `!==`)

r[comparison.strict]

The `===` operator compares **representation-level identity**.

r[comparison.strict.scalar-text]
For scalars, `===` compares the raw scalar text content AND the resolved type. `1 === 1` is `true`. `1 === 1.0` is `false` (different text). `"true" === true` is `false` (string vs boolean).

r[comparison.strict.style]
`===` does NOT compare style. A plain `hello` and a quoted `"hello"` that both resolve to the same string are `=== true`. Style is metadata, not identity. Use the `style` filter to compare styles explicitly.

r[comparison.strict.anchor-identity]
For aliased nodes, `===` compares anchor identity. Two alias nodes referencing the same anchor are `=== true`. Two independent nodes with identical content are `=== false` if they are not aliases of the same anchor. This enables detecting shared-reference relationships that `==` cannot distinguish.

### 3.3 Ordering (`<`, `<=`, `>`, `>=`)

r[comparison.ordering]

Ordering operators work on **resolved values** only. There is no strict-ordering equivalent.

r[comparison.ordering.numeric]
Numbers (integer and float) order numerically.

r[comparison.ordering.string]
Strings order lexicographically by Unicode code point.

r[comparison.ordering.cross-type]
Cross-type ordering follows jq's convention: `null < false < true < numbers < strings < sequences < mappings`.

---

## 4. Filters — jq-Compatible Core

r[filters]

yg implements the jq filter language via jaq-core. The following sections document where yg's behavior is identical to jq and where it diverges.

### 4.1 Identity and Path Access

r[filters.identity]
`.` is the identity filter. Produces the input unchanged.

r[filters.field]
`.foo` and `.foo.bar` access mapping values by string key. Equivalent to jq's object identifier-index. If the key does not exist, produces `null`.

r[filters.field.nonstring-keys]
YAML mappings may have non-string keys. `.["key"]` syntax works for any key type: `.[ 42 ]` accesses an integer key, `.[ true ]` accesses a boolean key. This extends jq, which only supports string keys.

r[filters.index]
`.[n]` accesses a sequence element by zero-based index. Negative indices count from the end.

r[filters.slice]
`.[m:n]` produces a subsequence or substring.

r[filters.iterator]
`.[]` iterates over sequence elements or mapping values.

r[filters.optional]
`.foo?` and `.[]?` suppress errors instead of failing.

r[filters.recurse]
`..` recursively descends into all values (recursive descent).

### 4.2 Pipe, Comma, Parentheses

r[filters.pipe]
`|` pipes the output of one filter as input to the next. Identical to jq.

r[filters.comma]
`,` produces multiple outputs. `(.foo, .bar)` outputs both `.foo` and `.bar`.

r[filters.paren]
`(expr)` groups expressions. Identical to jq.

### 4.3 Construction

r[filters.construct.seq]
`[expr]` collects all outputs of `expr` into a sequence.

r[filters.construct.mapping]
`{key: value}` constructs a mapping. Keys may be expressions. Identical to jq's object construction.

### 4.4 Conditionals and Logic

r[filters.cond.if]
`if cond then expr elif cond then expr else expr end` — identical to jq.

r[filters.cond.and-or-not]
`and`, `or`, `not` — identical to jq. These use jq truthiness: `false` and `null` are falsy, everything else is truthy.

r[filters.cond.try]
`try expr catch expr` — identical to jq.

r[filters.cond.alternative]
`expr // default` — alternative operator. Produces `default` if `expr` is `false` or `null`. Identical to jq.

### 4.5 Variable Binding and Reduction

r[filters.binding]
`expr as $name | ...` — bind the output of `expr` to `$name`. Identical to jq.

r[filters.reduce]
`reduce expr as $name (init; update)` — fold/reduce operation. Identical to jq.

r[filters.foreach]
`foreach expr as $name (init; update; extract)` — streaming fold. Identical to jq.

### 4.6 User-Defined Functions

r[filters.def]
`def name(args): body;` — define reusable filter functions. Identical to jq.

r[filters.def.recursion]
Recursive function definitions are supported. Identical to jq.

### 4.7 String Interpolation

r[filters.string-interp]
`"Hello \(.name)"` — string interpolation. Identical to jq.

### 4.8 Assignment

r[filters.assign.update]
`|=` — update-assignment. Applies a filter to the selected value in-place. Identical to jq.

r[filters.assign.plain]
`=` — plain assignment. Sets the selected path to a new value. Identical to jq.

r[filters.assign.arithmetic]
`+=`, `-=`, `*=`, `/=`, `%=`, `//=` — arithmetic update-assignment. Identical to jq.

### 4.9 Regular Expressions

r[filters.regex]
`test`, `match`, `capture`, `scan`, `split`, `sub`, `gsub` — identical to jq. PCRE-compatible patterns.

---

## 5. Filters — Divergences from jq

r[diverge]

These filters exist in jq but behave differently in yg due to YAML semantics.

### 5.1 `type`

r[diverge.type]
`type` returns the **YAML data model** type name as a string. The possible values are: `"null"`, `"boolean"`, `"integer"`, `"float"`, `"string"`, `"sequence"`, `"mapping"`.

r[diverge.type.rationale]
jq's `type` returns `"object"` and `"array"`. yg returns `"mapping"` and `"sequence"` because these are the YAML specification terms. This is an intentional divergence. Code that tests `type == "object"` will not work; use `type == "mapping"`.

### 5.2 `keys` and `keys_unsorted`

r[diverge.keys]
`keys` returns mapping keys in **insertion order** (the order they appear in the document). This diverges from jq, where `keys` returns sorted keys.

r[diverge.keys-sorted]
`keys_sorted` returns mapping keys in sorted order (alphabetical for strings). This is equivalent to jq's `keys`. Users porting jq scripts that rely on `keys` being sorted SHOULD replace with `keys_sorted`.

r[diverge.keys.seq]
For sequences, `keys` returns `[0, 1, 2, ...]` — the indices. Identical to jq.

### 5.3 `sort` and `sort_by`

r[diverge.sort]
`sort` and `sort_by(f)` sort using resolved-value comparison (`==` semantics). Identical to jq in behavior, operating on resolved values.

### 5.4 `to_entries`, `from_entries`, `with_entries`

r[diverge.entries]
`to_entries` produces `[{"key": k, "value": v}, ...]` from a mapping. Identical to jq. Key order follows insertion order.

r[diverge.entries.rich]
`to_entries_rich` produces entries with YAML metadata: `[{"key": k, "value": v, "key_tag": t, "value_tag": t, "key_style": s, "value_style": s, "anchor": a}, ...]`. Fields with no metadata are omitted.

### 5.5 `add`

r[diverge.add]
`add` on sequences of mappings produces a merged mapping. The merge follows YAML merge semantics: later keys override earlier keys, insertion order of first occurrence is preserved. This matches jq's `add` for objects.

### 5.6 Arithmetic on Strings and Mappings

r[diverge.multiply]
`*` on two mappings performs a recursive merge. Identical to jq.

r[diverge.add-strings]
`+` on two strings concatenates. Identical to jq.

r[diverge.add-mappings]
`+` on two mappings merges (right-hand side wins). Identical to jq.

---

## 6. Filters — YAML-Native Extensions

r[ext]

These filters do not exist in jq. They expose YAML-specific capabilities.

### 6.1 Node Metadata Filters

r[ext.tag]
`tag` returns the tag of the current node as a string. For nodes with explicit tags, returns the full tag (e.g., `"!!str"`, `"!!int"`, `"!include"`, `"tag:yaml.org,2002:str"`). For nodes with implicit tags (resolved by schema), returns `null`.

r[ext.set-tag]
`set_tag(t)` sets the tag on the current node. Used for mutation operations.

r[ext.anchor]
`anchor` returns the anchor name of the current node as a string, or `null` if no anchor is set.

r[ext.set-anchor]
`set_anchor(name)` sets an anchor on the current node.

r[ext.style]
`style` returns the presentation style of the current node as a string: `"plain"`, `"single-quoted"`, `"double-quoted"`, `"literal"`, `"folded"` for scalars; `"block"`, `"flow"` for collections.

r[ext.set-style]
`set_style(s)` sets the presentation style. Valid arguments depend on node kind.

r[ext.chomping]
`chomping` returns the block scalar chomping indicator: `"clip"`, `"strip"`, or `"keep"`. Returns `null` for non-block-scalar nodes.

### 6.2 Type Introspection

r[ext.resolved-type]
`resolved_type` returns the type name after schema resolution. For a plain scalar `yes` under the YAML 1.1 schema, `type` returns `"boolean"` but under the core schema, `type` returns `"string"`. `resolved_type` always returns the resolution result under the active schema.

r[ext.kind]
`kind` returns the YAML node kind without schema resolution: `"scalar"`, `"sequence"`, `"mapping"`, `"alias"`. This distinguishes scalars that haven't been resolved yet from their resolved types.

r[ext.is-alias]
`is_alias` returns `true` if the current node is an alias reference. `false` otherwise.

r[ext.raw]
`raw` returns the raw scalar text content as a string, without schema resolution. For the plain scalar `1.0`, `raw` returns the string `"1.0"` even though the resolved value is the float `1.0`. For non-scalar nodes, produces an error.

### 6.3 Schema Filters

r[ext.schema]
`schema` returns the name of the currently active schema as a string: `"core"`, `"failsafe"`, `"json"`, `"1.1"`.

r[ext.resolve]
`resolve` explicitly resolves the current node under the active schema. Useful after `set_tag` or when operating on raw scalars.

r[ext.resolve-as]
`resolve_as(schema_name)` resolves the current node under a specified schema. `"1.0" | resolve_as("1.1")` re-resolves the string `"1.0"` under YAML 1.1 rules.

### 6.4 Document Filters

r[ext.documents]
In a multi-document YAML stream, each document is a separate value in yg's input stream. Filters apply to each document independently (like jq's multi-value processing). No special modes needed.

r[ext.document-index]
`document_index` returns the zero-based index of the current document within its source stream.

r[ext.file-index]
`file_index` returns the zero-based index of the current input file (when processing multiple files).

r[ext.filename]
`filename` returns the name of the current input file as a string, or `null` for stdin.

### 6.5 Comment Filters (CST milestone)

r[ext.comments]
**Gated on CST milestone.** These filters require CST-level parsing and are not available until the CST layer is implemented.

r[ext.comments.leading]
`leading_comments` returns the leading comment(s) of the current node as a string (or sequence of strings).

r[ext.comments.inline]
`inline_comment` returns the inline comment of the current node, or `null`.

r[ext.comments.trailing]
`trailing_comments` returns the trailing comment(s) of the current node.

r[ext.comments.all]
`comments` returns all comments associated with the current node as a sequence of strings.

r[ext.comments.set-inline]
`set_inline_comment(s)` sets the inline comment on the current node.

r[ext.comments.set-leading]
`set_leading_comment(s)` sets the leading comment on the current node.

### 6.6 Merge Key Handling

r[ext.merge]
`merge` explicitly expands YAML merge keys (`<<:`) on the current mapping. Returns the mapping with merge key entries inlined and the merge key itself removed.

r[ext.merge.preserve]
By default, yg does NOT auto-expand merge keys. Merge keys are preserved as regular mapping entries with the `!!merge` tag. This diverges from mfyq which auto-expands in some contexts. Users who want expansion MUST call `merge` explicitly.

### 6.7 Anchor/Alias Operations

r[ext.explode]
`explode` dereferences all aliases in the current node, replacing them with copies of their anchor targets. Anchor names are removed. This matches jq-in-YAML behavior (no alias concept) and is the operation yg applies internally before `yg -j` JSON output.

r[ext.aliases]
`aliases` returns a mapping of anchor names to their aliased paths within the current node. Useful for auditing anchor usage.

r[ext.orphan-anchors]
`orphan_anchors` returns a sequence of anchor names that are defined but never referenced by an alias.

---

## 7. External Resolution

r[resolution]

yg supports loading external content through `!include` tags and sole-key
`$ref` mappings. Both capabilities are disabled by default and governed by the
active `LoaderConfig` resolution policy.

r[resolution.modes]
Each capability has one mode: `Disabled`, `Explicit`, or `Automatic`.
`Disabled` rejects the corresponding query filter. `Explicit` permits the
filter but preserves directives during ordinary loading. `Automatic` resolves
matching directives before query evaluation. An explicit filter MUST recurse
through the selected capability. The other capability recurses only when its
mode is `Automatic`.

### 7.1 `!include` Resolution

r[resolution.include]
An include directive MUST be a scalar or mapping tagged `!include`. Includes
MUST address filesystem resources relative to the containing document and MUST
NOT retrieve HTTP or HTTPS URLs.

r[resolution.include.filter]
`include` resolves the tagged current node under explicit include policy. A
scalar directive substitutes the root node from exactly one YAML document.

r[resolution.include.structured]
A structured include MUST contain exactly one selector: `path`, `glob`, or
`dir`. `format` MAY be `yaml` or `text`. `mode` MAY be `node`, `list`,
`named`, `merge`, or `splice`. Directory selection MAY set `recursive` and
`match`. Named output MAY set `key`. Mapping merges MAY set `on_conflict`.
Unknown options MUST produce an error. Duplicate, conflicting, or
mode-incompatible options MUST also produce an error.

r[resolution.include.aggregation]
`node` MUST require one match. `list` MUST return one result per match. `named`
MUST derive deterministic keys from filename stems or extensionless relative
paths. `merge` MUST accept homogeneous roots. All roots MUST be mappings or all
MUST be sequences. `splice` MUST appear as a sequence item. It MUST replace
that item with the items of matched sequence roots. Mapping merge-key inclusion
MUST preserve YAML precedence. Explicit containing entries win.

r[resolution.include.text]
Text format MUST decode valid UTF-8 without YAML parsing and MUST preserve the
exact text as a string. Text format supports only `node`, `list`, and `named`.

r[resolution.include.order]
Glob and directory matches MUST use deterministic normalized root-relative
ordering. Empty matches MUST produce an error. Duplicate names MUST produce an
error. Mixed merge root kinds MUST produce an error. Invalid document counts
and incompatible insertion contexts MUST produce errors.

r[resolution.include.sandboxing]
Included paths MUST remain within the canonical filesystem root. Path
traversal MUST fail. Denied targets and non-regular files MUST fail. Symbolic
links MUST fail unless policy permits them. A permitted symbolic link MUST
still resolve inside the root.

r[resolution.include.cycle]
Circular includes MUST produce an error. Include and ref traversal MUST share
one active cycle stack.

r[resolution.include.budget]
Every attempted include, including cache hits, MUST count toward the resolution
edge budget. Every materialization MUST count toward the materialized-byte
budget. Exceeding the depth limit MUST produce a structured error. The same is
true for edge and byte limits.

### 7.2 `$ref` Resolution

r[resolution.ref]
`ref` resolves the current mapping under explicit ref policy. A ref directive
MUST be a mapping. Its only entry MUST be a string-valued `$ref`. Sibling
entries MUST produce an error. Non-string values and duplicate `$ref` keys
MUST also produce an error.

r[resolution.ref.fragment]
`$ref` MUST use a URI reference with an optional RFC 6901 JSON Pointer
fragment. An absent fragment selects the document root. An empty fragment does
the same. A pointer MAY select string mapping keys. It MAY also select sequence
indices. Non-string YAML mapping keys are not addressable through RFC 6901.

r[resolution.ref.document]
The fragmentless target MUST decode as valid UTF-8 and parse as exactly one
YAML document. Selection MUST retain native YAML types. A missing target MUST
fail. A malformed pointer MUST fail. Invalid YAML, an empty stream, or multiple
documents MUST fail. A fragment that depends on anchors outside the selected
subtree MUST also fail.

r[resolution.ref.base]
Relative refs MUST resolve from the containing resource at every level. File
resources resolve from their containing directory; HTTP resources resolve
against their URL. A relative ref without a base MUST fail. A same-document ref
MUST NOT require an external transport scheme.

r[resolution.ref.schemes]
Every external ref scheme MUST appear in `RefPolicy.allow_schemes`. File refs
MUST obey the include filesystem boundary. An HTTP or HTTPS ref MUST match an
exact allowed host. It MUST reject user information before I/O.

r[resolution.ref.network]
HTTP retrieval MUST disable redirects, retries, cookies, authentication, and
persistent cache state. A non-success status MUST be a hard error. A timeout or
body-limit failure MUST be a hard error. Invalid UTF-8 and YAML failures MUST
also be hard errors. Yamalgam MUST parse response bytes with its own scanner
and parser.

### 7.3 Sessions, reports, and integrity

r[resolution.session]
One top-level YAML document MUST own one resolution session. Its canonical
resource cache MUST be shared across nested includes and refs. The same session
MUST share cycle state and depth. It MUST share the edge and materialized-byte
counts. It MUST also share one report. Separate top-level documents MUST
receive independent sessions.

r[resolution.report]
Every session MUST retain a deterministic `ResolutionReport` on success and
failure. The report MUST record canonical resources and parent-child edges. It
MUST record directive spans, requested references, and fragments. It MUST
record discovery matches, byte counts, and cache hits. It MAY record provenance
and integrity metadata. Each failed edge MUST retain its terminal outcome.

r[resolution.report.safe]
Public report and error rendering MUST redact URI credentials and query values
and MUST NOT contain response bodies. It MUST NOT contain cookies or
authorization data.

r[resolution.integrity]
Retrieved resources and reports MAY carry validators, provenance, and strong
content digests. M11 MUST NOT enforce pins or perform dependency-lockfile I/O.

---

## 8. CLI Interface

r[cli]

### 8.1 Core Usage

r[cli.usage]
`yg [flags] <filter> [files...]` — apply filter to YAML input. If no files given, read from stdin. If no filter given, default to `.` (identity/pretty-print).

r[cli.multi-file]
When multiple files are given, each file's documents are processed through the filter in sequence, as a unified stream. No special `eval-all` mode exists.

### 8.2 Input Flags

r[cli.null-input]
`-n` / `--null-input` — do not read input. Run the filter once with `null` as input. Identical to jq.

r[cli.slurp]
`-s` / `--slurp` — read all documents from all files into a single sequence and run the filter once. Identical to jq's `-s`. The meaning of this flag MUST NOT diverge from jq.

r[cli.raw-input]
`-R` / `--raw-input` — read each line as a string, not as YAML. Identical to jq.

r[cli.from-file]
`-f` / `--from-file` — read the filter from a file. Identical to jq.

### 8.3 Output Flags

r[cli.json-output]
`-j` / `--json-output` — output as JSON. Applies `explode` implicitly (dereferences aliases, drops metadata). This is the "pipe to jq" escape hatch.

r[cli.raw-output]
`-r` / `--raw-output` — output raw strings without quotes. Identical to jq.

r[cli.compact]
`-c` / `--compact-output` — compact output (single-line flow style for YAML, no whitespace for JSON).

r[cli.inplace]
`-i` / `--in-place` — modify the input file in place. Uses CST round-trip editing to preserve comments, style, and formatting for nodes not touched by the filter.

r[cli.sort-keys]
`-S` / `--sort-keys` — output mappings with sorted keys.

r[cli.color]
`--color=<when>` controls colorized output. `when` MUST be one of `auto`
(detect terminal capabilities, the default), `always`, or `never`. `-C` is
reserved for the global `--chdir` option and MUST NOT be interpreted as a color
flag.

### 8.4 Schema and Security Flags

r[cli.schema-flag]
`--schema=<name>` — set the schema for scalar resolution. Values: `core` (default), `failsafe`, `json`, `1.1`.

r[cli.trusted]
`--trusted` — start with `LoaderConfig::trusted()` and its generous resource
limits. Resolution remains disabled unless an `--allow-*` or `--resolve-*`
flag enables it.

r[cli.strict]
`--strict` — start with `LoaderConfig::strict()` and its tight resource limits.
Resolution remains disabled unless an `--allow-*` or `--resolve-*` flag
enables it. This preset is intended for CI pipelines that process untrusted
input.

r[cli.no-env]
`--no-env` — compile filters without the jq-compatible `env` capability. This
flag MUST select sandboxed filter compilation independently of `--strict`,
which controls resource limits for input processing.

r[cli.allow-includes]
`--allow-includes` — permit the explicit `include` filter for local files.

r[cli.resolve-includes]
`--resolve-includes` — resolve `!include` directives automatically. It MUST
conflict with `--allow-includes`.

r[cli.allow-refs]
`--allow-refs=<schemes>` — permit the explicit `ref` filter for the
comma-separated `file`, `http`, or `https` schemes.

r[cli.resolve-refs]
`--resolve-refs=<schemes>` — resolve `$ref` directives automatically through
the listed schemes. It MUST conflict with `--allow-refs`.

r[cli.allow-ref-host]
`--allow-ref-host=<host>` — permit one exact HTTP or HTTPS hostname. The flag
MAY repeat. Network schemes MUST require at least one host. A host entry without
a network scheme MUST be rejected.

r[cli.resolution-root]
`--resolution-root=<dir>` — set the canonical filesystem sandbox root. The
root MUST contain every initial file and file base.

r[cli.base-uri]
`--base-uri=<uri>` — set the fragmentless absolute base resource for standard
input and constructed values. Standard input MUST require a base for relative
refs. It MUST require a filesystem root for includes.

### 8.5 Exit Codes

r[cli.exit]
yg MUST use the following exit codes:

r[cli.exit.success]
`0` — success. Filter produced output (or `-e` flag not set).

r[cli.exit.falsy]
`1` — with `-e` flag: last output was `false` or `null`.

r[cli.exit.usage]
`2` — usage error (bad arguments, invalid filter syntax).

r[cli.exit.compile]
`3` — filter compilation error.

r[cli.exit.runtime]
`5` — runtime error (type error, missing key with no default, resource limit exceeded, resolution policy violation).

---

## 9. Output Serialization

r[output]

### 9.1 YAML Output (Default)

r[output.yaml]
Default output is YAML. yg MUST produce valid YAML 1.2 output.

r[output.yaml.style-preservation]
When using `yg -i` (in-place editing), nodes not modified by the filter MUST retain their original style, comments, and formatting. This requires CST round-trip editing and is gated on the CST milestone. Before CST, `yg -i` re-serializes with best-effort style matching.

r[output.yaml.style-default]
For newly constructed values (from filters, not from input), yg MUST choose presentation style intelligently: multi-line strings use literal block style (`|`), short strings use plain style, strings containing YAML-significant characters use quoted style.

r[output.yaml.quoting]
yg MUST quote scalars that would be misinterpreted under the YAML 1.2 core schema: values that look like `null`, `true`, `false`, numbers, or other reserved patterns MUST be quoted when they are strings. yg MUST NOT produce output that is ambiguous under the active schema.

### 9.2 JSON Output (`-j`)

r[output.json]
JSON output MUST be valid JSON. All YAML-specific metadata is discarded. Aliases are expanded. Non-string mapping keys are converted to strings (via their resolved string representation). Tags are discarded. Comments are discarded.

### 9.3 Multi-Document Output

r[output.multidoc]
When a filter produces multiple values, each value is emitted as a separate YAML document separated by `---`. This mirrors jq's behavior of emitting multiple JSON values separated by newlines.

r[output.multidoc.suppress]
`-N` / `--no-doc` — suppress document separators (`---`).

---

## 10. Schema Validation (Future)

r[validate]

r[validate.filter]
`validate(schema)` validates the current value against a JSON Schema or YAML Schema document. The argument is a path to the schema file. Returns the input value if valid; produces an error with diagnostic information if invalid.

r[validate.filter.inline]
`validate` (no argument) validates against the schema declared in the document's pragma (`# yg-schema-validate: path/to/schema.json`), if present.

r[validate.self-describing]
A YAML document MAY declare its own schema for validation via a comment pragma: `# yg-schema-validate: ./schema.json`. yg SHOULD recognize this pragma and provide a CLI mode (`yg lint` or `yg validate`) that validates all input documents against their declared schemas.

r[validate.output]
Validation errors are emitted as structured YAML diagnostics: path to the failing node, rule violated, expected vs actual value. The format MUST be machine-parseable for CI integration.

---

## 11. Implementation Phases

r[phases]

### Phase 1: Core Expression Engine

r[phases.core]
Integrate jaq-syn, jaq-core, jaq-std. Implement `YamlVal` with `ValT` trait. Implement all jq-compatible filters from Section 4. Implement divergent behaviors from Section 5. Implement basic YAML extensions: `tag`, `anchor`, `style`, `kind`, `raw`, `resolved_type`, `type`. Ship `yg` CLI with flags from Section 8 (except `--in-place`).

**Test strategy:** Every `program`/`input`/`output` triple from the jq manual (`manual.yml`) that operates on JSON-compatible values MUST pass when input/output are YAML-formatted equivalents. Document and track known divergences.

### Phase 2: Mutation and Round-Trip

r[phases.mutation]
Implement assignment operators (`=`, `|=`, `+=`, etc.) with YAML output. Implement `--in-place` with best-effort style preservation. Implement `set_tag`, `set_anchor`, `set_style`. Implement `merge` and `explode`.

### Phase 3: CST Round-Trip

r[phases.cst]
Integrate yamalgam CST layer. Implement comment filters (Section 6.5). Upgrade `--in-place` to full CST round-trip editing. Implement `to_entries_rich`.

### Phase 4: External Resolution

r[phases.resolution]
Milestone 11 implements `!include` and `$ref` filters, explicit and automatic
CLI policy, file and HTTP retrieval, cycle detection, sandboxing, resource
budgets, origin tracking, and in-memory reports. Dependency commands,
lockfiles, redirect-hop validation, authenticated retrieval, HTTP includes,
and parameterized includes remain future work.

### Phase 5: Schema Validation

r[phases.validation]
Implement `validate` filter. Implement `# yg-schema-validate:` pragma recognition. Implement `yg lint` / `yg validate` subcommands. Ship structured diagnostic output.

### Phase 6: jq Roundtrip Bridge

r[phases.roundtrip]
Implement `--roundtrip` / `--through` mode: serialize to JSON, pipe through external jq (or any command), diff result against original CST, apply delta. This is a research-grade feature and MAY be deferred indefinitely if the diff/patch semantics prove intractable.

---

## Appendix A: Comparison with mfyq

r[appendix.mfyq]

| Behavior | mfyq | yg |
|----------|------|-----|
| Expression language | jq-like but divergent | jq (via jaq) with documented YAML extensions |
| Multi-document handling | `eval` vs `eval-all` modes | Single mode, documents are stream values |
| Comment preservation | Best-effort, lossy in many operations | CST-based, lossless for unmodified nodes |
| Anchor handling | Auto-expands merge keys, lossy | Preserves anchors, explicit `merge`/`explode` |
| Key ordering | Depends on operation | Insertion order always (`keys_sorted` for sorted) |
| Style preservation | Delegates to go-yaml, lossy | CST-preserving for unmodified nodes |
| Boolean resolution | YAML 1.2 by default (configurable) | YAML 1.2 core by default (configurable) |
| Security model | None | `LoaderConfig` with resource limits and resolution policy |
| `!include` | Not supported | Supported, sandboxed, off by default |
| `$ref` | Not supported | Supported, policy-gated, off by default |
| Schema control | None | `--schema` flag, document pragmas |
| Strict comparison | Not available | `===` / `!==` operators |

## Appendix B: Filter Quick Reference

r[appendix.filters]

**YAML-native filters** (not in jq):
`tag`, `set_tag(t)`, `anchor`, `set_anchor(n)`, `style`, `set_style(s)`, `chomping`, `kind`, `raw`, `resolved_type`, `is_alias`, `schema`, `resolve`, `resolve_as(s)`, `document_index`, `file_index`, `filename`, `leading_comments`, `inline_comment`, `trailing_comments`, `comments`, `set_inline_comment(s)`, `set_leading_comment(s)`, `merge`, `explode`, `aliases`, `orphan_anchors`, `include`, `ref`, `validate(schema)`, `to_entries_rich`, `keys_sorted`.

**jq filters with changed semantics:**
`type` (returns YAML type names), `keys` (insertion order).

**All other jq/jaq-std filters:** Identical behavior. See the jq manual for documentation.
