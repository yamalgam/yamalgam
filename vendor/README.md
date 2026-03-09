# Vendor Dependencies

## yaml-test-suite

The [YAML Test Suite](https://github.com/yaml/yaml-test-suite) provides 351
standardized test cases for YAML parser compliance. Each `.yaml` file contains
one or more test cases with input YAML, expected event tree output, and metadata.

Used by `yamalgam-compare` compliance tests.

## libfyaml (archived)

libfyaml 0.9.5 was vendored here as a reference implementation during scanner
and parser development (milestones 1-7). A C harness (`tools/fyaml-tokenize`)
ran libfyaml's tokenizer and parser against the same YAML Test Suite inputs,
and compliance tests compared token/event streams side by side.

yamalgam reached 97.7% YAML Test Suite compliance through this process, then
began emitting tokens libfyaml doesn't (e.g., `Comment`). The C comparison
infrastructure was removed in favor of direct YAML Test Suite expected output
testing.

**Last commit with libfyaml:** [`6bdd931`](https://github.com/claylo/yamalgam/commit/6bdd931)
**libfyaml version:** 0.9.5
**Repository:** https://github.com/pantoniou/libfyaml
