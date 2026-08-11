//! Deterministic schema selection from document-local and configured sources.

use std::collections::BTreeMap;
use std::path;

use globset::Glob;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use yamalgam_core::{LocatedValue, Value};
use yamalgam_cst::{CstElement, CstNode, CstToken, TokenKind, parse_to_cst};
use yamalgam_resolve::ResourceId;

/// Where a selected schema URI came from.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaSource {
    /// A schema supplied directly by the caller.
    Explicit,
    /// A leading Yamalgam pragma or compatible editor modeline.
    Pragma,
    /// A top-level string `$schema` property.
    InlineProperty,
    /// A caller-configured path association.
    Association,
    /// A matching SchemaStore catalog entry.
    SchemaStore,
}

/// One configured path-to-schema association.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaAssociation {
    /// Schema URI selected when a pattern matches.
    pub uri: String,
    /// Glob patterns matched against normalized path suffixes.
    pub file_match: Vec<String>,
}

/// One selected schema and its precedence source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelectedSchema {
    /// Absolute schema URI.
    pub uri: String,
    /// Source which won selection precedence.
    pub source: SchemaSource,
}

/// The subset of a SchemaStore catalog consumed by Yamalgam.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaCatalog {
    /// Catalog entries in their original tie-breaking order.
    #[serde(default)]
    pub schemas: Vec<SchemaCatalogEntry>,
}

impl SchemaCatalog {
    /// Decode a semantic YAML or JSON value as a SchemaStore catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is not JSON-compatible or does not
    /// match the supported SchemaStore catalog shape.
    pub fn from_value(value: &Value) -> Result<Self, SchemaCatalogError> {
        let json = crate::schema::schema_value_to_json(value)?;
        serde_json::from_value(json).map_err(SchemaCatalogError::InvalidCatalog)
    }
}

/// One schema entry from the SchemaStore catalog.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaCatalogEntry {
    /// Human-readable schema name.
    pub name: String,
    /// Optional catalog description.
    #[serde(default)]
    pub description: Option<String>,
    /// File patterns associated with this schema.
    #[serde(default, rename = "fileMatch")]
    pub file_match: Vec<String>,
    /// Canonical schema URI.
    pub url: String,
    /// Optional version-specific schema URIs.
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
}

/// Ordered configured and catalog sources used to select document schemas.
#[derive(Clone, Debug, Default)]
pub struct SchemaSelector {
    /// User-configured associations, in precedence order.
    pub associations: Vec<SchemaAssociation>,
    /// Optional caller-supplied SchemaStore catalog.
    pub schema_store: Option<SchemaCatalog>,
}

/// Failure while selecting a schema for one or more YAML documents.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchemaSelectionError {
    /// A recognized leading directive did not contain exactly one URI token.
    #[error("malformed {directive} schema directive in document {document}")]
    MalformedDirective {
        /// Zero-based document index.
        document: usize,
        /// Directive syntax family.
        directive: &'static str,
    },
    /// Two directives at the same priority selected different values.
    #[error("conflicting {directive} schema directives in document {document}")]
    ConflictingDirectives {
        /// Zero-based document index.
        document: usize,
        /// Directive syntax family.
        directive: &'static str,
    },
    /// The YAML stream could not be composed to inspect inline properties.
    #[error("cannot inspect schema properties in invalid YAML: {message}")]
    InvalidDocument {
        /// Parser or composer summary.
        message: String,
    },
    /// A relative schema URI did not have a usable containing resource.
    #[error("relative schema URI in document {document} requires a containing resource")]
    MissingBase {
        /// Zero-based document index.
        document: usize,
    },
    /// A schema URI could not be parsed or resolved.
    #[error("invalid schema URI in document {document}")]
    InvalidUri {
        /// Zero-based document index.
        document: usize,
        /// URL parsing failure.
        #[source]
        source: url::ParseError,
    },
    /// A file resource could not be represented as an absolute file URI.
    #[error("containing file for document {document} cannot be represented as a URI")]
    InvalidFileBase {
        /// Zero-based document index.
        document: usize,
    },
    /// A configured association or catalog glob was invalid.
    #[error("invalid schema file-match pattern {pattern:?}")]
    InvalidGlob {
        /// Pattern rejected by `globset`.
        pattern: String,
        /// Glob compilation failure.
        #[source]
        source: globset::Error,
    },
}

/// Failure decoding a SchemaStore catalog document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchemaCatalogError {
    /// The catalog used YAML values that cannot represent JSON data.
    #[error(transparent)]
    InvalidValue(#[from] crate::SchemaError),
    /// The document did not match the supported SchemaStore catalog shape.
    #[error("invalid SchemaStore catalog: {0}")]
    InvalidCatalog(#[source] serde_json::Error),
}

#[derive(Clone, Debug, Default)]
struct DocumentDirectives {
    pragma: Option<String>,
    modeline: Option<String>,
}

impl SchemaSelector {
    /// Select at most one schema for every document in a YAML stream.
    ///
    /// `explicit` applies to every document. The literal value `none` at any
    /// winning precedence level disables all lower-priority sources for that
    /// document. Relative URIs resolve against `containing`.
    ///
    /// # Errors
    ///
    /// Returns a structured error for invalid YAML, malformed or conflicting
    /// leading directives, relative URIs without a base, and invalid globs.
    // r[impl validate.selection]
    // r[impl validate.self-describing]
    pub fn select(
        &self,
        input: &str,
        containing: Option<&ResourceId>,
        explicit: Option<&str>,
    ) -> Result<Vec<Option<SelectedSchema>>, SchemaSelectionError> {
        let documents = yamalgam_compose::from_str(input).map_err(|error| {
            SchemaSelectionError::InvalidDocument {
                message: error.to_string(),
            }
        })?;
        self.select_values(
            input,
            documents.iter(),
            documents.len(),
            containing,
            explicit,
        )
    }

    /// Select schemas using documents the caller has already composed.
    ///
    /// The source text is inspected only for leading comment directives; inline
    /// `$schema` properties come from `documents`.
    ///
    /// # Errors
    ///
    /// Returns a structured error for malformed or conflicting directives,
    /// relative URIs without a base, and invalid globs.
    pub fn select_composed(
        &self,
        input: &str,
        documents: &[LocatedValue],
        containing: Option<&ResourceId>,
        explicit: Option<&str>,
    ) -> Result<Vec<Option<SelectedSchema>>, SchemaSelectionError> {
        self.select_values(
            input,
            documents.iter().map(LocatedValue::value),
            documents.len(),
            containing,
            explicit,
        )
    }

    fn select_values<'document>(
        &self,
        input: &str,
        documents: impl IntoIterator<Item = &'document Value>,
        document_count: usize,
        containing: Option<&ResourceId>,
        explicit: Option<&str>,
    ) -> Result<Vec<Option<SelectedSchema>>, SchemaSelectionError> {
        if document_count == 0 {
            return Ok(Vec::new());
        }

        let directives = document_directives(input, document_count)?;
        let path = containing.and_then(resource_path);
        let mut selected = Vec::with_capacity(document_count);

        for (document, value) in documents.into_iter().enumerate() {
            let local = &directives[document];
            let candidate = explicit
                .map(|uri| (uri, SchemaSource::Explicit))
                .or_else(|| {
                    local
                        .pragma
                        .as_deref()
                        .map(|uri| (uri, SchemaSource::Pragma))
                })
                .or_else(|| {
                    local
                        .modeline
                        .as_deref()
                        .map(|uri| (uri, SchemaSource::Pragma))
                })
                .or_else(|| inline_schema(value).map(|uri| (uri, SchemaSource::InlineProperty)));

            if let Some((uri, source)) = candidate {
                selected.push(select_uri(uri, source, containing, document)?);
                continue;
            }

            if let Some(path) = path.as_deref()
                && let Some(uri) = first_association(&self.associations, path)?
            {
                selected.push(select_uri(
                    uri,
                    SchemaSource::Association,
                    containing,
                    document,
                )?);
                continue;
            }

            if let (Some(path), Some(catalog)) = (path.as_deref(), self.schema_store.as_ref())
                && let Some(uri) = catalog_match(catalog, path)?
            {
                selected.push(select_uri(
                    uri,
                    SchemaSource::SchemaStore,
                    containing,
                    document,
                )?);
                continue;
            }

            selected.push(None);
        }

        Ok(selected)
    }
}

fn inline_schema(value: &Value) -> Option<&str> {
    value.get("$schema").and_then(Value::as_str)
}

fn select_uri(
    uri: &str,
    source: SchemaSource,
    containing: Option<&ResourceId>,
    document: usize,
) -> Result<Option<SelectedSchema>, SchemaSelectionError> {
    if uri == "none" {
        return Ok(None);
    }
    resolve_uri(uri, containing, document).map(|uri| Some(SelectedSchema { uri, source }))
}

fn resolve_uri(
    uri: &str,
    containing: Option<&ResourceId>,
    document: usize,
) -> Result<String, SchemaSelectionError> {
    if let Ok(absolute) = Url::parse(uri) {
        return Ok(absolute.into());
    }

    let containing = containing.ok_or(SchemaSelectionError::MissingBase { document })?;
    let base = if let Some(url) = containing.as_url() {
        url.clone()
    } else if let Some(file) = containing.as_path() {
        let absolute =
            path::absolute(file).map_err(|_| SchemaSelectionError::InvalidFileBase { document })?;
        Url::from_file_path(absolute)
            .map_err(|()| SchemaSelectionError::InvalidFileBase { document })?
    } else {
        return Err(SchemaSelectionError::MissingBase { document });
    };
    base.join(uri)
        .map(String::from)
        .map_err(|source| SchemaSelectionError::InvalidUri { document, source })
}

fn resource_path(resource: &ResourceId) -> Option<String> {
    let path = resource
        .as_path()
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| resource.as_url().map(|url| url.path().to_owned()))?;
    Some(normalize_path(&path))
}

fn first_association<'association>(
    associations: &'association [SchemaAssociation],
    path: &str,
) -> Result<Option<&'association str>, SchemaSelectionError> {
    for association in associations {
        for pattern in &association.file_match {
            if pattern_matches(pattern, path)? {
                return Ok(Some(&association.uri));
            }
        }
    }
    Ok(None)
}

fn catalog_match<'catalog>(
    catalog: &'catalog SchemaCatalog,
    path: &str,
) -> Result<Option<&'catalog str>, SchemaSelectionError> {
    let mut selected: Option<(usize, &str)> = None;
    for entry in &catalog.schemas {
        let mut entry_score = None;
        for pattern in &entry.file_match {
            if pattern_matches(pattern, path)? {
                let score = literal_pattern_len(pattern);
                entry_score = Some(entry_score.map_or(score, |current: usize| current.max(score)));
            }
        }
        if let Some(score) = entry_score
            && selected.is_none_or(|(current, _)| score > current)
        {
            selected = Some((score, entry.url.as_str()));
        }
    }
    Ok(selected.map(|(_, uri)| uri))
}

fn pattern_matches(pattern: &str, path: &str) -> Result<bool, SchemaSelectionError> {
    let pattern = normalize_path(pattern);
    let matcher = Glob::new(&pattern)
        .map_err(|source| SchemaSelectionError::InvalidGlob {
            pattern: pattern.clone(),
            source,
        })?
        .compile_matcher();
    Ok(path_suffixes(path).any(|candidate| matcher.is_match(candidate)))
}

fn path_suffixes(path: &str) -> impl Iterator<Item = &str> {
    let path = path.trim_start_matches('/');
    std::iter::successors(Some(path), |current| {
        current.split_once('/').map(|(_, suffix)| suffix)
    })
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn literal_pattern_len(pattern: &str) -> usize {
    pattern
        .chars()
        .filter(|character| !matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
        .count()
}

fn document_directives(
    input: &str,
    document_count: usize,
) -> Result<Vec<DocumentDirectives>, SchemaSelectionError> {
    let cst = parse_to_cst(input);
    let mut tokens = Vec::new();
    collect_tokens(&cst, &mut tokens);

    let mut directives = vec![DocumentDirectives::default(); document_count];
    let mut document: usize = 0;
    let mut has_start_marker = false;
    let mut before_semantic = true;
    let mut ended = false;

    for token in tokens {
        match token.kind {
            TokenKind::DocumentStart => {
                if has_start_marker || !before_semantic || ended {
                    document = document.saturating_add(1);
                }
                if document >= document_count {
                    break;
                }
                has_start_marker = true;
                before_semantic = true;
                ended = false;
            }
            TokenKind::DocumentEnd => {
                before_semantic = false;
                ended = true;
            }
            TokenKind::Comment if before_semantic && !ended && document < document_count => {
                parse_comment(token.text.as_ref(), document, &mut directives[document])?;
            }
            TokenKind::Whitespace
            | TokenKind::Comment
            | TokenKind::VersionDirective
            | TokenKind::TagDirective => {}
            _ => before_semantic = false,
        }
    }

    Ok(directives)
}

fn collect_tokens<'tree, 'input>(
    node: &'tree CstNode<'input>,
    output: &mut Vec<&'tree CstToken<'input>>,
) {
    for child in node.children() {
        match child {
            CstElement::Node(node) => collect_tokens(node, output),
            CstElement::Token(token) => output.push(token),
        }
    }
}

fn parse_comment(
    comment: &str,
    document: usize,
    directives: &mut DocumentDirectives,
) -> Result<(), SchemaSelectionError> {
    let comment = comment.strip_prefix('#').unwrap_or(comment).trim();
    if let Some(value) = comment.strip_prefix("yg-schema-validate:") {
        let value = directive_value(value, document, "pragma")?;
        return record_directive(&mut directives.pragma, value, document, "pragma");
    }
    if let Some(value) = comment.strip_prefix("$schema:") {
        let value = directive_value(value, document, "modeline")?;
        return record_directive(&mut directives.modeline, value, document, "modeline");
    }
    if let Some(value) = comment
        .strip_prefix("yaml-language-server:")
        .and_then(|value| value.trim_start().strip_prefix("$schema="))
    {
        let value = directive_value(value, document, "modeline")?;
        return record_directive(&mut directives.modeline, value, document, "modeline");
    }
    Ok(())
}

fn directive_value(
    value: &str,
    document: usize,
    directive: &'static str,
) -> Result<String, SchemaSelectionError> {
    let mut tokens = value.split_whitespace();
    let Some(uri) = tokens.next() else {
        return Err(SchemaSelectionError::MalformedDirective {
            document,
            directive,
        });
    };
    if tokens.next().is_some() {
        return Err(SchemaSelectionError::MalformedDirective {
            document,
            directive,
        });
    }
    Ok(uri.to_owned())
}

fn record_directive(
    slot: &mut Option<String>,
    value: String,
    document: usize,
    directive: &'static str,
) -> Result<(), SchemaSelectionError> {
    if slot.as_ref().is_some_and(|current| current != &value) {
        return Err(SchemaSelectionError::ConflictingDirectives {
            document,
            directive,
        });
    }
    *slot = Some(value);
    Ok(())
}
