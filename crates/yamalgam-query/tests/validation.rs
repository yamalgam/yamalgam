//! Query-level schema validation contracts.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::json;
use yamalgam_core::{LoaderConfig, ResourceLimits, Severity, Value};
use yamalgam_query::engine::{self, InputMeta, RunCtx};
use yamalgam_query::{
    Meta, QuerySchemaResolver, ValidationFailure, ValueLimitError, YamlVal, YgError,
};
use yamalgam_resolve::{MemoryRetriever, ResourceId};
use yamalgam_schema::{SchemaOptions, SchemaValidator, ValidationDiagnostic, ValidationReport};

#[derive(Default)]
struct RegistryResolver {
    explicit: HashMap<String, Arc<SchemaValidator>>,
    implicit: HashMap<ResourceId, Arc<SchemaValidator>>,
}

impl QuerySchemaResolver for RegistryResolver {
    fn validator(
        &self,
        schema_ref: Option<&str>,
        input: &YamlVal,
    ) -> Result<Arc<SchemaValidator>, ValidationFailure> {
        if let Some(schema_ref) = schema_ref {
            return self
                .explicit
                .get(schema_ref)
                .cloned()
                .ok_or_else(|| failure("schema/missing", "schema was not found"));
        }

        let mut resources = HashSet::new();
        let mut pending = vec![input];
        while let Some(value) = pending.pop() {
            if let Some(resource) = value
                .meta()
                .and_then(|metadata| metadata.origin.as_ref())
                .map(|origin| origin.resource.clone())
            {
                resources.insert(resource);
            }
            if let Some(items) = value.as_sequence() {
                pending.extend(items);
            }
            if let Some(entries) = value.entries() {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
        }
        let mut resources = resources.into_iter();
        let resource = resources
            .next()
            .ok_or_else(|| failure("schema/missing", "input has no selected schema"))?;
        if resources.next().is_some() {
            return Err(failure(
                "schema/ambiguous-source",
                "input contains values from multiple source files",
            ));
        }
        self.implicit
            .get(&resource)
            .cloned()
            .ok_or_else(|| failure("schema/missing", "input has no selected schema"))
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
                span: None,
                labels: Vec::new(),
            }],
        },
    }
}

fn schema(document: serde_json::Value) -> Arc<SchemaValidator> {
    Arc::new(
        SchemaValidator::compile(
            &serde_json::from_value::<Value>(document).unwrap(),
            SchemaOptions::default(),
        )
        .unwrap(),
    )
}

fn evaluate(context: &RunCtx, input: YamlVal, filter: &str) -> Result<Vec<YamlVal>, YgError> {
    context.stage_input_meta(InputMeta::default());
    let filter = engine::compile_filter(filter, &[])?;
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        context,
        std::iter::once(Ok::<_, YgError>(input)),
        |value| {
            output.push(value);
            Ok::<_, YgError>(())
        },
    )?;
    Ok(output)
}

fn loaded_value(source: &str, resource: &str) -> YamlVal {
    yamalgam_query::resolve_document(
        source,
        ResourceId::new(resource).unwrap(),
        MemoryRetriever::default(),
        &LoaderConfig::moderate(),
    )
    .unwrap()
    .value
}

// r[verify validate.filter]
#[test]
fn validation_explicit_filter_returns_the_identical_metadata_bearing_value() {
    let mut resolver = RegistryResolver::default();
    resolver.explicit.insert(
        "memory://schemas/name".to_owned(),
        schema(json!({"type": "string"})),
    );
    let context = RunCtx::new(LoaderConfig::moderate()).with_schema_resolver(Arc::new(resolver));
    let input = YamlVal::str("api").with_meta(Meta {
        anchor: Some("service".into()),
        ..Meta::default()
    });

    let output = evaluate(
        &context,
        input.clone(),
        r#"validate("memory://schemas/name")"#,
    )
    .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], input);
    assert_eq!(output[0].meta(), input.meta());
}

#[test]
fn validation_invalid_input_preserves_the_typed_report() {
    let mut resolver = RegistryResolver::default();
    resolver.explicit.insert(
        "memory://schemas/name".to_owned(),
        schema(json!({"type": "string"})),
    );
    let context = RunCtx::new(LoaderConfig::moderate()).with_schema_resolver(Arc::new(resolver));

    let error = evaluate(
        &context,
        YamlVal::int(7),
        r#"validate("memory://schemas/name")"#,
    )
    .unwrap_err();
    let YgError::RuntimeValidation { failure, .. } = error else {
        panic!("expected typed validation failure, got {error:?}");
    };
    assert!(!failure.report.valid);
    assert_eq!(failure.report.diagnostics[0].code, "schema/type");
    assert_eq!(
        failure.report.diagnostics[0].actual.as_deref(),
        Some("integer")
    );
}

// r[verify validate.filter.inline]
#[test]
fn validation_no_argument_uses_the_inputs_source_selection() {
    let root = ResourceId::new("memory://inputs/service.yaml").unwrap();
    let input = loaded_value("name: api\n", "memory://inputs/service.yaml");
    let mut resolver = RegistryResolver::default();
    resolver
        .implicit
        .insert(root, schema(json!({"required": ["name"]})));
    let context = RunCtx::new(LoaderConfig::moderate()).with_schema_resolver(Arc::new(resolver));

    let output = evaluate(&context, input.clone(), "validate").unwrap();
    assert_eq!(output, vec![input]);
}

#[test]
fn validation_reports_disabled_missing_and_non_string_schema_arguments() {
    let disabled = evaluate(
        &RunCtx::new(LoaderConfig::moderate()),
        YamlVal::null(),
        "validate",
    )
    .unwrap_err();
    assert!(matches!(
        disabled,
        YgError::RuntimeValidation { ref failure, .. }
            if failure.report.diagnostics[0].code == "schema/disabled"
    ));

    let missing = evaluate(
        &RunCtx::new(LoaderConfig::moderate())
            .with_schema_resolver(Arc::new(RegistryResolver::default())),
        YamlVal::null(),
        r#"validate("memory://schemas/missing")"#,
    )
    .unwrap_err();
    assert!(matches!(
        missing,
        YgError::RuntimeValidation { ref failure, .. }
            if failure.report.diagnostics[0].code == "schema/missing"
    ));

    let non_string = evaluate(
        &RunCtx::new(LoaderConfig::moderate())
            .with_schema_resolver(Arc::new(RegistryResolver::default())),
        YamlVal::null(),
        "validate(1)",
    )
    .unwrap_err();
    assert!(matches!(non_string, YgError::Runtime(_)));
}

#[test]
fn validation_rejects_slurped_values_from_different_source_files() {
    let left = loaded_value("name: left\n", "memory://inputs/left.yaml");
    let right = loaded_value("name: right\n", "memory://inputs/right.yaml");
    let context = RunCtx::new(LoaderConfig::moderate())
        .with_schema_resolver(Arc::new(RegistryResolver::default()));

    let error = evaluate(&context, YamlVal::seq(vec![left, right]), "validate").unwrap_err();
    assert!(matches!(
        error,
        YgError::RuntimeValidation { ref failure, .. }
            if failure.report.diagnostics[0].code == "schema/ambiguous-source"
    ));
}

#[test]
fn validation_query_to_core_conversion_is_stack_safe_and_limit_aware() {
    let mut limited = ResourceLimits::none();
    limited.max_depth = Some(2);
    let value = YamlVal::seq(vec![YamlVal::seq(vec![YamlVal::seq(vec![YamlVal::int(
        1,
    )])])]);
    assert!(matches!(
        value.to_core_value(&limited),
        Err(ValueLimitError::Limit(_))
    ));

    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut value = YamlVal::map(vec![
                (YamlVal::str("integer"), YamlVal::int(1)),
                (YamlVal::str("float"), YamlVal::float(1.5)),
            ]);
            for _ in 0..2_000 {
                value = YamlVal::seq(vec![value]);
            }
            let converted = value.to_core_value(&ResourceLimits::none()).unwrap();
            let mut current = &converted;
            for _ in 0..2_000 {
                let Value::Sequence(items) = current else {
                    panic!("nesting and numeric kinds are preserved");
                };
                current = &items[0];
            }
            let Value::Mapping(mapping) = current else {
                panic!("mapping is preserved");
            };
            assert_eq!(
                mapping
                    .iter()
                    .map(|(key, value)| (key.as_str().unwrap(), value))
                    .collect::<Vec<_>>(),
                vec![
                    ("integer", &Value::Integer(1)),
                    ("float", &Value::Float(1.5)),
                ]
            );
            std::mem::forget(converted);
        })
        .unwrap()
        .join()
        .expect("iterative conversion must fit a small thread stack");
}
