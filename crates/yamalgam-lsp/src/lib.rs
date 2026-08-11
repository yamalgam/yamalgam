//! Language Server Protocol frontend for Yamalgam.

pub mod completion;
mod config;
pub mod diagnostics;
mod document;
mod hover;
mod server;

pub use completion::complete;
pub use config::ServerConfig;
pub use diagnostics::{SchemaWorkspace, document_diagnostics};
pub use document::{DocumentError, DocumentState};
pub use hover::hover;
pub use server::{ServerError, run, run_stdio};
