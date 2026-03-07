//! yamalgam-compare — Comparison logic for validating yamalgam against libfyaml.
//!
//! This crate provides the infrastructure for cross-implementation comparison:
//!
//! **Token-level:**
//! - [`TokenSnapshot`] / [`SpanSnapshot`]: implementation-neutral token representation
//! - [`CompareResult`]: enum covering all token comparison outcomes
//! - [`compare_token_streams`]: element-by-element token stream diffing
//! - [`run_c_tokenizer`] / [`run_rust_scanner`]: harness functions for each implementation
//! - [`compare_input`]: end-to-end token comparison of raw YAML input
//!
//! **Event-level:**
//! - [`EventSnapshot`]: implementation-neutral event representation
//! - [`CompareEventResult`]: enum covering all event comparison outcomes
//! - [`compare_event_streams`]: element-by-element event stream diffing
//! - [`run_c_events`] / [`run_rust_parser`]: harness functions for each implementation
//! - [`compare_events`]: end-to-end event comparison of raw YAML input

pub mod compare;
pub mod event_snapshot;
pub mod harness;
pub mod snapshot;

pub use compare::{
    CompareEventResult, CompareResult, compare_event_streams, compare_token_streams,
};
pub use event_snapshot::EventSnapshot;
pub use harness::{
    compare_events, compare_input, run_c_events, run_c_tokenizer, run_rust_parser, run_rust_scanner,
};
pub use snapshot::{SpanSnapshot, TokenSnapshot};
