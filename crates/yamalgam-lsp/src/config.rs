//! Language-server configuration independent of transport state.

use std::path::PathBuf;

use yamalgam_schema::SchemaAssociation;

/// Runtime language-server configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    /// User-configured file-pattern schema associations.
    pub schema_associations: Vec<SchemaAssociation>,
    /// Whether SchemaStore catalog matching is enabled.
    pub schema_store_enabled: bool,
    /// SchemaStore catalog URI.
    pub schema_store_url: String,
    /// Maximum diagnostics published for one document.
    pub max_diagnostics: usize,
    /// Workspace roots supplied by the client.
    pub workspace_roots: Vec<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            schema_associations: Vec::new(),
            schema_store_enabled: true,
            schema_store_url: "https://www.schemastore.org/api/json/catalog.json".to_string(),
            max_diagnostics: 64,
            workspace_roots: Vec::new(),
        }
    }
}
