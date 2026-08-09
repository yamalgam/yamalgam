#![allow(missing_docs)]

use std::path::PathBuf;

use yamalgam_resolve::{Fragment, ResourceError, ResourceId, resolve_reference};

#[test]
fn relative_file_references_use_the_containing_directory() {
    let base = ResourceId::from_file("workspace/config/app.yaml");
    let resolved = base.resolve("../shared.yaml#/a~1b/~0name").unwrap();

    assert_eq!(
        resolved.resource,
        ResourceId::File(PathBuf::from("workspace/shared.yaml"))
    );
    assert_eq!(resolved.fragment.pointer_tokens(), ["a/b", "~name"]);
}

#[test]
fn file_uri_references_become_file_identities() {
    let target = std::env::temp_dir().join("yamalgam-shared.yaml");
    let target_url = url::Url::from_file_path(&target).unwrap();
    let reference = format!("{target_url}#/root");
    let base = ResourceId::from_file("workspace/app.yaml");
    let resolved = base.resolve(&reference).unwrap();

    assert_eq!(resolved.resource.as_path(), Some(target.as_path()));
    assert_eq!(resolved.fragment.pointer_tokens(), ["root"]);
}

#[test]
fn http_references_join_and_normalize_hosts() {
    let base = ResourceId::new("HTTP://EXAMPLE.COM/a/current.yaml").unwrap();
    let resolved = base.resolve("../shared.yaml").unwrap();
    let url = resolved.resource.as_url().unwrap();

    assert_eq!(url.as_str(), "http://example.com/shared.yaml");
    assert_eq!(resolved.fragment, Fragment::Root);
}

#[test]
fn absent_and_empty_fragments_select_the_root() {
    let base = ResourceId::new("https://example.com/document.yaml").unwrap();

    assert_eq!(base.resolve("").unwrap().fragment, Fragment::Root);
    assert_eq!(base.resolve("#").unwrap().fragment, Fragment::Root);
}

#[test]
fn percent_decoding_precedes_json_pointer_decoding() {
    let base = ResourceId::from_file("workspace/config/app.yaml");
    let resolved = base.resolve("../shared.yaml#/a%7E1b/%7E0name").unwrap();

    assert_eq!(resolved.fragment.pointer_tokens(), ["a/b", "~name"]);
}

#[test]
fn malformed_fragments_fail_closed() {
    let base = ResourceId::new("https://example.com/document.yaml").unwrap();
    let cases = [
        ("#/%ZZ", ResourceError::InvalidPercentEncoding),
        ("#/bad~2escape", ResourceError::InvalidPointerEscape),
        ("#not-a-pointer", ResourceError::InvalidPointer),
    ];

    for (reference, expected) in cases {
        let error = base.resolve(reference).unwrap_err();
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected)
        );
    }
}

#[test]
fn resource_ids_reject_fragments() {
    assert!(matches!(
        ResourceId::new("https://example.com/document.yaml#/value"),
        Err(ResourceError::FragmentNotAllowed)
    ));
}

#[test]
fn relative_references_require_a_base() {
    assert!(matches!(
        resolve_reference(None, "relative.yaml"),
        Err(ResourceError::MissingBase)
    ));

    let absolute = resolve_reference(None, "https://example.com/root.yaml#/value").unwrap();
    assert_eq!(absolute.resource.host_str(), Some("example.com"));
    assert_eq!(absolute.fragment.pointer_tokens(), ["value"]);
}

#[test]
fn anonymous_resources_allow_same_document_and_absolute_but_not_relative_targets() {
    let base = ResourceId::anonymous();

    assert_eq!(base.resolve("#/value").unwrap().resource, base);
    assert_eq!(
        base.resolve("https://example.com/root.yaml")
            .unwrap()
            .resource
            .host_str(),
        Some("example.com")
    );
    assert!(matches!(
        base.resolve("./relative.yaml"),
        Err(ResourceError::MissingBase)
    ));
    assert_eq!(base.to_string(), "<anonymous>");
}

#[test]
fn display_and_debug_redact_uri_credentials_and_queries() {
    let left = ResourceId::new("https://alice:secret@example.com/data.yaml?token=one").unwrap();
    let right = ResourceId::new("https://alice:secret@example.com/data.yaml?token=two").unwrap();

    assert_ne!(left, right);
    assert_eq!(left.as_url().unwrap().username(), "alice");
    assert_eq!(left.as_url().unwrap().query(), Some("token=one"));

    for rendered in [left.to_string(), format!("{left:?}")] {
        assert!(rendered.contains("https://example.com/data.yaml"));
        assert!(!rendered.contains("alice"));
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("token"));
        assert!(!rendered.contains("one"));
    }
}

#[cfg(unix)]
#[test]
fn file_identities_preserve_non_utf8_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let path = PathBuf::from(OsString::from_vec(b"document-\xff.yaml".to_vec()));
    let resource = ResourceId::from_file(path.clone());

    assert_eq!(
        resource.as_path().unwrap().as_os_str().as_bytes(),
        path.as_os_str().as_bytes()
    );
}
