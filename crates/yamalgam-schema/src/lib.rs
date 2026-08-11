//! Native YAML adaptation for JSON Schema validation.

#![deny(unsafe_code)]

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
