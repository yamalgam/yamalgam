//! Implementation-neutral event representation for cross-implementation comparison.

use serde::{Deserialize, Serialize};

/// Implementation-neutral event representation for cross-implementation comparison.
///
/// Uses strings rather than enum types so it can represent events from both
/// the Rust parser and the C harness without coupling to either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSnapshot {
    /// Event type name (e.g., `"StreamStart"`, `"Scalar"`, `"MappingStart"`).
    pub kind: String,
    /// Anchor name if present (without `&`).
    pub anchor: Option<String>,
    /// Tag if present.
    pub tag: Option<String>,
    /// Scalar value or alias name.
    pub value: Option<String>,
    /// Whether document start/end is implicit.
    pub implicit: Option<bool>,
}
