//! Local-only navigation for includes, data references, and schema references.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use lsp_types::{GotoDefinitionResponse, LocationLink, Position, Range, Uri};
use yamalgam_core::{FilePolicy, LoaderConfig, Mark, ResolutionCapability, Span};
use yamalgam_resolve::{
    FileRetriever, Fragment, ResolutionEdge, ResolutionErrorCategory, ResolutionOutcome,
    ResolutionReport, ResourceId, ResourceRequest, ResourceRetriever,
};

use crate::{DocumentState, SchemaWorkspace};

/// One `$ref` token which may resolve through the active schema graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaReferenceSite {
    /// Exact reference value span in the edited document.
    pub source_span: Span,
    /// Active schema URI for the containing YAML document.
    pub schema_uri: String,
    /// Decoded reference value.
    pub reference: String,
}

/// Editor navigation facts collected for one document snapshot.
#[derive(Clone, Debug, Default)]
pub struct NavigationIndex {
    /// Local M11-style resolution edges.
    pub resolution_report: ResolutionReport,
    /// Schema references which can use `SchemaValidator` navigation.
    pub schema_refs: Vec<SchemaReferenceSite>,
}

/// Open and explicitly cached resources visible to definition navigation.
pub struct WorkspaceState {
    schemas: Arc<SchemaWorkspace>,
    roots: Vec<PathBuf>,
    resources: HashMap<ResourceId, (Uri, String)>,
}

impl WorkspaceState {
    /// Create a local-only workspace over shared schema state.
    #[must_use]
    pub fn new(schemas: Arc<SchemaWorkspace>, roots: Vec<PathBuf>) -> Self {
        Self {
            schemas,
            roots,
            resources: HashMap::new(),
        }
    }

    /// Insert or replace one already-open or already-cached resource.
    pub fn insert_resource(&mut self, uri: Uri, text: String) {
        if let Ok(id) = ResourceId::new(uri.as_str()) {
            self.resources.insert(id, (uri, text));
        }
    }

    /// Add one open document snapshot to the workspace view.
    pub fn insert_document(&mut self, document: &DocumentState) {
        self.insert_resource(document.uri.clone(), document.syntax.text());
    }

    fn resource(&self, id: &ResourceId) -> Option<(Uri, String)> {
        if let Some((uri, text)) = self.resources.get(id) {
            return Some((uri.clone(), text.clone()));
        }
        let path = id.as_path()?;
        for root in &self.roots {
            if !path.starts_with(root) {
                continue;
            }
            let retriever = FileRetriever::open(&FilePolicy {
                root: Some(root.clone()),
                allow: Vec::new(),
                deny: Vec::new(),
                follow_symlinks: false,
            })
            .ok()?;
            let resource = retriever
                .retrieve(&ResourceRequest {
                    id: id.clone(),
                    max_bytes: Some(16 * 1024 * 1024),
                })
                .ok()?;
            let text = String::from_utf8(resource.bytes).ok()?;
            return resource_uri(&resource.id).map(|uri| (uri, text));
        }
        None
    }
}

/// Build a deterministic local navigation report for one snapshot.
#[must_use]
pub fn navigation_index(document: &DocumentState, workspace: &WorkspaceState) -> NavigationIndex {
    let text = document.syntax.text();
    let sites = reference_sites(document, &text);
    let base = ResourceId::new(document.uri.as_str()).ok();
    let mut index = NavigationIndex::default();
    for site in sites {
        if site.kind == ReferenceKind::DataRef
            && let Some(selected) = document.schema_for_document(site.document_index)
        {
            index.schema_refs.push(SchemaReferenceSite {
                source_span: span_from_offsets(&text, site.start, site.end),
                schema_uri: selected.uri.clone(),
                reference: site.requested.clone(),
            });
        }
        let capability = match site.kind {
            ReferenceKind::Include => ResolutionCapability::Include,
            ReferenceKind::DataRef => ResolutionCapability::Ref,
        };
        let resolved = base
            .as_ref()
            .and_then(|base| base.resolve(&site.requested).ok());
        let (child, fragment, outcome) = match resolved {
            Some(resolved) => {
                let available = base.as_ref() == Some(&resolved.resource)
                    || workspace.resource(&resolved.resource).is_some();
                (
                    Some(resolved.resource),
                    resolved.fragment,
                    if available {
                        ResolutionOutcome::Resolved
                    } else {
                        ResolutionOutcome::Failed(ResolutionErrorCategory::Retrieval)
                    },
                )
            }
            None => (
                None,
                Fragment::Root,
                ResolutionOutcome::Failed(ResolutionErrorCategory::InvalidResource),
            ),
        };
        index.resolution_report.record_edge(ResolutionEdge {
            parent: base.clone().unwrap_or_else(ResourceId::anonymous),
            child,
            capability,
            source_span: span_from_offsets(&text, site.start, site.end),
            requested: site.requested,
            fragment,
            matches: Vec::new(),
            cache_hit: false,
            fetched_bytes: 0,
            materialized_bytes: 0,
            outcome,
        });
    }
    index
}

/// Navigate the reference under one UTF-16 cursor position.
#[must_use]
pub fn definition(
    document: &DocumentState,
    position: Position,
    workspace: &WorkspaceState,
) -> Option<GotoDefinitionResponse> {
    let offset = document.offset_at(position)?;
    let text = document.syntax.text();
    let site = reference_sites(document, &text)
        .into_iter()
        .find(|site| site.start <= offset && offset <= site.end)?;
    let origin = byte_range(document, site.start, site.end)?;

    if site.kind == ReferenceKind::DataRef
        && let Some(selected) = document.schema_for_document(site.document_index)
        && let Ok(validator) = workspace.schemas.validator(selected, 64)
        && let Some(location) = validator.definition_for_ref(&selected.uri, &site.requested)
    {
        let uri = Uri::from_str(&location.uri).ok()?;
        let range = location.span.map_or_else(zero_range, |span| {
            schema_location_id(&location.uri)
                .and_then(|id| workspace.resource(&id))
                .map_or_else(zero_range, |(_, target)| span_range(&target, span))
        });
        return Some(link(origin, uri, range, range));
    }

    let base = ResourceId::new(document.uri.as_str()).ok()?;
    let resolved = base.resolve(&site.requested).ok()?;
    let (target_uri, target_text) = if resolved.resource == base {
        (document.uri.clone(), text)
    } else {
        workspace.resource(&resolved.resource)?
    };
    let target_range = full_range(&target_text);
    let selection = match resolved.fragment {
        Fragment::Root => zero_range(),
        Fragment::Pointer(pointer) => {
            let pointer = pointer.tokens().iter().fold(String::new(), |path, token| {
                format!("{path}/{}", token.replace('~', "~0").replace('/', "~1"))
            });
            let located = yamalgam_compose::from_str_located_with_config(
                &target_text,
                &LoaderConfig::default(),
            )
            .ok()?;
            let span = located.first()?.span_at_pointer(&pointer)?;
            span_range(&target_text, span)
        }
    };
    Some(link(origin, target_uri, target_range, selection))
}

fn link(
    origin: Range,
    target_uri: Uri,
    target_range: Range,
    target_selection_range: Range,
) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Link(vec![LocationLink {
        origin_selection_range: Some(origin),
        target_uri,
        target_range,
        target_selection_range,
    }])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReferenceKind {
    Include,
    DataRef,
}

#[derive(Clone, Debug)]
struct ReferenceSite {
    kind: ReferenceKind,
    document_index: usize,
    requested: String,
    start: usize,
    end: usize,
}

fn reference_sites(document: &DocumentState, text: &str) -> Vec<ReferenceSite> {
    let mut sites = Vec::new();
    let mut line_start = 0;
    let mut structured_include_indent = None;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let trimmed = content.trim_start();
        let indent = content.len() - trimmed.len();
        if trimmed.starts_with("---") {
            structured_include_indent = None;
        }
        if let Some(marker) = content.find("!include") {
            let after = marker + "!include".len();
            if let Some((requested, start, end)) = scalar_after(content, after) {
                sites.push(ReferenceSite {
                    kind: ReferenceKind::Include,
                    document_index: document.document_index_at_offset(line_start + start),
                    requested,
                    start: line_start + start,
                    end: line_start + end,
                });
            } else {
                structured_include_indent = Some(indent);
            }
        } else if structured_include_indent.is_some_and(|parent| indent > parent)
            && trimmed.starts_with("path:")
        {
            if let Some(colon) = content.find(':')
                && let Some((requested, start, end)) = scalar_after(content, colon + 1)
            {
                sites.push(ReferenceSite {
                    kind: ReferenceKind::Include,
                    document_index: document.document_index_at_offset(line_start + start),
                    requested,
                    start: line_start + start,
                    end: line_start + end,
                });
            }
        } else if structured_include_indent.is_some_and(|parent| indent <= parent)
            && !trimmed.is_empty()
        {
            structured_include_indent = None;
        }
        if let Some(marker) = content.find("$ref")
            && let Some(colon) = content[marker + 4..].find(':')
            && let Some((requested, start, end)) = scalar_after(content, marker + 4 + colon + 1)
        {
            sites.push(ReferenceSite {
                kind: ReferenceKind::DataRef,
                document_index: document.document_index_at_offset(line_start + start),
                requested,
                start: line_start + start,
                end: line_start + end,
            });
        }
        line_start += line.len();
    }
    sites
}

fn scalar_after(line: &str, after: usize) -> Option<(String, usize, usize)> {
    let remainder = &line[after..];
    let whitespace = remainder.len() - remainder.trim_start().len();
    let start = after + whitespace;
    let value = &line[start..];
    let quote = value
        .chars()
        .next()
        .filter(|character| matches!(character, '\'' | '"'));
    let (start, value) = quote.map_or((start, value), |quote| {
        (start + quote.len_utf8(), &value[1..])
    });
    let length = if let Some(quote) = quote {
        value.find(quote)?
    } else {
        value
            .find(|character: char| {
                character.is_whitespace() || matches!(character, ',' | '}' | ']')
            })
            .unwrap_or(value.len())
    };
    (length > 0).then(|| (value[..length].to_owned(), start, start + length))
}

fn schema_location_id(uri: &str) -> Option<ResourceId> {
    ResourceId::new(uri.split_once('#').map_or(uri, |(base, _)| base)).ok()
}

fn resource_uri(id: &ResourceId) -> Option<Uri> {
    if let Some(uri) = id.as_url() {
        return Uri::from_str(uri.as_str()).ok();
    }
    let path = id.as_path()?;
    Uri::from_str(&format!("file://{}", path.display())).ok()
}

fn byte_range(document: &DocumentState, start: usize, end: usize) -> Option<Range> {
    Some(Range {
        start: document
            .syntax
            .byte_offset_to_utf16(start)
            .map(|(line, column)| Position::new(line, column))?,
        end: document
            .syntax
            .byte_offset_to_utf16(end)
            .map(|(line, column)| Position::new(line, column))?,
    })
}

fn span_range(text: &str, span: Span) -> Range {
    Range {
        start: position_at(text, span.start.offset),
        end: position_at(text, span.end.offset),
    }
}

fn full_range(text: &str) -> Range {
    Range {
        start: Position::new(0, 0),
        end: position_at(text, text.len()),
    }
}

fn zero_range() -> Range {
    Range::new(Position::new(0, 0), Position::new(0, 0))
}

fn position_at(text: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;
    for value in text[..offset.min(text.len())].chars() {
        if value == '\n' {
            line += 1;
            character = 0;
        } else {
            character += value.len_utf16() as u32;
        }
    }
    Position::new(line, character)
}

fn span_from_offsets(text: &str, start: usize, end: usize) -> Span {
    Span {
        start: mark_at(text, start),
        end: mark_at(text, end),
    }
}

fn mark_at(text: &str, offset: usize) -> Mark {
    let mut mark = Mark::default();
    for value in text[..offset.min(text.len())].chars() {
        mark.offset += value.len_utf8();
        if value == '\n' {
            mark.line += 1;
            mark.column = 0;
        } else {
            mark.column += 1;
        }
    }
    mark
}
