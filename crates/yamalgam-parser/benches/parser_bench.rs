//! Divan benchmarks for yamalgam-parser in isolation.
//!
//! Measures event-stream throughput across various input shapes.
//! Run with: `cargo bench -p yamalgam-parser`

// Benchmark code doesn't need documentation.
#![allow(missing_docs)]

use divan::Bencher;
use yamalgam_parser::Parser;

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
fn parse_small(bencher: Bencher) {
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Parser::new(SMALL).collect();
    });
}

#[divan::bench]
fn parse_medium(bencher: Bencher) {
    let input = medium();
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Parser::new(&input).collect();
    });
}

#[divan::bench]
fn parse_large(bencher: Bencher) {
    let input = large();
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Parser::new(&input).collect();
    });
}

#[divan::bench]
fn parse_anchored(bencher: Bencher) {
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&format!("anchor{i}: &a{i}\n  field: value-{i}\n"));
    }
    input.push_str("refs:\n");
    for i in 0..100 {
        for _ in 0..5 {
            input.push_str(&format!("  - *a{i}\n"));
        }
    }
    bencher.bench(|| {
        let _: Result<Vec<_>, _> = Parser::new(&input).collect();
    });
}
