# `yg` reference

## Invocation

```text
yg [OPTIONS] [FILTER] [FILE]...
yg doctor [OPTIONS]
yg info [OPTIONS]
```

The default filter is `.`. With no file, `yg` reads standard input. Pass `-`
as a file name to read standard input at that point in the file list.

## Query input options

| Option | Effect |
|--------|--------|
| `-n`, `--null-input` | Run once with null input. |
| `-s`, `--slurp` | Read all input documents into one sequence. |
| `-R`, `--raw-input` | Read each input line as a string. |
| `-f`, `--from-file FILE` | Read the filter from a file. |
| `--no-env` | Remove the `env` filter. |
| `--arg NAME VALUE` | Bind a string to `$NAME`. |
| `--argjson NAME JSON` | Parse JSON and bind it to `$NAME`. |

## Output options

| Option | Effect |
|--------|--------|
| `-j`, `--json-output` | Emit JSON and expand YAML aliases. |
| `-r`, `--raw-output` | Print string results without YAML quotes. |
| `-c`, `--compact-output` | Put each result on one line. |
| `-i`, `--in-place` | Replace input files with query output. |
| `-S`, `--sort-keys` | Sort mapping keys in output. |
| `-N`, `--no-doc` | Omit `---` between YAML results. |
| `-e`, `--exit-status` | Set status from the last result. |

In-place mode cannot be used with `-n`, `-s`, `-j`, or `--format json`.

## Schema and resource options

```sh
yg --schema core --strict \
  --resolve-refs=file,https \
  --allow-ref-host=config.example.test \
  --resolution-root=. \
  '.' input.yaml
```

| Option | Effect |
|--------|--------|
| `--schema core` | Use YAML 1.2 core rules. This is the default. |
| `--schema failsafe` | Resolve plain scalars as strings. |
| `--schema json` | Use JSON scalar rules. |
| `--schema 1.1` | Use legacy YAML 1.1 rules. |
| `--strict` | Use tight limits for input you do not trust. |
| `--trusted` | Use higher limits for trusted local input. |
| `--allow-includes` | Permit the explicit `include` filter for local files. |
| `--resolve-includes` | Resolve local `!include` directives while loading. |
| `--allow-refs=SCHEMES` | Permit the explicit `ref` filter for comma-separated schemes. |
| `--resolve-refs=SCHEMES` | Resolve `$ref` directives while loading through the listed schemes. |
| `--allow-ref-host=HOST` | Permit one exact HTTP or HTTPS host. Repeat for more hosts. |
| `--resolution-root=DIR` | Set the filesystem sandbox root. |
| `--base-uri=URI` | Set the base resource for standard input and constructed values. |

`--strict` and `--trusted` conflict. The default uses the moderate limit
profile.

`--allow-*` selects explicit query filters. `--resolve-*` selects automatic
loading, and the two modes conflict for the same capability. Valid ref schemes
are `file`, `http`, and `https`. Network schemes require at least one
`--allow-ref-host`; host entries without a network scheme are errors.

An input file's directory is its default resolution root. An explicit root
must contain every initial file and file base. Standard input needs
`--base-uri` for relative refs and `--resolution-root` for includes. A
top-level URL is still treated as a file path.

## Shared CLI options

| Option | Effect |
|--------|--------|
| `-C`, `--chdir DIR` | Run as if started in `DIR`. |
| `-q`, `--quiet` | Print errors, but hide warnings and info. |
| `-v`, `--verbose` | Add log detail. Repeat for more detail. |
| `--color auto\|always\|never` | Choose color output. Default: `auto`. |
| `--format auto\|text\|json` | Set shared output form. Default: `auto`. |
| `--config FILE` | Load this config above all discovered config. |
| `--version-only` | Print only the version number. |
| `-V`, `--version` | Print the version. |
| `-h`, `--help` | Print help. |

`--format` controls shared command output and also selects JSON query output.
Use `-j` as the short query form. `-C` changes the working directory; use
`--color` to control color.

## Subcommands

```sh
yg doctor --format text
yg info --format json
```

`doctor` checks config and log paths, then reports the active environment.
`info` reports package and config details. Both accept the shared CLI options.

A first argument of `doctor` or `info` selects a subcommand. Use `yg -- FILTER`
to force query mode, or start mapping keys with `.doctor` or `.info`.

## Native YAML filters

```console
$ yg -r '.service | kind' config.yaml
mapping
```

| Group | Filters |
|-------|---------|
| Node data | `tag`, `anchor`, `style`, `kind`, `is_alias`, `raw` |
| Node updates | `set_tag(value)`, `set_anchor(value)`, `set_style(value)` |
| Schema | `resolved_type`, `schema`, `resolve`, `resolve_as(schema)` |
| Structure | `merge`, `explode_aliases`, `aliases`, `orphan_anchors` |
| Input source | `filename`, `file_index`, `document_index` |
| External YAML | `include`, `ref` |

These filters extend the jq-compatible filters supplied by jaq. `keys` keeps
YAML mapping insertion order. Use `keys_sorted` when you want sorted keys.

## Configuration files

```toml
log_level = "debug"
log_dir = "/path/to/logs"
```

Those are the only current config-file fields. Resolution policy is selected
with CLI flags in M11. Unknown fields are errors. Config files
may use TOML, YAML, or JSON.

Project discovery checks these names:

```text
.yamalgam.toml    yamalgam.toml
.yamalgam.yaml    yamalgam.yaml
.yamalgam.yml     yamalgam.yml
.yamalgam.json    yamalgam.json
```

`yg` walks up from the working directory. The closest file wins, and a Git
root stops the search. The user config sits below project config. An explicit
`--config FILE` is the top layer.

Run `yg doctor` to see the platform user config directory. Run
`yg info --format json` to see the file and values that won.

## Environment variables

| Name | Effect |
|------|--------|
| `RUST_LOG` | Set a tracing filter, such as `yamalgam=trace`. |
| `YAMALGAM_LOG_PATH` | Set the log file path. |
| `YAMALGAM_LOG_DIR` | Set the log directory. |
| `XDG_CONFIG_HOME` | Set the user config base directory. |
| `XDG_CACHE_HOME` | Set the user cache base directory. |
| `XDG_DATA_HOME` | Set the user data base directory. |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success, or a truthy final value with `-e`. |
| 1 | False or null final value with `-e`. |
| 2 | CLI usage, filter syntax, missing input, or config error. |
| 3 | Filter compilation error, such as an unknown function. |
| 4 | Successful filter produced no value with `-e`. |
| 5 | YAML parse, query runtime, resource-limit, or output error. |

The jq `halt` filter may return other POSIX exit codes.

## Current compatibility limits

- `-i` loses comments and exact formatting until M13.
- `===`, `!==`, `?//`, jq streaming filters, and `-L` do not currently ship.
- Integers use `i64`; floats use IEEE 754 binary64.
- Regex uses the `regex-bites` subset. It has no lookaround or backreferences.
- A schema pragma in a YAML comment is not read. Use `--schema`.
- External resolution reports are library data; M11 has no dependency-report
  command or lockfile.
- HTTP redirects, cookies, authentication, persistent caching, and HTTP
  includes are not available.

For the full test count and each known difference, see the
[draft-01 implementation notes](../spec/yg-spec-draft-01-implementation-notes.md).

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

help="$("$YG_BIN" --help)"
for flag in --config --null-input --slurp --raw-input --from-file --no-env \
  --arg --argjson --json-output --raw-output --compact-output --in-place \
  --sort-keys --no-doc --schema --trusted --strict --exit-status \
  --allow-includes --resolve-includes --allow-refs --resolve-refs \
  --allow-ref-host --resolution-root --base-uri; do
  printf '%s\n' "$help" | grep -F -- "$flag" >/dev/null
done

check_status() {
  expected="$1"
  shift
  set +e
  "$@" >/dev/null 2>&1
  actual="$?"
  set -e
  test "$actual" -eq "$expected"
}

check_status 0 "$YG_BIN" -n null
check_status 1 "$YG_BIN" -n -e false
check_status 2 "$YG_BIN" -n '.a |'
check_status 3 "$YG_BIN" -n nosuchfilter
check_status 4 "$YG_BIN" -n -e empty
check_status 5 "$YG_BIN" -n '1 + true'
```
</script>-->
