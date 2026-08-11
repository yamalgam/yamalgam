//! Query-facing schema resolution and structured validation failures.

use std::fmt;
use std::sync::Arc;

use yamalgam_core::{Label, ResourceLimits, Severity, Span};
use yamalgam_schema::{SchemaValidator, ValidationDiagnostic, ValidationReport};

use crate::value::{RuntimeErrorPayload, ValueLimitError, YamlVal};

/// Schema lookup available to the `validate` query filter.
pub trait QuerySchemaResolver: Send + Sync {
    /// Resolve an explicit schema reference or the input's selected schema.
    ///
    /// # Errors
    ///
    /// Returns a complete structured failure when selection, retrieval, or
    /// compilation cannot produce a validator.
    fn validator(
        &self,
        schema_ref: Option<&str>,
        input: &YamlVal,
    ) -> Result<Arc<SchemaValidator>, ValidationFailure>;
}

/// Structured query validation failure retained through jaq execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationFailure {
    /// Complete validation or schema-resolution report.
    pub report: ValidationReport,
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.report.diagnostics.first() {
            Some(diagnostic) => formatter.write_str(&diagnostic.message),
            None => formatter.write_str("schema validation failed"),
        }
    }
}

impl std::error::Error for ValidationFailure {}

pub enum RuntimeValidationError {
    Value(ValueLimitError),
    Validation(ValidationFailure),
}

pub fn validate(
    resolver: Option<&dyn QuerySchemaResolver>,
    limits: &ResourceLimits,
    schema_ref: Option<&str>,
    input: YamlVal,
) -> Result<YamlVal, RuntimeValidationError> {
    let resolver = resolver.ok_or_else(|| {
        RuntimeValidationError::Validation(failure(
            "schema/disabled",
            "schema validation is not configured for this query",
        ))
    })?;
    let validator = resolver
        .validator(schema_ref, &input)
        .map_err(RuntimeValidationError::Validation)?;
    let value = input
        .to_core_value(limits)
        .map_err(RuntimeValidationError::Value)?;
    let report = validator.validate(&value);
    if report.valid {
        Ok(input)
    } else {
        Err(RuntimeValidationError::Validation(ValidationFailure {
            report,
        }))
    }
}

pub fn jaq_error(failure: ValidationFailure) -> jaq_core::Error<YamlVal> {
    let mut value = YamlVal::str(format!("schema validation failed: {failure}"));
    value.runtime_error = Some(Arc::new(RuntimeErrorPayload::Validation(failure)));
    jaq_core::Error::new(value)
}

pub fn take_failure(value: &YamlVal) -> Option<Arc<ValidationFailure>> {
    match value.runtime_error.as_deref()? {
        RuntimeErrorPayload::Validation(failure) => Some(Arc::new(failure.clone())),
        RuntimeErrorPayload::Resolution(_) => None,
    }
}

fn failure(code: &str, message: &str) -> ValidationFailure {
    ValidationFailure {
        report: ValidationReport {
            valid: false,
            diagnostics: vec![ValidationDiagnostic {
                code: code.to_owned(),
                severity: Severity::Error,
                message: message.to_owned(),
                keyword: String::new(),
                instance_path: String::new(),
                schema_path: String::new(),
                expected: None,
                actual: None,
                span: None::<Span>,
                labels: Vec::<Label>::new(),
            }],
        },
    }
}
