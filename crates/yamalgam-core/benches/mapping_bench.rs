//! Benchmarks ordered mapping insertion and borrowed scalar-key lookup.

#![allow(missing_docs)]

use divan::Bencher;
use yamalgam_core::{Mapping, Value};

fn main() {
    divan::main();
}

#[divan::bench(args = [100, 1_000, 10_000])]
fn insert_distinct_string_keys(bencher: Bencher, len: usize) {
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

#[divan::bench(args = [100, 1_000, 10_000])]
fn get_borrowed_string_keys(bencher: Bencher, len: usize) {
    let keys = (0..len)
        .map(|index| format!("key-{index}"))
        .collect::<Vec<_>>();
    let mut mapping = Mapping::new();
    for (index, key) in keys.iter().cloned().enumerate() {
        mapping.insert(Value::String(key), Value::Integer(index as i64));
    }

    bencher.bench_local(|| {
        for key in &keys {
            divan::black_box(mapping.get_str(divan::black_box(key.as_str())));
        }
    });
}

#[divan::bench(args = [100, 1_000, 10_000])]
fn get_absent_string_from_fallback_only_mapping(bencher: Bencher, len: usize) {
    let mut mapping = Mapping::new();
    for index in 0..len {
        mapping.insert(Value::Float(index as f64 + 0.5), Value::Null);
    }
    assert!(mapping.get_str("missing").is_none());

    bencher.bench_local(|| divan::black_box(mapping.get_str(divan::black_box("missing"))));
}
