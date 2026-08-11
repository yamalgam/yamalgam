//! Policy-bound loading for external JSON Schema resources.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value as JsonValue;
use thiserror::Error;
use yamalgam_core::{LimitExceeded, LoaderConfig, LocatedValue, ResolutionMode, ResourceLimit};
use yamalgam_resolve::{
    ResourceError, ResourceId, ResourceRequest, ResourceRetriever, RetrieveError,
};

use crate::navigation::{NavigationIndex, NavigationResource};
use crate::schema::{SchemaError, SchemaOptions, SchemaValidator, schema_value_to_json};

/// One loaded schema resource with semantic source locations.
#[derive(Clone, Debug)]
pub struct SchemaDocument {
    /// Canonical resource identity.
    pub id: ResourceId,
    /// Parsed YAML schema document.
    pub document: LocatedValue,
}

/// Failure while loading or traversing an external schema graph.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchemaRetrieveError {
    /// Reference resolution is disabled by the loader configuration.
    #[error("schema reference resolution is disabled")]
    Disabled,
    /// The loader policy denied the resource before retrieval.
    #[error("schema resource denied by policy: {id}")]
    PolicyDenied {
        /// Redacting resource identity.
        id: ResourceId,
    },
    /// A shared loader or resolution ceiling was exceeded.
    #[error(transparent)]
    LimitExceeded(#[from] LimitExceeded),
    /// A configured M11 resource backend failed.
    #[error("schema resource retrieval failed: {id}")]
    Backend {
        /// Redacting requested identity.
        id: ResourceId,
        /// Typed backend failure, retained without rendering its details publicly.
        #[source]
        source: RetrieveError,
    },
    /// Retrieved bytes were not UTF-8 text.
    #[error("schema resource is not UTF-8: {id}")]
    InvalidUtf8 {
        /// Redacting canonical identity.
        id: ResourceId,
        /// UTF-8 conversion failure.
        #[source]
        source: std::str::Utf8Error,
    },
    /// A resource was not exactly one valid YAML or JSON document.
    #[error("schema resource is not one valid document: {id}: {message}")]
    InvalidDocument {
        /// Redacting canonical identity.
        id: ResourceId,
        /// Parser or composer summary.
        message: String,
    },
    /// A schema URI reference could not be resolved.
    #[error("invalid schema reference from {base}")]
    InvalidReference {
        /// Redacting containing resource identity.
        base: ResourceId,
        /// URI-reference failure.
        #[source]
        source: ResourceError,
    },
    /// A file identity could not become an absolute URI.
    #[error("schema resource cannot be used as an absolute URI: {0}")]
    InvalidBase(ResourceId),
    /// Checked graph accounting overflowed.
    #[error("schema resource accounting overflow for {0}")]
    ResourceOverflow(&'static str),
}

#[derive(Default)]
struct SchemaSessionState {
    documents: HashMap<ResourceId, SchemaDocument>,
    schemas: HashMap<ResourceId, JsonValue>,
    aliases: HashMap<ResourceId, ResourceId>,
    edges: usize,
    fetched_bytes: usize,
}

/// One policy and budget scope for loading and compiling a schema graph.
pub struct SchemaSession {
    retriever: Arc<dyn ResourceRetriever>,
    config: LoaderConfig,
    state: Mutex<SchemaSessionState>,
}

impl SchemaSession {
    /// Create a schema session over an existing M11 resource backend.
    #[must_use]
    pub fn new(retriever: Arc<dyn ResourceRetriever>, config: LoaderConfig) -> Self {
        Self {
            retriever,
            config,
            state: Mutex::new(SchemaSessionState::default()),
        }
    }

    /// Inspect the shared loader and resolution policy.
    #[must_use]
    pub const fn config(&self) -> &LoaderConfig {
        &self.config
    }

    /// Load exactly one located YAML or JSON schema document.
    ///
    /// # Errors
    ///
    /// Returns a typed policy, limit, retrieval, encoding, or document error.
    pub fn load(&self, id: &ResourceId) -> Result<SchemaDocument, SchemaRetrieveError> {
        self.enforce_policy(id, None)?;
        let mut state = self.lock_state();
        self.load_document(&mut state, id)
    }

    /// Compile a root document and every reachable external schema.
    ///
    /// # Errors
    ///
    /// Returns schema conversion/compilation failures or typed retrieval errors.
    pub fn compile(&self, root: SchemaDocument) -> Result<SchemaValidator, SchemaError> {
        let root_schema = schema_value_to_json(&root.document.value)?;
        {
            let mut state = self.lock_state();
            state.documents.insert(root.id.clone(), root.clone());
            state.schemas.insert(root.id.clone(), root_schema.clone());
        }
        self.preload_graph(&root.id, &root_schema)?;

        let (resources, navigation) = {
            let state = self.lock_state();
            let mut resources = HashMap::with_capacity(state.schemas.len());
            let mut navigation_resources = Vec::with_capacity(state.schemas.len());
            for (id, schema) in &state.schemas {
                if let Some(uri) = resource_uri(id)? {
                    resources.insert(uri.clone(), schema.clone());
                    navigation_resources.push(NavigationResource {
                        uri,
                        schema: schema.clone(),
                        located: state
                            .documents
                            .get(id)
                            .map(|document| document.document.clone()),
                    });
                }
            }
            let mut navigation_aliases = HashMap::new();
            for (requested, canonical) in &state.aliases {
                if let (Some(requested_uri), Some(canonical_uri), Some(schema)) = (
                    resource_uri(requested)?,
                    resource_uri(canonical)?,
                    state.schemas.get(canonical),
                ) {
                    resources.insert(requested_uri.clone(), schema.clone());
                    navigation_aliases.insert(requested_uri, canonical_uri);
                }
            }
            let root_uri = resource_uri(&root.id)?;
            let navigation = NavigationIndex::from_resources(
                root_uri.as_deref(),
                navigation_resources,
                navigation_aliases,
            );
            (resources, navigation)
        };
        let options = SchemaOptions {
            base_uri: resource_uri(&root.id)?,
            ..SchemaOptions::default()
        };
        SchemaValidator::compile_json_with_retriever(
            root_schema,
            options,
            CachedSchemaRetriever { resources },
            navigation,
        )
    }

    fn preload_graph(
        &self,
        root_id: &ResourceId,
        root_schema: &JsonValue,
    ) -> Result<(), SchemaError> {
        struct GraphWork {
            id: ResourceId,
            schema: JsonValue,
            depth: usize,
            ancestors: Vec<ResourceId>,
        }

        let mut pending = VecDeque::from([GraphWork {
            id: root_id.clone(),
            schema: root_schema.clone(),
            depth: 0,
            ancestors: vec![root_id.clone()],
        }]);
        while let Some(work) = pending.pop_front() {
            for target in external_references(&work.schema, &work.id)? {
                let depth = work
                    .depth
                    .checked_add(1)
                    .ok_or(SchemaRetrieveError::ResourceOverflow("resolution depth"))?;
                let document = {
                    let mut state = self.lock_state();
                    charge_edge(&mut state, &self.config)?;
                    check_limit(
                        ResourceLimit::ResolutionDepth,
                        depth,
                        self.config.resolution.limits.max_depth,
                    )?;
                    self.enforce_policy(&target, Some(&work.id))?;
                    self.load_document(&mut state, &target)?
                };
                if work.ancestors.contains(&document.id) {
                    continue;
                }
                let schema = {
                    let mut state = self.lock_state();
                    if let Some(schema) = state.schemas.get(&document.id) {
                        schema.clone()
                    } else {
                        let schema = schema_value_to_json(&document.document.value)?;
                        state.schemas.insert(document.id.clone(), schema.clone());
                        schema
                    }
                };
                let mut ancestors = work.ancestors.clone();
                ancestors.push(document.id.clone());
                pending.push_back(GraphWork {
                    id: document.id,
                    schema,
                    depth,
                    ancestors,
                });
            }
        }
        Ok(())
    }

    fn load_document(
        &self,
        state: &mut SchemaSessionState,
        requested: &ResourceId,
    ) -> Result<SchemaDocument, SchemaRetrieveError> {
        let canonical = state.aliases.get(requested).unwrap_or(requested).clone();
        if let Some(document) = state.documents.get(&canonical) {
            return Ok(document.clone());
        }

        let remaining = self
            .config
            .resolution
            .limits
            .max_fetched_bytes
            .map(|maximum| maximum.saturating_sub(state.fetched_bytes));
        let max_bytes = match (self.config.limits.max_input_bytes, remaining) {
            (Some(input), Some(session)) => Some(input.min(session)),
            (Some(input), None) => Some(input),
            (None, Some(session)) => Some(session),
            (None, None) => None,
        };
        let session_caps_request = remaining.is_some_and(|session| {
            self.config
                .limits
                .max_input_bytes
                .is_none_or(|input| session <= input)
        });
        let retrieved = match self.retriever.retrieve(&ResourceRequest {
            id: requested.clone(),
            max_bytes,
        }) {
            Ok(resource) => resource,
            Err(RetrieveError::TooLarge { observed, .. }) => {
                let (limit, observed, maximum) = if session_caps_request {
                    let maximum = self
                        .config
                        .resolution
                        .limits
                        .max_fetched_bytes
                        .expect("session request cap has a maximum");
                    let observed = state
                        .fetched_bytes
                        .checked_add(observed)
                        .ok_or(SchemaRetrieveError::ResourceOverflow("fetched byte count"))?;
                    (ResourceLimit::FetchedBytes, observed, maximum)
                } else {
                    let maximum = self
                        .config
                        .limits
                        .max_input_bytes
                        .expect("input request cap has a maximum");
                    (ResourceLimit::InputBytes, observed, maximum)
                };
                return Err(LimitExceeded::new(limit, observed, maximum).into());
            }
            Err(source) => {
                return Err(SchemaRetrieveError::Backend {
                    id: requested.clone(),
                    source,
                });
            }
        };
        self.config.limits.check_input_size(retrieved.bytes.len())?;
        let next_fetched = state
            .fetched_bytes
            .checked_add(retrieved.bytes.len())
            .ok_or(SchemaRetrieveError::ResourceOverflow("fetched byte count"))?;
        check_limit(
            ResourceLimit::FetchedBytes,
            next_fetched,
            self.config.resolution.limits.max_fetched_bytes,
        )?;
        state.fetched_bytes = next_fetched;

        let canonical = retrieved.id;
        if canonical != *requested {
            state.aliases.insert(requested.clone(), canonical.clone());
        }
        let text = std::str::from_utf8(&retrieved.bytes).map_err(|source| {
            SchemaRetrieveError::InvalidUtf8 {
                id: canonical.clone(),
                source,
            }
        })?;
        let mut parse_config = self.config.clone();
        parse_config.resolution.include.mode = ResolutionMode::Disabled;
        parse_config.resolution.refs.mode = ResolutionMode::Disabled;
        let document = yamalgam_compose::from_str_single_located_with_config(text, &parse_config)
            .map_err(|error| SchemaRetrieveError::InvalidDocument {
            id: canonical.clone(),
            message: error.to_string(),
        })?;
        let document = SchemaDocument {
            id: canonical.clone(),
            document,
        };
        state.documents.insert(canonical, document.clone());
        Ok(document)
    }

    fn enforce_policy(
        &self,
        id: &ResourceId,
        parent: Option<&ResourceId>,
    ) -> Result<(), SchemaRetrieveError> {
        if parent.is_some_and(|parent| parent == id) {
            return Ok(());
        }
        let policy = &self.config.resolution.refs;
        if policy.mode == ResolutionMode::Disabled {
            return Err(SchemaRetrieveError::Disabled);
        }
        let scheme_allowed = policy
            .allow_schemes
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(id.scheme()));
        let host_allowed = !matches!(id.scheme(), "http" | "https")
            || id.host_str().is_some_and(|host| {
                policy
                    .allow_hosts
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(host))
            });
        let file_configured = id.scheme() != "file" || self.config.resolution.files.root.is_some();
        if !scheme_allowed || !host_allowed || !file_configured || id.has_userinfo() {
            return Err(SchemaRetrieveError::PolicyDenied { id: id.clone() });
        }
        Ok(())
    }

    fn lock_state(&self) -> MutexGuard<'_, SchemaSessionState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Debug)]
struct CachedSchemaRetriever {
    resources: HashMap<String, JsonValue>,
}

impl jsonschema::Retrieve for CachedSchemaRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
        self.resources
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| Box::new(UncachedSchema) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[derive(Debug, Error)]
#[error("schema resource was not preloaded")]
struct UncachedSchema;

fn external_references(
    schema: &JsonValue,
    root_id: &ResourceId,
) -> Result<Vec<ResourceId>, SchemaRetrieveError> {
    let mut scoped_nodes = Vec::new();
    let mut embedded = HashSet::from([root_id.clone()]);
    let mut pending = vec![(schema, root_id.clone())];
    while let Some((node, inherited_scope)) = pending.pop() {
        let JsonValue::Object(object) = node else {
            continue;
        };
        let scope = match object.get("$id").and_then(JsonValue::as_str) {
            Some(id) => resolve(&inherited_scope, id)?,
            None => inherited_scope,
        };
        embedded.insert(scope.clone());
        scoped_nodes.push((node, scope.clone()));
        let children = schema_children(object);
        pending.extend(
            children
                .into_iter()
                .rev()
                .map(|child| (child, scope.clone())),
        );
    }

    let mut references = Vec::new();
    for (node, scope) in scoped_nodes {
        let object = node.as_object().expect("scoped nodes are objects");
        for keyword in ["$ref", "$dynamicRef"] {
            if let Some(reference) = object.get(keyword).and_then(JsonValue::as_str) {
                let target = resolve(&scope, reference)?;
                if !embedded.contains(&target) {
                    references.push(target);
                }
            }
        }
    }
    Ok(references)
}

fn schema_children(object: &serde_json::Map<String, JsonValue>) -> Vec<&JsonValue> {
    const SINGLE: &[&str] = &[
        "additionalItems",
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "items",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    ];
    const ARRAYS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];
    const MAPS: &[&str] = &[
        "$defs",
        "definitions",
        "dependentSchemas",
        "patternProperties",
        "properties",
    ];

    let mut children = Vec::new();
    for keyword in SINGLE {
        if let Some(value @ (JsonValue::Bool(_) | JsonValue::Object(_))) = object.get(*keyword) {
            children.push(value);
        }
    }
    for keyword in ARRAYS {
        if let Some(JsonValue::Array(values)) = object.get(*keyword) {
            children.extend(values);
        }
    }
    for keyword in MAPS {
        if let Some(JsonValue::Object(values)) = object.get(*keyword) {
            children.extend(values.values());
        }
    }
    children
}

fn resolve(base: &ResourceId, reference: &str) -> Result<ResourceId, SchemaRetrieveError> {
    let resource_reference = reference
        .split_once('#')
        .map_or(reference, |(resource, _)| resource);
    base.resolve(resource_reference)
        .map(|resolved| resolved.resource)
        .map_err(|source| SchemaRetrieveError::InvalidReference {
            base: base.clone(),
            source,
        })
}

fn resource_uri(id: &ResourceId) -> Result<Option<String>, SchemaRetrieveError> {
    if let Some(url) = id.as_url() {
        return Ok(Some(url.as_str().to_owned()));
    }
    let Some(path) = id.as_path() else {
        return Ok(None);
    };
    let path =
        std::path::absolute(path).map_err(|_| SchemaRetrieveError::InvalidBase(id.clone()))?;
    url::Url::from_file_path(path)
        .map(|url| Some(url.into()))
        .map_err(|()| SchemaRetrieveError::InvalidBase(id.clone()))
}

fn charge_edge(
    state: &mut SchemaSessionState,
    config: &LoaderConfig,
) -> Result<(), SchemaRetrieveError> {
    let next = state
        .edges
        .checked_add(1)
        .ok_or(SchemaRetrieveError::ResourceOverflow(
            "resolution edge count",
        ))?;
    state.edges = next;
    check_limit(
        ResourceLimit::ResolutionEdges,
        next,
        config.resolution.limits.max_edges,
    )
}

fn check_limit(
    limit: ResourceLimit,
    observed: usize,
    maximum: Option<usize>,
) -> Result<(), SchemaRetrieveError> {
    if let Some(maximum) = maximum
        && observed > maximum
    {
        return Err(LimitExceeded::new(limit, observed, maximum).into());
    }
    Ok(())
}
