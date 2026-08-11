# Query YAML

Start with a YAML stream. This file has two documents:

```yaml file=examples/environments.yaml
---
name: dev
services:
  - name: api
    image: ghcr.io/acme/api:dev
    enabled: true
---
name: prod
services:
  - name: api
    image: ghcr.io/acme/api:1.4.0
    enabled: true
  - name: debug
    image: ghcr.io/acme/debug:latest
    enabled: false
```

## Select values

```console
$ yg -r '.name' examples/environments.yaml
dev
prod
```

A filter runs once for each YAML document. Field access starts with a dot.
The `-r` flag prints strings without YAML quotes.

## Compose filters

```console
$ yg -r '.services[] | select(.enabled) | .image' examples/environments.yaml
ghcr.io/acme/api:dev
ghcr.io/acme/api:1.4.0
```

The pipe sends each result to the next filter. `[]` visits each item, and
`select` keeps values that pass its test.

## Read document streams

```console
$ yg -j -c -s 'map(.name)' examples/environments.yaml
["dev","prod"]
```

Normal input runs one file and one document at a time. With `-s`, `yg` reads
all input into one sequence before it runs the filter.

When you pass more than one file, normal mode finishes one file before it
starts the next. The slurp flag joins their document streams.

## Read raw text or start with null

```console
$ printf 'dev\nprod\n' | yg -R -r ascii_upcase
DEV
PROD
```

`-R` reads each input line as a string. Use `-n` when a filter does not need
input:

```console
$ yg -n -c '{ready: true}'
{ready: true}
```

## Bind values without shell interpolation

```console
$ yg -r --arg env prod 'select(.name == $env) | .name' examples/environments.yaml
prod
```

`--arg NAME VALUE` binds a string. `--argjson NAME JSON` parses the value as
JSON first:

```console
$ yg -n --argjson count 3 '$count + 1'
4
```

These flags keep data out of the filter source. This avoids shell quoting bugs
and filter injection.

## Keep filters in `.ygq` files

Save this filter as `examples/enabled-images.ygq`:

```jq file=examples/enabled-images.ygq
.services[] | select(.enabled) | .image
```

Run it with `-f`:

```console
$ yg -r -f examples/enabled-images.ygq examples/environments.yaml
ghcr.io/acme/api:dev
ghcr.io/acme/api:1.4.0
```

The `.ygq` suffix tells people and editors that the file is a `yg` query. The
`-f` flag does not require that suffix.

## Resolve external YAML

Create a shared document:

```yaml file=examples/resolution/shared.yaml
defaults:
  replicas: 3
  image: ghcr.io/acme/api:1.4.0
```

Reference part of it from another document:

```yaml file=examples/resolution/app.yaml
service:
  $ref: ./shared.yaml#/defaults
```

Resolve file references while loading:

```console
$ yg --resolve-refs=file '.' examples/resolution/app.yaml
service:
  replicas: 3
  image: ghcr.io/acme/api:1.4.0
```

External access is off by default. `--resolve-refs` turns on automatic
resolution for the listed schemes. The fragment is an RFC 6901 JSON Pointer;
omit it to substitute the complete YAML document.

Use `--allow-refs` when the query should choose which value to resolve:

```console
$ yg --allow-refs=file '.service | ref | .replicas' examples/resolution/app.yaml
3
```

Includes use the same explicit-versus-automatic split:

```yaml file=examples/resolution/included.yaml
service: !include shared.yaml
```

```console
$ yg --allow-includes '.service | include | .defaults.image' examples/resolution/included.yaml
ghcr.io/acme/api:1.4.0
```

For HTTP references, allow both the scheme and each exact host:

```sh
yg --resolve-refs=file,https \
  --allow-ref-host=example.test \
  --resolution-root=. \
  '.' app.yaml
```

`yg` never treats a top-level URL as an input file. HTTP enters through
`$ref`, and redirects stay disabled in M11. See
[external YAML](yaml-features.md#load-external-yaml) for include aggregation,
text mode, filesystem roots, and the security boundary.

## Choose a YAML schema

```console
$ printf 'value: yes\n' | yg -q --schema 1.1 -r '.value | type'
boolean
```

The default `core` schema follows YAML 1.2 core rules. `failsafe` resolves all
plain scalars as strings. `json` uses JSON rules. `1.1` supports legacy YAML
1.1 values such as `yes` and `no`.

YAML 1.1 mode prints a warning unless you pass `-q`.

## Validate a query value

The `--schema` option above chooses how YAML scalars resolve. The
`validate(schema)` filter is different: it applies a JSON Schema Draft 2020-12
document to the current query value.

```yaml file=examples/query-schema.yaml
type: object
required: [name]
properties:
  name: {type: string}
```

```yaml file=examples/query-service.yaml
name: api
```

Local schema retrieval is explicit and capability-rooted:

```console
$ yg --allow-refs=file --resolution-root=examples \
    'validate("query-schema.yaml")' examples/query-service.yaml
name: api
```

The filter returns the same metadata-bearing value when it is valid and raises
a structured runtime validation error when it is not. Use `yg validate` for
multi-file reports and CI-friendly exit 1 on validation findings. See
[Validate YAML with JSON Schema](validation.md).

## Control YAML, JSON, raw, and compact output

```console
$ yg -j -c -s 'map(.name)' examples/environments.yaml
["dev","prod"]
```

- YAML output is the default.
- `-j` emits JSON and expands YAML aliases.
- `-r` prints string results without quotes.
- `-c` puts each result on one line.
- `-S` sorts mapping keys. The default keeps insertion order.
- `-N` omits `---` between more than one YAML result.

## Use filters named `doctor` or `info`

```console
$ printf 'doctor: ready\n' | yg -r '.doctor'
ready
```

A leading `doctor` or `info` argument selects a subcommand. Start mapping keys
with a dot, as shown above. Use `yg -- FILTER` when the first filter argument
itself would look like a subcommand; `--` ends option and subcommand parsing.

## Use exit status in scripts

```sh
if yg -e '.services.api.enabled' config.yaml >/dev/null; then
  echo "API enabled"
fi
```

With `-e`, a false or null last value exits 1. No output exits 4. Other error
classes use distinct codes in the [reference](reference.md#exit-codes).

## Next steps

- [Work with YAML features](yaml-features.md)
- [Edit files in place](editing.md)
- [Look up all options and filters](reference.md)

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

actual="$("$YG_BIN" -r -f examples/enabled-images.ygq examples/environments.yaml)"
expected='ghcr.io/acme/api:dev
ghcr.io/acme/api:1.4.0'
test "$actual" = "$expected"

actual="$("$YG_BIN" -j -c -s 'map(.name)' examples/environments.yaml)"
test "$actual" = '["dev","prod"]'

actual="$("$YG_BIN" -r --arg env prod 'select(.name == $env) | .name' examples/environments.yaml)"
test "$actual" = 'prod'

actual="$(printf 'dev\nprod\n' | "$YG_BIN" -R -r ascii_upcase)"
expected='DEV
PROD'
test "$actual" = "$expected"

actual="$(printf 'value: yes\n' | "$YG_BIN" -q --schema 1.1 -r '.value | type')"
test "$actual" = 'boolean'

actual="$("$YG_BIN" --allow-refs=file --resolution-root=examples 'validate("query-schema.yaml")' examples/query-service.yaml)"
printf '%s\n' "$actual" | grep -F 'name: api' >/dev/null

actual="$("$YG_BIN" --resolve-refs=file -j -c '.' examples/resolution/app.yaml)"
test "$actual" = '{"service":{"replicas":3,"image":"ghcr.io/acme/api:1.4.0"}}'

actual="$("$YG_BIN" --allow-refs=file -r '.service | ref | .replicas' examples/resolution/app.yaml)"
test "$actual" = '3'

actual="$("$YG_BIN" --allow-includes -r '.service | include | .defaults.image' examples/resolution/included.yaml)"
test "$actual" = 'ghcr.io/acme/api:1.4.0'
```
</script>-->
