//! Gungraun benchmarks for yamalgam-core
//!
//! CPU instruction count benchmarks for deterministic CI regression detection.
//! Uses Valgrind under the hood - results are consistent across runs.
//!
//! Run with: `cargo bench --bench gungraun_benchmarks`
//!
//! Platform support:
//! - Linux x86_64/ARM: Fully supported
//! - macOS Intel (x86_64): Fully supported
//! - macOS ARM (M1/M2/M3): NOT supported (Valgrind limitation)
//! - Windows: NOT supported
//!
//! AUTO-GENERATED from crates/yamalgam-core/benches/benchmarks.toml.
//! Do not edit directly. Run `cargo xtask gen-benchmarks` to regenerate.
//!
//! See docs/benchmarks-howto.md for more information.

#![allow(missing_docs, unsafe_code)]

use gungraun::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use yamalgam_core::config::{Config, ConfigLoader, ConfigSources};

#[library_benchmark]
fn load_defaults() -> (Config, ConfigSources) {
    black_box(
        ConfigLoader::new()
            .with_user_config(false)
            .without_boundary_marker()
            .load()
            .unwrap(),
    )
}

#[library_benchmark]
fn construct_loader() -> ConfigLoader {
    black_box(ConfigLoader::new())
}

library_benchmark_group!(
    name = all_benchmarks;
    benchmarks = load_defaults, construct_loader
);

main!(library_benchmark_groups = all_benchmarks);
