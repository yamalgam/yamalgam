# Get started with `yg`

`yg` is a pre-release YAML query tool. Install it from a source checkout:

```sh
cargo install --path crates/yamalgam
```

## Run your first query

Save this YAML as `examples/services.yaml`:

```yaml file=examples/services.yaml
services:
  api:
    image: ghcr.io/acme/api:1.4.0
    replicas: 3
  worker:
    image: ghcr.io/acme/worker:2.0.0
    replicas: 2
```

Read the API image:

```console
$ yg -r '.services.api.image' examples/services.yaml
ghcr.io/acme/api:1.4.0
```

The filter starts at the root value. Each `.name` step selects a mapping
entry. The `-r` flag prints a string without YAML quotes.

## Read from standard input

If you omit the input path, `yg` reads standard input:

```console
$ printf 'name: yamalgam\n' | yg -r '.name'
yamalgam
```

## Keep a filter in a file

Save a filter as `examples/api-image.ygq`:

```jq file=examples/api-image.ygq
.services.api.image
```

Pass the file with `-f`:

```console
$ yg -r -f examples/api-image.ygq examples/services.yaml
ghcr.io/acme/api:1.4.0
```

The `.ygq` suffix marks a reusable `yg` query. It is a file-name convention;
`-f` also accepts other names. Editors can treat `.ygq` as jq syntax.

## Next steps

- [Query YAML](querying.md)
- [Validate YAML](validation.md)
- [Set up the language server](language-server.md)
- [Work with YAML features](yaml-features.md)
- [Edit files in place](editing.md)
- [Look up options and filters](reference.md)
- [Troubleshoot a command](troubleshooting.md)

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

actual="$("$YG_BIN" -r -f examples/api-image.ygq examples/services.yaml)"
test "$actual" = "ghcr.io/acme/api:1.4.0"

actual="$(printf 'name: yamalgam\n' | "$YG_BIN" -r '.name')"
test "$actual" = "yamalgam"
```
</script>-->
