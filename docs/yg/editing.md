# Edit YAML files in place

## Update one file atomically

> **M10 fidelity limit:** `-i` reserializes YAML. It preserves supported node
> styles, tags, anchors, permissions, and document structure, but loses
> comments and exact formatting. M13 adds CST-spliced editing.

Start with this file:

```yaml file=examples/deployment.yaml
# production deployment
image:
  repository: ghcr.io/acme/api
  tag: "old"
replicas: 3
```

Change its image tag:

```sh
yg -i '.image.tag = "2026.08.08"' examples/deployment.yaml
```

`-i` writes a temporary file beside the source. It flushes and syncs the new
data, then replaces the source in one atomic step.

## Use a `.ygq` update program

Save the update as `examples/update-tag.ygq`:

```jq file=examples/update-tag.ygq
.image.tag = "2026.08.08"
```

Run the program with `-f`:

```sh
yg -i -f examples/update-tag.ygq examples/deployment.yaml
```

This keeps longer edits out of shell history and makes them easy to review.

## Know what M10 preserves

```sh
yg -i '.replicas += 1' examples/deployment.yaml
```

M10 keeps supported scalar styles, tags, anchors, file permissions, and YAML
document boundaries. It also keeps mapping order unless the filter changes it.

## Know what M10 rewrites

```sh
yg -i '.' examples/deployment.yaml
```

Even an identity edit reserializes the file. Comments are lost. Whitespace,
quotes, and other layout may change when the value model cannot retain them.
Review the diff before you commit an M10 edit.

## Avoid unsafe replacement targets

```console
$ yg -i '.enabled = true' link-to-config.yaml
yg: refusing to replace symbolic link link-to-config.yaml; edit its target path directly
```

`-i` rejects standard input and symbolic links. Pass the link target path if
you mean to replace it. A file with more than one hard link is allowed, but
`yg` warns that only the named path will point to the new file.

The filter must emit at least one value for each file. Empty output is an error,
and the source stays unchanged.

## Handle multiple files and failures

```sh
yg -i '.image.tag = "2026.08.08"' deploy-a.yaml deploy-b.yaml
```

`yg` stages every new file before it replaces any source. A parse, query, or
output error during staging leaves all source paths unchanged.

In-place mode rejects `-n`, `-s`, `-j`, and `--format json`. Those modes do not
have safe per-file YAML replacement rules.

<!--<script type="text/markdown">
```sh name=test
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

cp examples/deployment.yaml examples/edited.yaml
"$YG_BIN" -i -f examples/update-tag.ygq examples/edited.yaml
test "$("$YG_BIN" -r '.image.tag' examples/edited.yaml)" = '2026.08.08'
! grep -q '# production deployment' examples/edited.yaml
grep -q '# production deployment' examples/deployment.yaml
```
</script>-->
