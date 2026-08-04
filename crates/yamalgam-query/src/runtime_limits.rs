//! Scoped resource limits for jaq trait methods that do not receive `RunCtx`.

use std::cell::RefCell;

use yamalgam_core::{LoaderConfig, ResourceLimits};

thread_local! {
    static ACTIVE: RefCell<ResourceLimits> =
        RefCell::new(LoaderConfig::moderate().limits);
}

pub struct Guard(ResourceLimits);

pub fn activate(limits: &ResourceLimits) -> Guard {
    let previous = ACTIVE.with(|active| active.replace(limits.clone()));
    Guard(previous)
}

pub fn check_collection_size(size: usize) -> Result<(), String> {
    ACTIVE.with(|active| active.borrow().check_collection_size(size))
}

pub fn check_scalar_size(size: usize) -> Result<(), String> {
    ACTIVE.with(|active| active.borrow().check_scalar_size(size))
}

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            active.replace(self.0.clone());
        });
    }
}
