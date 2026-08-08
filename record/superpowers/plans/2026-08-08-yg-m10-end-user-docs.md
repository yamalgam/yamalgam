# `yg` M10 End-User Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish tested end-user guides for the working M10 `yg` command.

**Architecture:** A small Markdown suite under `docs/yg/` separates onboarding,
querying, YAML-native behavior, editing, reference, and troubleshooting. Every
guide embeds its own `.ygq` programs and YAML fixtures; `mdcode` extracts them
and runs a hidden shell check against the current `yg` binary. One repository
script drives those examples locally and in CI.

**Tech Stack:** Markdown, `yg`, jq-compatible filter syntax, `.ygq` filter
files, `mdcode` v0.2.0, Bash, Just, Rumdl, Lychee, Bito 2.0.0, GitHub Actions.

## Global Constraints

- Document M10 behavior only. Mention M11 or M13 only to name a current limit.
- Use `.ygq` as the canonical extension for reusable `yg` filter programs.
- Keep `-f` extension-agnostic; `.ygq` is a convention, not validation.
- Use `jq` as the Markdown fence language and `.ygq` in `mdcode` `file=`
  metadata.
- Associate `*.ygq` with jq tooling. Do not create or vendor a Tree-sitter
  grammar.
- Run examples against the current workspace binary, never mocked output.
- Keep `docs/spec/` unchanged.
- Put end-user guides in `docs/`; keep plans and design history in `record/`.
- State prominently that M10 `-i` loses comments and exact formatting.
- Target Flesch-Kincaid grade 8 or lower for every end-user guide.
- Do not add Rust dependencies or change public APIs.
- Do not change any project version.
- Use `just` recipes for the contributor-facing entry points.
- Use Bash with `set -euo pipefail`; do not add Python.

## File Map

| Path | Responsibility |
|------|----------------|
| `docs/README.md` | Route readers to `yg` user guides or bundled YAML specs. |
| `docs/yg/README.md` | Install from the workspace and complete a first query. |
| `docs/yg/querying.md` | Explain jq-style filters, inputs, variables, schemas, and output. |
| `docs/yg/yaml-features.md` | Explain tags, anchors, aliases, styles, merge keys, and native filters. |
| `docs/yg/editing.md` | Explain atomic in-place edits and current fidelity limits. |
| `docs/yg/reference.md` | List CLI options, native filters, config, environment, and exit codes. |
| `docs/yg/troubleshooting.md` | Diagnose config, limits, logs, parse failures, and query failures. |
| `README.md` | Keep the short product pitch and link to the guide hub. |
| `.gitattributes` | Treat `.ygq` as LF text and jq for GitHub Linguist. |
| `scripts/check-yg-doc-examples` | Build `yg`, extract every guide's fixtures, and run named checks. |
| `.justfile` | Expose `docs-examples` and `docs-check`. |
| `.github/workflows/ci.yml` | Run executable guide examples on pull requests and `main`. |
| `record/superpowers/specs/2026-08-08-yg-m10-end-user-docs-design.md` | Mark the accepted design implemented after all checks pass. |

---

### Task 1: Documentation Hub and Five-Minute Start

**Files:**

- Create: `docs/README.md`
- Create: `docs/yg/README.md`
- Modify: `README.md`
- Modify: `.gitattributes`

**Interfaces:**

**Consumes:** The current `yg -f FILE INPUT` CLI and workspace install path.

**Produces:** The guide navigation, `.ygq` convention, and the first executable
`mdcode` example used by Task 4.

- [ ] **Step 1: Record `.ygq` as jq-compatible text**

Add this line beside the other text-file rules in `.gitattributes`:

```gitattributes
*.ygq text eol=lf linguist-language=jq
```

- [ ] **Step 2: Create the documentation index**

Write `docs/README.md` with these sections and links:

```markdown
# Documentation

## Use `yg`

- [Get started with `yg`](yg/README.md)
- [Query YAML](yg/querying.md)
- [Work with YAML features](yg/yaml-features.md)
- [Edit files in place](yg/editing.md)
- [`yg` reference](yg/reference.md)
- [Troubleshoot `yg`](yg/troubleshooting.md)

## YAML specifications

The [`spec/`](spec/) directory contains the bundled YAML language
specifications and the `yg` query-language drafts. Those files are reference
material, not the user guide.
```

- [ ] **Step 3: Write the getting-started guide**

Create `docs/yg/README.md` in this order:

1. `# Get started with yg`
2. Pre-release note: install from a source checkout with
   `cargo install --path crates/yamalgam`.
3. First query from a file.
4. First query from standard input.
5. Reusable `.ygq` filters with `-f`.
6. A short “Next steps” list linking every sibling guide.

Embed these visible fixtures so `mdcode` can materialize the example:

````markdown
```yaml file=examples/services.yaml
services:
  api:
    image: ghcr.io/acme/api:1.4.0
    replicas: 3
  worker:
    image: ghcr.io/acme/worker:2.0.0
    replicas: 2
```

```jq file=examples/api-image.ygq
.services.api.image
```
````

Show this runnable command and output before explaining the filter:

```console
$ yg -r -f examples/api-image.ygq examples/services.yaml
ghcr.io/acme/api:1.4.0
```

Add this hidden check at the end of the guide:

````markdown
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
````

- [ ] **Step 4: Link the root README to the guide**

Keep the existing `## yg` pitch and command examples. Replace the
implementation-notes-only ending with two links:

```markdown
Start with the [`yg` user guide](docs/yg/README.md). See the
[draft-01 implementation notes](docs/spec/yg-spec-draft-01-implementation-notes.md)
for compatibility counts and deliberate deviations.
```

- [ ] **Step 5: Run the first executable example**

Run:

```bash
cargo build -p yamalgam --locked
YG_BIN="$PWD/target/debug/yg" mdcode run --quiet --name test docs/yg/README.md
```

Expected: exit 0 with no output.

- [ ] **Step 6: Check prose, Markdown, and links**

Run:

```bash
bito readability docs/README.md --max-grade 8
bito readability docs/yg/README.md --max-grade 8
rumdl check README.md docs/README.md docs/yg/README.md
```

Expected: both readability grades are at most 8 and Rumdl reports no errors.
Link checking waits until Task 3 creates every page linked from the indexes.

- [ ] **Step 7: Commit the onboarding slice**

```bash
git add .gitattributes README.md docs/README.md docs/yg/README.md
git commit -m "docs(yg): add M10 getting started guide"
```

---

### Task 2: Querying and YAML-Native Guides

**Files:**

- Create: `docs/yg/querying.md`
- Create: `docs/yg/yaml-features.md`

**Interfaces:**

**Consumes:** The navigation from Task 1 and the native filter registry in
`crates/yamalgam-query/src/funs.rs`.

**Produces:** The task guides linked by the later reference and troubleshooting
pages.

- [ ] **Step 1: Write executable query examples before the prose**

Start `docs/yg/querying.md` with these extracted inputs and filter:

````markdown
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

```jq file=examples/enabled-images.ygq
.services[] | select(.enabled) | .image
```
````

Add a hidden `sh name=test` block that checks all of these commands:

```sh
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
```

- [ ] **Step 2: Complete the querying guide around those examples**

Use these headings and concrete commands:

```markdown
# Query YAML
## Select values
## Compose filters
## Read document streams
## Read raw text or start with null
## Bind values without shell interpolation
## Keep filters in `.ygq` files
## Choose a YAML schema
## Control YAML, JSON, raw, and compact output
## Use filters named `doctor` or `info`
## Use exit status in scripts
## Next steps
```

Cover `-s`, `-R`, `-n`, `-f`, `--arg`, `--argjson`, `-j`, `-r`, `-c`, `-S`,
`-N`, `--schema`, `-e`, and the `yg -- FILTER` terminator. State that normal
multi-file input is processed one file at a time, while `-s` materializes the
unified input sequence.

- [ ] **Step 3: Write executable YAML-feature examples**

Create `docs/yg/yaml-features.md` with this fixture:

````markdown
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
````

Add a hidden `sh name=test` block that checks:

```sh
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

- [ ] **Step 4: Complete the YAML-feature guide**

Use these headings:

```markdown
# Work with YAML features
## Inspect tags, anchors, aliases, and styles
## Keep mapping insertion order
## Address non-string mapping keys
## Resolve scalars under a schema
## Expand merge keys
## Audit and expand aliases
## Change tags, anchors, and styles
## Understand what JSON output discards
```

Document these M10 filters from the native registry:

- Inspection: `tag`, `anchor`, `style`, `kind`, `is_alias`, `raw`,
  `resolved_type`, `schema`.
- Mutation: `set_tag(value)`, `set_anchor(value)`, `set_style(value)`.
- Resolution: `resolve`, `resolve_as(schema)`.
- Input context: `filename`, `file_index`, `document_index`.
- YAML structure: `merge`, `aliases`, `orphan_anchors`, `explode_aliases`.

Explain that `keys` preserves insertion order, `keys_sorted` opts into sorting,
and `-j` expands aliases while discarding YAML presentation metadata.

- [ ] **Step 5: Execute and review both guides**

Run:

```bash
YG_BIN="$PWD/target/debug/yg" mdcode run --quiet --name test docs/yg/querying.md
YG_BIN="$PWD/target/debug/yg" mdcode run --quiet --name test docs/yg/yaml-features.md
bito readability docs/yg/querying.md --max-grade 8
bito readability docs/yg/yaml-features.md --max-grade 8
rumdl check docs/yg/querying.md docs/yg/yaml-features.md
```

Expected: both example suites exit 0, both grades are at most 8, and Rumdl
reports no errors. Link checking waits until Task 3 creates all guide targets.

- [ ] **Step 6: Commit the query guides**

```bash
git add docs/yg/querying.md docs/yg/yaml-features.md
git commit -m "docs(yg): explain queries and YAML features"
```

---

### Task 3: Editing, Reference, and Troubleshooting

**Files:**

- Create: `docs/yg/editing.md`
- Create: `docs/yg/reference.md`
- Create: `docs/yg/troubleshooting.md`

**Interfaces:**

**Consumes:** The task guides from Tasks 1 and 2, `yg --help`, `yg doctor`,
`yg info`, and the CLI integration tests.

**Produces:** The complete M10 operational and lookup surface.

- [ ] **Step 1: Write and test the in-place editing guide**

Create `docs/yg/editing.md` with these headings:

```markdown
# Edit YAML files in place
## Update one file atomically
## Use a `.ygq` update program
## Know what M10 preserves
## Know what M10 rewrites
## Avoid unsafe replacement targets
## Handle multiple files and failures
```

Put this warning before the first `-i` command:

> **M10 fidelity limit:** `-i` reserializes YAML. It preserves supported node
> styles, tags, anchors, permissions, and document structure, but loses
> comments and exact formatting. M13 adds CST-spliced editing.

Embed these two files:

````markdown
```yaml file=examples/deployment.yaml
# production deployment
image:
  repository: ghcr.io/acme/api
  tag: "old"
replicas: 3
```

```jq file=examples/update-tag.ygq
.image.tag = "2026.08.08"
```
````

The hidden test must copy the fixture, run `-i`, confirm the new tag, confirm
the comment is gone, and confirm the original fixture was not changed:

```sh
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

cp examples/deployment.yaml examples/edited.yaml
"$YG_BIN" -i -f examples/update-tag.ygq examples/edited.yaml
test "$("$YG_BIN" -r '.image.tag' examples/edited.yaml)" = '2026.08.08'
! grep -q '# production deployment' examples/edited.yaml
grep -q '# production deployment' examples/deployment.yaml
```

State all current conflicts: `-i` rejects stdin, `-n`, `-s`, `-j`, and
`--format json`; rejects symbolic links and empty filter output; stages all
files before replacing any; warns for hard links; and preserves permissions.

- [ ] **Step 2: Write the complete lookup reference**

Create `docs/yg/reference.md` with these sections:

```markdown
# `yg` reference
## Invocation
## Query input options
## Output options
## Schema and resource options
## Shared CLI options
## Subcommands
## Native YAML filters
## Configuration files
## Environment variables
## Exit codes
## M10 compatibility limits
```

Copy option names and defaults from fresh `yg --help`, `yg doctor --help`, and
`yg info --help` output. Include the two config fields only: `log_level` and
`log_dir`. List project discovery names `.yamalgam.{toml,yaml,yml,json}` and
`yamalgam.{toml,yaml,yml,json}`, user config, closest-file precedence, the Git
boundary, and `--config FILE` as the top layer.

List environment variables `RUST_LOG`, `YAMALGAM_LOG_PATH`,
`YAMALGAM_LOG_DIR`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `XDG_DATA_HOME`.

Use this exit-code table:

| Code | Meaning |
|------|---------|
| 0 | Success, or a truthy final value with `-e`. |
| 1 | False or null final value with `-e`. |
| 2 | CLI usage, filter syntax, missing input, or config error. |
| 3 | Filter compilation error, such as an unknown function. |
| 4 | Successful filter produced no value with `-e`. |
| 5 | YAML parse, query runtime, resource-limit, or output error. |

Add this hidden test to grep current help for documented options and check exit
codes 0 through 5:

```sh
set -eu
: "${YG_BIN:?set YG_BIN to the yg executable}"

help="$("$YG_BIN" --help)"
for flag in --config --null-input --slurp --raw-input --from-file --no-env \
  --arg --argjson --json-output --raw-output --compact-output --in-place \
  --sort-keys --no-doc --schema --trusted --strict --exit-status; do
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

- [ ] **Step 3: Write the troubleshooting guide**

Create `docs/yg/troubleshooting.md` with these sections:

```markdown
# Troubleshoot `yg`
## Start with `yg doctor`
## Find the active configuration
## Distinguish syntax, compile, parse, and runtime failures
## Choose strict, default, or trusted limits
## Disable environment access
## Turn on logs
## Explain YAML 1.1 warnings
## Report a useful failure
```

Show `yg doctor --format text`, `yg doctor --format json`, and
`yg info --format json`. Explain that `--strict` tightens resource limits,
default uses the moderate profile, `--trusted` raises limits for local trusted
input, and `--no-env` independently removes the `env` filter.

Embed this config:

````markdown
```toml file=.yamalgam.toml
log_level = "debug"
```
````

Add this hidden test to confirm discovery and distinguish syntax, compile, and
runtime failures:

```sh
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
```

- [ ] **Step 4: Execute and review all three guides**

Run:

```bash
for doc in docs/yg/editing.md docs/yg/reference.md docs/yg/troubleshooting.md; do
  YG_BIN="$PWD/target/debug/yg" mdcode run --quiet --name test "$doc"
  bito readability "$doc" --max-grade 8
done
rumdl check docs/yg/editing.md docs/yg/reference.md docs/yg/troubleshooting.md
lychee --offline docs/yg/editing.md docs/yg/reference.md docs/yg/troubleshooting.md
```

Expected: all example suites exit 0; every grade is at most 8; Rumdl and
Lychee report no errors.

- [ ] **Step 5: Commit the operational guides**

```bash
git add docs/yg/editing.md docs/yg/reference.md docs/yg/troubleshooting.md
git commit -m "docs(yg): add editing and operations reference"
```

---

### Task 4: Executable Documentation Automation

**Files:**

- Create: `scripts/check-yg-doc-examples`
- Modify: `.justfile`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**

**Consumes:** Every `docs/yg/*.md` file and its required hidden
`sh name=test` block.

**Produces:** `scripts/check-yg-doc-examples`, `just docs-examples`,
`just docs-check`, and the CI `docs` job.

- [ ] **Step 1: Verify the automation entry point does not exist**

Run:

```bash
just docs-examples
```

Expected: failure because `.justfile` has no `docs-examples` recipe.

- [ ] **Step 2: Create the single-source example runner**

Create executable `scripts/check-yg-doc-examples` with this content:

```bash
#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
cd "$repo_root"

if ! command -v mdcode >/dev/null 2>&1; then
  echo "mdcode is required: go install github.com/szkiba/mdcode@v0.2.0" >&2
  exit 127
fi

cargo build -p yamalgam --locked
yg_bin="$repo_root/target/debug/yg"

docs=(docs/yg/*.md)
if [[ ! -e "${docs[0]}" ]]; then
  echo "no docs/yg Markdown files found" >&2
  exit 1
fi

for doc in "${docs[@]}"; do
  if ! grep -q '^```sh name=test$' "$doc"; then
    echo "$doc: missing hidden sh name=test block" >&2
    exit 1
  fi
  echo "checking $doc"
  YG_BIN="$yg_bin" mdcode run --quiet --name test "$doc"
done
```

The shell glob is portable to macOS Bash 3.2 and keeps the guide list dynamic.
Mark the file executable.

- [ ] **Step 3: Add Just entry points**

Add these recipes after `doc-test`:

```just
# Run executable examples embedded in the yg user guides.
docs-examples:
  scripts/check-yg-doc-examples

# Run executable, prose, Markdown, and offline-link checks for yg docs.
docs-check: docs-examples
  #!/usr/bin/env bash
  set -euo pipefail
  docs=(README.md docs/README.md docs/yg/*.md)
  rumdl check "${docs[@]}"
  lychee --offline "${docs[@]}"
  for doc in docs/README.md docs/yg/*.md; do
    bito readability "$doc" --max-grade 8
  done
```

Do not add `docs-check` to `just check`; the dedicated CI job installs the
non-Rust toolchain without making every Rust-only local check install Go.

- [ ] **Step 4: Add the CI docs job**

Add a `docs` job to `.github/workflows/ci.yml`:

```yaml
  docs:
    name: docs
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - name: Checkout
        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2

      - name: Rust toolchain
        uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: stable

      - name: Rust cache
        uses: ./.github/actions/setup-rust-cache
        with:
          cache-key-suffix: docs
          toolchain: stable

      - name: Go toolchain
        uses: actions/setup-go@924ae3a1cded613372ab5595356fb5720e22ba16 # v6
        with:
          go-version: "1.21"
          cache: false

      - name: Install mdcode
        run: go install github.com/szkiba/mdcode@v0.2.0

      - name: Check executable documentation
        run: scripts/check-yg-doc-examples
```

- [ ] **Step 5: Run the new automation**

Run:

```bash
just docs-examples
just docs-check
```

Expected: all six guide checks pass; Rumdl and Lychee report no errors; every
end-user document scores grade 8 or lower.

- [ ] **Step 6: Run focused CLI regression tests**

Run:

```bash
cargo nextest run -p yamalgam
```

Expected: every `yamalgam` crate test passes.

- [ ] **Step 7: Commit the automation**

```bash
git add scripts/check-yg-doc-examples .justfile .github/workflows/ci.yml
git commit -m "ci(docs): test yg guide examples"
```

---

### Task 5: Editorial and Repository Verification

**Files:**

- Modify: `docs/README.md`
- Modify: `docs/yg/README.md`
- Modify: `docs/yg/querying.md`
- Modify: `docs/yg/yaml-features.md`
- Modify: `docs/yg/editing.md`
- Modify: `docs/yg/reference.md`
- Modify: `docs/yg/troubleshooting.md`
- Modify: `record/superpowers/specs/2026-08-08-yg-m10-end-user-docs-design.md`

**Interfaces:**

**Consumes:** All completed guides and automation.

**Produces:** A reviewed user-doc set and an `Implemented` design record.

- [ ] **Step 1: Audit every claim against M10 evidence**

For each guide, compare commands and claims with:

```text
target/debug/yg --help
target/debug/yg doctor --help
target/debug/yg info --help
crates/yamalgam/src/lib.rs
crates/yamalgam/src/commands/query.rs
crates/yamalgam-query/src/funs.rs
crates/yamalgam/tests/yg.rs
crates/yamalgam/tests/config_integration.rs
docs/spec/yg-spec-draft-01-implementation-notes.md
```

Remove any claim supported only by the future-facing draft specification.

- [ ] **Step 2: Run the end-user editorial firewall**

Review each file for example-first structure, paragraphs no longer than four
sentences, defined jargon, and no use of “simply,” “just,” “obviously,” or
“easy.” Run:

```bash
for doc in docs/README.md docs/yg/*.md; do
  bito readability "$doc" --max-grade 8
done
```

Expected: every file scores grade 8 or lower. Run the independent editorial
review against the Doc Writer persona and resolve every blocking issue.

- [ ] **Step 3: Run documentation verification**

Run:

```bash
just docs-check
git diff --check
```

Expected: executable examples, Rumdl, Lychee, and Bito pass; Git reports no
whitespace errors.

- [ ] **Step 4: Run the repository gate**

Run:

```bash
just check
```

Expected: formatting, Clippy, dependency policy, nextest, and doc tests pass.

- [ ] **Step 5: Mark the design implemented**

In `record/superpowers/specs/2026-08-08-yg-m10-end-user-docs-design.md`, change:

```markdown
**Status:** Accepted
```

to:

```markdown
**Status:** Implemented
```

- [ ] **Step 6: Commit final editorial changes and design status**

```bash
git add docs README.md record/superpowers/specs/2026-08-08-yg-m10-end-user-docs-design.md
git commit -m "docs(yg): finalize M10 user documentation"
```

- [ ] **Step 7: Verify the final commit and clean tree**

Run:

```bash
git status --short
git log -5 --oneline
```

Expected: an empty status and focused documentation commits for Tasks 1
through 5.
