//! Loader configuration for resource limits and security policy.
//!
//! [`LoaderConfig`] is the central type that governs how yamalgam loads and
//! processes YAML documents. It controls two concerns:
//!
//! - **Resource limits** — bounds on input size, nesting depth, alias
//!   expansions, and other vectors for denial-of-service attacks.
//! - **Resolution policy** — whether `!include` directives and `$ref`
//!   references may be followed, and under what constraints. Ordinary
//!   high-level loading paths reject enabled policy until built-in resolvers
//!   ship; resolver-aware paths delegate enforcement to the supplied resolver.
//!
//! Four preset constructors cover common use cases:
//!
//! | Preset | Use case |
//! |--------|----------|
//! | [`moderate()`](LoaderConfig::moderate) | Default — balanced limits for typical workloads |
//! | [`strict()`](LoaderConfig::strict) | Untrusted input — tight caps on everything |
//! | [`trusted()`](LoaderConfig::trusted) | Local files you control — generous limits |
//! | [`unchecked()`](LoaderConfig::unchecked) | No limits at all |
//!
//! See [ADR-0006](../../record/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)
//! for design rationale.

use std::path::PathBuf;
use std::time::Duration;

use crate::tag_resolution::TagResolution;

// ---------------------------------------------------------------------------
// LoaderConfig
// ---------------------------------------------------------------------------

/// Top-level loader configuration.
///
/// Combines [`ResourceLimits`] (caps on sizes, depth, expansions) with
/// [`ResolutionPolicy`] (include/ref handling).
///
/// The [`Default`] implementation returns [`LoaderConfig::moderate()`].
#[derive(Debug, Clone, PartialEq, Eq)]
// y[impl model.loading.well-formed]
// y[impl model.loading.reject-ill-formed]
pub struct LoaderConfig {
    /// Resource limits applied during loading.
    pub limits: ResourceLimits,
    /// Policy for resolving `!include` and `$ref` references.
    pub resolution: ResolutionPolicy,
    /// Tag resolution scheme for plain scalar typing.
    pub tag_resolution: TagResolution,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self::moderate()
    }
}

impl LoaderConfig {
    /// Balanced limits suitable for most workloads.
    ///
    /// - `max_depth` = 256
    /// - `max_input_bytes` = 256 MB
    /// - `max_scalar_bytes` = 64 MB
    /// - `max_key_bytes` = 1 MB
    /// - `max_alias_expansions` = 10 000
    /// - `max_anchor_count` = 10 000
    /// - `max_merge_depth` = 64
    /// - `max_collection_size` = 1 000 000
    /// - `max_output_bytes` = 256 MB
    /// - Resolution disabled.
    #[must_use]
    pub const fn moderate() -> Self {
        Self {
            limits: ResourceLimits::moderate(),
            resolution: ResolutionPolicy::moderate(),
            tag_resolution: TagResolution::Yaml12,
        }
    }

    /// Tight limits for untrusted input.
    ///
    /// - `max_depth` = 64
    /// - `max_input_bytes` = 10 MB
    /// - `max_scalar_bytes` = 1 MB
    /// - `max_key_bytes` = 4 KB
    /// - `max_alias_expansions` = 100
    /// - `max_anchor_count` = 100
    /// - `max_merge_depth` = 10
    /// - `max_collection_size` = 100 000
    /// - `max_output_bytes` = 10 MB
    /// - Resolution disabled.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: Some(10 * 1024 * 1024),
                max_scalar_bytes: Some(1024 * 1024),
                max_key_bytes: Some(4 * 1024),
                max_depth: Some(64),
                max_alias_expansions: Some(100),
                max_anchor_count: Some(100),
                max_merge_depth: Some(10),
                max_collection_size: Some(100_000),
                max_output_bytes: Some(10 * 1024 * 1024),
            },
            resolution: ResolutionPolicy::strict(),
            tag_resolution: TagResolution::Yaml12,
        }
    }

    /// Generous limits for local files you control.
    ///
    /// - `max_depth` = 1024
    /// - `max_input_bytes` = None (unlimited)
    /// - `max_scalar_bytes` = None (unlimited)
    /// - `max_key_bytes` = None (unlimited)
    /// - `max_alias_expansions` = 1 000 000
    /// - `max_anchor_count` = None (unlimited)
    /// - `max_merge_depth` = 256
    /// - `max_collection_size` = None (unlimited)
    /// - `max_output_bytes` = None (unlimited)
    /// - Resolution disabled.
    #[must_use]
    pub const fn trusted() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: None,
                max_scalar_bytes: None,
                max_key_bytes: None,
                max_depth: Some(1024),
                max_alias_expansions: Some(1_000_000),
                max_anchor_count: None,
                max_merge_depth: Some(256),
                max_collection_size: None,
                max_output_bytes: None,
            },
            resolution: ResolutionPolicy::trusted(),
            tag_resolution: TagResolution::Yaml12,
        }
    }

    /// No limits at all. Use only when you fully trust the input.
    #[must_use]
    pub const fn unchecked() -> Self {
        Self {
            limits: ResourceLimits::none(),
            resolution: ResolutionPolicy::unchecked(),
            tag_resolution: TagResolution::Yaml12,
        }
    }

    /// Set the tag resolution scheme.
    #[must_use]
    pub const fn with_tag_resolution(mut self, tag_resolution: TagResolution) -> Self {
        self.tag_resolution = tag_resolution;
        self
    }
}

// ---------------------------------------------------------------------------
// ResourceLimits
// ---------------------------------------------------------------------------

/// Category of resource governed by a [`ResourceLimits`] check.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceLimit {
    /// Total input size in bytes.
    InputBytes,
    /// Size of one scalar value in bytes.
    ScalarBytes,
    /// Size of one mapping key in bytes.
    KeyBytes,
    /// Structural nesting depth.
    Depth,
    /// Number of alias expansions.
    AliasExpansions,
    /// Number of anchors.
    AnchorCount,
    /// Recursive merge-key depth.
    MergeDepth,
    /// Number of entries in one collection.
    CollectionSize,
    /// Serialized output size in bytes.
    OutputBytes,
    /// External-resolution recursion depth.
    ResolutionDepth,
    /// Number of attempted external-resolution edges.
    ResolutionEdges,
    /// Bytes materialized through external resolution.
    MaterializedBytes,
}

impl std::fmt::Display for ResourceLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InputBytes => "input size",
            Self::ScalarBytes => "scalar size",
            Self::KeyBytes => "key size",
            Self::Depth => "depth",
            Self::AliasExpansions => "alias expansions",
            Self::AnchorCount => "anchor count",
            Self::MergeDepth => "merge depth",
            Self::CollectionSize => "collection size",
            Self::OutputBytes => "output size",
            Self::ResolutionDepth => "resolution depth",
            Self::ResolutionEdges => "resolution edges",
            Self::MaterializedBytes => "materialized bytes",
        })
    }
}

/// Structured failure returned when a configured resource limit is exceeded.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LimitExceeded {
    /// Resource category whose limit was exceeded.
    pub limit: ResourceLimit,
    /// Value observed by the caller.
    pub observed: usize,
    /// Configured maximum value.
    pub maximum: usize,
}

impl LimitExceeded {
    /// Construct a structured resource-limit failure.
    #[must_use]
    pub const fn new(limit: ResourceLimit, observed: usize, maximum: usize) -> Self {
        Self {
            limit,
            observed,
            maximum,
        }
    }
}

impl std::fmt::Display for LimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} exceeds maximum of {}",
            self.limit, self.observed, self.maximum
        )
    }
}

impl std::error::Error for LimitExceeded {}

/// Caps on resource consumption during YAML loading.
///
/// Each field is `Option<usize>` — `None` means unlimited. The `check_*`
/// helpers return `Ok(())` when the value is within the limit or the limit
/// is `None`, and [`LimitExceeded`] otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum total input size in bytes.
    pub max_input_bytes: Option<usize>,
    /// Maximum size of a single scalar value in bytes.
    pub max_scalar_bytes: Option<usize>,
    /// Maximum size of a mapping key in bytes.
    pub max_key_bytes: Option<usize>,
    /// Maximum nesting depth (mappings, sequences, flow collections).
    pub max_depth: Option<usize>,
    /// Maximum number of alias expansions (Billion Laughs protection).
    pub max_alias_expansions: Option<usize>,
    /// Maximum number of anchors in a document.
    pub max_anchor_count: Option<usize>,
    /// Maximum depth when recursively resolving merge keys (`<<`).
    pub max_merge_depth: Option<usize>,
    /// Maximum number of entries in one sequence or mapping.
    pub max_collection_size: Option<usize>,
    /// Maximum serialized output size in bytes.
    pub max_output_bytes: Option<usize>,
}

impl ResourceLimits {
    /// Balanced limits suitable for most workloads.
    #[must_use]
    pub const fn moderate() -> Self {
        Self {
            max_input_bytes: Some(256 * 1024 * 1024),
            max_scalar_bytes: Some(64 * 1024 * 1024),
            max_key_bytes: Some(1024 * 1024),
            max_depth: Some(256),
            max_alias_expansions: Some(10_000),
            max_anchor_count: Some(10_000),
            max_merge_depth: Some(64),
            max_collection_size: Some(1_000_000),
            max_output_bytes: Some(256 * 1024 * 1024),
        }
    }

    /// No limits — every field is `None`.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            max_input_bytes: None,
            max_scalar_bytes: None,
            max_key_bytes: None,
            max_depth: None,
            max_alias_expansions: None,
            max_anchor_count: None,
            max_merge_depth: None,
            max_collection_size: None,
            max_output_bytes: None,
        }
    }

    /// Check whether `depth` is within [`max_depth`](Self::max_depth).
    ///
    /// Returns `Ok(())` if the limit is `None` or `depth <= max`.
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `depth` exceeds the
    /// configured maximum.
    pub const fn check_depth(&self, depth: usize) -> Result<(), LimitExceeded> {
        check_limit(depth, self.max_depth, ResourceLimit::Depth)
    }

    /// Check whether `size` is within [`max_scalar_bytes`](Self::max_scalar_bytes).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `size` exceeds the
    /// configured maximum.
    pub const fn check_scalar_size(&self, size: usize) -> Result<(), LimitExceeded> {
        check_limit(size, self.max_scalar_bytes, ResourceLimit::ScalarBytes)
    }

    /// Check whether `size` is within [`max_key_bytes`](Self::max_key_bytes).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `size` exceeds the
    /// configured maximum.
    pub const fn check_key_size(&self, size: usize) -> Result<(), LimitExceeded> {
        check_limit(size, self.max_key_bytes, ResourceLimit::KeyBytes)
    }

    /// Check whether `size` is within [`max_input_bytes`](Self::max_input_bytes).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `size` exceeds the
    /// configured maximum.
    pub const fn check_input_size(&self, size: usize) -> Result<(), LimitExceeded> {
        check_limit(size, self.max_input_bytes, ResourceLimit::InputBytes)
    }

    /// Check whether `count` is within [`max_alias_expansions`](Self::max_alias_expansions).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `count` exceeds the
    /// configured maximum.
    pub const fn check_alias_expansions(&self, count: usize) -> Result<(), LimitExceeded> {
        check_limit(
            count,
            self.max_alias_expansions,
            ResourceLimit::AliasExpansions,
        )
    }

    /// Check whether `count` is within [`max_anchor_count`](Self::max_anchor_count).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `count` exceeds the
    /// configured maximum.
    pub const fn check_anchor_count(&self, count: usize) -> Result<(), LimitExceeded> {
        check_limit(count, self.max_anchor_count, ResourceLimit::AnchorCount)
    }

    /// Check whether `depth` is within [`max_merge_depth`](Self::max_merge_depth).
    ///
    /// # Errors
    ///
    /// Returns a structured [`LimitExceeded`] when `depth` exceeds the
    /// configured maximum.
    pub const fn check_merge_depth(&self, depth: usize) -> Result<(), LimitExceeded> {
        check_limit(depth, self.max_merge_depth, ResourceLimit::MergeDepth)
    }

    /// Check whether a collection length is within the configured limit.
    pub const fn check_collection_size(&self, size: usize) -> Result<(), LimitExceeded> {
        check_limit(
            size,
            self.max_collection_size,
            ResourceLimit::CollectionSize,
        )
    }

    /// Check whether serialized output is within the configured byte limit.
    pub const fn check_output_size(&self, size: usize) -> Result<(), LimitExceeded> {
        check_limit(size, self.max_output_bytes, ResourceLimit::OutputBytes)
    }
}

/// Shared limit check: returns `Err` if `value > max`, `Ok(())` otherwise.
/// `None` means unlimited.
const fn check_limit(
    value: usize,
    limit: Option<usize>,
    resource: ResourceLimit,
) -> Result<(), LimitExceeded> {
    if let Some(max) = limit
        && value > max
    {
        return Err(LimitExceeded::new(resource, value, max));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ResolutionPolicy
// ---------------------------------------------------------------------------

/// Controls whether a resolution capability is available and automatic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResolutionMode {
    /// Reject explicit and automatic use of the capability.
    #[default]
    Disabled,
    /// Permit explicit query actions without automatic loading.
    Explicit,
    /// Permit explicit query actions and resolve directives while loading.
    Automatic,
}

impl ResolutionMode {
    /// Returns `true` when an explicit action may use this capability.
    #[must_use]
    pub const fn allows_explicit(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Returns `true` when ordinary loading resolves this capability.
    #[must_use]
    pub const fn resolves_automatically(self) -> bool {
        matches!(self, Self::Automatic)
    }
}

/// External-resolution capability selected by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionCapability {
    /// `!include` resolution.
    Include,
    /// `$ref` resolution.
    Ref,
}

/// Shared limits for one external-resolution session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionLimits {
    /// Maximum external-resolution recursion depth.
    pub max_depth: Option<usize>,
    /// Maximum attempted resolution edges, including cache hits.
    pub max_edges: Option<usize>,
    /// Maximum bytes materialized into resolved output.
    pub max_materialized_bytes: Option<usize>,
}

/// Policy governing `!include` and `$ref` resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionPolicy {
    /// Policy for `!include` directives.
    pub include: IncludePolicy,
    /// Policy for `$ref` references.
    pub refs: RefPolicy,
    /// Shared filesystem boundary for includes and file references.
    pub files: FilePolicy,
    /// Shared session limits for all resolution capabilities.
    pub limits: ResolutionLimits,
}

impl ResolutionPolicy {
    /// Balanced limits with both capabilities disabled.
    #[must_use]
    pub const fn moderate() -> Self {
        Self {
            include: IncludePolicy {
                mode: ResolutionMode::Disabled,
            },
            refs: RefPolicy {
                mode: ResolutionMode::Disabled,
                allow_schemes: Vec::new(),
                allow_hosts: Vec::new(),
                timeout: Duration::from_secs(30),
            },
            files: FilePolicy {
                root: None,
                allow: Vec::new(),
                deny: Vec::new(),
                follow_symlinks: false,
            },
            limits: ResolutionLimits {
                max_depth: Some(10),
                max_edges: Some(256),
                max_materialized_bytes: Some(256 * 1024 * 1024),
            },
        }
    }

    /// Tight limits with both capabilities disabled.
    #[must_use]
    pub const fn strict() -> Self {
        let mut policy = Self::moderate();
        policy.limits = ResolutionLimits {
            max_depth: Some(5),
            max_edges: Some(32),
            max_materialized_bytes: Some(10 * 1024 * 1024),
        };
        policy
    }

    /// Generous limits with both capabilities disabled.
    #[must_use]
    pub const fn trusted() -> Self {
        let mut policy = Self::moderate();
        policy.limits = ResolutionLimits {
            max_depth: Some(64),
            max_edges: Some(4_096),
            max_materialized_bytes: None,
        };
        policy
    }

    /// Unlimited resolution budgets with both capabilities disabled.
    #[must_use]
    pub const fn unchecked() -> Self {
        let mut policy = Self::moderate();
        policy.limits = ResolutionLimits {
            max_depth: None,
            max_edges: None,
            max_materialized_bytes: None,
        };
        policy
    }

    /// Compatibility alias for balanced, disabled resolution.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::moderate()
    }

    /// Returns `true` when include or reference resolution is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.include.mode.allows_explicit() || self.refs.mode.allows_explicit()
    }

    /// Returns `true` when an explicit action may use `capability`.
    #[must_use]
    pub const fn allows_explicit(&self, capability: ResolutionCapability) -> bool {
        self.mode(capability).allows_explicit()
    }

    /// Returns `true` when ordinary loading resolves `capability`.
    #[must_use]
    pub const fn resolves_automatically(&self, capability: ResolutionCapability) -> bool {
        self.mode(capability).resolves_automatically()
    }

    const fn mode(&self, capability: ResolutionCapability) -> ResolutionMode {
        match capability {
            ResolutionCapability::Include => self.include.mode,
            ResolutionCapability::Ref => self.refs.mode,
        }
    }
}

impl Default for ResolutionPolicy {
    fn default() -> Self {
        Self::moderate()
    }
}

// ---------------------------------------------------------------------------
// IncludePolicy
// ---------------------------------------------------------------------------

/// Controls how `!include` directives are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludePolicy {
    /// Permission and automatic-loading mode.
    pub mode: ResolutionMode,
}

// ---------------------------------------------------------------------------
// FilePolicy
// ---------------------------------------------------------------------------

/// Shared filesystem boundary for includes and file references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePolicy {
    /// Root directory capability for resolving file targets.
    pub root: Option<PathBuf>,
    /// Glob patterns for allowed root-relative paths.
    pub allow: Vec<String>,
    /// Glob patterns for denied root-relative paths (checked before allow).
    pub deny: Vec<String>,
    /// Whether to follow symbolic links that remain inside the root.
    pub follow_symlinks: bool,
}

// ---------------------------------------------------------------------------
// RefPolicy
// ---------------------------------------------------------------------------

/// Controls how `$ref` references are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPolicy {
    /// Permission and automatic-loading mode.
    pub mode: ResolutionMode,
    /// Allowed URL schemes (e.g. `["https"]`).
    pub allow_schemes: Vec<String>,
    /// Allowed hostnames for remote refs.
    pub allow_hosts: Vec<String>,
    /// Timeout for fetching remote refs.
    pub timeout: Duration,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_moderate() {
        let cfg = LoaderConfig::default();
        assert_eq!(cfg.limits.max_depth, Some(256));
        assert_eq!(cfg.resolution.include.mode, ResolutionMode::Disabled);
        assert_eq!(cfg.resolution.refs.mode, ResolutionMode::Disabled);
    }

    #[test]
    fn resolution_presets_match_adr_0016() {
        let cases = [
            (
                LoaderConfig::moderate(),
                Some(10),
                Some(256),
                Some(256 * 1024 * 1024),
            ),
            (
                LoaderConfig::strict(),
                Some(5),
                Some(32),
                Some(10 * 1024 * 1024),
            ),
            (LoaderConfig::trusted(), Some(64), Some(4_096), None),
            (LoaderConfig::unchecked(), None, None, None),
        ];

        for (config, depth, edges, bytes) in cases {
            assert_eq!(config.resolution.limits.max_depth, depth);
            assert_eq!(config.resolution.limits.max_edges, edges);
            assert_eq!(config.resolution.limits.max_materialized_bytes, bytes);
            assert!(!config.resolution.is_enabled());
        }
    }

    #[test]
    fn explicit_and_automatic_modes_are_distinct() {
        let mut policy = ResolutionPolicy::moderate();
        policy.include.mode = ResolutionMode::Explicit;
        policy.refs.mode = ResolutionMode::Automatic;

        assert!(policy.allows_explicit(ResolutionCapability::Include));
        assert!(!policy.resolves_automatically(ResolutionCapability::Include));
        assert!(policy.allows_explicit(ResolutionCapability::Ref));
        assert!(policy.resolves_automatically(ResolutionCapability::Ref));
    }

    #[test]
    fn resolution_limit_errors_are_structured() {
        let cases = [
            (
                ResourceLimit::ResolutionDepth,
                "resolution depth 11 exceeds maximum of 10",
            ),
            (
                ResourceLimit::ResolutionEdges,
                "resolution edges 11 exceeds maximum of 10",
            ),
            (
                ResourceLimit::MaterializedBytes,
                "materialized bytes 11 exceeds maximum of 10",
            ),
        ];

        for (limit, message) in cases {
            let error = LimitExceeded::new(limit, 11, 10);
            assert_eq!(error.limit, limit);
            assert_eq!(error.observed, 11);
            assert_eq!(error.maximum, 10);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn strict_has_tight_limits() {
        let cfg = LoaderConfig::strict();
        assert_eq!(cfg.limits.max_depth, Some(64));
        assert_eq!(cfg.limits.max_scalar_bytes, Some(1024 * 1024));
        assert_eq!(cfg.limits.max_alias_expansions, Some(100));
    }

    #[test]
    fn trusted_has_generous_limits() {
        let cfg = LoaderConfig::trusted();
        assert_eq!(cfg.limits.max_input_bytes, None);
        assert_eq!(cfg.limits.max_scalar_bytes, None);
        assert_eq!(cfg.limits.max_depth, Some(1024));
    }

    #[test]
    fn unchecked_has_no_limits() {
        let cfg = LoaderConfig::unchecked();
        assert_eq!(cfg.limits.max_input_bytes, None);
        assert_eq!(cfg.limits.max_scalar_bytes, None);
        assert_eq!(cfg.limits.max_key_bytes, None);
        assert_eq!(cfg.limits.max_depth, None);
        assert_eq!(cfg.limits.max_alias_expansions, None);
        assert_eq!(cfg.limits.max_anchor_count, None);
        assert_eq!(cfg.limits.max_merge_depth, None);
    }

    #[test]
    fn resolution_disabled_by_default() {
        let cfg = LoaderConfig::default();
        assert_eq!(cfg.resolution.include.mode, ResolutionMode::Disabled);
        assert_eq!(cfg.resolution.refs.mode, ResolutionMode::Disabled);
        assert!(cfg.resolution.files.allow.is_empty());
        assert!(cfg.resolution.files.deny.is_empty());
        assert!(cfg.resolution.refs.allow_schemes.is_empty());
        assert!(cfg.resolution.refs.allow_hosts.is_empty());
    }

    #[test]
    fn resolution_policy_reports_enabled_capabilities() {
        let mut policy = ResolutionPolicy::disabled();
        assert!(!policy.is_enabled());

        policy.include.mode = ResolutionMode::Explicit;
        assert!(policy.is_enabled());

        policy.include.mode = ResolutionMode::Disabled;
        policy.refs.mode = ResolutionMode::Automatic;
        assert!(policy.is_enabled());
    }

    #[test]
    fn resource_limits_none_is_all_none() {
        let limits = ResourceLimits::none();
        assert_eq!(limits.max_input_bytes, None);
        assert_eq!(limits.max_scalar_bytes, None);
        assert_eq!(limits.max_key_bytes, None);
        assert_eq!(limits.max_depth, None);
        assert_eq!(limits.max_alias_expansions, None);
        assert_eq!(limits.max_anchor_count, None);
        assert_eq!(limits.max_merge_depth, None);
    }

    #[test]
    fn check_depth_allows_within_limit() {
        let limits = ResourceLimits {
            max_depth: Some(256),
            ..ResourceLimits::none()
        };
        assert!(limits.check_depth(10).is_ok());
    }

    #[test]
    fn check_depth_rejects_over_limit() {
        let limits = ResourceLimits {
            max_depth: Some(256),
            ..ResourceLimits::none()
        };
        let error = limits.check_depth(300).unwrap_err();
        assert_eq!(error.limit, ResourceLimit::Depth);
        assert_eq!(error.observed, 300);
        assert_eq!(error.maximum, 256);
        assert_eq!(error.to_string(), "depth 300 exceeds maximum of 256");
    }

    #[test]
    fn every_resource_check_reports_its_structured_category() {
        let limits = ResourceLimits {
            max_input_bytes: Some(1),
            max_scalar_bytes: Some(1),
            max_key_bytes: Some(1),
            max_depth: Some(1),
            max_alias_expansions: Some(1),
            max_anchor_count: Some(1),
            max_merge_depth: Some(1),
            max_collection_size: Some(1),
            max_output_bytes: Some(1),
        };
        let failures = [
            limits.check_input_size(2),
            limits.check_scalar_size(2),
            limits.check_key_size(2),
            limits.check_depth(2),
            limits.check_alias_expansions(2),
            limits.check_anchor_count(2),
            limits.check_merge_depth(2),
            limits.check_collection_size(2),
            limits.check_output_size(2),
        ];
        let categories = [
            ResourceLimit::InputBytes,
            ResourceLimit::ScalarBytes,
            ResourceLimit::KeyBytes,
            ResourceLimit::Depth,
            ResourceLimit::AliasExpansions,
            ResourceLimit::AnchorCount,
            ResourceLimit::MergeDepth,
            ResourceLimit::CollectionSize,
            ResourceLimit::OutputBytes,
        ];

        for (failure, category) in failures.into_iter().zip(categories) {
            let error = failure.unwrap_err();
            assert_eq!(error.limit, category);
            assert_eq!(error.observed, 2);
            assert_eq!(error.maximum, 1);
        }
    }

    #[test]
    fn check_depth_allows_when_unlimited() {
        let limits = ResourceLimits::none();
        assert!(limits.check_depth(999_999).is_ok());
    }

    #[test]
    fn check_depth_allows_at_exact_limit() {
        let limits = ResourceLimits {
            max_depth: Some(256),
            ..ResourceLimits::none()
        };
        assert!(limits.check_depth(256).is_ok());
    }

    #[test]
    fn check_scalar_size_rejects_over_limit() {
        let limits = ResourceLimits {
            max_scalar_bytes: Some(1_000_000),
            ..ResourceLimits::none()
        };
        assert!(limits.check_scalar_size(2_000_000).is_err());
    }

    #[test]
    fn check_key_size_rejects_over_limit() {
        let limits = ResourceLimits {
            max_key_bytes: Some(4096),
            ..ResourceLimits::none()
        };
        assert!(limits.check_key_size(5000).is_err());
        assert!(limits.check_key_size(4096).is_ok());
    }

    #[test]
    fn check_input_size_rejects_over_limit() {
        let limits = ResourceLimits {
            max_input_bytes: Some(10 * 1024 * 1024),
            ..ResourceLimits::none()
        };
        assert!(limits.check_input_size(11 * 1024 * 1024).is_err());
        assert!(limits.check_input_size(10 * 1024 * 1024).is_ok());
    }

    #[test]
    fn default_tag_resolution_is_yaml12() {
        let cfg = LoaderConfig::default();
        assert_eq!(cfg.tag_resolution, TagResolution::Yaml12);
    }

    #[test]
    fn with_tag_resolution_builder() {
        let cfg = LoaderConfig::strict().with_tag_resolution(TagResolution::Yaml11);
        assert_eq!(cfg.tag_resolution, TagResolution::Yaml11);
        assert_eq!(cfg.limits.max_depth, Some(64));
    }
}
