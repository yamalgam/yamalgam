# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

### Option 1: GitHub Security Advisories (Preferred)

Use [GitHub's private vulnerability reporting](https://github.com/yamalgam/yamalgam/security/advisories/new)
to submit a report directly.

### Option 2: Email

Send an email to `security@yamalgam.com` with:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact assessment
- Any suggested fixes (optional)

## What to Expect

- **Acknowledgment**: Within 48 hours of your report
- **Initial Assessment**: Within 7 days
- **Resolution Timeline**: Depends on severity and complexity

We will keep you informed of progress toward a fix and may ask for additional
information or guidance.

## Disclosure Policy

- We follow [coordinated disclosure](https://en.wikipedia.org/wiki/Coordinated_vulnerability_disclosure)
- Security fixes will be released as soon as practical
- We will credit reporters in release notes (unless anonymity is requested)

## Scope

This security policy applies to:

- The yamalgam CLI tool
- The yamalgam-core library
- Official distribution channels (crates.io, GitHub releases)

## Security Best Practices

When using yamalgam:

- Keep your installation up to date
- Verify checksums when downloading releases
- Report any suspicious behavior

## Unsafe Code Census

Workspace crates deny unsafe code. `just unsafe-census` additionally runs the
compiler's syntax-aware `unsafe_code` lint across the normal dependency graph
shipped by `yg`. It excludes benchmark and development-only dependencies, uses
the lockfile, disables compiler wrappers, caps compilation at one job, and
keeps its artifacts under `target/unsafe-census`.

Cargo-geiger 0.13 is not used as audit evidence in this repository because it
can exit successfully after an I/O error without emitting a census. A security
review must capture the diagnostics from `just unsafe-census`; an empty tool
report is not coverage.

Thank you for helping keep yamalgam secure!
