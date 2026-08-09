//! Canonical resource identities and URI-reference resolution.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use percent_encoding::percent_decode_str;
use url::Url;

use crate::pointer::{JsonPointer, PointerError};

/// Canonical identity of one fragmentless external resource.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ResourceId {
    /// A non-retrievable root with no relative external-resource base.
    Anonymous,
    /// A filesystem resource. Paths retain their platform-native encoding.
    File(PathBuf),
    /// A URL resource, including memory, HTTP, and application-defined schemes.
    Uri(Url),
}

impl ResourceId {
    /// Construct an identity for standard input or another anonymous root.
    #[must_use]
    pub const fn anonymous() -> Self {
        Self::Anonymous
    }

    /// Parse a fragmentless URL or UTF-8 file identity.
    ///
    /// Use [`Self::from_file`] when a file path is not valid UTF-8 or when a
    /// literal `#` is part of its filename.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::FragmentNotAllowed`] when `value` contains a
    /// URI fragment, or [`ResourceError::InvalidFileUri`] when a `file:` URL
    /// cannot be represented as a platform path.
    pub fn new(value: &str) -> Result<Self, ResourceError> {
        if value.contains('#') {
            return Err(ResourceError::FragmentNotAllowed);
        }
        if Path::new(value).is_absolute() {
            return Ok(Self::from_file(value));
        }
        match Url::parse(value) {
            Ok(url) => Self::from_url(url),
            Err(url::ParseError::RelativeUrlWithoutBase) => Ok(Self::from_file(value)),
            Err(source) => Err(ResourceError::InvalidUri(source)),
        }
    }

    /// Construct a file identity without changing its native path encoding.
    #[must_use]
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self::File(normalize_path(&path.into()))
    }

    /// Construct an identity from a parsed, fragmentless URL.
    ///
    /// `file:` URLs are converted to [`ResourceId::File`].
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::FragmentNotAllowed`] when `url` has a fragment,
    /// or [`ResourceError::InvalidFileUri`] when a `file:` URL cannot become a
    /// platform path.
    pub fn from_url(url: Url) -> Result<Self, ResourceError> {
        if url.fragment().is_some() {
            return Err(ResourceError::FragmentNotAllowed);
        }
        if url.scheme() == "file" {
            return url
                .to_file_path()
                .map(Self::from_file)
                .map_err(|()| ResourceError::InvalidFileUri);
        }
        Ok(Self::Uri(url))
    }

    /// Resolve a URI reference against this containing resource.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError`] for malformed fragments, invalid URLs, an
    /// invalid base identity, or a file URL that cannot become a native path.
    pub fn resolve(&self, reference: &str) -> Result<ResolvedReference, ResourceError> {
        resolve_reference(Some(self), reference)
    }

    /// Return the native path for a file identity.
    #[must_use]
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Anonymous | Self::Uri(_) => None,
        }
    }

    /// Return the complete internal URL identity.
    ///
    /// This accessor may contain user information and query values. Do not
    /// render it in diagnostics; [`Display`](fmt::Display) and [`Debug`](fmt::Debug)
    /// are redacted for that purpose.
    #[must_use]
    pub const fn as_url(&self) -> Option<&Url> {
        match self {
            Self::Anonymous | Self::File(_) => None,
            Self::Uri(url) => Some(url),
        }
    }

    /// Return the resource scheme. File identities report `file`.
    #[must_use]
    pub fn scheme(&self) -> &str {
        match self {
            Self::Anonymous => "anonymous",
            Self::File(_) => "file",
            Self::Uri(url) => url.scheme(),
        }
    }

    /// Return the normalized URL host, when present.
    #[must_use]
    pub fn host_str(&self) -> Option<&str> {
        self.as_url().and_then(Url::host_str)
    }

    /// Return whether the internal URL contains user information.
    #[must_use]
    pub fn has_userinfo(&self) -> bool {
        self.as_url()
            .is_some_and(|url| !url.username().is_empty() || url.password().is_some())
    }

    fn validate(&self) -> Result<(), ResourceError> {
        match self {
            Self::Uri(url) if url.fragment().is_some() => Err(ResourceError::FragmentNotAllowed),
            _ => Ok(()),
        }
    }

    fn redacted(&self) -> String {
        match self {
            Self::Anonymous => "<anonymous>".to_owned(),
            Self::File(path) => path.display().to_string(),
            Self::Uri(url) => {
                let mut redacted = url.clone();
                redacted.set_fragment(None);
                redacted.set_query(None);
                let _ = redacted.set_password(None);
                let _ = redacted.set_username("");
                redacted.to_string()
            }
        }
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.redacted())
    }
}

impl fmt::Debug for ResourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anonymous => formatter.write_str(&self.redacted()),
            Self::File(_) => formatter
                .debug_tuple("File")
                .field(&self.redacted())
                .finish(),
            Self::Uri(_) => formatter
                .debug_tuple("Uri")
                .field(&self.redacted())
                .finish(),
        }
    }
}

/// One resolved resource and the fragment selected from it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResolvedReference {
    /// Fragmentless fetch and parse identity.
    pub resource: ResourceId,
    /// Document root or decoded RFC 6901 pointer.
    pub fragment: Fragment,
}

/// A document root or decoded RFC 6901 pointer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Fragment {
    /// Select the complete YAML document.
    Root,
    /// Select a node through an RFC 6901 pointer.
    Pointer(JsonPointer),
}

impl Fragment {
    /// Return decoded pointer tokens. The document root has no tokens.
    #[must_use]
    pub fn pointer_tokens(&self) -> &[String] {
        match self {
            Self::Root => &[],
            Self::Pointer(pointer) => pointer.tokens(),
        }
    }
}

/// Resolve a URI reference with an optional containing-resource base.
///
/// Absolute URL references do not require a base. Relative references do.
///
/// # Errors
///
/// Returns [`ResourceError::MissingBase`] for a relative reference without a
/// base, or another [`ResourceError`] for malformed resources or fragments.
pub fn resolve_reference(
    base: Option<&ResourceId>,
    reference: &str,
) -> Result<ResolvedReference, ResourceError> {
    let (resource_reference, encoded_fragment) = match reference.split_once('#') {
        Some((resource, fragment)) => (resource, Some(fragment)),
        None => (reference, None),
    };
    let fragment = parse_fragment(encoded_fragment)?;
    let resource = resolve_resource(base, resource_reference)?;
    Ok(ResolvedReference { resource, fragment })
}

/// Failure while parsing a resource identity or URI reference.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResourceError {
    /// A fragment was supplied where a fragmentless identity was required.
    #[error("resource identity must not contain a fragment")]
    FragmentNotAllowed,
    /// A relative reference had no containing-resource base.
    #[error("relative resource reference requires a base")]
    MissingBase,
    /// A URL could not be parsed.
    #[error("invalid resource URI")]
    InvalidUri(#[source] url::ParseError),
    /// A `file:` URL could not become a native path.
    #[error("file URI cannot be represented as a native path")]
    InvalidFileUri,
    /// A fragment contained a malformed percent escape.
    #[error("URI fragment contains an invalid percent escape")]
    InvalidPercentEncoding,
    /// A decoded fragment was not valid UTF-8.
    #[error("URI fragment is not valid UTF-8")]
    InvalidFragmentEncoding,
    /// A non-empty fragment did not begin with `/`.
    #[error("URI fragment is not an RFC 6901 JSON Pointer")]
    InvalidPointer,
    /// A pointer token contained a malformed `~` escape.
    #[error("URI fragment contains an invalid JSON Pointer escape")]
    InvalidPointerEscape,
}

fn resolve_resource(
    base: Option<&ResourceId>,
    reference: &str,
) -> Result<ResourceId, ResourceError> {
    if reference.is_empty() {
        let base = base.ok_or(ResourceError::MissingBase)?;
        base.validate()?;
        return Ok(base.clone());
    }

    if let Ok(url) = Url::parse(reference) {
        return ResourceId::from_url(url);
    }

    let path = Path::new(reference);
    if path.is_absolute() {
        return Ok(ResourceId::from_file(path));
    }

    let base = base.ok_or(ResourceError::MissingBase)?;
    base.validate()?;
    match base {
        ResourceId::Anonymous => Err(ResourceError::MissingBase),
        ResourceId::File(base_path) => {
            let parent = base_path.parent().unwrap_or_else(|| Path::new(""));
            Ok(ResourceId::from_file(parent.join(path)))
        }
        ResourceId::Uri(base_url) => base_url
            .join(reference)
            .map_err(ResourceError::InvalidUri)
            .and_then(ResourceId::from_url),
    }
}

fn parse_fragment(fragment: Option<&str>) -> Result<Fragment, ResourceError> {
    let Some(encoded) = fragment else {
        return Ok(Fragment::Root);
    };
    validate_percent_encoding(encoded)?;
    let decoded = percent_decode_str(encoded)
        .decode_utf8()
        .map_err(|_| ResourceError::InvalidFragmentEncoding)?;
    if decoded.is_empty() {
        return Ok(Fragment::Root);
    }
    JsonPointer::parse(&decoded)
        .map(Fragment::Pointer)
        .map_err(|error| match error {
            PointerError::NotAbsolute => ResourceError::InvalidPointer,
            PointerError::InvalidEscape => ResourceError::InvalidPointerEscape,
            PointerError::InvalidIndex => unreachable!("resource parsing does not select arrays"),
        })
}

fn validate_percent_encoding(value: &str) -> Result<(), ResourceError> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(ResourceError::InvalidPercentEncoding);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = matches!(
                    normalized.components().next_back(),
                    Some(Component::Normal(_))
                );
                if can_pop {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}
