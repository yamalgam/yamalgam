//! Structured schema-validation reports.

use jsonschema::ValidationError;
use serde::Serialize;
use yamalgam_core::{Label, LocatedValue, LocationKind, Severity, Span, Value};

use crate::preflight::{InstanceStep, RepresentationError, RepresentationErrorKind};

/// One machine-readable validation failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationDiagnostic {
    /// Stable Yamalgam diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Human-readable failure summary.
    pub message: String,
    /// JSON Schema keyword, or YAML representation rule, that failed.
    pub keyword: String,
    /// RFC 6901 path to the failing instance value.
    pub instance_path: String,
    /// RFC 6901 path to the failing schema keyword.
    pub schema_path: String,
    /// Compact expected-value summary when available.
    pub expected: Option<String>,
    /// Compact actual-value summary when available.
    pub actual: Option<String>,
    /// Primary YAML source span when locations are available.
    pub span: Option<Span>,
    /// Related YAML source locations.
    pub labels: Vec<Label>,
}

/// Complete result of validating one YAML document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    /// Whether the instance satisfies both YAML representation rules and the schema.
    pub valid: bool,
    /// Validation failures, capped by [`crate::SchemaOptions::max_diagnostics`].
    pub diagnostics: Vec<ValidationDiagnostic>,
}

pub fn schema_diagnostic(
    error: &ValidationError<'_>,
    located: Option<&LocatedValue>,
) -> ValidationDiagnostic {
    let keyword = error.kind().keyword();
    let instance_path = error.instance_path().to_string();
    ValidationDiagnostic {
        code: if keyword == "$ref" {
            "schema/ref".to_owned()
        } else {
            format!("schema/{keyword}")
        },
        severity: Severity::Error,
        message: error.masked().to_string(),
        keyword: keyword.to_owned(),
        schema_path: error.schema_path().to_string(),
        span: located.and_then(|value| value.span_at_pointer(&instance_path)),
        instance_path,
        expected: None,
        actual: None,
        labels: Vec::new(),
    }
}

pub fn representation_diagnostic(
    error: &RepresentationError,
    located: Option<&LocatedValue>,
) -> ValidationDiagnostic {
    let (code, keyword, message) = match &error.kind {
        RepresentationErrorKind::ComplexMappingKey => (
            "yaml/complex-key",
            "complex-key",
            "complex YAML mapping keys cannot be represented as JSON Schema properties".to_owned(),
        ),
        RepresentationErrorKind::NonFiniteNumber => (
            "yaml/non-finite-number",
            "non-finite-number",
            "non-finite YAML numbers cannot be validated as JSON values".to_owned(),
        ),
        RepresentationErrorKind::ProjectedKeyCollision { name } => (
            "yaml/projected-key-collision",
            "projected-key-collision",
            format!("multiple YAML keys project to property name {name:?}"),
        ),
    };
    let span = located.and_then(|value| span_for_route(value, &error.route));
    let labels = error
        .conflicting_route
        .as_ref()
        .and_then(|route| located.and_then(|value| span_for_route(value, route)))
        .map(|span| {
            vec![Label {
                span,
                message: "first key with this projected property name".to_owned(),
            }]
        })
        .unwrap_or_default();

    ValidationDiagnostic {
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        keyword: keyword.to_owned(),
        instance_path: error.instance_path.clone(),
        schema_path: String::new(),
        expected: None,
        actual: None,
        span,
        labels,
    }
}

fn span_for_route(located: &LocatedValue, route: &[InstanceStep]) -> Option<Span> {
    let mut value = &located.value;
    let mut locations = &located.locations;
    for step in route {
        match (step, value, &locations.kind) {
            (
                InstanceStep::Sequence(index),
                Value::Sequence(items),
                LocationKind::Sequence(children),
            ) => {
                value = items.get(*index)?;
                locations = children.get(*index)?;
            }
            (
                InstanceStep::MappingKey(index),
                Value::Mapping(mapping),
                LocationKind::Mapping(entries),
            ) => {
                value = mapping.iter().nth(*index)?.0;
                locations = &entries.get(*index)?.key;
            }
            (
                InstanceStep::MappingValue(index),
                Value::Mapping(mapping),
                LocationKind::Mapping(entries),
            ) => {
                value = mapping.iter().nth(*index)?.1;
                locations = &entries.get(*index)?.value;
            }
            _ => return None,
        }
    }
    Some(locations.span)
}
