# Alias Materialization Budget Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound cumulative composer alias cloning by the configured output-byte budget while preserving current public `Value` and loader APIs.

**Architecture:** Replace composer anchor values with private cached `(Value, materialized_weight)` records. Compute each weight iteratively as the recursive `Value` footprint plus owned UTF-8 string bytes, then charge a checked cumulative total against `ResourceLimits::max_output_bytes` before cloning an alias; retain the independent alias-count limit.

**Tech Stack:** Rust 2024, `yamalgam-compose`, `yamalgam-core::ResourceLimits`, `cargo nextest`, Just.

## Global Constraints

- Add no dependency and no public configuration field.
- Preserve `Value::Clone`, document-local anchor scope, anchor redefinition, alias-count behavior, and unlimited `LoaderConfig::unchecked()` behavior.
- Compute materialized weight iteratively so unchecked deep values do not add a recursive traversal hazard.
- Charge every alias before cloning and report arithmetic overflow as `ComposeError::LimitExceeded`.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN with focused `cargo nextest` runs before broader verification.

---

### Task 1: Cache anchor weights and enforce the cumulative alias budget

**Files:**

Modify and test `crates/yamalgam-compose/src/lib.rs`.

**Interfaces:**

Consumes `ResourceLimits::check_alias_expansions` and `ResourceLimits::check_output_size`.

Produces private `AnchoredValue`, `materialized_weight(&Value)`, `checked_materialized_total`, and `Composer::remember_anchor`; no public API changes.

- [ ] **Step 1: Add focused failing weight and budget tests**

Add these helpers and tests beside the existing composer limit tests:

```rust
fn output_config(max_output_bytes: Option<usize>) -> LoaderConfig {
    LoaderConfig {
        limits: ResourceLimits {
            max_alias_expansions: None,
            max_output_bytes,
            ..ResourceLimits::none()
        },
        ..LoaderConfig::unchecked()
    }
}

#[test]
fn materialized_weight_counts_value_storage_and_string_bytes() {
    let value = Value::Sequence(vec![
        Value::String("alpha".into()),
        Value::Integer(7),
    ]);
    let expected = 3 * std::mem::size_of::<Value>() + "alpha".len();

    assert_eq!(materialized_weight(&value).unwrap(), expected);
}

#[test]
fn alias_materialization_budget_honors_exact_boundary() {
    let input = "base: &base [payload]\none: *base\ntwo: *base";
    let anchored = Value::Sequence(vec![Value::String("payload".into())]);
    let weight = materialized_weight(&anchored).unwrap();

    assert!(compose_with_unchecked_parser(input, &output_config(Some(weight * 2))).is_ok());
    assert!(matches!(
        compose_with_unchecked_parser(input, &output_config(Some(weight * 2 - 1))),
        Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
    ));
}

#[test]
fn alias_materialization_budget_is_size_weighted_and_cumulative() {
    let input = "small: &small x\nlarge: &large [one, two, three, four]\na: *small\nb: *large";
    let small = materialized_weight(&Value::String("x".into())).unwrap();
    let large = materialized_weight(&Value::Sequence(vec![
        Value::String("one".into()),
        Value::String("two".into()),
        Value::String("three".into()),
        Value::String("four".into()),
    ]))
    .unwrap();

    assert!(matches!(
        compose_with_unchecked_parser(input, &output_config(Some(small + large - 1))),
        Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
    ));
}

#[test]
fn alias_materialization_arithmetic_overflow_is_a_limit_error() {
    assert!(matches!(
        checked_materialized_total(usize::MAX, 1, &ResourceLimits::none()),
        Err(ComposeError::LimitExceeded(message))
            if message == "alias materialization size overflow"
    ));
}
```

Add the exact scope and redefinition regressions:

```rust
#[test]
fn anchor_redefinition_replaces_value_and_cached_weight() {
    let input = "first: &ref x\none: *ref\nreplace: &ref [one, two, three]\ntwo: *ref";
    let small = materialized_weight(&Value::String("x".into())).unwrap();
    let redefined = Value::Sequence(vec![
        Value::String("one".into()),
        Value::String("two".into()),
        Value::String("three".into()),
    ]);
    let large = materialized_weight(&redefined).unwrap();
    let mut exact = output_config(Some(small + large));
    exact.limits.max_anchor_count = Some(1);
    let documents = compose_with_unchecked_parser(input, &exact).unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].get("two"), Some(&redefined));

    let mut below = output_config(Some(small + large - 1));
    below.limits.max_anchor_count = Some(1);
    assert!(matches!(
        compose_with_unchecked_parser(input, &below),
        Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
    ));
}

#[test]
fn aliases_cannot_resolve_anchors_from_a_previous_document() {
    let input = "---\na: &ref value\n---\nb: *ref";
    assert!(matches!(
        compose_with_unchecked_parser(input, &output_config(None)),
        Err(ComposeError::UndefinedAlias(name)) if name == "ref"
    ));
}

#[test]
fn alias_materialization_bytes_are_cumulative_across_documents() {
    let input = "---\na: &ref [payload]\nb: *ref\n---\nc: &ref [payload]\nd: *ref";
    let anchored = Value::Sequence(vec![Value::String("payload".into())]);
    let weight = materialized_weight(&anchored).unwrap();

    assert!(compose_with_unchecked_parser(input, &output_config(Some(weight * 2))).is_ok());
    assert!(matches!(
        compose_with_unchecked_parser(input, &output_config(Some(weight * 2 - 1))),
        Err(ComposeError::LimitExceeded(message)) if message.contains("output size")
    ));
}

#[test]
fn unlimited_output_budget_allows_cumulative_alias_materialization() {
    let input = "base: &base [one, two, three]\na: *base\nb: *base\nc: *base";
    assert!(compose_with_unchecked_parser(input, &output_config(None)).is_ok());
}
```

Add an isolated traversal worker so recursive `Value::Drop` cannot mask the
weight traversal result:

```rust
const DEEP_WEIGHT_WORKER: &str = "YAMALGAM_DEEP_WEIGHT_WORKER";

#[test]
fn materialized_weight_is_stack_safe_on_a_deep_public_value() {
    if std::env::var_os(DEEP_WEIGHT_WORKER).is_some() {
        let mut value = Value::Null;
        for _ in 0..100_000 {
            value = Value::Sequence(vec![value]);
        }
        assert_eq!(
            materialized_weight(&value).unwrap(),
            100_001 * std::mem::size_of::<Value>()
        );
        std::mem::forget(value);
        return;
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .env(DEEP_WEIGHT_WORKER, "1")
        .args([
            "--exact",
            "tests::materialized_weight_is_stack_safe_on_a_deep_public_value",
            "--nocapture",
        ])
        .output()
        .expect("run deep materialized-weight worker");
    assert!(
        output.status.success(),
        "deep weight worker failed: status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
```

- [ ] **Step 2: Run the focused tests to verify RED**

Run:

```bash
cargo nextest run -p yamalgam-compose -E 'test(materialized_weight_counts_value_storage_and_string_bytes) | test(alias_materialization_budget_honors_exact_boundary) | test(alias_materialization_budget_is_size_weighted_and_cumulative) | test(alias_materialization_arithmetic_overflow_is_a_limit_error) | test(anchor_redefinition_replaces_value_and_cached_weight) | test(alias_materialization_bytes_are_cumulative_across_documents) | test(materialized_weight_is_stack_safe_on_a_deep_public_value)'
```

Expected: compilation fails because `materialized_weight` and `checked_materialized_total` do not exist.

- [ ] **Step 3: Add private cached anchor state and cumulative accounting**

Add:

```rust
struct AnchoredValue {
    value: Value,
    materialized_weight: usize,
}

pub struct Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    events: std::iter::Peekable<I>,
    anchors: HashMap<String, AnchoredValue>,
    config: ResourceLimits,
    alias_expansion_count: usize,
    alias_materialized_bytes: usize,
    tag_resolver: Box<dyn TagResolver>,
}
```

Initialize both counters to zero in `new_with_config`. Keep both counters cumulative across the stream, matching the existing alias-count scope; continue clearing only document-local anchors at document boundaries.

- [ ] **Step 4: Implement iterative materialized weight calculation**

Add this free helper near the existing composer helpers:

```rust
fn materialized_weight(value: &Value) -> Result<usize, ComposeError> {
    let mut total = 0_usize;
    let mut pending = vec![value];

    while let Some(value) = pending.pop() {
        total = total
            .checked_add(std::mem::size_of::<Value>())
            .ok_or_else(|| {
                ComposeError::LimitExceeded("anchor materialized weight overflow".into())
            })?;
        match value {
            Value::String(string) => {
                total = total.checked_add(string.len()).ok_or_else(|| {
                    ComposeError::LimitExceeded("anchor materialized weight overflow".into())
                })?;
            }
            Value::Sequence(items) => pending.extend(items),
            Value::Mapping(mapping) => {
                for (key, value) in mapping.iter() {
                    pending.push(key);
                    pending.push(value);
                }
            }
            Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Float(_) => {}
        }
    }

    Ok(total)
}
```

This weight covers every materialized `Value` slot plus every separately owned string buffer. It is independent of YAML presentation style and does not recurse on the call stack.

- [ ] **Step 5: Centralize anchor insertion and cache each weight once**

Add:

```rust
fn remember_anchor(&mut self, name: String, value: &Value) -> Result<(), ComposeError> {
    let materialized_weight = materialized_weight(value)?;
    self.anchors.insert(
        name,
        AnchoredValue {
            value: value.clone(),
            materialized_weight,
        },
    );
    self.config
        .check_anchor_count(self.anchors.len())
        .map_err(ComposeError::LimitExceeded)
}
```

Replace the scalar, sequence, and mapping anchor branches with `self.remember_anchor(name.into_owned(), &value)?`. Anchor redefinition replaces both the cached value and its weight without increasing the distinct-anchor count.

- [ ] **Step 6: Charge checked cumulative weight before alias cloning**

Add:

```rust
fn checked_materialized_total(
    current: usize,
    increment: usize,
    limits: &ResourceLimits,
) -> Result<usize, ComposeError> {
    let next = current.checked_add(increment).ok_or_else(|| {
        ComposeError::LimitExceeded("alias materialization size overflow".into())
    })?;
    limits
        .check_output_size(next)
        .map_err(ComposeError::LimitExceeded)?;
    Ok(next)
}
```

In the alias branch, use checked addition for `alias_expansion_count`, enforce the existing count limit, resolve the cached anchor, call `checked_materialized_total`, assign the returned total, and only then clone `anchored.value`. Preserve `UndefinedAlias` for missing anchors.

- [ ] **Step 7: Run focused and affected-crate tests to verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-compose -E 'test(materialized_weight_counts_value_storage_and_string_bytes) | test(alias_materialization_budget_honors_exact_boundary) | test(alias_materialization_budget_is_size_weighted_and_cumulative) | test(alias_materialization_arithmetic_overflow_is_a_limit_error) | test(alias_expansion_limit) | test(undefined_alias_errors)'
cargo nextest run -p yamalgam-compose
```

Expected: all focused tests and the full composer suite pass.

- [ ] **Step 8: Run final verification and commit the code action**

Run:

```bash
just fmt
just clippy
just test
cargo test --doc -p yamalgam-compose
git diff --check
```

Expected: formatting is stable, clippy is warning-free, all workspace tests and composer doc tests pass, and the patch has no whitespace errors.

Stage only `crates/yamalgam-compose/src/lib.rs`, then commit:

```bash
git commit -S -m "fix(compose): bound alias materialization"
```

After independent review and signature verification, append one ledger entry for `composer-alias-deep-clone-is-size-unbudgeted`, naming the exact full code SHA, and commit that audit-only update separately.
