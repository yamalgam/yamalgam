//! Query resolution session implementation and jaq error side channel.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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

    /// Consume the adapter and return the report accumulated by its session.
    #[must_use]
    pub fn into_report(self) -> ResolutionReport {
        self.session.into_report()
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
        let node =
            crate::resolution::value_to_owned_node(value, &base, capability, &self.config.limits)?;
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
    static PENDING_ERRORS: RefCell<HashMap<u64, ResolveError>> = RefCell::new(HashMap::new());
    static ACTIVE_GUARDS: Cell<usize> = const { Cell::new(0) };
}

static NEXT_ERROR_TOKEN: AtomicU64 = AtomicU64::new(1);

pub struct Guard {
    pending: HashMap<u64, ResolveError>,
}

pub fn activate() -> Guard {
    ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_add(1)));
    Guard {
        pending: PENDING_ERRORS.with(|errors| errors.replace(HashMap::new())),
    }
}

pub fn jaq_error(error: ResolveError) -> jaq_core::Error<YamlVal> {
    let message = error.to_string();
    let mut value = YamlVal::str(format!("external resolution failed: {message}"));
    let token = ACTIVE_GUARDS.with(|guards| guards.get() > 0).then(|| {
        NEXT_ERROR_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .ok()
    });
    let token = token.flatten();
    if let Some(token) = token {
        value.runtime_error_token = Some(token);
        PENDING_ERRORS.with(|errors| {
            errors.borrow_mut().insert(token, error);
        });
    }
    jaq_core::Error::new(value)
}

pub fn take_error(value: &YamlVal) -> Option<ResolveError> {
    let token = value.runtime_error_token?;
    PENDING_ERRORS.with(|errors| errors.borrow_mut().remove(&token))
}

impl Drop for Guard {
    fn drop(&mut self) {
        PENDING_ERRORS.with(|errors| {
            errors.replace(std::mem::take(&mut self.pending));
        });
        ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_sub(1)));
    }
}
