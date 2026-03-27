//! Divan benchmarks for yamalgam-core
//!
//! Wall-clock time benchmarks for fast local iteration.
//! Run with: `cargo bench --bench divan_benchmarks`
//!
//! AUTO-GENERATED from crates/yamalgam-core/benches/benchmarks.kdl.
//! Do not edit directly. Run `cargo xtask gen-benchmarks` to regenerate.
//!
//! See docs/benchmarks-howto.md for more information.

use std::hint::black_box;

use yamalgam_core::config::{Config, ConfigLoader};

fn main() {
    divan::main();
}

mod config {
    use super::*;

    #[divan::bench]
    fn load_defaults() -> Config {
        black_box(
            ConfigLoader::new()
                .with_user_config(false)
                .without_boundary_marker()
                .load()
                .unwrap(),
        )
    }

    #[divan::bench]
    fn construct_loader() -> ConfigLoader {
        black_box(ConfigLoader::new())
    }

}

