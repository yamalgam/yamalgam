//! yamalgam-compare — Comparison logic for validating yamalgam scanner against libfyaml.
//!
//! This crate provides the infrastructure for cross-implementation comparison:
//!
//! - [`TokenSnapshot`] / [`SpanSnapshot`]: implementation-neutral token representation
//! - [`CompareResult`]: enum covering all comparison outcomes
//! - [`compare_token_streams`]: element-by-element token stream diffing
//! - [`run_c_tokenizer`] / [`run_rust_scanner`]: harness functions for each implementation
//! - [`compare_input`]: end-to-end comparison of raw YAML input

pub mod compare;
pub mod harness;
pub mod snapshot;

pub use compare::{CompareResult, compare_token_streams};
pub use harness::{compare_input, run_c_tokenizer, run_rust_scanner};
pub use snapshot::{SpanSnapshot, TokenSnapshot};
