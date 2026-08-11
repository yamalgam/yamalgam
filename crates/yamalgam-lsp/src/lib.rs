//! Language Server Protocol frontend for Yamalgam.

mod config;
mod document;
mod server;

pub use config::ServerConfig;
pub use document::{DocumentError, DocumentState};
pub use server::{ServerError, run, run_stdio};
