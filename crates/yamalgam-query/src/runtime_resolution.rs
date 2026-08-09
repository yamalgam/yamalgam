//! Query resolution session implementation and jaq error side channel.

use std::cell::RefCell;

use yamalgam_core::{LoaderConfig, ResolutionCapability};
use yamalgam_parser::{ResolutionErrorCategory, ResolveError};
use yamalgam_resolve::{
    OwnedDocument, ResolutionReport, ResolutionSession, ResourceId, ResourceRetriever,
};

use crate::resolution::{QueryResolver, ResolutionSessionKey};
use crate::value::YamlVal;

#[cfg(test)]
thread_local! {
    static SESSION_CREATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub struct SessionQueryResolver<R> {
    session: ResolutionSession<R>,
    base: Option<ResourceId>,
    config: LoaderConfig,
}

impl<R: ResourceRetriever> SessionQueryResolver<R> {
    pub fn new(retriever: R, config: LoaderConfig, base: Option<ResourceId>) -> Self {
        #[cfg(test)]
        SESSION_CREATIONS.set(SESSION_CREATIONS.get().saturating_add(1));
        Self {
            session: ResolutionSession::new(retriever, config.clone()),
            base,
            config,
        }
    }

    #[allow(clippy::result_large_err)] // ResolutionSession uses the shared typed error contract.
    pub fn resolve_input(
        &mut self,
        document: OwnedDocument,
    ) -> Result<OwnedDocument, ResolveError> {
        if automatically_resolves(&self.config) {
            self.session.resolve_root(document)
        } else {
            let origin = document.root.origin.clone();
            self.session.prime_root(origin, document.clone());
            Ok(document)
        }
    }
}

#[cfg(test)]
pub fn reset_session_creations() {
    SESSION_CREATIONS.set(0);
}

#[cfg(test)]
pub fn session_creations() -> usize {
    SESSION_CREATIONS.get()
}

impl<R: ResourceRetriever> QueryResolver for SessionQueryResolver<R> {
    #[allow(clippy::result_large_err)] // The accepted public contract returns ResolveError directly.
    fn resolve_value(
        &mut self,
        capability: ResolutionCapability,
        value: YamlVal,
    ) -> Result<YamlVal, ResolveError> {
        if !self.config.resolution.allows_explicit(capability) {
            return Err(resolution_error(
                ResolutionErrorCategory::DisabledCapability,
                capability,
                "<query value>",
            ));
        }
        let base = value
            .meta()
            .and_then(|metadata| metadata.origin.as_ref())
            .map(|origin| origin.resource.clone())
            .or_else(|| self.base.clone())
            .ok_or_else(|| {
                resolution_error(
                    ResolutionErrorCategory::MissingBase,
                    capability,
                    "<query value>",
                )
            })?;
        let node = crate::resolution::value_to_owned_node(value, &base, capability)?;
        let node = self.session.resolve_explicit(capability, node)?;
        crate::resolution::value_from_owned_node(
            node,
            ResolutionSessionKey::UNREGISTERED,
            &self.config,
        )
        .map_err(|error| ResolveError::Custom(Box::new(error)))
    }

    fn report(&self) -> &ResolutionReport {
        self.session.report()
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

pub fn resolution_error(
    category: ResolutionErrorCategory,
    capability: ResolutionCapability,
    target: &str,
) -> ResolveError {
    ResolveError::Resolution {
        category,
        capability,
        span: Default::default(),
        target: target.into(),
        reference_chain: Vec::new(),
        source: None,
    }
}

thread_local! {
    static PENDING_ERROR: RefCell<Option<ResolveError>> = const { RefCell::new(None) };
}

pub struct Guard {
    pending: Option<ResolveError>,
}

pub fn activate() -> Guard {
    Guard {
        pending: PENDING_ERROR.with(|error| error.replace(None)),
    }
}

pub fn jaq_error(error: ResolveError) -> jaq_core::Error<YamlVal> {
    let message = error.to_string();
    PENDING_ERROR.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending.is_none() {
            *pending = Some(error);
        }
    });
    jaq_core::Error::str(format_args!("external resolution failed: {message}"))
}

pub fn take_error() -> Option<ResolveError> {
    PENDING_ERROR.with(|error| error.borrow_mut().take())
}

impl Drop for Guard {
    fn drop(&mut self) {
        PENDING_ERROR.with(|error| {
            error.replace(self.pending.take());
        });
    }
}
