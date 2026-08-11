//! Native YAML adaptation for JSON Schema validation.

#![deny(unsafe_code)]

pub(crate) mod diagnostic;
mod preflight;
mod schema;
mod value;

pub use diagnostic::{ValidationDiagnostic, ValidationReport};
pub use schema::{SchemaError, SchemaOptions, SchemaRetrieveError, SchemaValidator};
