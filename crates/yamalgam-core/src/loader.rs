//! Loader configuration for resource limits and security policy.
//!
//! [`LoaderConfig`] is the central type that governs how yamalgam loads and
//! processes YAML documents. It controls two concerns:
//!
//! - **Resource limits** — bounds on input size, nesting depth, alias
//!   expansions, and other vectors for denial-of-service attacks.
//! - **Resolution policy** — whether `!include` directives and `$ref`
//!   references are followed, and under what constraints.
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
//! See [ADR-0006](../../docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)
//! for design rationale.

use std::path::PathBuf;
use std::time::Duration;

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
    /// - Resolution disabled.
    #[must_use]
    pub const fn moderate() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: Some(256 * 1024 * 1024),
                max_scalar_bytes: Some(64 * 1024 * 1024),
                max_key_bytes: Some(1024 * 1024),
                max_depth: Some(256),
                max_alias_expansions: Some(10_000),
                max_anchor_count: Some(10_000),
                max_merge_depth: Some(64),
            },
            resolution: ResolutionPolicy::disabled(),
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
            },
            resolution: ResolutionPolicy::disabled(),
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
            },
            resolution: ResolutionPolicy::disabled(),
        }
    }

    /// No limits at all. Use only when you fully trust the input.
    #[must_use]
    pub const fn unchecked() -> Self {
        Self {
            limits: ResourceLimits::none(),
            resolution: ResolutionPolicy::disabled(),
        }
    }
}

// ---------------------------------------------------------------------------
// ResourceLimits
// ---------------------------------------------------------------------------

/// Caps on resource consumption during YAML loading.
///
/// Each field is `Option<usize>` — `None` means unlimited. The `check_*`
/// helpers return `Ok(())` when the value is within the limit or the limit
/// is `None`, and `Err(String)` otherwise.
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
}

impl ResourceLimits {
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
        }
    }

    /// Check whether `depth` is within [`max_depth`](Self::max_depth).
    ///
    /// Returns `Ok(())` if the limit is `None` or `depth <= max`.
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message when `depth` exceeds the
    /// configured maximum.
    pub fn check_depth(&self, depth: usize) -> Result<(), String> {
        check_limit(depth, self.max_depth, "depth")
    }

    /// Check whether `size` is within [`max_scalar_bytes`](Self::max_scalar_bytes).
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message when `size` exceeds the
    /// configured maximum.
    pub fn check_scalar_size(&self, size: usize) -> Result<(), String> {
        check_limit(size, self.max_scalar_bytes, "scalar size")
    }

    /// Check whether `size` is within [`max_key_bytes`](Self::max_key_bytes).
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message when `size` exceeds the
    /// configured maximum.
    pub fn check_key_size(&self, size: usize) -> Result<(), String> {
        check_limit(size, self.max_key_bytes, "key size")
    }

    /// Check whether `size` is within [`max_input_bytes`](Self::max_input_bytes).
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message when `size` exceeds the
    /// configured maximum.
    pub fn check_input_size(&self, size: usize) -> Result<(), String> {
        check_limit(size, self.max_input_bytes, "input size")
    }
}

/// Shared limit check: returns `Err` if `value > max`, `Ok(())` otherwise.
/// `None` means unlimited.
fn check_limit(value: usize, limit: Option<usize>, name: &str) -> Result<(), String> {
    if let Some(max) = limit
        && value > max
    {
        return Err(format!("{name} {value} exceeds maximum of {max}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ResolutionPolicy
// ---------------------------------------------------------------------------

/// Policy governing `!include` and `$ref` resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionPolicy {
    /// Policy for `!include` directives.
    pub include: IncludePolicy,
    /// Policy for `$ref` references.
    pub refs: RefPolicy,
}

impl ResolutionPolicy {
    /// Both `!include` and `$ref` disabled, empty allow/deny lists.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            include: IncludePolicy {
                enabled: false,
                root: None,
                allow: Vec::new(),
                deny: Vec::new(),
                max_depth: 10,
                max_total_bytes: None,
                follow_symlinks: false,
            },
            refs: RefPolicy {
                enabled: false,
                allow_schemes: Vec::new(),
                allow_hosts: Vec::new(),
                timeout: Duration::from_secs(30),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// IncludePolicy
// ---------------------------------------------------------------------------

/// Controls how `!include` directives are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludePolicy {
    /// Whether `!include` processing is enabled.
    pub enabled: bool,
    /// Root directory for resolving relative include paths.
    pub root: Option<PathBuf>,
    /// Glob patterns for allowed include paths.
    pub allow: Vec<String>,
    /// Glob patterns for denied include paths (checked before allow).
    pub deny: Vec<String>,
    /// Maximum include recursion depth.
    pub max_depth: usize,
    /// Maximum total bytes read across all includes.
    pub max_total_bytes: Option<usize>,
    /// Whether to follow symbolic links when resolving includes.
    pub follow_symlinks: bool,
}

// ---------------------------------------------------------------------------
// RefPolicy
// ---------------------------------------------------------------------------

/// Controls how `$ref` references are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPolicy {
    /// Whether `$ref` processing is enabled.
    pub enabled: bool,
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
        assert!(!cfg.resolution.include.enabled);
        assert!(!cfg.resolution.refs.enabled);
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
        assert!(!cfg.resolution.include.enabled);
        assert!(!cfg.resolution.refs.enabled);
        assert!(cfg.resolution.include.allow.is_empty());
        assert!(cfg.resolution.include.deny.is_empty());
        assert!(cfg.resolution.refs.allow_schemes.is_empty());
        assert!(cfg.resolution.refs.allow_hosts.is_empty());
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
        assert!(limits.check_depth(300).is_err());
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
}
