# Test fixtures

Real-world YAML used by `tests/integration.rs`. Every file here must
deserialize through the streaming serde path; files are organized by source.

## Provenance and licenses

| Directory     | Source                                                  | License    |
|---------------|---------------------------------------------------------|------------|
| `yq/`         | <https://github.com/mikefarah/yq> `examples/`           | MIT        |
| `yamlfmt/`    | <https://github.com/google/yamlfmt> `integrationtest/command/testdata/*/before/` | Apache-2.0 |
| `prettier/`   | <https://github.com/prettier/prettier> `tests/format/yaml/` (selected dirs) | MIT        |
| `real-world/` | Written for this repository                             | repo license |

Fixtures are test data only — they are not compiled into any artifact and
are excluded from crates.io packaging along with the rest of `tests/`.

## Curation notes

- **yamllint** (from the original M9 plan's source list) was skipped: it is
  GPL-3.0 and vendoring its fixtures isn't worth the license question.
- Intentionally invalid files were excluded at the source (`yq/bad.yaml`,
  `yq/front-matter.yaml`, prettier's `_errors_/`, yamlfmt's
  `strip_directives` data — directives without a `---` marker are invalid
  YAML), as were prettier's tool-specific pragma dirs and its vendored
  copies of the YAML/JSON test suites (we vendor the YAML Test Suite
  ourselves under `vendor/yaml-test-suite/`).
- Valid-but-unsupported files are kept and tracked in `KNOWN_FAILING` in
  `tests/integration.rs` (single-pair flow entries `[a: b]`, tab
  indentation, `? key` without value) — they pin today's limitations and
  the staleness check flags them the moment the parser learns the
  construct.
- prettier's `ansible/` dir was skipped (GPL-ecosystem provenance);
  `real-world/ansible/` is an original playbook written for this repo.
- `real-world/` files deliberately exercise anchors, aliases, merge keys
  (`<<`), multi-document streams, block scalars, flow collections, and the
  stringly-typed quantities common in k8s/CI configs.

## Merge keys

The Composer applies YAML merge-key semantics (`<<`); the streaming serde
deserializer — like serde_yaml's — does not, and surfaces `<<` as a literal
key. `tests/integration.rs` therefore skips the Composer-agreement check for
files containing `<<:` (the success check still applies).
