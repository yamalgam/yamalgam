//! Conversion of recovering parser and schema reports to LSP diagnostics.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard};

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use serde_json::{Value as JsonValue, json};
use yamalgam_core::{LoaderConfig, Severity, Span};
use yamalgam_resolve::{ResourceId, ResourceRetriever};
use yamalgam_schema::{
    SchemaError, SchemaOptions, SchemaSession, SchemaValidator, SelectedSchema,
    ValidationDiagnostic,
};

use crate::{DocumentState, ServerConfig};

/// Cached schema loading and compilation shared by editor features.
pub struct SchemaWorkspace {
    session: SchemaSession,
    validators: Mutex<HashMap<(String, u64), Arc<SchemaValidator>>>,
}

impl SchemaWorkspace {
    /// Create a workspace over one policy-bound resource retriever.
    #[must_use]
    pub fn new(retriever: Arc<dyn ResourceRetriever>, config: LoaderConfig) -> Self {
        Self {
            session: SchemaSession::new(retriever, config),
            validators: Mutex::new(HashMap::new()),
        }
    }

    /// Create a workspace with an empty in-memory resource backend.
    #[must_use]
    pub fn empty(config: LoaderConfig) -> Self {
        Self::new(
            Arc::new(yamalgam_resolve::MemoryRetriever::default()),
            config,
        )
    }

    /// Return the compiled validator for a selected schema.
    ///
    /// # Errors
    ///
    /// Returns retrieval, policy, conversion, or schema compilation failures.
    pub fn validator(
        &self,
        selected: &SelectedSchema,
        max_diagnostics: usize,
    ) -> Result<Arc<SchemaValidator>, SchemaError> {
        let id = schema_resource_id(&selected.uri)?;
        let document = self.session.load(&id)?;
        let digest = semantic_digest(&document.document.value);
        let key = (document.id.to_string(), digest);
        if let Some(validator) = self.lock_validators().get(&key) {
            return Ok(Arc::clone(validator));
        }

        let validator = Arc::new(self.session.compile_with_options(
            document,
            SchemaOptions {
                max_diagnostics,
                ..SchemaOptions::default()
            },
        )?);
        self.lock_validators().insert(key, Arc::clone(&validator));
        Ok(validator)
    }

    fn lock_validators(&self) -> MutexGuard<'_, HashMap<(String, u64), Arc<SchemaValidator>>> {
        self.validators
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Build syntax and schema diagnostics for the current document snapshot.
#[must_use]
pub fn document_diagnostics(
    document: &DocumentState,
    schemas: &SchemaWorkspace,
    config: &ServerConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for error in document.syntax.syntax().diagnostics() {
        if diagnostics.len() >= config.max_diagnostics {
            break;
        }
        let span = error.span();
        let offset = span.map_or(0, |span| span.start.offset);
        diagnostics.push(Diagnostic {
            range: span_range(document, span),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("yaml/syntax".to_owned())),
            source: Some("yamalgam".to_owned()),
            message: error.to_string(),
            data: Some(json!({
                "documentIndex": document_index_at(document, offset),
                "instancePath": "",
                "schemaPath": "",
                "schemaUri": JsonValue::Null,
            })),
            ..Diagnostic::default()
        });
    }

    if diagnostics.len() >= config.max_diagnostics {
        return diagnostics;
    }
    if let Some(message) = document.schema_selection_error() {
        diagnostics.push(lsp_diagnostic(
            document,
            0,
            "schema/selection",
            DiagnosticSeverity::ERROR,
            message,
            None,
            "",
            "",
            None,
        ));
        return diagnostics;
    }

    for (document_index, instance) in document.located.iter().enumerate() {
        if diagnostics.len() >= config.max_diagnostics {
            break;
        }
        let Some(selected) = document.schema_for_document(document_index) else {
            continue;
        };
        let remaining = config.max_diagnostics.saturating_sub(diagnostics.len());
        match schemas.validator(selected, remaining) {
            Ok(validator) => {
                for diagnostic in validator.validate_located(instance).diagnostics {
                    diagnostics.push(validation_diagnostic(
                        document,
                        document_index,
                        selected,
                        diagnostic,
                    ));
                }
            }
            Err(error) => diagnostics.push(lsp_diagnostic(
                document,
                document_index,
                schema_error_code(&error),
                DiagnosticSeverity::ERROR,
                schema_error_message(&error),
                None,
                "",
                "",
                Some(redacted_schema_uri(&selected.uri)),
            )),
        }
    }
    diagnostics.truncate(config.max_diagnostics);
    diagnostics
}

fn validation_diagnostic(
    document: &DocumentState,
    document_index: usize,
    selected: &SelectedSchema,
    diagnostic: ValidationDiagnostic,
) -> Diagnostic {
    let severity = severity(diagnostic.severity);
    lsp_diagnostic(
        document,
        document_index,
        &diagnostic.code,
        severity,
        diagnostic.message,
        diagnostic.span,
        &diagnostic.instance_path,
        &diagnostic.schema_path,
        Some(redacted_schema_uri(&selected.uri)),
    )
}

#[allow(clippy::too_many_arguments)]
fn lsp_diagnostic(
    document: &DocumentState,
    document_index: usize,
    code: &str,
    severity: DiagnosticSeverity,
    message: impl Into<String>,
    span: Option<Span>,
    instance_path: &str,
    schema_path: &str,
    schema_uri: Option<String>,
) -> Diagnostic {
    Diagnostic {
        range: span_range(document, span),
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_owned())),
        source: Some("yamalgam".to_owned()),
        message: message.into(),
        data: Some(json!({
            "documentIndex": document_index,
            "instancePath": instance_path,
            "schemaPath": schema_path,
            "schemaUri": schema_uri,
        })),
        ..Diagnostic::default()
    }
}

fn span_range(document: &DocumentState, span: Option<Span>) -> Range {
    let span = span.unwrap_or_default();
    Range {
        start: byte_position(document, span.start.offset),
        end: byte_position(document, span.end.offset),
    }
}

fn byte_position(document: &DocumentState, offset: usize) -> Position {
    document
        .syntax
        .byte_offset_to_utf16(offset.min(document.syntax.text().len()))
        .map_or_else(
            || Position::new(0, 0),
            |(line, column)| Position::new(line, column),
        )
}

fn document_index_at(document: &DocumentState, offset: usize) -> usize {
    document.document_index_at_offset(offset)
}

const fn severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Hint => DiagnosticSeverity::HINT,
    }
}

fn schema_resource_id(uri: &str) -> Result<ResourceId, SchemaError> {
    let fragmentless = uri.split_once('#').map_or(uri, |(base, _)| base);
    ResourceId::new(fragmentless).map_err(|source| {
        yamalgam_schema::SchemaRetrieveError::InvalidReference {
            base: ResourceId::anonymous(),
            source,
        }
        .into()
    })
}

fn semantic_digest(value: &yamalgam_core::Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_value(value, &mut hasher);
    hasher.finish()
}

fn hash_value(value: &yamalgam_core::Value, hasher: &mut impl Hasher) {
    std::mem::discriminant(value).hash(hasher);
    match value {
        yamalgam_core::Value::Null => {}
        yamalgam_core::Value::Bool(value) => value.hash(hasher),
        yamalgam_core::Value::Integer(value) => value.hash(hasher),
        yamalgam_core::Value::Float(value) => value.to_bits().hash(hasher),
        yamalgam_core::Value::String(value) => value.hash(hasher),
        yamalgam_core::Value::Sequence(values) => {
            for value in values {
                hash_value(value, hasher);
            }
        }
        yamalgam_core::Value::Mapping(mapping) => {
            for (key, value) in mapping.iter() {
                hash_value(key, hasher);
                hash_value(value, hasher);
            }
        }
    }
}

pub(super) fn redacted_schema_uri(uri: &str) -> String {
    schema_resource_id(uri).map_or_else(|_| "<invalid-schema-uri>".to_owned(), |id| id.to_string())
}

const fn schema_error_code(error: &SchemaError) -> &'static str {
    match error {
        SchemaError::InvalidSchema { .. }
        | SchemaError::UnsupportedPattern { .. }
        | SchemaError::NonStringKey { .. }
        | SchemaError::NonFiniteNumber { .. } => "schema/invalid",
        SchemaError::Retrieval(_) => "schema/retrieval",
        _ => "schema/invalid",
    }
}

fn schema_error_message(error: &SchemaError) -> String {
    match error {
        SchemaError::Retrieval(_) => "selected schema could not be loaded".to_owned(),
        _ => error.to_string(),
    }
}
