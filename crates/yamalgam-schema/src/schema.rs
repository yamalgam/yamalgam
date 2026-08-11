//! Checked schema compilation and native YAML instance validation.

use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use thiserror::Error;
use yamalgam_core::{LocatedValue, Value};

use crate::diagnostic::{
    ValidationReport, diagnostic_limit_warning, representation_diagnostic, retain_diagnostic,
    schema_diagnostic,
};
use crate::navigation::{NavigationIndex, SchemaContext, SchemaLocation};
use crate::preflight::PreparedInstance;
use crate::retrieval::SchemaRetrieveError;
use crate::value::YamlJson;

/// Options controlling schema compilation and report size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaOptions {
    /// Base URI used to resolve relative schema identifiers and references.
    pub base_uri: Option<String>,
    /// Whether the JSON Schema `format` keyword is asserted.
    pub validate_formats: bool,
    /// Maximum number of diagnostics retained in one report.
    pub max_diagnostics: usize,
}

impl Default for SchemaOptions {
    fn default() -> Self {
        Self {
            base_uri: None,
            validate_formats: false,
            max_diagnostics: 100,
        }
    }
}

/// Failure to convert or compile a JSON Schema document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchemaError {
    /// A schema mapping used a non-string key.
    #[error("schema mapping key at {path} is not a string")]
    NonStringKey {
        /// Path to the invalid key position.
        path: String,
    },
    /// A schema contained NaN or infinity.
    #[error("schema number at {path} is not finite")]
    NonFiniteNumber {
        /// Path to the invalid number.
        path: String,
    },
    /// The schema does not satisfy the Draft 2020-12 meta-schema.
    #[error("invalid schema at {schema_path}: {message}")]
    InvalidSchema {
        /// Upstream compilation failure.
        message: String,
        /// JSON Pointer to the invalid schema node.
        schema_path: String,
    },
    /// A pattern uses syntax unsupported by the linear-time regex engine.
    #[error("unsupported schema pattern {pattern:?}: {message}")]
    UnsupportedPattern {
        /// Rejected regular expression.
        pattern: String,
        /// Regex compilation failure.
        message: String,
    },
    /// External schema retrieval failed.
    #[error(transparent)]
    Retrieval(#[from] SchemaRetrieveError),
}

/// A compiled Draft 2020-12 validator over native Yamalgam values.
pub struct SchemaValidator {
    schema: JsonValue,
    validator: jsonschema::Validator<YamlJson>,
    options: SchemaOptions,
    navigation: NavigationIndex,
}

impl SchemaValidator {
    /// Compile one semantic YAML value as a Draft 2020-12 schema.
    pub fn compile(schema: &Value, options: SchemaOptions) -> Result<Self, SchemaError> {
        let schema = schema_value_to_json(schema)?;
        let navigation = NavigationIndex::single(schema.clone(), None, options.base_uri.as_deref());
        let builder = validator_options(&options);
        let validator = builder
            .build(&schema)
            .map_err(|error| compile_error(&schema, &error))?;
        Ok(Self {
            schema,
            validator,
            options,
            navigation,
        })
    }

    /// Compile a located schema document.
    pub fn compile_located(
        schema: &LocatedValue,
        options: SchemaOptions,
    ) -> Result<Self, SchemaError> {
        let json = schema_value_to_json(&schema.value)?;
        let navigation = NavigationIndex::single(
            json.clone(),
            Some(schema.clone()),
            options.base_uri.as_deref(),
        );
        let validator = validator_options(&options)
            .build(&json)
            .map_err(|error| compile_error(&json, &error))?;
        Ok(Self {
            schema: json,
            validator,
            options,
            navigation,
        })
    }

    pub(crate) fn compile_json_with_retriever(
        schema: JsonValue,
        options: SchemaOptions,
        retriever: impl jsonschema::Retrieve + 'static,
        navigation: NavigationIndex,
    ) -> Result<Self, SchemaError> {
        let validator = validator_options(&options)
            .with_retriever(retriever)
            .build(&schema)
            .map_err(|error| compile_error(&schema, &error))?;
        Ok(Self {
            schema,
            validator,
            options,
            navigation,
        })
    }

    /// Validate an unlocated semantic YAML value.
    #[must_use]
    pub fn validate(&self, instance: &Value) -> ValidationReport {
        self.validate_inner(instance, None)
    }

    /// Validate a semantic YAML value and attach source spans to diagnostics.
    #[must_use]
    pub fn validate_located(&self, instance: &LocatedValue) -> ValidationReport {
        self.validate_inner(&instance.value, Some(instance))
    }

    /// Returns the checked JSON-compatible schema document.
    #[must_use]
    pub const fn schema(&self) -> &JsonValue {
        &self.schema
    }

    /// Return schema annotations and property candidates applicable at an
    /// RFC 6901 instance path.
    #[must_use]
    pub fn context_at(&self, instance_path: &str) -> SchemaContext {
        self.navigation.context_at(instance_path)
    }

    /// Resolve a schema reference to its indexed resource, pointer, and span.
    #[must_use]
    pub fn definition_for_ref(&self, schema_uri: &str, reference: &str) -> Option<SchemaLocation> {
        self.navigation.definition_for_ref(schema_uri, reference)
    }

    fn validate_inner(&self, instance: &Value, located: Option<&LocatedValue>) -> ValidationReport {
        let prepared = match PreparedInstance::new(instance) {
            Ok(prepared) => prepared,
            Err(errors) => {
                let mut diagnostics =
                    Vec::with_capacity(errors.len().min(self.options.max_diagnostics));
                for error in &errors {
                    retain_diagnostic(
                        &mut diagnostics,
                        representation_diagnostic(error, located),
                        self.options.max_diagnostics,
                    );
                }
                if errors.len() > self.options.max_diagnostics {
                    diagnostics.push(diagnostic_limit_warning(
                        self.options.max_diagnostics,
                        errors.len(),
                    ));
                }
                return ValidationReport {
                    valid: false,
                    diagnostics,
                };
            }
        };

        let mut observed = 0_usize;
        let mut diagnostics = Vec::new();
        for error in self.validator.iter_errors(prepared.root()) {
            observed = observed.saturating_add(1);
            retain_diagnostic(
                &mut diagnostics,
                schema_diagnostic(&error, located),
                self.options.max_diagnostics,
            );
        }
        if observed > self.options.max_diagnostics {
            diagnostics.push(diagnostic_limit_warning(
                self.options.max_diagnostics,
                observed,
            ));
        }
        ValidationReport {
            valid: observed == 0,
            diagnostics,
        }
    }
}

pub fn schema_value_to_json(schema: &Value) -> Result<JsonValue, SchemaError> {
    enum ArenaNode {
        Scalar(JsonValue),
        Sequence(Vec<usize>),
        Mapping(Vec<(String, usize)>),
    }

    let mut arena = vec![None];
    let mut pending = vec![(schema, String::new(), 0_usize)];
    while let Some((value, path, index)) = pending.pop() {
        let node = match value {
            Value::Null => ArenaNode::Scalar(JsonValue::Null),
            Value::Bool(value) => ArenaNode::Scalar(JsonValue::Bool(*value)),
            Value::Integer(value) => ArenaNode::Scalar(JsonValue::Number(JsonNumber::from(*value))),
            Value::Float(value) => {
                let number = JsonNumber::from_f64(*value)
                    .ok_or_else(|| SchemaError::NonFiniteNumber { path: path.clone() })?;
                ArenaNode::Scalar(JsonValue::Number(number))
            }
            Value::String(value) => ArenaNode::Scalar(JsonValue::String(value.clone())),
            Value::Sequence(items) => {
                let mut children = Vec::with_capacity(items.len());
                let mut child_work = Vec::with_capacity(items.len());
                for (item_index, item) in items.iter().enumerate() {
                    let child_index = arena.len();
                    arena.push(None);
                    children.push(child_index);
                    child_work.push((
                        item,
                        append_path(&path, &item_index.to_string()),
                        child_index,
                    ));
                }
                pending.extend(child_work.into_iter().rev());
                ArenaNode::Sequence(children)
            }
            Value::Mapping(mapping) => {
                let mut children = Vec::with_capacity(mapping.len());
                let mut child_work = Vec::with_capacity(mapping.len());
                for (entry_index, (key, value)) in mapping.iter().enumerate() {
                    let Some(key) = key.as_str() else {
                        return Err(SchemaError::NonStringKey {
                            path: append_path(&path, &format!("<key:{entry_index}>")),
                        });
                    };
                    let child_index = arena.len();
                    arena.push(None);
                    children.push((key.to_owned(), child_index));
                    child_work.push((value, append_path(&path, key), child_index));
                }
                pending.extend(child_work.into_iter().rev());
                ArenaNode::Mapping(children)
            }
        };
        arena[index] = Some(node);
    }

    let mut converted = (0..arena.len()).map(|_| None).collect::<Vec<_>>();
    for index in (0..arena.len()).rev() {
        let value = match arena[index].take().expect("every arena node is visited") {
            ArenaNode::Scalar(value) => value,
            ArenaNode::Sequence(children) => JsonValue::Array(
                children
                    .into_iter()
                    .map(|child| {
                        converted[child]
                            .take()
                            .expect("child converted before parent")
                    })
                    .collect(),
            ),
            ArenaNode::Mapping(children) => JsonValue::Object(
                children
                    .into_iter()
                    .map(|(key, child)| {
                        (
                            key,
                            converted[child]
                                .take()
                                .expect("child converted before parent"),
                        )
                    })
                    .collect::<JsonMap<_, _>>(),
            ),
        };
        converted[index] = Some(value);
    }
    Ok(converted[0].take().expect("root schema converted"))
}

fn validator_options(
    options: &SchemaOptions,
) -> jsonschema::ValidationOptions<'static, std::sync::Arc<dyn jsonschema::Retrieve>, YamlJson> {
    let mut builder = jsonschema::options_for::<YamlJson>()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(options.validate_formats)
        .with_pattern_options(jsonschema::PatternOptions::regex());
    if let Some(base_uri) = &options.base_uri {
        builder = builder.with_base_uri(base_uri);
    }
    builder
}

fn compile_error(schema: &JsonValue, error: &jsonschema::ValidationError<'_>) -> SchemaError {
    let message = error.to_string();
    let schema_path = error.schema_path().to_string();
    if let Some(pattern) = schema.pointer(&schema_path).and_then(JsonValue::as_str)
        && (schema_path.ends_with("/pattern") || error.kind().keyword() == "pattern")
    {
        return SchemaError::UnsupportedPattern {
            pattern: pattern.to_owned(),
            message,
        };
    }
    SchemaError::InvalidSchema {
        message,
        schema_path,
    }
}

fn append_path(path: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{path}/{escaped}")
}
