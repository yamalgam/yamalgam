//! Native YAML adaptation for JSON Schema validation.

#![deny(unsafe_code)]

// Task 3 wires the private representation into the public validator API. Keep
// Task 1 independently verifiable without exposing implementation types.
#[allow(dead_code, reason = "the validator consumes this adapter in Task 3")]
mod preflight;
#[allow(dead_code, reason = "the validator consumes this adapter in Task 3")]
mod value;
