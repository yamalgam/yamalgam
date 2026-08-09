//! Built-in external-resolution event middleware.

use std::collections::VecDeque;
use std::mem;

use yamalgam_core::{LoaderConfig, ResolutionCapability};
use yamalgam_parser::{Event, ResolveError, ResolvedEventBuf, Resolver};

use crate::{
    DocumentError, OwnedDocument, ResolutionReport, ResolutionSession, ResourceId,
    ResourceRetriever, SessionError,
};

/// Yamalgam's `$ref` and `!include` resolver over the parser event contract.
pub struct BuiltinResolver<R> {
    session: ResolutionSession<R>,
    root: ResourceId,
    state: ResolverState,
    completed_reports: Vec<ResolutionReport>,
}

enum ResolverState {
    Idle,
    Buffering(Vec<Event<'static>>),
    Failed,
}

impl<R: ResourceRetriever> BuiltinResolver<R> {
    /// Create a resolver for one source stream and loader policy.
    #[must_use]
    pub fn new(retriever: R, root: ResourceId, config: LoaderConfig) -> Self {
        Self::with_session(ResolutionSession::new(retriever, config), root)
    }

    /// Create a resolver from an existing empty resolution session.
    #[must_use]
    pub const fn with_session(session: ResolutionSession<R>, root: ResourceId) -> Self {
        Self {
            session,
            root,
            state: ResolverState::Idle,
            completed_reports: Vec::new(),
        }
    }

    /// Inspect reports for every completed top-level document.
    #[must_use]
    pub fn reports(&self) -> &[ResolutionReport] {
        &self.completed_reports
    }

    /// Consume the resolver and return every completed report.
    #[must_use]
    pub fn into_reports(self) -> Vec<ResolutionReport> {
        self.completed_reports
    }

    fn process<'input>(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        if config != self.session.config() {
            return Err(ResolveError::Custom(Box::new(
                SessionError::LoaderConfigMismatch,
            )));
        }
        if !automatically_resolves(config) {
            return Ok(yamalgam_parser::resolve::smallvec![event]);
        }
        if matches!(self.state, ResolverState::Failed) {
            return Ok(ResolvedEventBuf::new());
        }
        if matches!(event, Event::StreamStart | Event::StreamEnd) {
            return Ok(yamalgam_parser::resolve::smallvec![event]);
        }

        let is_document_end = matches!(event, Event::DocumentEnd { .. });
        let owned = event.into_owned();
        match &mut self.state {
            ResolverState::Idle => self.state = ResolverState::Buffering(vec![owned]),
            ResolverState::Buffering(events) => events.push(owned),
            ResolverState::Failed => unreachable!("failed state returned above"),
        }
        if !is_document_end {
            return Ok(ResolvedEventBuf::new());
        }

        let ResolverState::Buffering(events) = mem::replace(&mut self.state, ResolverState::Idle)
        else {
            unreachable!("document end always has a buffer")
        };
        let result = OwnedDocument::from_events(self.root.clone(), events.iter().cloned())
            .map_err(document_error)
            .and_then(|document| self.session.resolve_root(document));
        self.completed_reports
            .push(self.session.reset_for_next_root());
        match result {
            Ok(document) => Ok(replay_document(events, &document)),
            Err(error) => {
                self.state = ResolverState::Failed;
                Err(error)
            }
        }
    }
}

impl<'input, R: ResourceRetriever> Resolver<'input> for BuiltinResolver<R> {
    fn on_event(&mut self, event: Event<'input>) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        let config = self.session.config().clone();
        self.process(event, &config)
    }

    fn on_event_with_config(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<ResolvedEventBuf<'input>, ResolveError> {
        self.process(event, config)
    }
}

const fn automatically_resolves(config: &LoaderConfig) -> bool {
    config
        .resolution
        .resolves_automatically(ResolutionCapability::Include)
        || config
            .resolution
            .resolves_automatically(ResolutionCapability::Ref)
}

fn document_error(error: DocumentError) -> ResolveError {
    ResolveError::Custom(Box::new(error))
}

fn replay_document<'input>(
    original: Vec<Event<'static>>,
    document: &OwnedDocument,
) -> ResolvedEventBuf<'input> {
    let document_start = original
        .iter()
        .position(|event| matches!(event, Event::DocumentStart { .. }))
        .expect("parser document has a start event");
    let document_end = original
        .iter()
        .rposition(|event| matches!(event, Event::DocumentEnd { .. }))
        .expect("parser document has an end event");
    let mut output = ResolvedEventBuf::new();
    output.extend(original[..=document_start].iter().cloned());

    let mut source = VecDeque::from(original[document_start + 1..document_end].to_vec());
    for resolved in document.root.to_events() {
        if resolved.span().is_some_and(|span| span.start == span.end) {
            flush_through_synthetic(&mut source, &mut output, &resolved);
        } else {
            flush_through_match(&mut source, &mut output, &resolved);
        }
        output.push(resolved);
    }
    output.extend(source.into_iter().filter(Event::is_structural));
    output.extend(original[document_end..].iter().cloned());
    output
}

fn flush_through_synthetic<'input>(
    source: &mut VecDeque<Event<'static>>,
    output: &mut ResolvedEventBuf<'input>,
    synthetic: &Event<'_>,
) {
    let Some(offset) = synthetic.span().map(|span| span.start.offset) else {
        return;
    };
    while source
        .front()
        .and_then(Event::span)
        .is_some_and(|span| span.start.offset <= offset)
    {
        let event = source.pop_front().expect("front event exists");
        if event.is_structural() {
            output.push(event);
        }
    }
}

fn flush_through_match<'input>(
    source: &mut VecDeque<Event<'static>>,
    output: &mut ResolvedEventBuf<'input>,
    resolved: &Event<'_>,
) {
    while let Some(event) = source.pop_front() {
        if event.is_structural() {
            output.push(event);
        } else if same_semantic_event(&event, resolved) {
            break;
        }
    }
}

fn same_semantic_event(left: &Event<'_>, right: &Event<'_>) -> bool {
    mem::discriminant(left) == mem::discriminant(right) && left.span() == right.span()
}
