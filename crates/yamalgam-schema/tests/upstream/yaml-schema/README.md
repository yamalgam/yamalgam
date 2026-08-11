# Adapted `yaml-schema` boundary cases

These tests adapt the scalar mapping-key boundary from `yaml-schema` 0.9.3.

- Repository: <https://github.com/yaml-schema/yaml-schema>
- Audited commit: `1c9403a7e4eaf2e5de2ea39594279ce73df258fc`
- Original paths: `features/objects.feature`, `src/utils.rs`, and
  `src/validation/objects.rs`
- License: MIT; preserved in `LICENSE`

Yamalgam keeps the upstream canonical scalar projections for null, Boolean,
integer, finite float, and string keys. It deliberately changes the boundary
in three ways: projected-name collisions are rejected, `propertyNames` always
uses standard string semantics, and source locations are retained through
Yamalgam's separate location tree. The upstream evaluator, Saphyr DOM,
Cucumber harness, typed-key extension, and retrieval code are not copied.
