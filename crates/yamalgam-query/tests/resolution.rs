//! External resolution in the YAML-native query value model.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};

use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_parser::{ResolutionErrorCategory, ResolveError};
use yamalgam_query::EmitOpts;
use yamalgam_query::emit::to_yaml;
use yamalgam_query::engine::{self, InputMeta, RunCtx};
use yamalgam_query::{
    Meta, ResolvedInput, YamlVal, YgError, resolution_handle, resolve_document, resolve_documents,
};
use yamalgam_resolve::{
    DiscoveryRequest, ResourceId, ResourceProvenance, ResourceRequest, ResourceRetriever,
    RetrieveError, RetrievedResource,
};

#[derive(Clone, Default)]
struct CountingRetriever {
    state: Arc<Mutex<RetrieverState>>,
}

#[derive(Default)]
struct RetrieverState {
    resources: HashMap<ResourceId, RetrievedResource>,
    calls: Vec<ResourceId>,
}

impl CountingRetriever {
    fn insert(&self, id: ResourceId, input: impl Into<Vec<u8>>) {
        self.state.lock().unwrap().resources.insert(
            id.clone(),
            RetrievedResource {
                id,
                bytes: input.into(),
                provenance: ResourceProvenance::default(),
            },
        );
    }

    fn calls(&self) -> Vec<ResourceId> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl ResourceRetriever for CountingRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(request.id.clone());
        state
            .resources
            .get(&request.id)
            .cloned()
            .ok_or_else(|| RetrieveError::NotFound(request.id.clone()))
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
    }
}

fn id(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn explicit_config() -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.include.mode = ResolutionMode::Explicit;
    config.resolution.refs.mode = ResolutionMode::Explicit;
    config.resolution.refs.allow_schemes = vec!["memory".into()];
    config
}

fn automatic_config() -> LoaderConfig {
    let mut config = explicit_config();
    config.resolution.include.mode = ResolutionMode::Automatic;
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config
}

fn evaluate(context: &RunCtx, input: ResolvedInput, filter: &str) -> Result<Vec<YamlVal>, YgError> {
    let (value, key) = context.register_resolved_input(input)?;
    context.stage_input_meta(InputMeta::default());
    context.stage_input_resolution(Some(key));
    let filter = engine::compile_filter(filter, &[])?;
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        context,
        std::iter::once(Ok::<_, YgError>(value)),
        |value| {
            output.push(value);
            Ok::<_, YgError>(())
        },
    )?;
    Ok(output)
}

fn assert_origin(value: &YamlVal, resource: &ResourceId) {
    assert_eq!(
        value
            .meta()
            .and_then(|metadata| metadata.origin.as_ref())
            .map(|origin| &origin.resource),
        Some(resource)
    );
}

#[test]
fn resolution_adapter_preserves_yaml_types_presentation_aliases_and_origins() {
    let root = id("memory://fixtures/root.yaml");
    let input = concat!(
        "'null': null\n",
        "'bool': true\n",
        "'int': 42\n",
        "'float': 3.5\n",
        "'string': 'true'\n",
        "'tagged': !custom value\n",
        "'literal': |+\n  line\n\n",
        "'flow': &items [one, two]\n",
        "'alias': *items\n",
        "? [complex, key]\n: {nested: value}\n",
    );

    let resolved = resolve_document(
        input,
        root.clone(),
        CountingRetriever::default(),
        &explicit_config(),
    )
    .unwrap();
    let value = &resolved.value;

    assert_eq!(value.get_str("null").unwrap().type_name(), "null");
    assert_eq!(value.get_str("bool").unwrap().as_boolean(), Some(true));
    assert_eq!(value.get_str("int").unwrap().as_i64(), Some(42));
    assert_eq!(value.get_str("float").unwrap().as_f64(), Some(3.5));
    assert_eq!(
        value.get_str("string").unwrap().as_str_content(),
        Some("true")
    );
    assert_eq!(
        value.get_str("tagged").unwrap().tag_shorthand().as_deref(),
        Some("!custom")
    );
    assert_eq!(
        value.get_str("literal").unwrap().style_name(),
        Some("literal")
    );
    assert_eq!(
        value
            .get_str("alias")
            .unwrap()
            .meta()
            .and_then(|metadata| metadata.alias_of.as_deref()),
        Some("items")
    );
    assert!(
        value
            .entries()
            .unwrap()
            .any(|(key, _)| key.as_sequence().is_some())
    );
    assert_origin(value, &root);
    assert_origin(value.get_str("literal").unwrap(), &root);
}

// r[verify resolution.modes]
#[test]
fn automatic_resolution_precedes_query_and_keeps_imported_resource_origins() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let retriever = CountingRetriever::default();
    retriever.insert(shared.clone(), "nested: {answer: 42}\n");

    let resolved = resolve_document(
        "local: here\nimported:\n  $ref: ./shared.yaml#/nested\n",
        root.clone(),
        retriever,
        &automatic_config(),
    )
    .unwrap();

    assert_eq!(
        resolved
            .value
            .get_str("imported")
            .and_then(|value| value.get_str("answer"))
            .and_then(YamlVal::as_i64),
        Some(42)
    );
    assert_origin(resolved.value.get_str("local").unwrap(), &root);
    assert_origin(resolved.value.get_str("imported").unwrap(), &shared);
    assert_origin(
        resolved
            .value
            .get_str("imported")
            .unwrap()
            .get_str("answer")
            .unwrap(),
        &shared,
    );
}

#[test]
fn automatic_resolution_failures_remain_typed_input_errors() {
    let result = resolve_document(
        "$ref: ./missing.yaml\n",
        id("memory://fixtures/root.yaml"),
        CountingRetriever::default(),
        &automatic_config(),
    );
    let Err(error) = result else {
        panic!("missing resource unexpectedly resolved")
    };

    assert!(matches!(
        error,
        YgError::InputResolution {
            error,
            ..
        } if matches!(error.as_ref(), ResolveError::Resolution {
            category: ResolutionErrorCategory::Retrieval,
            ..
        })
    ));
}

// r[verify resolution.session]
#[test]
fn document_streams_receive_independent_sessions_over_one_retriever() {
    let root = id("memory://fixtures/root.yaml");
    let retriever = CountingRetriever::default();
    retriever.insert(id("memory://fixtures/first.yaml"), "value: first\n");
    retriever.insert(id("memory://fixtures/second.yaml"), "value: second\n");

    let inputs = resolve_documents(
        "---\n$ref: ./first.yaml\n---\n$ref: ./second.yaml\n",
        root,
        retriever,
        &automatic_config(),
    )
    .unwrap();

    assert_eq!(inputs.len(), 2);
    assert_eq!(
        inputs[0]
            .value
            .get_str("value")
            .and_then(YamlVal::as_str_content),
        Some("first")
    );
    assert_eq!(
        inputs[1]
            .value
            .get_str("value")
            .and_then(YamlVal::as_str_content),
        Some("second")
    );
    assert!(!std::rc::Rc::ptr_eq(
        &inputs[0].resolution,
        &inputs[1].resolution
    ));
}

// r[verify resolution.include.filter]
// r[verify resolution.ref]
#[test]
fn explicit_include_and_ref_filters_recurse_with_policy_modes() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let nested = id("memory://fixtures/nested/child.yaml");
    let leaf = id("memory://fixtures/nested/leaf.yaml");
    let included = id("memory://fixtures/nested/included.yaml");
    let retriever = CountingRetriever::default();
    retriever.insert(shared, "!include ./nested/included.yaml\n");
    retriever.insert(included, "included: true\n");
    retriever.insert(nested, "$ref: ./leaf.yaml\n");
    retriever.insert(leaf, "answer: 42\n");

    let mut config = explicit_config();
    config.resolution.include.mode = ResolutionMode::Automatic;
    let context = RunCtx::new(config.clone());
    let input = resolve_document(
        concat!(
            "with_include:\n  $ref: ./shared.yaml\n",
            "recursive_ref:\n  $ref: ./nested/child.yaml\n",
        ),
        root,
        retriever,
        &config,
    )
    .unwrap();

    let output = evaluate(
        &context,
        input,
        "[(.with_include | ref), (.recursive_ref | ref)]",
    )
    .unwrap();
    let values = output[0].as_sequence().unwrap();
    assert_eq!(
        values[0].get_str("included").and_then(YamlVal::as_boolean),
        Some(true)
    );
    assert_eq!(
        values[1].get_str("answer").and_then(YamlVal::as_i64),
        Some(42),
        "recursive ref output: {:?}",
        values[1]
    );
}

#[test]
fn explicit_include_filter_accepts_scalar_and_structured_directives() {
    let root = id("memory://fixtures/root.yaml");
    let first = id("memory://fixtures/first.yaml");
    let second = id("memory://fixtures/second.yaml");
    let retriever = CountingRetriever::default();
    retriever.insert(first, "first: 1\n");
    retriever.insert(second, "second: 2\n");
    let config = explicit_config();
    let context = RunCtx::new(config.clone());
    let input = resolve_document(
        concat!(
            "scalar: !include ./first.yaml\n",
            "structured: !include {path: ./second.yaml, mode: node}\n",
        ),
        root,
        retriever,
        &config,
    )
    .unwrap();

    let output = evaluate(
        &context,
        input,
        "[(.scalar | include), (.structured | include)]",
    )
    .unwrap();
    let values = output[0].as_sequence().unwrap();

    assert_eq!(
        values[0].get_str("first").and_then(YamlVal::as_i64),
        Some(1)
    );
    assert_eq!(
        values[1].get_str("second").and_then(YamlVal::as_i64),
        Some(2)
    );
}

#[test]
fn constructed_directives_use_active_base_and_strings_stay_strings() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let retriever = CountingRetriever::default();
    retriever.insert(shared, "true\n");
    let config = explicit_config();
    let context = RunCtx::new(config.clone());
    let input = resolve_document("source: value\n", root, retriever, &config).unwrap();

    let output = evaluate(&context, input, r#"{"$ref": "./shared.yaml"} | ref"#).unwrap();

    assert_eq!(output, [YamlVal::from(true)]);
}

#[test]
fn constructed_relative_directive_without_a_base_is_typed() {
    let config = explicit_config();
    let context = RunCtx::new(config.clone());
    let input = ResolvedInput {
        value: YamlVal::map(vec![(YamlVal::str("source"), YamlVal::str("value"))]),
        resolution: resolution_handle(CountingRetriever::default(), None, config),
    };

    let error = evaluate(&context, input, r#"{"$ref": "./shared.yaml"} | ref"#).unwrap_err();

    assert!(
        matches!(
            &error,
            YgError::RuntimeResolution {
                error,
                ..
            } if matches!(error.as_ref(), ResolveError::Resolution {
                category: ResolutionErrorCategory::MissingBase,
                ..
            })
        ),
        "disabled filter error: {error:?}"
    );
}

#[test]
fn disabled_filter_fails_before_io_and_keeps_a_failure_report() {
    let root = id("memory://fixtures/root.yaml");
    let retriever = CountingRetriever::default();
    let config = LoaderConfig::unchecked();
    let context = RunCtx::new(config.clone());
    let input = resolve_document(
        "!include ./missing.yaml\n",
        root,
        retriever.clone(),
        &config,
    )
    .unwrap();

    let error = evaluate(&context, input, "include").unwrap_err();

    assert!(
        matches!(
            &error,
            YgError::RuntimeResolution {
                error,
                ..
            } if matches!(error.as_ref(), ResolveError::Resolution {
                category: ResolutionErrorCategory::DisabledCapability,
                ..
            })
        ),
        "disabled filter error: {error:?}"
    );
    assert!(retriever.calls().is_empty());
}

#[test]
fn slurped_values_select_their_own_opaque_session() {
    let root = id("memory://fixtures/root.yaml");
    let target = id("memory://fixtures/value.yaml");
    let config = explicit_config();
    let context = RunCtx::new(config.clone());
    let mut values = Vec::new();
    let mut keys = Vec::new();
    let mut handles = Vec::new();
    for expected in ["first", "second"] {
        let retriever = CountingRetriever::default();
        retriever.insert(target.clone(), format!("{expected}\n"));
        let input =
            resolve_document("$ref: ./value.yaml\n", root.clone(), retriever, &config).unwrap();
        handles.push(input.resolution.clone());
        let (value, key) = context.register_resolved_input(input).unwrap();
        values.push(value);
        keys.push(key);
    }
    context.stage_input_meta(InputMeta::default());
    context.stage_input_resolution(Some(keys[0]));
    let filter = engine::compile_filter(".[] | ref", &[]).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        &context,
        std::iter::once(Ok::<_, YgError>(YamlVal::seq(values))),
        |value| {
            output.push(value);
            Ok::<_, YgError>(())
        },
    )
    .unwrap();

    assert_eq!(output, [YamlVal::str("first"), YamlVal::str("second")]);
    assert_eq!(handles[0].borrow().report().edges().len(), 1);
    assert_eq!(handles[1].borrow().report().edges().len(), 1);
}

#[test]
fn runtime_resolution_error_and_report_survive_the_jaq_boundary() {
    let root = id("memory://fixtures/root.yaml");
    let config = explicit_config();
    let context = RunCtx::new(config.clone());
    context.stage_input_meta(InputMeta {
        filename: Some("input.yaml".into()),
        document_index: 2,
        ..InputMeta::default()
    });
    let input = resolve_document(
        "$ref: ./missing.yaml\n",
        root,
        CountingRetriever::default(),
        &config,
    )
    .unwrap();
    let handle = input.resolution.clone();

    let error = evaluate(&context, input, "ref").unwrap_err();

    assert!(matches!(
        &error,
        YgError::RuntimeResolution {
            error,
            message,
        } if message.starts_with("<stdin> document 1:") && matches!(
            error.as_ref(),
            ResolveError::Resolution {
                category: ResolutionErrorCategory::Retrieval,
                ..
            }
        )
    ));
    let report = handle.borrow();
    assert_eq!(report.report().edges().len(), 1);
    assert!(matches!(
        report.report().edges()[0].outcome,
        yamalgam_resolve::ResolutionOutcome::Failed(ResolutionErrorCategory::Retrieval)
    ));
}

#[test]
fn metadata_origin_does_not_affect_value_equality() {
    let loaded = resolve_document(
        "same\n",
        id("memory://fixtures/root.yaml"),
        CountingRetriever::default(),
        &explicit_config(),
    )
    .unwrap()
    .value;
    let mut metadata = loaded.meta().unwrap().clone();
    metadata.origin = None;
    let without_origin = loaded.clone().with_meta(metadata);

    let mut loaded_hash = DefaultHasher::new();
    loaded.hash(&mut loaded_hash);
    let mut plain_hash = DefaultHasher::new();
    without_origin.hash(&mut plain_hash);

    assert_eq!(loaded, without_origin);
    assert_eq!(loaded.cmp(&without_origin), std::cmp::Ordering::Equal);
    assert_eq!(loaded_hash.finish(), plain_hash.finish());
    assert_eq!(
        to_yaml(&loaded, &EmitOpts::default()).unwrap(),
        to_yaml(&without_origin, &EmitOpts::default()).unwrap()
    );
    assert_eq!(Meta::default().origin, None);
}
