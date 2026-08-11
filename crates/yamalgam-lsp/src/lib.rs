//! Language Server Protocol frontend for Yamalgam.

mod config;
mod diagnostics;
mod document;
mod server;

pub use config::ServerConfig;
pub use diagnostics::{SchemaWorkspace, document_diagnostics};
pub use document::{DocumentError, DocumentState};
pub use server::{ServerError, run, run_stdio};
