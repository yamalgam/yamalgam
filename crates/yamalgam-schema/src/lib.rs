//! Native YAML adaptation for JSON Schema validation.

// r[impl validate]
// r[impl phases.validation]

#![deny(unsafe_code)]

#[cfg(test)]
std::thread_local! {
    static SCHEMA_DIAGNOSTIC_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn increment_schema_diagnostic_calls() {
    SCHEMA_DIAGNOSTIC_CALLS.set(SCHEMA_DIAGNOSTIC_CALLS.get() + 1);
}

#[cfg(test)]
fn reset_schema_diagnostic_calls() {
    SCHEMA_DIAGNOSTIC_CALLS.set(0);
}

#[cfg(test)]
fn schema_diagnostic_calls() -> usize {
    SCHEMA_DIAGNOSTIC_CALLS.get()
}

pub(crate) mod diagnostic;
mod navigation;
mod preflight;
mod retrieval;
pub(crate) mod schema;
mod selection;
mod value;

pub use diagnostic::{ValidationDiagnostic, ValidationReport};
pub use navigation::{PropertyCandidate, SchemaContext, SchemaLocation};
pub use retrieval::{SchemaDocument, SchemaRetrieveError, SchemaSession};
pub use schema::{SchemaError, SchemaOptions, SchemaValidator};
pub use selection::{
    SchemaAssociation, SchemaCatalog, SchemaCatalogEntry, SchemaCatalogError, SchemaSelectionError,
    SchemaSelector, SchemaSource, SelectedSchema,
};
