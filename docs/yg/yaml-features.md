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
```
</script>-->
