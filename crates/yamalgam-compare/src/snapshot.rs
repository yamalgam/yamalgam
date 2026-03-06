//! Implementation-neutral token and span representations for cross-implementation comparison.

use serde::{Deserialize, Serialize};

/// Implementation-neutral token representation for cross-implementation comparison.
///
/// Uses strings rather than enum types so it can represent tokens from both
/// the Rust scanner and the C harness without coupling to either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSnapshot {
    /// Token type name (e.g., `"Scalar"`, `"StreamStart"`, `"BlockMappingStart"`).
    pub kind: String,
    /// Text content for scalars, anchors, aliases, tags. `None` for structural tokens.
    pub value: Option<String>,
    /// Scalar style if applicable (e.g., `"Plain"`, `"DoubleQuoted"`).
    pub style: Option<String>,
    /// Source location.
    pub span: SpanSnapshot,
}

/// Implementation-neutral source location.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSnapshot {
    /// Zero-indexed start line.
    pub line: u32,
    /// Zero-indexed start column.
    pub column: u32,
    /// Start byte offset.
    pub offset: usize,
    /// Zero-indexed end line.
    pub end_line: u32,
    /// Zero-indexed end column.
    pub end_column: u32,
    /// End byte offset.
    pub end_offset: usize,
}
