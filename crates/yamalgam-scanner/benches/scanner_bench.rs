//! Divan benchmarks for yamalgam-scanner in isolation.
//!
//! Measures raw tokenization throughput across various input shapes.
//! Run with: `cargo bench -p yamalgam-scanner`

// Benchmark code doesn't need documentation.
#![allow(missing_docs)]

use divan::Bencher;
use yamalgam_scanner::scanner::Scanner;

fn main() {
    divan::main();
}

// Small: simple mapping (~50 bytes)
const SMALL: &str = "name: yamalgam\nversion: 0.1.0\nlicense: MIT\n";

// Medium: nested config (~2KB, kubernetes-style)
fn medium() -> String {
    "apiVersion: apps/v1\n\
     kind: Deployment\n\
     metadata:\n\
     \x20 name: app\n\
     \x20 labels:\n\
     \x20   app: web\n\
     spec:\n\
     \x20 replicas: 3\n\
     \x20 selector:\n\
     \x20   matchLabels:\n\
     \x20     app: web\n\
     \x20 template:\n\
     \x20   metadata:\n\
     \x20     labels:\n\
     \x20       app: web\n\
     \x20   spec:\n\
     \x20     containers:\n\
     \x20       - name: web\n\
     \x20         image: nginx:1.25\n\
     \x20         ports:\n\
     \x20           - containerPort: 80\n\
     \x20         env:\n\
     \x20           - name: NODE_ENV\n\
     \x20             value: production\n"
        .to_owned()
}

// Large: 1000 records (~120KB)
fn large() -> String {
    let mut out = String::with_capacity(120_000);
    out.push_str("records:\n");
    for i in 0..1000 {
        out.push_str(&format!(
            "  - id: {i}\n    name: \"record-{i}\"\n    active: {}\n",
            i % 2 == 0
        ));
    }
    out
}

#[divan::bench]
fn scan_small(bencher: Bencher) {
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Scanner::new(SMALL).collect();
    });
}

#[divan::bench]
fn scan_medium(bencher: Bencher) {
    let input = medium();
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Scanner::new(&input).collect();
    });
}

#[divan::bench]
fn scan_large(bencher: Bencher) {
    let input = large();
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Scanner::new(&input).collect();
    });
}

#[divan::bench]
fn scan_nested_256(bencher: Bencher) {
    let mut input = String::new();
    for i in 0..256 {
        input.push_str(&"  ".repeat(i));
        input.push_str(&format!("l{i}:\n"));
    }
    input.push_str(&"  ".repeat(256));
    input.push_str("leaf: value\n");
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Scanner::new(&input).collect();
    });
}

#[divan::bench]
fn scan_large_scalar(bencher: Bencher) {
    let mut input = String::from("content: |\n");
    let line = "  The quick brown fox jumps over the lazy dog.\n";
    while input.len() < 1_000_000 {
        input.push_str(line);
    }
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Scanner::new(&input).collect();
    });
}
