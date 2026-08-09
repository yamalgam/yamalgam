---
audit: 2026-08-09-13-m11-final-review
last_updated: 2026-08-09
status:
  fixed: 9
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 11
---

# Actions Taken: M11 External Resource Resolution

Summary of remediation status for the
[2026-08-09 M11 external-resource-resolution audit](README.md).

---

## 2026-08-09 — close URI selector dispatch gaps

**Disposition:** fixed

**Addresses:**
[scheme-relative-ref-becomes-local-file](README.md#scheme-relative-ref-becomes-local-file),
[include-uri-bypasses-relative-selector-contract](README.md#include-uri-bypasses-relative-selector-contract)

**Commit:** `88ab70284a75e95b5c0b0eafed4d04d7e5c205a1`

**Author:** Codex

URI-base resolution now recognizes RFC 3986 network-path references before
platform path dispatch, while retaining the existing absolute-file behavior.
`!include` validation rejects all absolute URI selectors before retrieval or
discovery. Regression coverage exercises scheme inheritance and file, memory,
custom, and HTTP include schemes.

---

## 2026-08-09 — seal and share resource identities

**Disposition:** fixed

**Addresses:**
[resource-id-public-variants-bypass-canonical-invariants](README.md#resource-id-public-variants-bypass-canonical-invariants),
[owned-nodes-clone-resource-identities-per-node](README.md#owned-nodes-clone-resource-identities-per-node)

**Commit:** `cf58f50f7b3500d847c015d684f5a8a35572fb44`

**Author:** Codex

`ResourceId` is now a private checked wrapper. File paths and URLs are created
only through normalizing constructors and stored behind immutable `Arc`
ownership, making every ordinary identity clone constant-time. A compile-fail
doctest guards the public construction boundary, and unit coverage verifies
that file and URI clones share their owned storage.

---

## 2026-08-09 — harden filesystem retrieval and discovery

**Disposition:** fixed

**Addresses:**
[file-policy-checks-race-path-reopen](README.md#file-policy-checks-race-path-reopen),
[symlink-discovery-skips-alias-policy](README.md#symlink-discovery-skips-alias-policy),
[recursive-file-discovery-has-no-depth-bound](README.md#recursive-file-discovery-has-no-depth-bound),
[symlink-aliases-make-glob-results-enumeration-dependent](README.md#symlink-aliases-make-glob-results-enumeration-dependent),
[discovery-results-do-not-enforce-deterministic-order](README.md#discovery-results-do-not-enforce-deterministic-order)

**Commit:** `ac64840f59559fb6b27c102086771683f85abb01`

**Author:** Codex

Filesystem reads now resolve the configured symlink policy once, open each
canonical path component without following symlinks, validate the opened file,
and read at most the configured ceiling plus one byte from that same handle.
Discovery uses an explicit work stack, lexically sorted directory entries, and
an active-path cycle set that permits distinct aliases without recursive stack
growth. Both alias and canonical paths pass policy checks. The session contract
also sorts and deduplicates results from custom retrievers before aggregation.
Regression coverage exercises denied aliases, independently selected aliases,
deep traversal, file-only allow patterns, and unsorted duplicate discoveries.
