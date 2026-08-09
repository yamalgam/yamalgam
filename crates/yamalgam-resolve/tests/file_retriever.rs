#![allow(missing_docs)]

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use yamalgam_core::FilePolicy;
use yamalgam_resolve::{
    DiscoveryRequest, DiscoverySelector, FileRetriever, ResourceId, ResourceRequest,
    ResourceRetriever, RetrieveError,
};

fn policy(root: &Path) -> FilePolicy {
    FilePolicy {
        root: Some(root.to_path_buf()),
        allow: Vec::new(),
        deny: Vec::new(),
        follow_symlinks: false,
    }
}

const fn request(id: ResourceId) -> ResourceRequest {
    ResourceRequest {
        id,
        max_bytes: None,
    }
}

fn setup() -> TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn retrieves_regular_relative_and_absolute_files_without_extension_rules() {
    let root = setup();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/document.yaml"), b"answer: 42").unwrap();
    fs::write(root.path().join("LICENSE"), b"terms").unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();

    let relative = retriever
        .retrieve(&request(ResourceId::from_file("nested/document.yaml")))
        .unwrap();
    assert_eq!(relative.bytes, b"answer: 42");
    assert_eq!(
        relative.id,
        ResourceId::from_file(root.path().join("nested/document.yaml"))
    );
    let absolute = retriever
        .retrieve(&request(ResourceId::from_file(root.path().join("LICENSE"))))
        .unwrap();
    assert_eq!(absolute.bytes, b"terms");
}

// r[verify resolution.include.sandboxing]
#[test]
fn rejects_traversal_and_absolute_targets_outside_the_root() {
    let root = setup();
    let outside = setup();
    fs::write(outside.path().join("secret.yaml"), b"secret").unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();

    for id in [
        ResourceId::from_file("../secret.yaml"),
        ResourceId::from_file(outside.path().join("secret.yaml")),
    ] {
        assert!(matches!(
            retriever.retrieve(&request(id)),
            Err(RetrieveError::OutsideRoot(_))
        ));
    }
}

#[test]
fn deny_patterns_win_before_allow_patterns() {
    let root = setup();
    fs::create_dir_all(root.path().join("public/private")).unwrap();
    fs::write(root.path().join("public/ok.yaml"), b"ok").unwrap();
    fs::write(root.path().join("public/private/no.yaml"), b"no").unwrap();
    let mut configured = policy(root.path());
    configured.allow = vec!["public/**/*.yaml".into(), "public/*.yaml".into()];
    configured.deny = vec!["public/private/**".into()];
    let retriever = FileRetriever::open(&configured).unwrap();

    assert!(
        retriever
            .retrieve(&request(ResourceId::from_file("public/ok.yaml")))
            .is_ok()
    );
    assert!(matches!(
        retriever.retrieve(&request(ResourceId::from_file("public/private/no.yaml"))),
        Err(RetrieveError::PolicyDenied { .. })
    ));
    assert!(matches!(
        retriever.retrieve(&request(ResourceId::from_file("other.yaml"))),
        Err(RetrieveError::PolicyDenied { .. })
    ));
}

#[test]
fn rejects_directories_and_enforces_byte_ceilings_inside_the_backend() {
    let root = setup();
    fs::create_dir(root.path().join("directory")).unwrap();
    fs::write(root.path().join("large.yaml"), b"12345").unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();

    assert!(matches!(
        retriever.retrieve(&request(ResourceId::from_file("directory"))),
        Err(RetrieveError::NotRegularFile(_))
    ));
    assert!(matches!(
        retriever.retrieve(&ResourceRequest {
            id: ResourceId::from_file("large.yaml"),
            max_bytes: Some(4),
        }),
        Err(RetrieveError::TooLarge {
            observed: 5,
            maximum: 4,
            ..
        })
    ));
}

#[cfg(unix)]
#[test]
fn rejects_non_regular_unix_socket_targets() {
    use std::os::unix::net::UnixListener;

    let root = setup();
    let _listener = UnixListener::bind(root.path().join("service.sock")).unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();
    assert!(matches!(
        retriever.retrieve(&request(ResourceId::from_file("service.sock"))),
        Err(RetrieveError::NotRegularFile(_))
    ));
}

#[cfg(unix)]
// r[verify resolution.include.sandboxing]
#[test]
fn rejects_final_and_intermediate_symlinks_by_default() {
    use std::os::unix::fs::symlink;

    let root = setup();
    fs::create_dir(root.path().join("real")).unwrap();
    fs::write(root.path().join("real/data.yaml"), b"value").unwrap();
    symlink("real/data.yaml", root.path().join("file-link.yaml")).unwrap();
    symlink("real", root.path().join("dir-link")).unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();

    for path in ["file-link.yaml", "dir-link/data.yaml"] {
        assert!(matches!(
            retriever.retrieve(&request(ResourceId::from_file(path))),
            Err(RetrieveError::SymlinkDenied(_))
        ));
    }
}

#[cfg(unix)]
// r[verify resolution.include.sandboxing]
#[test]
fn enabled_symlinks_canonicalize_inside_and_reject_escape() {
    use std::os::unix::fs::symlink;

    let root = setup();
    let outside = setup();
    fs::create_dir(root.path().join("real")).unwrap();
    fs::write(root.path().join("real/data.yaml"), b"inside").unwrap();
    fs::write(outside.path().join("secret.yaml"), b"outside").unwrap();
    symlink("real/data.yaml", root.path().join("inside.yaml")).unwrap();
    symlink(
        outside.path().join("secret.yaml"),
        root.path().join("outside.yaml"),
    )
    .unwrap();
    let mut configured = policy(root.path());
    configured.follow_symlinks = true;
    let retriever = FileRetriever::open(&configured).unwrap();

    let inside = retriever
        .retrieve(&request(ResourceId::from_file("inside.yaml")))
        .unwrap();
    assert_eq!(inside.bytes, b"inside");
    assert_eq!(
        inside.id,
        ResourceId::from_file(root.path().join("real/data.yaml"))
    );
    assert!(matches!(
        retriever.retrieve(&request(ResourceId::from_file("outside.yaml"))),
        Err(RetrieveError::OutsideRoot(_))
    ));
}

// r[verify resolution.include.order]
// r[verify resolution.include.sandboxing]
#[test]
fn glob_and_directory_discovery_are_sorted_and_mode_aware() {
    let root = setup();
    fs::create_dir_all(root.path().join("data/nested")).unwrap();
    for path in [
        "data/z.yaml",
        "data/a.yml",
        "data/note.txt",
        "data/nested/m.yaml",
    ] {
        fs::write(root.path().join(path), path).unwrap();
    }
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();
    let base = ResourceId::from_file(root.path().join("root.yaml"));

    let globbed = retriever
        .discover(&DiscoveryRequest {
            base: base.clone(),
            selector: DiscoverySelector::Glob {
                pattern: "data/*.y*ml".into(),
            },
            max_matches: None,
        })
        .unwrap();
    assert_eq!(
        globbed,
        ["data/a.yml", "data/z.yaml"].map(|path| ResourceId::from_file(root.path().join(path)))
    );

    let shallow = retriever
        .discover(&DiscoveryRequest {
            base: base.clone(),
            selector: DiscoverySelector::Directory {
                path: "data".into(),
                recursive: false,
                matches: Vec::new(),
            },
            max_matches: None,
        })
        .unwrap();
    assert_eq!(shallow, globbed);

    let recursive = retriever
        .discover(&DiscoveryRequest {
            base,
            selector: DiscoverySelector::Directory {
                path: "data".into(),
                recursive: true,
                matches: Vec::new(),
            },
            max_matches: None,
        })
        .unwrap();
    assert_eq!(
        recursive,
        ["data/a.yml", "data/nested/m.yaml", "data/z.yaml"]
            .map(|path| ResourceId::from_file(root.path().join(path)))
    );
}

#[test]
fn discovery_returns_empty_and_stops_at_the_match_ceiling() {
    let root = setup();
    for path in ["a.yaml", "b.yaml", "c.yaml"] {
        fs::write(root.path().join(path), path).unwrap();
    }
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();
    let base = ResourceId::from_file(root.path().join("root.yaml"));

    let empty = retriever
        .discover(&DiscoveryRequest {
            base: base.clone(),
            selector: DiscoverySelector::Glob {
                pattern: "*.json".into(),
            },
            max_matches: Some(1),
        })
        .unwrap();
    assert!(empty.is_empty());
    assert!(matches!(
        retriever.discover(&DiscoveryRequest {
            base,
            selector: DiscoverySelector::Glob {
                pattern: "*.yaml".into(),
            },
            max_matches: Some(2),
        }),
        Err(RetrieveError::TooManyMatches {
            observed: 3,
            maximum: 2,
        })
    ));
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_file_identities_remain_lossless() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let root = setup();
    let name = OsString::from_vec(b"data-\xff.yaml".to_vec());
    let path = root.path().join(&name);
    fs::write(&path, b"value").unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();
    let result = retriever
        .retrieve(&request(ResourceId::from_file(path.clone())))
        .unwrap();
    assert_eq!(
        result.id.as_path().unwrap().as_os_str().as_bytes(),
        path.as_os_str().as_bytes()
    );
}

#[cfg(unix)]
#[test]
fn discovery_rejects_matching_symlinks_and_skips_symlink_directories() {
    use std::os::unix::fs::symlink;

    let root = setup();
    fs::create_dir_all(root.path().join("real/nested")).unwrap();
    fs::write(root.path().join("real/data.yaml"), b"value").unwrap();
    fs::write(root.path().join("real/nested/hidden.yaml"), b"hidden").unwrap();
    symlink("real/data.yaml", root.path().join("linked.yaml")).unwrap();
    symlink("real/nested", root.path().join("linked-dir")).unwrap();
    let retriever = FileRetriever::open(&policy(root.path())).unwrap();
    let base = ResourceId::from_file(root.path().join("root.yaml"));

    assert!(matches!(
        retriever.discover(&DiscoveryRequest {
            base: base.clone(),
            selector: DiscoverySelector::Glob {
                pattern: "*.yaml".into(),
            },
            max_matches: None,
        }),
        Err(RetrieveError::SymlinkDenied(_))
    ));
    let recursive = retriever
        .discover(&DiscoveryRequest {
            base,
            selector: DiscoverySelector::Directory {
                path: "real".into(),
                recursive: true,
                matches: Vec::new(),
            },
            max_matches: None,
        })
        .unwrap();
    assert_eq!(recursive.len(), 2);
    assert!(
        recursive
            .iter()
            .all(|id| !id.to_string().contains("linked-dir"))
    );
}

#[test]
fn opening_requires_a_root() {
    let mut configured = policy(Path::new("unused"));
    configured.root = None;
    assert!(matches!(
        FileRetriever::open(&configured),
        Err(RetrieveError::MissingRoot)
    ));
}

#[test]
fn opening_rejects_invalid_policy_patterns() {
    let root = setup();
    let mut configured = policy(root.path());
    configured.allow = vec!["[invalid".into()];
    assert!(matches!(
        FileRetriever::open(&configured),
        Err(RetrieveError::InvalidPattern(_))
    ));
}
