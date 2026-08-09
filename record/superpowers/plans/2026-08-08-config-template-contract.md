# Configuration Template Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make configuration typos fail loudly and ensure shipped templates
advertise only settings the CLI consumes.

**Architecture:** Treat the existing two-field CLI `Config` type as the
configuration schema. Enforce that schema through serde and reduce both format
templates to those exact fields; do not invent behavior for placeholder
settings.

**Tech Stack:** Rust 2024, serde, librebar configuration loading, TOML/YAML
templates, cargo-nextest, Just.

## Constraints

- Add no dependency and no configuration field.
- Preserve `log_level` and `log_dir` behavior and precedence.
- Reject unknown keys in TOML, YAML, and JSON through the shared serde type.
- Remove every unsupported setting from both templates, including commented
  examples.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN before broad verification.

---

### Task 1: Enforce and document the real configuration schema

**Files:** Modify `crates/yamalgam/src/config.rs`,
`crates/yamalgam/tests/config_integration.rs`,
`config/yamalgam.toml.example`, and `config/yamalgam.yaml.example`.

- [ ] **Step 1: Replace the unknown-field acceptance regression**

Change the existing `unknown_config_field_is_ignored` integration test into a
test that uses a misspelled `log_level`, requires command failure, and requires
stderr to name an unknown field.

Add a template-contract test that requires both templates to retain
`log_level` and `log_dir`, and forbids the unsupported `otel_endpoint`,
`api_url`, feature, telemetry, and output-setting examples identified by the
audit.

- [ ] **Step 2: Run focused tests to verify RED**

```bash
cargo nextest run -p yamalgam --test config_integration -E 'test(unknown_config_field_fails_loudly) | test(config_templates_only_advertise_supported_fields)'
```

Expected: the misspelled field is currently accepted and the templates still
contain unsupported examples.

- [ ] **Step 3: Enforce the schema and reduce templates**

Add `deny_unknown_fields` to the existing serde container attributes on
`Config`. In both templates, retain the active `log_level` example and one
commented `log_dir` override, then remove the unsupported OpenTelemetry, API,
feature, telemetry, output-format, and color examples.

- [ ] **Step 4: Verify, review, and commit**

```bash
cargo nextest run -p yamalgam --test config_integration
just fmt
just clippy
just test
just doc-test
git --no-pager diff --check
```

After independent review, create the focused signed code commit, then append
and separately commit its exact SHA in the audit ledger.
