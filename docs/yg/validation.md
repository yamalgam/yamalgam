# Validate YAML with JSON Schema

`yg validate` checks YAML with JSON Schema Draft 2020-12. It keeps YAML types
and source spans. A CI error can point back to the bad YAML node.

## Validate with an explicit schema

Save a schema:

```yaml file=examples/validation/service.schema.yaml
type: object
required: [name]
properties:
  name:
    type: string
  replicas:
    type: integer
    minimum: 1
additionalProperties: false
```

Then save a matching document:

```yaml file=examples/validation/service.yaml
name: api
replicas: 3
```

Run validation:

```console
$ yg --format text validate --schema service.schema.yaml examples/validation/service.yaml
- filename: examples/validation/service.yaml
  document: 0
  valid: true
  diagnostics: []
```

`lint` is an alias for the same command:

```sh
yg lint --schema service.schema.yaml examples/validation/service.yaml
```

## Select a schema from the document

A leading pragma can select a schema relative to the YAML file:

```yaml file=examples/validation/pragma-service.yaml
# yg-schema-validate: ./service.schema.yaml
name: worker
replicas: 2
```

No `--schema` flag is needed:

```sh
yg validate examples/validation/pragma-service.yaml
```

The first match wins. The order is:

1. `--schema`
2. `# yg-schema-validate:` or a compatible `yaml-language-server` modeline
3. A top-level string `$schema` property
4. The first matching `schemas` configuration entry
5. A matching SchemaStore entry, when SchemaStore is enabled

The value `none` stops the search. Each item in a `---` stream gets its own
schema and check.

## Validate inside a query

The `validate(schema)` filter returns the same input when it is valid. Local
schema access uses the same opt-in rule as `ref`:

```console
$ yg --allow-refs=file --resolution-root=examples/validation \
    'validate("service.schema.yaml")' examples/validation/service.yaml
name: api
replicas: 3
```

The no-argument `validate` filter can use a top-level `$schema` key. Use
`yg validate` for comment pragmas, config matches, SchemaStore, many files, or
full reports.

## Use JSON reports in CI

This document fails the `name` type rule:

```yaml file=examples/validation/invalid-service.yaml
name: 7
replicas: 3
```

Run with the shared JSON format:

```sh
yg --format json validate \
  --schema service.schema.yaml \
  examples/validation/invalid-service.yaml
```

The result is an array in input order. Stream order is kept too. Each item has
`filename`, zero-based `document`, `valid`, and `diagnostics`. Each error has
these stable fields:

| Field | Meaning |
|-------|---------|
| `code` | Machine-readable category such as `schema/type`. |
| `severity` | `error` or `warning`. |
| `message` | Human-readable summary. |
| `keyword` | JSON Schema keyword that failed. |
| `instance_path` | RFC 6901 path into the YAML instance. |
| `schema_path` | RFC 6901 path into the schema. |
| `expected`, `actual` | Optional rule-specific values. |
| `span`, `labels` | Source location and related locations when available. |

The command uses four exit codes:

| Code | Meaning |
|------|---------|
| 0 | Every selected document is valid. |
| 1 | At least one document has validation findings. |
| 2 | Flags or policy arguments are invalid. |
| 5 | YAML parsing, schema selection, retrieval, or compilation failed. |

## Resolve local schema references

Schema `$ref` uses the root schema's file rules and cache. This root schema
loads a local rule:

```yaml file=examples/validation/ref.schema.yaml
$defs:
  name:
    $ref: ./name.schema.yaml
type: object
required: [name]
properties:
  name:
    $ref: "#/$defs/name"
```

```yaml file=examples/validation/name.schema.yaml
type: string
minLength: 1
```

```sh
yg validate \
  --schema ref.schema.yaml \
  examples/validation/service.yaml
```

Local schemas stay in the current directory by default. Use
`--resolution-root DIR` to set another root. Files outside it are denied.
Symlink escapes are denied too.

## Configure associations and SchemaStore

Project config can map paths to schemas:

```yaml
schemas:
  - uri: ./schemas/service.yaml
    file_match:
      - services/*.yaml
      - service.yaml

schema_store:
  enabled: false
  url: https://www.schemastore.org/api/json/catalog.json
```

SchemaStore is off by default. Turn it on with `--schema-store` or
`schema_store.enabled: true`. The catalog may be a local file. For a web
catalog or schema, also allow its scheme and exact host:

```sh
yg validate --schema-store \
  --allow-refs=https \
  --allow-ref-host=www.schemastore.org \
  service.yaml
```

Network access is off by default. Yamalgam does not use proxy secrets or follow
redirects. A schema cannot grant access to a host.

## YAML values at the JSON Schema boundary

Schema files must fit the JSON data model. Their map keys are strings. Their
numbers are finite. Input stays as native YAML. Scalar map keys get stable text
names for object rules. Their YAML type and span are kept.

Complex keys and non-finite numbers are errors. So are two keys with the same
text name. `propertyNames` checks that text. It does not treat a key as a typed
YAML value.

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

valid="$($YG_BIN --format text validate --schema service.schema.yaml examples/validation/service.yaml)"
printf '%s\n' "$valid" | grep -F 'valid: true' >/dev/null

$YG_BIN lint --schema service.schema.yaml examples/validation/service.yaml >/dev/null
$YG_BIN validate examples/validation/pragma-service.yaml >/dev/null

query="$($YG_BIN --allow-refs=file --resolution-root=examples/validation 'validate("service.schema.yaml")' examples/validation/service.yaml)"
printf '%s\n' "$query" | grep -F 'name: api' >/dev/null

set +e
invalid="$($YG_BIN --format json validate --schema service.schema.yaml examples/validation/invalid-service.yaml)"
status="$?"
set -e
test "$status" -eq 1
printf '%s\n' "$invalid" | grep -F '"code": "schema/type"' >/dev/null

$YG_BIN validate --schema ref.schema.yaml examples/validation/service.yaml >/dev/null

set +e
$YG_BIN validate --schema https://example.test/schema.json examples/validation/service.yaml >/dev/null 2>&1
status="$?"
set -e
test "$status" -eq 5
```
</script>-->
