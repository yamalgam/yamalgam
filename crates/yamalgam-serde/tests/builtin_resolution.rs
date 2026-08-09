//! Built-in resolution through the serde deserializer.

use serde_json::json;
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_resolve::{
    BuiltinResolver, MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource,
};

fn fixture() -> (BuiltinResolver<MemoryRetriever>, LoaderConfig) {
    let root = ResourceId::from_file("/fixtures/root.yaml");
    let shared = ResourceId::from_file("/fixtures/shared.yaml");
    let included = ResourceId::from_file("/fixtures/included.yaml");
    let mut retriever = MemoryRetriever::default();
    for (id, input) in [
        (shared, "service: {name: api, ports: [80, 443]}\n"),
        (included, "enabled: true\n"),
    ] {
        retriever.insert(RetrievedResource {
            id,
            bytes: input.as_bytes().to_vec(),
            provenance: ResourceProvenance::default(),
        });
    }
    let mut config = LoaderConfig::unchecked();
    config.resolution.include.mode = ResolutionMode::Automatic;
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["file".into()];
    let resolver = BuiltinResolver::new(retriever, root, config.clone());
    (resolver, config)
}

#[test]
fn builtin_resolution_deserializes_ref_and_include_values() {
    let input = concat!(
        "from_ref:\n",
        "  $ref: ./shared.yaml#/service\n",
        "from_include: !include ./included.yaml\n",
    );
    let (resolver, config) = fixture();

    let value: serde_json::Value =
        yamalgam_serde::from_str_with_resolver_and_config(input, resolver, &config).unwrap();

    assert_eq!(
        value,
        json!({
            "from_ref": {"name": "api", "ports": [80, 443]},
            "from_include": {"enabled": true}
        })
    );
}
