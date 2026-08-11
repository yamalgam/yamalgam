//! Native YAML adaptation for JSON Schema validation.

#![deny(unsafe_code)]

pub(crate) mod diagnostic;
mod preflight;
mod retrieval;
pub(crate) mod schema;
mod value;

pub use diagnostic::{ValidationDiagnostic, ValidationReport};
pub use retrieval::{SchemaDocument, SchemaRetrieveError, SchemaSession};
pub use schema::{SchemaError, SchemaOptions, SchemaValidator};
