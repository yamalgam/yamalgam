# Indexed Mapping Lookup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve ordered `Mapping` semantics while making equality-safe scalar insertion and borrowed string lookup nearly constant-time.

**Architecture:** Keep `Vec<(Value, Value)>` as the sole semantic state and add a lazily initialized private scalar fingerprint index. Randomized fingerprints cover null, bool, integer, and string keys; bucket candidates are checked against the ordered entries, while float and collection keys retain the existing linear `PartialEq` fallback.

**Tech Stack:** Rust 2024, `std::collections::{HashMap, hash_map::RandomState}`, `cargo nextest`, Divan benchmarks.

## Global Constraints

- Preserve insertion order, duplicate replacement in place, `const Mapping::new()`, public signatures, and current `PartialEq` semantics.
- Do not index floats, sequences, or mappings because their current equality is not a sound `Eq + Hash` contract.
- Do not add a new third-party production dependency; Divan is already used by workspace benchmarks and remains dev-only.
- Keep the private index out of `Mapping` equality and debug output because it is not semantic state.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN with focused `cargo nextest` runs before broader verification.

---

### Task 1: Add the scalar side index and allocation-free borrowed lookup

**Files:**

Modify `crates/yamalgam-core/src/value.rs:13-137`.

Test `crates/yamalgam-core/src/value.rs:345-470`.

**Interfaces:**

Consumes the existing `Value`, `Mapping::new`, `Mapping::insert`, `Mapping::get`, and `Value::get` APIs.

Produces private `ScalarIndex`; private fingerprint and position helpers; public `Mapping::get_str(&str) -> Option<&Value>`; unchanged existing public behavior.

- [ ] **Step 1: Write focused failing unit tests**

Add tests in `value.rs` that require the missing index and `get_str` API:

```rust
#[test]
fn scalar_keys_use_a_complete_side_index() {
    let mut mapping = Mapping::new();
    for index in 0_i64..10_000 {
        mapping.insert(Value::Integer(index), Value::Integer(index));
    }

    let scalar_index = mapping.scalar_index.as_ref().expect("scalar index");
    assert_eq!(
        scalar_index.positions.values().map(Vec::len).sum::<usize>(),
        mapping.len()
    );
    assert_eq!(mapping.get(&Value::Integer(9_999)), Some(&Value::Integer(9_999)));
}

#[test]
fn borrowed_string_lookup_uses_the_scalar_index() {
    let mut mapping = Mapping::new();
    mapping.insert(Value::from("alpha"), Value::Integer(1));
    mapping.insert(Value::from("beta"), Value::Integer(2));

    assert_eq!(mapping.get_str("beta"), Some(&Value::Integer(2)));
    assert_eq!(Value::Mapping(mapping).get("alpha"), Some(&Value::Integer(1)));
}
```

Add these exact regression cases beside them:

```rust
#[test]
fn scalar_and_fallback_keys_preserve_replacement_semantics() {
    let keys = [
        Value::Null,
        Value::Bool(true),
        Value::Integer(7),
        Value::from("seven"),
        Value::Sequence(vec![Value::Integer(7)]),
    ];
    let mut mapping = Mapping::new();
    for key in &keys {
        assert_eq!(mapping.insert(key.clone(), Value::Integer(1)), None);
        assert_eq!(
            mapping.insert(key.clone(), Value::Integer(2)),
            Some(Value::Integer(1))
        );
    }
    assert_eq!(mapping.len(), keys.len());
    for key in &keys {
        assert_eq!(mapping.get(key), Some(&Value::Integer(2)));
    }
}

#[test]
fn fallback_keys_preserve_float_and_nested_mapping_equality() {
    let mut signed_zero = Mapping::new();
    assert_eq!(
        signed_zero.insert(Value::Float(-0.0), Value::Integer(1)),
        None
    );
    assert_eq!(
        signed_zero.insert(Value::Float(0.0), Value::Integer(2)),
        Some(Value::Integer(1))
    );
    assert_eq!(signed_zero.len(), 1);

    let mut nan = Mapping::new();
    assert_eq!(nan.insert(Value::Float(f64::NAN), Value::Integer(1)), None);
    assert_eq!(nan.insert(Value::Float(f64::NAN), Value::Integer(2)), None);
    assert_eq!(nan.len(), 2);

    let mut nested_key = Mapping::new();
    nested_key.insert(Value::from("nested"), Value::Bool(true));
    let nested_key = Value::Mapping(nested_key);
    let mut outer = Mapping::new();
    assert_eq!(outer.insert(nested_key.clone(), Value::Integer(1)), None);
    assert_eq!(
        outer.insert(nested_key, Value::Integer(2)),
        Some(Value::Integer(1))
    );
    assert_eq!(outer.len(), 1);
}

#[test]
fn fingerprint_collisions_compare_actual_string_keys() {
    let mut mapping = Mapping::new();
    mapping.insert(Value::from("left"), Value::Integer(1));
    mapping.insert(Value::from("right"), Value::Integer(2));
    let scalar_index = mapping.scalar_index.as_mut().expect("scalar index");
    let fingerprint = scalar_index.fingerprint_str("right");
    scalar_index.positions.get_mut(&fingerprint).unwrap().insert(0, 0);

    assert_eq!(mapping.get_str("right"), Some(&Value::Integer(2)));
    assert_eq!(
        mapping.insert(Value::from("right"), Value::Integer(3)),
        Some(Value::Integer(2))
    );
    assert_eq!(mapping.len(), 2);
}

#[test]
fn clone_equality_and_debug_ignore_index_identity() {
    let mut mapping = Mapping::new();
    mapping.insert(Value::from("key"), Value::Integer(1));
    let mut same_entries = mapping.clone();
    same_entries
        .scalar_index
        .as_mut()
        .expect("scalar index")
        .positions
        .clear();

    assert_eq!(mapping, same_entries);
    assert_eq!(
        format!("{mapping:?}"),
        "Mapping { entries: [(String(\"key\"), Integer(1))] }"
    );
}
```

- [ ] **Step 2: Run the focused tests to verify RED**

Run:

```bash
cargo nextest run -p yamalgam-core -E 'test(scalar_keys_use_a_complete_side_index) | test(borrowed_string_lookup_uses_the_scalar_index)'
```

Expected: compilation fails because `Mapping::scalar_index` and `Mapping::get_str` do not exist.

- [ ] **Step 3: Implement the lazy randomized scalar index**

Add private index state and explicit semantic trait implementations:

```rust
#[derive(Clone)]
struct ScalarIndex {
    hash_builder: RandomState,
    positions: HashMap<u64, Vec<usize>>,
}

#[derive(Clone)]
pub struct Mapping {
    entries: Vec<(Value, Value)>,
    scalar_index: Option<ScalarIndex>,
}

impl PartialEq for Mapping {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
```

Implement the index with explicit variant tags and actual-key collision checks:

```rust
impl ScalarIndex {
    fn new() -> Self {
        Self {
            hash_builder: RandomState::new(),
            positions: HashMap::new(),
        }
    }

    fn is_indexable(key: &Value) -> bool {
        matches!(
            key,
            Value::Null | Value::Bool(_) | Value::Integer(_) | Value::String(_)
        )
    }

    fn fingerprint_value(&self, key: &Value) -> Option<u64> {
        let mut hasher = self.hash_builder.build_hasher();
        match key {
            Value::Null => 0_u8.hash(&mut hasher),
            Value::Bool(value) => {
                1_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::Integer(value) => {
                2_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::String(value) => {
                3_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::Float(_) | Value::Sequence(_) | Value::Mapping(_) => return None,
        }
        Some(hasher.finish())
    }

    fn fingerprint_str(&self, key: &str) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        3_u8.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }
}

impl Mapping {
    fn indexed_position(&self, fingerprint: u64, key: &Value) -> Option<usize> {
        self.scalar_index
            .as_ref()?
            .positions
            .get(&fingerprint)?
            .iter()
            .copied()
            .find(|&position| &self.entries[position].0 == key)
    }

    fn linear_position(&self, key: &Value) -> Option<usize> {
        self.entries.iter().position(|(candidate, _)| candidate == key)
    }
}
```

Route `insert` through `indexed_position` for indexable keys, creating `ScalarIndex` on the first such key. Append the new position to `positions.entry(fingerprint).or_default()` only after pushing the ordered entry. Route floats, sequences, and mappings through `linear_position`. Implement custom `Debug` with only the `entries` field and initialize `scalar_index: None` in const `Mapping::new()`.

- [ ] **Step 4: Implement borrowed string lookup without a temporary `Value`**

Add:

```rust
#[must_use]
pub fn get_str(&self, key: &str) -> Option<&Value> {
    if let Some(index) = &self.scalar_index {
        let fingerprint = index.fingerprint_str(key);
        return index.positions.get(&fingerprint)?.iter().copied().find_map(
            |position| match &self.entries[position] {
                (Value::String(candidate), value) if candidate == key => Some(value),
                _ => None,
            },
        );
    }

    self.entries.iter().find_map(|(candidate, value)| match candidate {
        Value::String(candidate) if candidate == key => Some(value),
        _ => None,
    })
}
```

Delegate `Value::get(&str)` directly to `Mapping::get_str`. If no scalar index exists, use a borrowed linear string comparison; never call `to_owned` or construct a temporary `Value::String`.

- [ ] **Step 5: Run focused and crate tests to verify GREEN**

Run:

```bash
cargo nextest run -p yamalgam-core value::tests
cargo nextest run -p yamalgam-core
```

Expected: all value tests and the full core crate suite pass.

### Task 2: Add a reproducible wide-map benchmark and verify the change

**Files:**

Modify `crates/yamalgam-core/Cargo.toml`.

Create `crates/yamalgam-core/benches/mapping_bench.rs`.

**Interfaces:**

Consumes `Mapping::insert` and `Mapping::get_str` from Task 1.

Produces the `mapping_bench` Divan target with wide insertion and borrowed lookup measurements at fixed sizes.

- [ ] **Step 1: Register the existing workspace benchmark tool**

Add `divan = "0.1"` under `yamalgam-core` dev-dependencies and:

```toml
[[bench]]
name = "mapping_bench"
harness = false
```

- [ ] **Step 2: Add fixed-size insertion and borrowed lookup benchmarks**

Create `mapping_bench.rs` with `#![allow(missing_docs)]`, `divan::main()`, and per-iteration owned key vectors generated outside the timed insertion closure:

```rust
#[divan::bench(args = [100, 1_000, 10_000])]
fn insert_distinct_string_keys(bencher: divan::Bencher, len: usize) {
    bencher
        .with_inputs(|| {
            (0..len)
                .map(|index| format!("key-{index}"))
                .collect::<Vec<_>>()
        })
        .bench_local_values(|keys| {
            let mut mapping = Mapping::new();
            for (index, key) in keys.into_iter().enumerate() {
                mapping.insert(Value::String(key), Value::Integer(index as i64));
            }
            divan::black_box(mapping)
        });
}
```

Add a lookup benchmark that builds the mapping once, repeatedly passes borrowed `&str` keys to `get_str`, and black-boxes the returned references.

- [ ] **Step 3: Compile and smoke-run the benchmark**

Run:

```bash
cargo bench -p yamalgam-core --bench mapping_bench --no-run
cargo bench -p yamalgam-core --bench mapping_bench -- --sample-count 10
```

Expected: the benchmark compiles and reports all fixed insertion and lookup sizes.

- [ ] **Step 4: Run final focused verification**

Run:

```bash
cargo nextest run -p yamalgam-core
just clippy
git diff --check
```

Expected: all core tests pass, all-target/all-feature clippy is warning-free, and the patch has no whitespace errors.

- [ ] **Step 5: Commit the focused code change**

Stage only `crates/yamalgam-core/src/value.rs`, `crates/yamalgam-core/Cargo.toml`, `crates/yamalgam-core/benches/mapping_bench.rs`, and any Cargo lockfile change, then commit:

```bash
git commit -S -m "perf(core): index scalar mapping keys"
```

After independent review and signature verification, append one ledger entry addressing both `composer-mapping-insertion-is-quadratic` and `borrowed-value-get-allocates-key`, naming the exact full code SHA, and commit that audit-only update separately.
