//! yamalgam-compare — YAML Test Suite compliance and cross-implementation comparison.
//!
//! This crate provides the infrastructure for validating yamalgam against the
//! YAML Test Suite and comparing event/token streams across implementations.
//!
//! **Token-level:**
//! - [`TokenSnapshot`] / [`SpanSnapshot`]: implementation-neutral token representation
//! - [`CompareResult`]: enum covering all token comparison outcomes
//! - [`compare_token_streams`]: element-by-element token stream diffing
//! - [`run_rust_scanner`]: harness function for yamalgam's scanner
//!
//! **Event-level:**
//! - [`EventSnapshot`]: implementation-neutral event representation
//! - [`CompareEventResult`]: enum covering all event comparison outcomes
//! - [`compare_event_streams`]: element-by-element event stream diffing
//! - [`run_rust_parser`]: harness function for yamalgam's parser
//!
//! **YAML Test Suite:**
//! - [`TestCase`] / [`extract_test_cases`]: extract test cases from suite files
//! - [`parse_tree`]: parse the suite's tree event format into [`EventSnapshot`]s

pub mod compare;
pub mod event_snapshot;
pub mod harness;
pub mod snapshot;
pub mod test_case;
pub mod tree_format;

pub use compare::{
    CompareEventResult, CompareResult, compare_event_streams, compare_event_streams_with_tags,
    compare_token_streams,
};
pub use event_snapshot::EventSnapshot;
pub use harness::{run_rust_parser, run_rust_scanner};
pub use snapshot::{SpanSnapshot, TokenSnapshot};
pub use test_case::{TestCase, extract_test_cases};
pub use tree_format::parse_tree;
