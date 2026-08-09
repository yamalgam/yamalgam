//! Dynamic-dispatch benchmarks for parser consumers.
//!
//! Run with: `cargo bench -p yamalgam-bench --bench dispatch`

#![allow(missing_docs)]

use std::hint::black_box;

use divan::Bencher;
use yamalgam_core::{LoaderConfig, TagResolution, TagResolver as _};
use yamalgam_parser::resolve::BoxedResolveStream;
use yamalgam_parser::{Event, NoopResolver, ParseError, Parser, ResolveError, ResolvedEvents};

fn main() {
    divan::main();
}

fn map_parse_error<'input>(
    result: Result<Event<'input>, ParseError>,
) -> Result<Event<'input>, ResolveError> {
    result.map_err(ResolveError::Parse)
}

fn consume_events<'input>(
    events: impl Iterator<Item = Result<Event<'input>, ResolveError>>,
) -> usize {
    events
        .map(|event| {
            black_box(event.expect("benchmark input must parse"));
            1
        })
        .sum()
}

fn boxed_events(input: &str) -> usize {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    let upstream: BoxedResolveStream<'_> = Box::new(parser);
    consume_events(ResolvedEvents::new_with_config(
        upstream,
        NoopResolver,
        &config,
    ))
}

fn typed_events(input: &str) -> usize {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    consume_events(ResolvedEvents::new_with_config(
        parser,
        NoopResolver,
        &config,
    ))
}

fn collect_boxed_events(input: &str) -> Vec<Event<'_>> {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    let upstream: BoxedResolveStream<'_> = Box::new(parser);
    ResolvedEvents::new_with_config(upstream, NoopResolver, &config)
        .collect::<Result<_, _>>()
        .expect("benchmark input must parse")
}

fn collect_typed_events(input: &str) -> Vec<Event<'_>> {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    ResolvedEvents::new_with_config(parser, NoopResolver, &config)
        .collect::<Result<_, _>>()
        .expect("benchmark input must parse")
}

fn boxed_composer(input: &str) -> Vec<yamalgam_core::Value> {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    let upstream: BoxedResolveStream<'_> = Box::new(parser);
    let events = ResolvedEvents::new_with_config(upstream, NoopResolver, &config);
    let mut composer = yamalgam_compose::Composer::new_with_config(events, &config);
    composer
        .compose_stream()
        .expect("benchmark input must compose")
}

fn typed_composer(input: &str) -> Vec<yamalgam_core::Value> {
    let config = LoaderConfig::default();
    let parser = Parser::with_config(input, &config).map(map_parse_error);
    let events = ResolvedEvents::new_with_config(parser, NoopResolver, &config);
    let mut composer = yamalgam_compose::Composer::new_with_config(events, &config);
    composer
        .compose_stream()
        .expect("benchmark input must compose")
}

fn erased_serde(input: &str) -> usize {
    let value: serde_json::Value =
        yamalgam_serde::from_str(input).expect("benchmark input must deserialize");
    black_box(value.as_object().map_or(0, serde_json::Map::len))
}

#[divan::bench]
fn events_small_boxed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    assert_eq!(collect_boxed_events(&input), collect_typed_events(&input));
    bencher.bench_local(|| boxed_events(black_box(&input)));
}

#[divan::bench]
fn events_small_typed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    assert_eq!(collect_boxed_events(&input), collect_typed_events(&input));
    bencher.bench_local(|| typed_events(black_box(&input)));
}

#[divan::bench]
fn events_medium_boxed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    assert_eq!(collect_boxed_events(&input), collect_typed_events(&input));
    bencher.bench_local(|| boxed_events(black_box(&input)));
}

#[divan::bench]
fn events_medium_typed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    assert_eq!(collect_boxed_events(&input), collect_typed_events(&input));
    bencher.bench_local(|| typed_events(black_box(&input)));
}

#[divan::bench]
fn composer_small_boxed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    assert_eq!(boxed_composer(&input), typed_composer(&input));
    bencher.bench_local(|| black_box(boxed_composer(black_box(&input))));
}

#[divan::bench]
fn composer_small_typed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    assert_eq!(boxed_composer(&input), typed_composer(&input));
    bencher.bench_local(|| black_box(typed_composer(black_box(&input))));
}

#[divan::bench]
fn composer_medium_boxed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    assert_eq!(boxed_composer(&input), typed_composer(&input));
    bencher.bench_local(|| black_box(boxed_composer(black_box(&input))));
}

#[divan::bench]
fn composer_medium_typed(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    assert_eq!(boxed_composer(&input), typed_composer(&input));
    bencher.bench_local(|| black_box(typed_composer(black_box(&input))));
}

#[divan::bench]
fn serde_small_erased(bencher: Bencher) {
    let input = yamalgam_bench::inputs::kubernetes_deployment();
    bencher.bench_local(|| erased_serde(black_box(&input)));
}

#[divan::bench]
fn serde_medium_erased(bencher: Bencher) {
    let input = yamalgam_bench::inputs::records(1_000);
    bencher.bench_local(|| erased_serde(black_box(&input)));
}

const SCALARS: [&str; 16] = [
    "null",
    "true",
    "false",
    "0",
    "42",
    "-17",
    "0.5",
    ".inf",
    ".nan",
    "plain",
    "2026-08-08",
    "yes",
    "off",
    "0x2a",
    "1e6",
    "with spaces",
];

#[divan::bench]
fn tag_resolution_concrete(bencher: Bencher) {
    let resolver = TagResolution::Yaml12;
    bencher.bench_local(|| {
        SCALARS
            .iter()
            .filter(|scalar| resolver.resolves_as_string(black_box(scalar)))
            .count()
    });
}

#[divan::bench]
fn tag_resolution_dynamic(bencher: Bencher) {
    let resolver: Box<dyn yamalgam_core::TagResolver> = Box::new(TagResolution::Yaml12);
    bencher.bench_local(|| {
        SCALARS
            .iter()
            .filter(|scalar| resolver.resolves_as_string(black_box(scalar)))
            .count()
    });
}
