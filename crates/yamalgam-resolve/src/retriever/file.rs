//! Capability-rooted filesystem retrieval and discovery.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf};

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use yamalgam_core::FilePolicy;

use super::{
    DiscoveryRequest, DiscoverySelector, ResourceProvenance, ResourceRequest, ResourceRetriever,
    RetrievedResource,
};
use crate::{ResourceId, RetrieveError};

/// Filesystem backend confined to one directory capability.
// r[impl resolution.include.sandboxing]
// r[impl resolution.include.order]
pub struct FileRetriever {
    root_path: PathBuf,
    root: Dir,
    allow: GlobSet,
    deny: GlobSet,
    follow_symlinks: bool,
}

impl FileRetriever {
    /// Open the configured filesystem capability and compile its policy.
    ///
    /// This is the backend's sole ambient filesystem operation. Retrieval and
    /// discovery after construction are relative to the opened directory.
    ///
    /// # Errors
    ///
    /// Returns [`RetrieveError::MissingRoot`] when no root is configured,
    /// [`RetrieveError::InvalidPattern`] for malformed policy globs, or an I/O
    /// error when the root cannot be opened.
    pub fn open(policy: &FilePolicy) -> Result<Self, RetrieveError> {
        let configured_root = policy.root.as_ref().ok_or(RetrieveError::MissingRoot)?;
        let root_path =
            std::path::absolute(configured_root).map_err(|source| RetrieveError::Io {
                id: ResourceId::from_file(configured_root),
                source,
            })?;
        let allow = compile_globs(&policy.allow)?;
        let deny = compile_globs(&policy.deny)?;
        let root = Dir::open_ambient_dir(&root_path, ambient_authority())
            .map_err(|source| map_io(ResourceId::from_file(&root_path), source))?;
        Ok(Self {
            root_path,
            root,
            allow,
            deny,
            follow_symlinks: policy.follow_symlinks,
        })
    }

    fn relative_path(&self, id: &ResourceId) -> Result<PathBuf, RetrieveError> {
        let path = id
            .as_path()
            .ok_or_else(|| RetrieveError::UnsupportedScheme(id.scheme().to_owned()))?;
        let relative = if path.is_absolute() {
            path.strip_prefix(&self.root_path)
                .map_err(|_| RetrieveError::OutsideRoot(id.clone()))?
        } else {
            path
        };
        normalize_relative(relative).ok_or_else(|| RetrieveError::OutsideRoot(id.clone()))
    }

    fn approved(&self, relative: &Path) -> Result<(), RetrieveError> {
        let id = self.id_for(relative);
        if self.deny.is_match(relative)
            || (!self.allow.is_empty() && !self.allow.is_match(relative))
        {
            return Err(RetrieveError::PolicyDenied { id });
        }
        Ok(())
    }

    fn canonical_existing(&self, relative: &Path) -> Result<PathBuf, RetrieveError> {
        let requested_id = self.id_for(relative);
        if self.follow_symlinks {
            return self
                .root
                .canonicalize(capability_path(relative))
                .map_err(|source| {
                    if source.kind() == io::ErrorKind::PermissionDenied {
                        RetrieveError::OutsideRoot(requested_id.clone())
                    } else {
                        map_io(requested_id.clone(), source)
                    }
                })
                .and_then(|canonical| {
                    normalize_relative(&canonical).ok_or(RetrieveError::OutsideRoot(requested_id))
                });
        }

        let mut current = PathBuf::new();
        for component in relative.components() {
            current.push(component.as_os_str());
            let metadata = self
                .root
                .symlink_metadata(&current)
                .map_err(|source| map_io(self.id_for(&current), source))?;
            if metadata.file_type().is_symlink() {
                return Err(RetrieveError::SymlinkDenied(self.id_for(&current)));
            }
        }
        Ok(relative.to_path_buf())
    }

    fn canonical_directory(&self, relative: &Path) -> Result<PathBuf, RetrieveError> {
        let canonical = self.canonical_existing(relative)?;
        let metadata = self
            .root
            .metadata(capability_path(&canonical))
            .map_err(|source| map_io(self.id_for(&canonical), source))?;
        if !metadata.is_dir() {
            return Err(RetrieveError::NotRegularFile(self.id_for(&canonical)));
        }
        Ok(canonical)
    }

    fn id_for(&self, relative: &Path) -> ResourceId {
        ResourceId::from_file(self.root_path.join(relative))
    }

    fn record_match(
        &self,
        canonical: PathBuf,
        maximum: Option<usize>,
        matches: &mut Vec<PathBuf>,
    ) -> Result<(), RetrieveError> {
        self.approved(&canonical)?;
        matches.push(canonical);
        if let Some(maximum) = maximum
            && matches.len() > maximum
        {
            return Err(RetrieveError::TooManyMatches {
                observed: maximum.saturating_add(1),
                maximum,
            });
        }
        Ok(())
    }

    fn walk(
        &self,
        scan_dir: &Path,
        match_dir: &Path,
        state: &mut WalkState<'_>,
    ) -> Result<(), RetrieveError> {
        if !state.visited.insert(scan_dir.to_path_buf()) {
            return Ok(());
        }
        let entries = self
            .root
            .read_dir(capability_path(scan_dir))
            .map_err(|source| map_io(self.id_for(scan_dir), source))?;
        for entry in entries {
            let entry = entry.map_err(|source| map_io(self.id_for(scan_dir), source))?;
            let name = entry.file_name();
            let scan_path = scan_dir.join(&name);
            let match_path = match_dir.join(&name);
            let file_type = entry
                .file_type()
                .map_err(|source| map_io(self.id_for(&scan_path), source))?;

            if file_type.is_symlink() {
                if !self.follow_symlinks {
                    if (state.matcher)(&match_path, &name) {
                        return Err(RetrieveError::SymlinkDenied(self.id_for(&scan_path)));
                    }
                    continue;
                }
                let canonical = self.canonical_existing(&scan_path)?;
                let metadata = self
                    .root
                    .metadata(&canonical)
                    .map_err(|source| map_io(self.id_for(&canonical), source))?;
                if metadata.is_dir() {
                    if state.recursive {
                        self.walk(&canonical, &match_path, state)?;
                    }
                } else if metadata.is_file() && (state.matcher)(&match_path, &name) {
                    self.record_match(canonical, state.maximum, state.matches)?;
                } else if (state.matcher)(&match_path, &name) {
                    return Err(RetrieveError::NotRegularFile(self.id_for(&canonical)));
                }
                continue;
            }

            if file_type.is_dir() {
                if state.recursive {
                    self.walk(&scan_path, &match_path, state)?;
                }
            } else if file_type.is_file() && (state.matcher)(&match_path, &name) {
                self.record_match(scan_path, state.maximum, state.matches)?;
            } else if (state.matcher)(&match_path, &name) {
                return Err(RetrieveError::NotRegularFile(self.id_for(&scan_path)));
            }
        }
        Ok(())
    }
}

impl ResourceRetriever for FileRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let relative = self.relative_path(&request.id)?;
        self.approved(&relative)?;
        let canonical = self.canonical_existing(&relative)?;
        self.approved(&canonical)?;
        let id = self.id_for(&canonical);
        let metadata = self
            .root
            .metadata(&canonical)
            .map_err(|source| map_io(id.clone(), source))?;
        if !metadata.is_file() {
            return Err(RetrieveError::NotRegularFile(id));
        }
        if let Some(maximum) = request.max_bytes
            && metadata.len() > maximum as u64
        {
            return Err(RetrieveError::TooLarge {
                id,
                observed: usize::try_from(metadata.len()).unwrap_or(usize::MAX),
                maximum,
            });
        }
        let bytes = self
            .root
            .read(&canonical)
            .map_err(|source| map_io(id.clone(), source))?;
        if let Some(maximum) = request.max_bytes
            && bytes.len() > maximum
        {
            return Err(RetrieveError::TooLarge {
                id,
                observed: bytes.len(),
                maximum,
            });
        }
        Ok(RetrievedResource {
            id,
            bytes,
            provenance: ResourceProvenance::default(),
        })
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        let base = self.relative_path(&request.base)?;
        let parent = base.parent().unwrap_or_else(|| Path::new(""));
        let canonical_parent = self.canonical_directory(parent)?;
        let mut matches = Vec::new();
        let mut visited = BTreeSet::new();

        match &request.selector {
            DiscoverySelector::Glob { pattern } => {
                reject_escaping_pattern(pattern, &request.base)?;
                let glob = compile_globs(std::slice::from_ref(pattern))?;
                let matcher = |candidate: &Path, _name: &OsStr| {
                    candidate
                        .strip_prefix(parent)
                        .is_ok_and(|relative| glob.is_match(relative))
                };
                let mut state = WalkState {
                    recursive: true,
                    matcher: &matcher,
                    maximum: request.max_matches,
                    matches: &mut matches,
                    visited: &mut visited,
                };
                self.walk(&canonical_parent, parent, &mut state)?;
            }
            DiscoverySelector::Directory {
                path,
                recursive,
                matches: patterns,
            } => {
                let requested_dir = parent.join(path);
                let requested_dir = normalize_relative(&requested_dir)
                    .ok_or_else(|| RetrieveError::OutsideRoot(request.base.clone()))?;
                let canonical_dir = self.canonical_directory(&requested_dir)?;
                self.approved(&canonical_dir)?;
                let defaults = ["*.yaml".to_owned(), "*.yml".to_owned()];
                let patterns = if patterns.is_empty() {
                    defaults.as_slice()
                } else {
                    patterns.as_slice()
                };
                let globs = compile_globs(patterns)?;
                let matcher = |_candidate: &Path, name: &OsStr| globs.is_match(Path::new(name));
                let mut state = WalkState {
                    recursive: *recursive,
                    matcher: &matcher,
                    maximum: request.max_matches,
                    matches: &mut matches,
                    visited: &mut visited,
                };
                self.walk(&canonical_dir, &requested_dir, &mut state)?;
            }
        }

        matches.sort_by(|left, right| compare_paths(left, right));
        matches.dedup();
        Ok(matches
            .into_iter()
            .map(|relative| self.id_for(&relative))
            .collect())
    }
}

struct WalkState<'a> {
    recursive: bool,
    matcher: &'a dyn Fn(&Path, &OsStr) -> bool,
    maximum: Option<usize>,
    matches: &'a mut Vec<PathBuf>,
    visited: &'a mut BTreeSet<PathBuf>,
}

fn compile_globs(patterns: &[String]) -> Result<GlobSet, RetrieveError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let mut glob = GlobBuilder::new(pattern);
        glob.literal_separator(true);
        let glob = glob
            .build()
            .map_err(|_| RetrieveError::InvalidPattern(pattern.clone()))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|source| RetrieveError::InvalidPattern(source.to_string()))
}

fn normalize_relative(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => normalized.push(component),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn capability_path(path: &Path) -> &Path {
    if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    }
}

fn reject_escaping_pattern(pattern: &str, id: &ResourceId) -> Result<(), RetrieveError> {
    if Path::new(pattern).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(RetrieveError::OutsideRoot(id.clone()));
    }
    Ok(())
}

fn map_io(id: ResourceId, source: io::Error) -> RetrieveError {
    if source.kind() == io::ErrorKind::NotFound {
        RetrieveError::NotFound(id)
    } else {
        RetrieveError::Io { id, source }
    }
}

#[cfg(unix)]
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    use std::os::unix::ffi::OsStrExt;

    left.as_os_str()
        .as_bytes()
        .cmp(right.as_os_str().as_bytes())
}

#[cfg(windows)]
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    use std::os::windows::ffi::OsStrExt;

    left.as_os_str()
        .encode_wide()
        .cmp(right.as_os_str().encode_wide())
}

#[cfg(not(any(unix, windows)))]
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    left.cmp(right)
}
