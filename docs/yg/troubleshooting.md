# Troubleshoot `yg`

## Start with `yg doctor`

```sh
yg doctor --format text
```

This checks config discovery and the log directory. It also lists key paths
and any XDG or logging variables that are set.

Use JSON when a script needs the report:

```sh
yg doctor --format json
```

## Find the active configuration

```sh
yg info --format json
```

The `config` object shows the winning file and values. To test another working
directory, use `-C`:

```sh
yg -C path/to/project info --format json
```

This project config sets the base log level:

```toml file=.yamalgam.toml
log_level = "debug"
```

`yg` searches up from its working directory and stops at a Git root. The
closest project file wins. Pass `--config FILE` to put one file above all
discovered config.

## Distinguish syntax, compile, parse, and runtime failures

```console
$ yg -n '.a |'
yg: filter syntax error: ...
```

The exit code tells you which stage failed:

- Code 2: CLI use, filter syntax, config, or input path.
- Code 3: filter compile, such as an unknown function.
- Code 5: YAML parse, query runtime, limit, or output.

The message adds a file and document number when an input value caused the
error. See the full [exit-code table](reference.md#exit-codes).

## Choose strict, default, or trusted limits

```sh
yg --strict '.items[]' untrusted.yaml
yg --trusted '.items[]' large-local.yaml
```

`--strict` uses tight limits for input you do not trust. The default uses the
moderate profile. `--trusted` raises limits for large, trusted local data.

Limits apply to parsing, query work, generated values, output, resolution
depth and edges, materialized resource bytes, and HTTP response bodies. Do not
use `--trusted` as a way to silence a limit on unknown input.

## Diagnose include and ref failures

```yaml file=examples/troubleshooting/missing.yaml
$ref: ./not-there.yaml
```

```console
$ yg --resolve-refs=file '.' examples/troubleshooting/missing.yaml
yg: examples/troubleshooting/missing.yaml document 1: ... resource not found ...
$ echo $?
5
```

Resolution errors retain the input file and document number. Start with the
policy flags, then check the base and root:

- No retrieval happens unless `--allow-*` or `--resolve-*` enables it.
- `--allow-*` permits the explicit `include` or `ref` filter.
- `--resolve-*` resolves matching directives while loading.
- File targets must remain below `--resolution-root`. Without that flag, an
  input file's containing directory is the root.
- Standard input needs `--base-uri` for relative refs and
  `--resolution-root` for includes.
- HTTP and HTTPS require both the scheme and an exact `--allow-ref-host`.

HTTP status errors and redirects are hard failures. Redirects are disabled,
so add the final URL as the `$ref` target instead of allowlisting a redirect
destination. Includes cannot use HTTP.

If retrieval succeeds but parsing fails, check the resource itself. It must be
valid UTF-8 YAML with exactly one document. A `$ref` mapping may contain no
sibling keys, and its fragment must be an RFC 6901 JSON Pointer.

## Disable environment access

```console
$ yg --no-env -n env
yg: <filter>: undefined filter env
```

`--no-env` removes the `env` filter. This setting is separate from strict,
default, and trusted resource limits.

## Turn on logs

```sh
RUST_LOG=yamalgam=trace yg '.' input.yaml
```

Use `RUST_LOG` for a tracing filter. Set `YAMALGAM_LOG_PATH` for one log file,
or `YAMALGAM_LOG_DIR` for a directory. `-v` adds detail and may be repeated.
`-q` hides warnings and info, but not errors.

Run `yg doctor` if the log path cannot be found or written.

## Explain YAML 1.1 warnings

```console
$ yg --schema 1.1 '.enabled' legacy.yaml
warning: YAML 1.1 schema compatibility mode resolves yes/no/on/off as booleans
```

The warning calls out a data-model change. Pass `-q` after you confirm that
the file needs YAML 1.1 rules. Use the default `core` schema for YAML 1.2.

## Report a useful failure

```sh
yg --version
yg info --format json
yg doctor --format json
yg --color never --strict 'FILTER' input.yaml
```

Include those results, the exit code, and the smallest YAML and filter that
still fail. Remove secrets from config, paths, environment data, and YAML
before you share the report.

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

info="$("$YG_BIN" -C . info --format json)"
printf '%s\n' "$info" | grep -F '.yamalgam.toml' >/dev/null
printf '%s\n' "$info" | grep -F '"log_level": "debug"' >/dev/null

doctor="$("$YG_BIN" doctor --format json)"
printf '%s\n' "$doctor" | grep -F '"checks"' >/dev/null

check_status() {
  expected="$1"
  shift
  set +e
  "$@" >/dev/null 2>&1
  actual="$?"
  set -e
  test "$actual" -eq "$expected"
}

check_status 2 "$YG_BIN" -n '.a |'
check_status 3 "$YG_BIN" -n nosuchfilter
check_status 5 "$YG_BIN" -n '1 + true'

set +e
error="$("$YG_BIN" --resolve-refs=file '.' examples/troubleshooting/missing.yaml 2>&1 >/dev/null)"
status="$?"
set -e
test "$status" -eq 5
printf '%s\n' "$error" | grep -F 'examples/troubleshooting/missing.yaml document 1:' >/dev/null
printf '%s\n' "$error" | grep -F 'resource not found' >/dev/null
```
</script>-->
