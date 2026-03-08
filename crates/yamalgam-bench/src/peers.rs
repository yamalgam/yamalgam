//! Thin wrappers normalizing each YAML peer's parse API.
//!
//! Each function takes `&str`, parses to completion, and discards the result.
//! The goal is measuring parse throughput, not deserialization correctness.

/// Parse with yamalgam scanner (token stream only).
pub fn yamalgam_scan(input: &str) {
    let _tokens: Result<Vec<_>, _> = yamalgam_scanner::scanner::Scanner::new(input).collect();
}

/// Parse with yamalgam parser (event stream).
pub fn yamalgam_parse(input: &str) {
    let _events: Result<Vec<_>, _> = yamalgam_parser::Parser::new(input).collect();
}

/// Parse with `yaml_serde` (serde `Value` deserialization).
pub fn yaml_serde_parse(input: &str) {
    let _: Result<serde_json::Value, _> = yaml_serde::from_str(input);
}

/// Parse with `libyaml-safer` (event iteration).
pub fn libyaml_safer_parse(input: &str) {
    let mut bytes = input.as_bytes();
    let mut parser = libyaml_safer::Parser::new();
    parser.set_input_string(&mut bytes);
    for event in &mut parser {
        let _ = event;
    }
}

/// Parse with `yaml-rust2` (DOM load).
pub fn yaml_rust2_parse(input: &str) {
    let _ = yaml_rust2::YamlLoader::load_from_str(input);
}

/// Parse with `saphyr-parser` (event sink).
pub fn saphyr_parser_parse(input: &str) {
    struct Sink;
    impl<'input> saphyr_parser::EventReceiver<'input> for Sink {
        fn on_event(&mut self, _ev: saphyr_parser::Event<'input>) {}
    }
    let mut parser = saphyr_parser::Parser::new_from_str(input);
    let mut sink = Sink;
    let _ = parser.load(&mut sink, true);
}

/// Parse with `saphyr` (DOM load, like `yaml-rust2`).
pub fn saphyr_parse(input: &str) {
    use saphyr::LoadableYamlNode as _;
    let _ = saphyr::Yaml::load_from_str(input);
}

/// Parse with `serde-saphyr` (serde deserialization, no native Value type).
pub fn serde_saphyr_parse(input: &str) {
    let _: Result<serde_json::Value, _> = serde_saphyr::from_str(input);
}

/// Parse with `rust-yaml` (DOM load).
pub fn rust_yaml_parse(input: &str) {
    let yaml = rust_yaml::Yaml::new();
    let _ = yaml.load(input.as_bytes());
}
