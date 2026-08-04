//! Scoped resource limits for jaq trait methods that do not receive `RunCtx`.

use std::cell::{Cell, RefCell};

use yamalgam_core::{LoaderConfig, ResourceLimits};

thread_local! {
    static ACTIVE: RefCell<ResourceLimits> =
        RefCell::new(LoaderConfig::moderate().limits);
    static ACTIVE_GUARDS: Cell<usize> = const { Cell::new(0) };
    static PENDING_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub struct Guard {
    limits: ResourceLimits,
    pending_error: Option<String>,
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

pub fn check_collection_size(size: usize) -> Result<(), String> {
    ACTIVE.with(|active| active.borrow().check_collection_size(size))
}

pub fn check_scalar_size(size: usize) -> Result<(), String> {
    ACTIVE.with(|active| active.borrow().check_scalar_size(size))
}

pub fn record_error(message: String) -> bool {
    if !ACTIVE_GUARDS.with(|guards| guards.get() > 0) {
        return false;
    }
    PENDING_ERROR.with(|error| {
        let mut error = error.borrow_mut();
        if error.is_none() {
            *error = Some(message);
        }
    });
    true
}

pub fn take_error() -> Option<String> {
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
