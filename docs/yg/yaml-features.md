# Work with YAML features

Use this file for the examples below:

```yaml file=examples/native.yaml
base: &base
  image: ghcr.io/acme/api:1.4.0
  replicas: 2
service:
  <<: *base
  replicas: 4
message: >-
  ready to serve
duration: !duration 30s
copy: *base
unused: &unused value
42: integer key
```

## Inspect tags, anchors, aliases, and styles

```console
$ yg -r '.duration | tag' examples/native.yaml
!duration
$ yg -r '.base | anchor' examples/native.yaml
base
$ yg -r '.copy | kind' examples/native.yaml
alias
$ yg -r '.message | style' examples/native.yaml
folded
```

YAML nodes carry data and presentation details. Use `tag`, `anchor`, `style`,
and `kind` to read them. `is_alias` tests whether a value is an alias. `raw`
returns the source spelling of a scalar.

`resolved_type` reports the type chosen by the active schema. `schema` reports
that schema's name.

## Keep mapping insertion order

```console
$ yg -j -c 'keys' examples/native.yaml
["base","service","message","duration","copy","unused",42]
```

`keys` keeps a mapping's insertion order. Use `keys_sorted` when you want a
sorted result. The `-S` output flag also sorts mapping keys when they are
emitted.

## Address non-string mapping keys

```console
$ yg -r '.[42]' examples/native.yaml
integer key
```

YAML mapping keys are not limited to strings. A number in brackets selects the
integer key. The string expression `."42"` would select a different key.

## Resolve scalars under a schema

```console
$ printf 'enabled: yes\n' | yg -q --schema 1.1 -r '.enabled | resolved_type'
boolean
```

`resolve` applies the active schema to a scalar. `resolve_as(schema)` uses a
named schema for that value. The command's `--schema` flag sets the input and
output schema.

## Expand merge keys

```console
$ yg -r '.service | merge | .image' examples/native.yaml
ghcr.io/acme/api:1.4.0
```

`merge` applies YAML merge keys and returns a mapping. Local entries win over
entries from the merged mapping.

## Audit and expand aliases

```console
$ yg -j -c 'orphan_anchors' examples/native.yaml
["unused"]
$ yg -r 'explode_aliases | .copy | is_alias' examples/native.yaml
false
```

`aliases` lists alias uses. `orphan_anchors` lists anchors that no alias uses.
`explode_aliases` replaces aliases with their target values. Resource limits
still apply while aliases expand.

## Change tags, anchors, and styles

```console
$ yg '.duration | set_tag("!timeout")' examples/native.yaml
!timeout 30s
```

Use `set_tag(value)`, `set_anchor(value)`, and `set_style(value)` to change
YAML metadata. The requested style must fit the value. An invalid style change
returns an error instead of changing the data.

The `filename`, `file_index`, and `document_index` filters report where the
current input came from.

## Load external YAML

External loading is off until you choose a capability. This command resolves
local `!include` tags while it loads the input:

```yaml file=examples/includes/defaults.yaml
image: ghcr.io/acme/api:1.4.0
replicas: 2
```

```yaml file=examples/includes/routes-a.yaml
- /health
- /ready
```

```yaml file=examples/includes/routes-b.yaml
- /metrics
```

```yaml file=examples/includes/profile-dev.yaml
name: dev
```

```yaml file=examples/includes/profile-prod.yaml
name: prod
```

```yaml file=examples/includes/settings/timeouts.yaml
timeout: 30
```

```yaml file=examples/includes/settings/retries.yaml
retries: 2
```

```text file=examples/includes/notice.txt
deploy after approval
keep this line too
```

```yaml file=examples/includes/app.yaml
service:
  <<: !include defaults.yaml
  replicas: 4
routes:
  - !include
      glob: routes-*.yaml
      mode: splice
profiles: !include
  glob: profile-*.yaml
  mode: list
named_profiles: !include
  glob: profile-*.yaml
  mode: named
  key: stem
settings: !include
  dir: settings
  mode: merge
notice: !include
  path: notice.txt
  format: text
```

```console
$ yg --resolve-includes -j -c '.service' examples/includes/app.yaml
{"image":"ghcr.io/acme/api:1.4.0","replicas":4}
$ yg --resolve-includes -j -c '.routes' examples/includes/app.yaml
["/health","/ready","/metrics"]
```

The scalar form substitutes one YAML document. Under `<<`, it merges a
mapping and keeps explicit entries from the containing mapping. The structured
form requires exactly one of `path`, `glob`, or `dir`. Every selector must be
relative to the document that contains it.

Structured includes support these modes:

| Mode | Result |
|------|--------|
| `node` | One matched YAML root. This is the default. |
| `list` | One sequence item per matched resource. |
| `named` | A mapping keyed by filename stem or extensionless relative path. |
| `merge` | Merged mappings or concatenated sequences. Input roots must agree. |
| `splice` | Sequence items inserted at the directive's position. |

`dir` matches `*.yaml` and `*.yml` filenames by default. It scans only that
directory unless `recursive: true` enables descendants. Use `match` to replace
the defaults with one filename glob or a sequence of filename globs. `named`
accepts `key: stem` or `key: relative`. Mapping `merge` accepts
`on_conflict: error`, `first`, or `last`. Matches always use deterministic
root-relative ordering.

Set `format: text` to load exact UTF-8 text as a YAML string. Text works with
`node`, `list`, and `named`; it does not parse the file as YAML.

## Resolve `$ref` mappings

```yaml file=examples/refs/catalog.yaml
defaults:
  color: blue
  enabled: true
```

```yaml file=examples/refs/app.yaml
whole:
  $ref: ./catalog.yaml
selected:
  $ref: ./catalog.yaml#/defaults/color
```

```console
$ yg --resolve-refs=file -r '.selected' examples/refs/app.yaml
blue
$ yg --resolve-refs=file -j -c '.whole' examples/refs/app.yaml
{"defaults":{"color":"blue","enabled":true}}
```

`$ref` uses a URI reference plus an optional RFC 6901 JSON Pointer fragment.
No fragment—or an empty fragment—selects the complete document. The mapping
must contain only `$ref`; sibling keys are errors. The selected node keeps its
native YAML type.

Relative targets start from the document that contains the directive. A file
input uses its containing directory. Standard input needs `--base-uri` for
relative refs and `--resolution-root` for includes:

```sh
yg --resolve-refs=file \
  --base-uri="$PWD/config/app.yaml" \
  --resolution-root="$PWD/config" \
  '.' < config-from-stdin.yaml
```

The filesystem root is a hard boundary. Targets must be regular files;
traversal outside the root and symbolic links fail. Without an explicit root,
each input file's directory is its root.

HTTP refs require an allowed scheme and exact host:

```sh
yg --resolve-refs=https \
  --allow-ref-host=config.example.test \
  '.' app.yaml
```

M11 does not follow redirects or send cookies or authentication. It has no
persistent HTTP cache, dependency command, or lockfile. HTTP includes are not
supported. Retrieved YAML must be valid UTF-8 and contain exactly one
document.

## Understand what JSON output discards

```console
$ yg -j -c '.copy' examples/native.yaml
{"image":"ghcr.io/acme/api:1.4.0","replicas":2}
```

JSON has no tags, anchors, aliases, styles, or non-string mapping keys. The
`-j` flag expands aliases and drops YAML presentation data. Use YAML output
when those details matter.

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

test "$("$YG_BIN" -r '.duration | tag' examples/native.yaml)" = '!duration'
test "$("$YG_BIN" -r '.base | anchor' examples/native.yaml)" = 'base'
test "$("$YG_BIN" -r '.message | style' examples/native.yaml)" = 'folded'
test "$("$YG_BIN" -r '.copy | kind' examples/native.yaml)" = 'alias'
test "$("$YG_BIN" -r '.service | merge | .image' examples/native.yaml)" = 'ghcr.io/acme/api:1.4.0'
test "$("$YG_BIN" -j -c 'orphan_anchors' examples/native.yaml)" = '["unused"]'
test "$("$YG_BIN" -r '.[42]' examples/native.yaml)" = 'integer key'
test "$("$YG_BIN" -r 'explode_aliases | .copy | is_alias' examples/native.yaml)" = 'false'

test "$("$YG_BIN" --resolve-includes -j -c '.service' examples/includes/app.yaml)" = \
  '{"image":"ghcr.io/acme/api:1.4.0","replicas":4}'
test "$("$YG_BIN" --resolve-includes -j -c '.routes' examples/includes/app.yaml)" = \
  '["/health","/ready","/metrics"]'
test "$("$YG_BIN" --resolve-includes -j -c '.profiles | map(.name)' examples/includes/app.yaml)" = \
  '["dev","prod"]'
test "$("$YG_BIN" --resolve-includes -j -c '.named_profiles | keys' examples/includes/app.yaml)" = \
  '["profile-dev","profile-prod"]'
test "$("$YG_BIN" --resolve-includes -j -c '.settings' examples/includes/app.yaml)" = \
  '{"retries":2,"timeout":30}'
expected='deploy after approval
keep this line too'
test "$("$YG_BIN" --resolve-includes -r '.notice' examples/includes/app.yaml)" = "$expected"

test "$("$YG_BIN" --resolve-refs=file -r '.selected' examples/refs/app.yaml)" = 'blue'
test "$("$YG_BIN" --resolve-refs=file -j -c '.whole' examples/refs/app.yaml)" = \
  '{"defaults":{"color":"blue","enabled":true}}'
```
</script>-->
