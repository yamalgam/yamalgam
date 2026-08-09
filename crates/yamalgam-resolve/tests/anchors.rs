//! Imported anchor self-containment and deterministic namespace behavior.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_parser::{Parser, ResolutionErrorCategory, ResolveError};
use yamalgam_resolve::{
    DiscoveryRequest, OwnedDocument, OwnedKind, OwnedNode, ResolutionSession, ResourceId,
    ResourceProvenance, ResourceRequest, ResourceRetriever, RetrieveError, RetrievedResource,
};

#[derive(Clone, Default)]
struct AnchorRetriever {
    resources: Arc<Mutex<HashMap<ResourceId, RetrievedResource>>>,
}

impl AnchorRetriever {
    fn insert(&self, id: ResourceId, input: impl Into<Vec<u8>>) {
        self.resources.lock().unwrap().insert(
            id.clone(),
            RetrievedResource {
                id,
                bytes: input.into(),
                provenance: ResourceProvenance::default(),
            },
        );
    }
}

impl ResourceRetriever for AnchorRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        self.resources
            .lock()
            .unwrap()
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

fn document(origin: &ResourceId, input: &str) -> OwnedDocument {
    let events = Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap();
    OwnedDocument::from_events(origin.clone(), events).unwrap()
}

fn config() -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["memory".into()];
    config.resolution.include.mode = ResolutionMode::Automatic;
    config
}

fn resolve(retriever: AnchorRetriever, input: &str) -> Result<OwnedDocument, ResolveError> {
    let root = id("memory://root/document.yaml");
    ResolutionSession::new(retriever, config()).resolve_root(document(&root, input))
}

fn sequence(node: &OwnedNode) -> &[OwnedNode] {
    let OwnedKind::Sequence { items, .. } = &node.kind else {
        panic!("expected sequence, got {:?}", node.kind);
    };
    items
}

fn mapping_value<'a>(node: &'a OwnedNode, key: &str) -> &'a OwnedNode {
    let OwnedKind::Mapping { entries, .. } = &node.kind else {
        panic!("expected mapping, got {:?}", node.kind);
    };
    entries
        .iter()
        .find_map(|(candidate, value)| {
            matches!(&candidate.kind, OwnedKind::Scalar(scalar) if scalar.value == key)
                .then_some(value)
        })
        .unwrap_or_else(|| panic!("missing key {key}"))
}

fn anchors_and_aliases(node: &OwnedNode) -> (Vec<String>, Vec<String>) {
    let mut anchors = Vec::new();
    let mut aliases = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &node.anchor {
            anchors.push(anchor.clone());
        }
        match &node.kind {
            OwnedKind::Alias { name } => aliases.push(name.clone()),
            OwnedKind::Sequence { items, .. } => pending.extend(items),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }
    anchors.sort();
    aliases.sort();
    (anchors, aliases)
}

#[test]
fn repeated_cached_fragment_gets_a_unique_stable_namespace_per_edge() {
    let retriever = AnchorRetriever::default();
    let shared = id("memory://shared/document.yaml");
    retriever.insert(shared, "root: &shared {value: retained}\ncopy: *shared\n");
    let input =
        "- {$ref: 'memory://shared/document.yaml'}\n- {$ref: 'memory://shared/document.yaml'}\n";

    let first = resolve(retriever.clone(), input).unwrap();
    let second = resolve(retriever, input).unwrap();

    assert_eq!(first, second);
    let items = sequence(&first.root);
    let (first_anchors, first_aliases) = anchors_and_aliases(&items[0]);
    let (second_anchors, second_aliases) = anchors_and_aliases(&items[1]);
    assert_eq!(first_anchors, ["yg_r2_e1_shared"]);
    assert_eq!(first_aliases, first_anchors);
    assert_eq!(second_anchors, ["yg_r2_e2_shared"]);
    assert_eq!(second_aliases, second_anchors);
}

#[test]
fn two_resources_with_the_same_source_anchor_never_collide() {
    let retriever = AnchorRetriever::default();
    let left = id("memory://shared/left.yaml");
    let right = id("memory://shared/right.yaml");
    retriever.insert(left, "&same [left, *same]\n");
    retriever.insert(right, "&same [right, *same]\n");

    let output = resolve(
        retriever,
        "- {$ref: 'memory://shared/left.yaml'}\n- {$ref: 'memory://shared/right.yaml'}\n",
    )
    .unwrap();

    let items = sequence(&output.root);
    assert_eq!(anchors_and_aliases(&items[0]).0, ["yg_r2_e1_same"]);
    assert_eq!(anchors_and_aliases(&items[1]).0, ["yg_r3_e2_same"]);
}

#[test]
fn imported_namespace_avoids_anchor_names_reserved_by_the_root() {
    let retriever = AnchorRetriever::default();
    let child = id("memory://shared/child.yaml");
    retriever.insert(child, "&shared [imported, *shared]\n");
    let input = concat!(
        "root: &yg_r2_e1_shared original\n",
        "imported: {$ref: 'memory://shared/child.yaml'}\n",
        "root-copy: *yg_r2_e1_shared\n",
    );

    let output = resolve(retriever, input).unwrap();
    let imported = mapping_value(&output.root, "imported");
    let (anchors, aliases) = anchors_and_aliases(imported);

    assert_eq!(anchors, aliases);
    assert_eq!(anchors.len(), 1);
    assert_ne!(anchors[0], "yg_r2_e1_shared");
    assert!(matches!(
        &mapping_value(&output.root, "root-copy").kind,
        OwnedKind::Alias { name } if name == "yg_r2_e1_shared"
    ));
}

#[test]
fn selected_fragment_may_use_internal_anchors_but_not_external_ones() {
    let retriever = AnchorRetriever::default();
    let shared = id("memory://shared/fragments.yaml");
    retriever.insert(
        shared,
        "outside: &outside external\ngood:\n  value: &inside retained\n  copy: *inside\nbad:\n  copy: *outside\n",
    );

    let good = resolve(
        retriever.clone(),
        "$ref: 'memory://shared/fragments.yaml#/good'\n",
    )
    .unwrap();
    let (anchors, aliases) = anchors_and_aliases(&good.root);
    assert_eq!(anchors, ["yg_r2_e1_inside"]);
    assert_eq!(aliases, anchors);

    let error = resolve(retriever, "$ref: 'memory://shared/fragments.yaml#/bad'\n").unwrap_err();
    let ResolveError::Resolution {
        category,
        target,
        reference_chain,
        ..
    } = error
    else {
        panic!("expected typed fragment error");
    };
    assert_eq!(category, ResolutionErrorCategory::NonSelfContainedFragment);
    assert!(target.contains("fragments.yaml#/bad"));
    assert!(reference_chain.is_empty());
}

#[test]
fn selected_fragment_rejects_aliases_before_their_in_fragment_definition() {
    let retriever = AnchorRetriever::default();
    let shared = id("memory://shared/forward.yaml");
    retriever.insert(shared, "fragment: [*later, &later retained]\n");

    let error = resolve(
        retriever,
        "$ref: 'memory://shared/forward.yaml#/fragment'\n",
    )
    .unwrap_err();

    let ResolveError::Resolution {
        category,
        source: Some(source),
        ..
    } = error
    else {
        panic!("expected typed fragment error");
    };
    assert_eq!(category, ResolutionErrorCategory::NonSelfContainedFragment);
    assert!(source.to_string().contains("external anchor later"));
}

#[test]
fn aliases_inside_mapping_keys_are_rewritten_with_their_definition() {
    let retriever = AnchorRetriever::default();
    let shared = id("memory://shared/keys.yaml");
    retriever.insert(shared, "? &key [complex]\n: first\n? *key\n: second\n");

    let output = resolve(retriever, "$ref: 'memory://shared/keys.yaml'\n").unwrap();

    let (anchors, aliases) = anchors_and_aliases(&output.root);
    assert_eq!(anchors, ["yg_r2_e1_key"]);
    assert_eq!(aliases, anchors);
}

#[test]
fn nested_imports_get_independent_namespaces_and_anchor_like_text_is_untouched() {
    let retriever = AnchorRetriever::default();
    let parent = id("memory://shared/parent.yaml");
    let child = id("memory://shared/child.yaml");
    retriever.insert(
        parent,
        "outer: &same [parent, *same]\nchild: !include child.yaml\ntext: 'literal &same and *same'\n",
    );
    retriever.insert(child, "&same [child, *same]\n");

    let output = resolve(retriever, "$ref: 'memory://shared/parent.yaml'\n").unwrap();

    let (anchors, aliases) = anchors_and_aliases(&output.root);
    assert_eq!(anchors, ["yg_r2_e1_same", "yg_r3_e2_same"]);
    assert_eq!(aliases, anchors);
    let OwnedKind::Scalar(text) = &mapping_value(&output.root, "text").kind else {
        panic!("expected scalar text");
    };
    assert_eq!(text.value, "literal &same and *same");
}
