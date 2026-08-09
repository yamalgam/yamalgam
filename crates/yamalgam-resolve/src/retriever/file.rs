//! Capability-rooted filesystem retrieval and discovery.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use yamalgam_core::FilePolicy;

use super::{
    DiscoveryRequest, DiscoveryResult, DiscoverySelector, ResourceProvenance, ResourceRequest,
    ResourceRetriever, RetrievedResource,
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

    fn traversal_approved(&self, relative: &Path) -> Result<(), RetrieveError> {
        if self.deny.is_match(relative) {
            return Err(RetrieveError::PolicyDenied {
                id: self.id_for(relative),
            });
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
        let directory = self.open_stable_directory(&canonical)?;
        let metadata = directory
            .dir_metadata()
            .map_err(|source| map_io(self.id_for(&canonical), source))?;
        if !metadata.is_dir() {
            return Err(RetrieveError::NotRegularFile(self.id_for(&canonical)));
        }
        Ok(canonical)
    }

    fn open_stable_directory(&self, relative: &Path) -> Result<Dir, RetrieveError> {
        let mut directory = self
            .root
            .try_clone()
            .map_err(|source| map_io(self.id_for(Path::new("")), source))?;
        let mut current = PathBuf::new();
        for component in relative.components() {
            current.push(component.as_os_str());
            directory = directory
                .open_dir_nofollow(component.as_os_str())
                .map_err(|source| map_io(self.id_for(&current), source))?;
        }
        Ok(directory)
    }

    fn open_stable_file(&self, relative: &Path) -> Result<File, RetrieveError> {
        let id = self.id_for(relative);
        let Some(name) = relative.file_name() else {
            return Err(RetrieveError::NotRegularFile(id));
        };
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let directory = self.open_stable_directory(parent)?;
        let file_type = directory
            .symlink_metadata(name)
            .map_err(|source| map_io(id.clone(), source))?
            .file_type();
        if !file_type.is_file() {
            return Err(RetrieveError::NotRegularFile(id));
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        directory
            .open_with(name, &options)
            .map_err(|source| map_io(id, source))
    }

    fn id_for(&self, relative: &Path) -> ResourceId {
        ResourceId::from_file(self.root_path.join(relative))
    }

    fn record_match(
        &self,
        requested: &Path,
        canonical: PathBuf,
        state: &mut WalkState<'_>,
    ) -> Result<(), RetrieveError> {
        self.approved(requested)?;
        self.approved(&canonical)?;
        if state.matches.insert(canonical)
            && let Some(maximum) = state.max_matches
            && state.matches.len() > maximum
        {
            return Err(RetrieveError::TooManyMatches {
                observed: state.matches.len(),
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
        let mut active = BTreeSet::new();
        let mut work = vec![WalkFrame::Enter {
            scan_dir: scan_dir.to_path_buf(),
            match_dir: match_dir.to_path_buf(),
            policy_dir: match_dir.to_path_buf(),
            depth: 0,
        }];

        while let Some(frame) = work.pop() {
            let WalkFrame::Enter {
                scan_dir,
                match_dir,
                policy_dir,
                depth,
            } = frame
            else {
                let WalkFrame::Exit(scan_dir) = frame else {
                    unreachable!();
                };
                active.remove(&scan_dir);
                continue;
            };
            if !active.insert(scan_dir.clone()) {
                continue;
            }
            state.visit()?;

            let directory = self.open_stable_directory(&scan_dir)?;
            let entries = directory
                .entries()
                .map_err(|source| map_io(self.id_for(&scan_dir), source))?;
            let mut names = Vec::new();
            for entry in entries {
                state.visit()?;
                names.push(
                    entry
                        .map(|entry| entry.file_name())
                        .map_err(|source| map_io(self.id_for(&scan_dir), source))?,
                );
            }
            names.sort_by(|left, right| compare_paths(Path::new(left), Path::new(right)));

            let mut children = Vec::new();
            for name in names {
                let scan_path = scan_dir.join(&name);
                let match_path = match_dir.join(&name);
                let policy_path = policy_dir.join(&name);
                let file_type = directory
                    .symlink_metadata(&name)
                    .map_err(|source| map_io(self.id_for(&scan_path), source))?
                    .file_type();

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
                        .symlink_metadata(&canonical)
                        .map_err(|source| map_io(self.id_for(&canonical), source))?;
                    if metadata.is_dir() {
                        if state.descends_from(depth) {
                            self.traversal_approved(&policy_path)?;
                            self.traversal_approved(&canonical)?;
                            children.push(WalkFrame::Enter {
                                scan_dir: canonical,
                                match_dir: match_path,
                                policy_dir: policy_path,
                                depth: depth.checked_add(1).ok_or(
                                    RetrieveError::ResourceOverflow("discovery traversal depth"),
                                )?,
                            });
                        }
                    } else if metadata.is_file() && (state.matcher)(&match_path, &name) {
                        self.record_match(&policy_path, canonical, state)?;
                    } else if (state.matcher)(&match_path, &name) {
                        self.approved(&policy_path)?;
                        self.approved(&canonical)?;
                        return Err(RetrieveError::NotRegularFile(self.id_for(&canonical)));
                    }
                    continue;
                }

                if file_type.is_dir() {
                    if state.descends_from(depth) {
                        self.traversal_approved(&policy_path)?;
                        self.traversal_approved(&scan_path)?;
                        children.push(WalkFrame::Enter {
                            scan_dir: scan_path,
                            match_dir: match_path,
                            policy_dir: policy_path,
                            depth: depth.checked_add(1).ok_or(RetrieveError::ResourceOverflow(
                                "discovery traversal depth",
                            ))?,
                        });
                    }
                } else if file_type.is_file() && (state.matcher)(&match_path, &name) {
                    self.record_match(&policy_path, scan_path.clone(), state)?;
                } else if (state.matcher)(&match_path, &name) {
                    self.approved(&policy_path)?;
                    return Err(RetrieveError::NotRegularFile(self.id_for(&scan_path)));
                }
            }

            work.push(WalkFrame::Exit(scan_dir));
            work.extend(children.into_iter().rev());
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
        let mut file = self.open_stable_file(&canonical)?;
        let metadata = file
            .metadata()
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
        let mut bytes = Vec::new();
        if let Some(maximum) = request.max_bytes {
            let limit = u64::try_from(maximum.saturating_add(1)).unwrap_or(u64::MAX);
            file.by_ref()
                .take(limit)
                .read_to_end(&mut bytes)
                .map_err(|source| map_io(id.clone(), source))?;
        } else {
            file.read_to_end(&mut bytes)
                .map_err(|source| map_io(id.clone(), source))?;
        }
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

    fn discover(&self, request: &DiscoveryRequest) -> Result<DiscoveryResult, RetrieveError> {
        let base = self.relative_path(&request.base)?;
        let parent = base.parent().unwrap_or_else(|| Path::new(""));
        self.traversal_approved(parent)?;
        let mut matches = BTreeSet::new();
        let mut visited = 0;

        match &request.selector {
            DiscoverySelector::Glob { pattern } => {
                reject_escaping_pattern(pattern, &request.base)?;
                let glob = compile_globs(std::slice::from_ref(pattern))?;
                let (static_prefix, max_depth) = glob_scan_plan(pattern);
                let requested_dir = parent.join(static_prefix);
                let requested_dir = normalize_relative(&requested_dir)
                    .ok_or_else(|| RetrieveError::OutsideRoot(request.base.clone()))?;
                let canonical_dir = match self.canonical_directory(&requested_dir) {
                    Ok(directory) => directory,
                    Err(RetrieveError::NotFound(_)) => {
                        return Ok(DiscoveryResult {
                            resources: Vec::new(),
                            visited: 0,
                        });
                    }
                    Err(error) => return Err(error),
                };
                self.traversal_approved(&requested_dir)?;
                self.traversal_approved(&canonical_dir)?;
                let matcher = |candidate: &Path, _name: &OsStr| {
                    candidate
                        .strip_prefix(parent)
                        .is_ok_and(|relative| glob.is_match(relative))
                };
                let mut state = WalkState {
                    max_depth,
                    matcher: &matcher,
                    max_matches: request.max_matches,
                    max_visits: request.max_visits,
                    visited: &mut visited,
                    matches: &mut matches,
                };
                self.walk(&canonical_dir, &requested_dir, &mut state)?;
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
                self.traversal_approved(&requested_dir)?;
                self.traversal_approved(&canonical_dir)?;
                let defaults = ["*.yaml".to_owned(), "*.yml".to_owned()];
                let patterns = if patterns.is_empty() {
                    defaults.as_slice()
                } else {
                    patterns.as_slice()
                };
                let globs = compile_globs(patterns)?;
                let matcher = |_candidate: &Path, name: &OsStr| globs.is_match(Path::new(name));
                let mut state = WalkState {
                    max_depth: (!*recursive).then_some(0),
                    matcher: &matcher,
                    max_matches: request.max_matches,
                    max_visits: request.max_visits,
                    visited: &mut visited,
                    matches: &mut matches,
                };
                self.walk(&canonical_dir, &requested_dir, &mut state)?;
            }
        }

        let mut resources = matches.into_iter().collect::<Vec<_>>();
        resources.sort_by(|left, right| compare_paths(left, right));
        Ok(DiscoveryResult {
            resources: resources
                .into_iter()
                .map(|relative| self.id_for(&relative))
                .collect(),
            visited,
        })
    }
}

struct WalkState<'a> {
    max_depth: Option<usize>,
    matcher: &'a dyn Fn(&Path, &OsStr) -> bool,
    max_matches: Option<usize>,
    max_visits: Option<usize>,
    visited: &'a mut usize,
    matches: &'a mut BTreeSet<PathBuf>,
}

impl WalkState<'_> {
    fn visit(&mut self) -> Result<(), RetrieveError> {
        let observed = self
            .visited
            .checked_add(1)
            .ok_or(RetrieveError::ResourceOverflow("discovery visit count"))?;
        if let Some(maximum) = self.max_visits
            && observed > maximum
        {
            return Err(RetrieveError::TooManyVisits { observed, maximum });
        }
        *self.visited = observed;
        Ok(())
    }

    const fn descends_from(&self, depth: usize) -> bool {
        match self.max_depth {
            Some(maximum) => depth < maximum,
            None => true,
        }
    }
}

enum WalkFrame {
    Enter {
        scan_dir: PathBuf,
        match_dir: PathBuf,
        policy_dir: PathBuf,
        depth: usize,
    },
    Exit(PathBuf),
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

fn glob_scan_plan(pattern: &str) -> (PathBuf, Option<usize>) {
    let components = pattern
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .collect::<Vec<_>>();
    let dynamic = components
        .iter()
        .position(|component| has_glob_meta(component))
        .unwrap_or_else(|| components.len().saturating_sub(1));
    let prefix = components[..dynamic].iter().collect::<PathBuf>();
    let remaining = &components[dynamic..];
    let max_depth = (!remaining.iter().any(|component| component.contains("**")))
        .then_some(remaining.len().saturating_sub(1));
    (prefix, max_depth)
}

fn has_glob_meta(component: &str) -> bool {
    let mut escaped = false;
    for byte in component.bytes() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if matches!(byte, b'*' | b'?' | b'[' | b'{') {
            return true;
        }
    }
    false
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
