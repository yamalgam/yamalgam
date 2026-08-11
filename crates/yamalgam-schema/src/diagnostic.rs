//! Structured schema-validation reports.

use std::cmp::Ordering;

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::{JsonType, ValidationError};
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
    let (expected, actual) = expected_actual(error);
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
        expected,
        actual,
        labels: Vec::new(),
    }
}

pub fn representation_diagnostic(
    error: &RepresentationError,
    located: Option<&LocatedValue>,
) -> ValidationDiagnostic {
    let (code, keyword, message, expected, actual) = match &error.kind {
        RepresentationErrorKind::ComplexMappingKey => (
            "yaml/complex-key",
            "complex-key",
            "complex YAML mapping keys cannot be represented as JSON Schema properties".to_owned(),
            Some("scalar mapping key".to_owned()),
            Some("sequence or mapping".to_owned()),
        ),
        RepresentationErrorKind::DepthLimitExceeded { maximum, observed } => (
            "yaml/depth-limit",
            "depth-limit",
            format!("YAML instance depth {observed} exceeds the limit of {maximum}"),
            Some(format!("depth at most {maximum}")),
            Some(format!("depth {observed}")),
        ),
        RepresentationErrorKind::NonFiniteNumber => (
            "yaml/non-finite-number",
            "non-finite-number",
            "non-finite YAML numbers cannot be validated as JSON values".to_owned(),
            Some("finite number".to_owned()),
            Some("non-finite number".to_owned()),
        ),
        RepresentationErrorKind::ProjectedKeyCollision { name } => (
            "yaml/projected-key-collision",
            "projected-key-collision",
            format!("multiple YAML keys project to property name {name:?}"),
            Some("unique projected property name".to_owned()),
            Some(compact_text(name.clone())),
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
        expected,
        actual,
        span,
        labels,
    }
}

pub fn retain_diagnostic(
    diagnostics: &mut Vec<ValidationDiagnostic>,
    diagnostic: ValidationDiagnostic,
    maximum: usize,
) {
    if maximum == 0 {
        return;
    }
    let index = diagnostics
        .partition_point(|existing| diagnostic_order(existing, &diagnostic) != Ordering::Greater);
    if diagnostics.len() < maximum {
        diagnostics.insert(index, diagnostic);
    } else if index < maximum {
        diagnostics.insert(index, diagnostic);
        diagnostics.pop();
    }
}

pub fn diagnostic_limit_warning(maximum: usize, observed: usize) -> ValidationDiagnostic {
    ValidationDiagnostic {
        code: "schema/diagnostic-limit".to_owned(),
        severity: Severity::Warning,
        message: format!("validation produced {observed} diagnostics; showing the first {maximum}"),
        keyword: "diagnostic-limit".to_owned(),
        instance_path: String::new(),
        schema_path: String::new(),
        expected: Some(format!("at most {maximum} diagnostics")),
        actual: Some(observed.to_string()),
        span: None,
        labels: Vec::new(),
    }
}

fn diagnostic_order(left: &ValidationDiagnostic, right: &ValidationDiagnostic) -> Ordering {
    (&left.instance_path, &left.schema_path, &left.code).cmp(&(
        &right.instance_path,
        &right.schema_path,
        &right.code,
    ))
}

fn expected_actual(error: &ValidationError<'_>) -> (Option<String>, Option<String>) {
    let actual_type = || Some(json_type(error.instance()).to_owned());
    match error.kind() {
        ValidationErrorKind::AdditionalItems { limit } => {
            (Some(format!("at most {limit} items")), actual_type())
        }
        ValidationErrorKind::AdditionalProperties { unexpected }
        | ValidationErrorKind::UnevaluatedProperties { unexpected } => (
            Some("no additional properties".to_owned()),
            Some(compact_json(&serde_json::json!(unexpected))),
        ),
        ValidationErrorKind::Constant { expected_value } => {
            (Some(compact_json(expected_value)), actual_type())
        }
        ValidationErrorKind::Enum { options } => (Some(compact_json(options)), actual_type()),
        ValidationErrorKind::ExclusiveMaximum { limit }
        | ValidationErrorKind::ExclusiveMinimum { limit }
        | ValidationErrorKind::Maximum { limit }
        | ValidationErrorKind::Minimum { limit } => (Some(compact_json(limit)), actual_type()),
        ValidationErrorKind::Format { format } => (Some(format.clone()), actual_type()),
        ValidationErrorKind::MaxItems { limit }
        | ValidationErrorKind::MinItems { limit }
        | ValidationErrorKind::MaxLength { limit }
        | ValidationErrorKind::MinLength { limit }
        | ValidationErrorKind::MaxProperties { limit }
        | ValidationErrorKind::MinProperties { limit } => (Some(limit.to_string()), actual_type()),
        ValidationErrorKind::MultipleOf { multiple_of } => {
            (Some(multiple_of.to_string()), actual_type())
        }
        ValidationErrorKind::Pattern { pattern } => (Some(pattern.clone()), actual_type()),
        ValidationErrorKind::Required { property } => (
            Some(format!("property {}", compact_json(property))),
            Some("missing".to_owned()),
        ),
        ValidationErrorKind::Type { kind } => (Some(type_summary(kind)), actual_type()),
        ValidationErrorKind::Referencing(_) => (Some("resolvable reference".to_owned()), None),
        ValidationErrorKind::UnevaluatedItems { unexpected } => (
            Some("no unevaluated items".to_owned()),
            Some(compact_json(&serde_json::json!(unexpected))),
        ),
        _ => (None, actual_type()),
    }
}

fn type_summary(kind: &TypeKind) -> String {
    match kind {
        TypeKind::Single(kind) => kind.to_string(),
        TypeKind::Multiple(kinds) => kinds
            .iter()
            .map(JsonType::as_str)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(number)
            if number.is_i64()
                || number.is_u64()
                || number.as_f64().is_some_and(|value| value.fract() == 0.0) =>
        {
            "integer"
        }
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn compact_json(value: &serde_json::Value) -> String {
    compact_text(value.to_string())
}

fn compact_text(mut text: String) -> String {
    const MAX_BYTES: usize = 160;
    if text.len() <= MAX_BYTES {
        return text;
    }
    let mut end = MAX_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push('…');
    text
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
