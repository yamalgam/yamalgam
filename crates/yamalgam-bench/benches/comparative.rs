//! Comparative benchmarks: yamalgam vs YAML peers.
//!
//! Run with: `cargo bench -p yamalgam-bench`

// Benchmark code doesn't need documentation.
#![allow(missing_docs)]

use divan::Bencher;

fn main() {
    divan::main();
}

// =============================================================================
// Small input (~2KB Kubernetes deployment)
// =============================================================================

#[divan::bench]
fn small_yamalgam_scan(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::yamalgam_scan(&input));
}

#[divan::bench]
fn small_yamalgam_parse(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::yamalgam_parse(&input));
}

#[divan::bench]
fn small_yaml_serde(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::yaml_serde_parse(&input));
}

#[divan::bench]
fn small_libyaml_safer(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::libyaml_safer_parse(&input));
}

#[divan::bench]
fn small_yaml_rust2(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::yaml_rust2_parse(&input));
}

#[divan::bench]
fn small_saphyr_parser(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::saphyr_parser_parse(&input));
}

#[divan::bench]
fn small_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::saphyr_parse(&input));
}

#[divan::bench]
fn small_serde_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::serde_saphyr_parse(&input));
}

#[divan::bench]
fn small_rust_yaml(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench(|| yamalgam_bench::peers::rust_yaml_parse(&input));
}

// =============================================================================
// Medium input (~120KB, 1K records)
// =============================================================================

#[divan::bench]
fn medium_yamalgam_scan(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::yamalgam_scan(&input));
}

#[divan::bench]
fn medium_yamalgam_parse(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::yamalgam_parse(&input));
}

#[divan::bench]
fn medium_yaml_serde(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::yaml_serde_parse(&input));
}

#[divan::bench]
fn medium_libyaml_safer(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::libyaml_safer_parse(&input));
}

#[divan::bench]
fn medium_yaml_rust2(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::yaml_rust2_parse(&input));
}

#[divan::bench]
fn medium_saphyr_parser(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::saphyr_parser_parse(&input));
}

#[divan::bench]
fn medium_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::saphyr_parse(&input));
}

#[divan::bench]
fn medium_serde_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::serde_saphyr_parse(&input));
}

#[divan::bench]
fn medium_rust_yaml(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench(|| yamalgam_bench::peers::rust_yaml_parse(&input));
}

// =============================================================================
// Large input (~1.2MB, 10K records)
// =============================================================================

#[divan::bench]
fn large_yamalgam_scan(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::yamalgam_scan(&input));
}

#[divan::bench]
fn large_yamalgam_parse(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::yamalgam_parse(&input));
}

#[divan::bench]
fn large_yaml_serde(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::yaml_serde_parse(&input));
}

#[divan::bench]
fn large_libyaml_safer(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::libyaml_safer_parse(&input));
}

#[divan::bench]
fn large_yaml_rust2(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::yaml_rust2_parse(&input));
}

#[divan::bench]
fn large_saphyr_parser(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::saphyr_parser_parse(&input));
}

#[divan::bench]
fn large_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::saphyr_parse(&input));
}

#[divan::bench]
fn large_serde_saphyr(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::serde_saphyr_parse(&input));
}

#[divan::bench]
fn large_rust_yaml(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(10_000);
    bencher.bench(|| yamalgam_bench::peers::rust_yaml_parse(&input));
}
