//! Scoped resource limits for jaq trait methods that do not receive `RunCtx`.
//!
//! Jaq's sequence-construction boundary returns a value rather than a
//! `Result`. Those constructors stop before exceeding the active limit and
//! record the first violation in `PENDING_ERROR`; the engine drains that slot
//! before any partially constructed value reaches an output visitor.

use std::cell::{Cell, RefCell};

use yamalgam_core::{LimitExceeded, LoaderConfig, ResourceLimits};

pub enum PendingError {
    Limit(LimitExceeded),
    ResourceOverflow(&'static str),
    Message(String),
}

thread_local! {
    static ACTIVE: RefCell<ResourceLimits> =
        RefCell::new(LoaderConfig::moderate().limits);
    static ACTIVE_GUARDS: Cell<usize> = const { Cell::new(0) };
    static PENDING_ERROR: RefCell<Option<PendingError>> = const { RefCell::new(None) };
}

pub struct Guard {
    limits: ResourceLimits,
    pending_error: Option<PendingError>,
}

pub fn activate(limits: &ResourceLimits) -> Guard {
    let limits = ACTIVE.with(|active| active.replace(limits.clone()));
    let pending_error = PENDING_ERROR.with(|error| error.replace(None));
    ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_add(1)));
    Guard {
        limits,
        pending_error,
    }
}

pub fn current() -> ResourceLimits {
    ACTIVE.with(|active| active.borrow().clone())
}

pub fn check_collection_size(size: usize) -> Result<(), LimitExceeded> {
    ACTIVE.with(|active| active.borrow().check_collection_size(size))
}

pub fn check_scalar_size(size: usize) -> Result<(), LimitExceeded> {
    ACTIVE.with(|active| active.borrow().check_scalar_size(size))
}

pub fn record_error(message: String) -> bool {
    record(PendingError::Message(message))
}

pub fn record_limit(error: LimitExceeded) -> bool {
    record(PendingError::Limit(error))
}

pub fn record_overflow(resource: &'static str) -> bool {
    record(PendingError::ResourceOverflow(resource))
}

pub fn jaq_limit_error<V: From<String>>(error: LimitExceeded) -> jaq_core::Error<V> {
    record_limit(error);
    jaq_core::Error::str(error)
}

pub fn jaq_overflow_error<V: From<String>>(resource: &'static str) -> jaq_core::Error<V> {
    record_overflow(resource);
    jaq_core::Error::str(format!("{resource} overflow"))
}

pub fn jaq_value_error<V: From<String>>(
    error: crate::value::ValueLimitError,
) -> jaq_core::Error<V> {
    match error {
        crate::value::ValueLimitError::Limit(error) => jaq_limit_error(error),
        crate::value::ValueLimitError::ResourceOverflow(resource) => jaq_overflow_error(resource),
    }
}

fn record(pending: PendingError) -> bool {
    if !ACTIVE_GUARDS.with(|guards| guards.get() > 0) {
        return false;
    }
    PENDING_ERROR.with(|error| {
        let mut error = error.borrow_mut();
        if error.is_none() {
            *error = Some(pending);
        }
    });
    true
}

pub fn take_error() -> Option<PendingError> {
    PENDING_ERROR.with(|error| error.borrow_mut().take())
}

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            active.replace(self.limits.clone());
        });
        PENDING_ERROR.with(|error| {
            error.replace(self.pending_error.take());
        });
        ACTIVE_GUARDS.with(|guards| guards.set(guards.get().saturating_sub(1)));
    }
}
