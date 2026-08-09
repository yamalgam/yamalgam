---
audit_date: 2026-08-08
project: yamalgam
commit: f72e662bb2e5e83bfd4d3c4c4ce58d805e851c90
scope: Full repository static audit — all workspace crates, xtask, fuzz package, dependencies, configuration, and current documentation
auditor: "Codex (GPT-5-generated, independently GPT-5-verified)"
verification:
  confirmed: 4
  adjusted: 1
  disputed: 1
findings:
  critical: 0
  significant: 0
  moderate: 4
  advisory: 1
  note: 0
---

# Audit: yamalgam — Full Repository After Remediation

This full-repository audit at commit
`f72e662bb2e5e83bfd4d3c4c4ce58d805e851c90` finds a materially stronger
workspace after the prior 27-finding remediation campaign. **The Resource
Limit Error Surface** enforces the limits but exposes failures only as strings;
**The Comparison Fixture Input Surface** retains two concrete UTF-8 panic
paths outside the shipped parser. **The Repository Dependency Policy Surface**
is clean for the root workspace but omits the excluded fuzz graph, while **The
Scanner Presentation Metadata Surface** still falls short of ADR-0002's
fidelity contract. No finding is critical or significant, and the shipped
parser's safety posture is solid.

## The Resource Limit Error Surface

*Resource limits are enforced, but their public check API collapses the
breached limit and observed value into display text.*

### resource-limit-checks-return-unstructured-strings

Public resource-limit checks erase structured limit details into strings

**moderate** · criterion `5.4` ·
`crates/yamalgam-core/src/loader.rs:224-234` · effort: medium ·
<img src="assets/sparkline-resource-limit-checks-return-unstructured-strings.svg" height="14" alt="commit activity for resource-limit-checks-return-unstructured-strings" />

`ResourceLimits` exposes its failure contract as `String`. Callers cannot
inspect the breached limit, observed value, or configured maximum without
parsing display text. The other public check methods repeat this signature, so
parser, scanner, compose, serde, CST, and query wrap opaque messages rather
than preserving structured limit fields.

```rust crates/yamalgam-core/src/loader.rs:224-234
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
```

**Remediation:** Introduce a public limit error carrying the limit category,
observed value, and configured maximum. Return it from each check method and
preserve it as a source or typed payload through consumer error types.

*Verdict: Enforcement is present and consistently called; the gap is
diagnostic structure, not missing protection.*

---

## The Comparison Fixture Input Surface

*Two comparison helpers validate byte lengths before slicing, but do not prove
that the chosen byte is a UTF-8 boundary.*

### compare-tree-parser-panics-on-nonascii-style

The public tree-format parser panics on a non-ASCII style character

**moderate** · criterion `11.1` ·
`crates/yamalgam-compare/src/tree_format.rs:202-215` · effort: trivial ·
<img src="assets/sparkline-compare-tree-parser-panics-on-nonascii-style.svg" height="14" alt="commit activity for compare-tree-parser-panics-on-nonascii-style" />

`parse_tree("=VALé")` reaches `parse_val` with `after_props` equal to `"é"`.
The non-empty check succeeds, but byte offset 1 falls inside the two-byte
character. The slice panics instead of rejecting the malformed external
tree-format line.

```rust crates/yamalgam-compare/src/tree_format.rs:202-215
/// Parse a `=VAL` line remainder into a Scalar [`EventSnapshot`].
///
/// The remainder has the form: `[&anchor] [<tag>] STYLE_CHAR value`
/// where `STYLE_CHAR` is one of `:`, `'`, `"`, `|`, `>`.
fn parse_val(rest: &str) -> Option<EventSnapshot> {
    let (anchor, tag, after_props) = parse_val_properties(rest);

    // The next character is the style indicator, followed by the raw value.
    if after_props.is_empty() {
        return None;
    }

    let value = &after_props[1..]; // everything after the style char
    let decoded = unescape_tree_value(value);
```

> One non-ASCII character in malformed expected-tree text turns a comparison
> failure into a process panic.

**Related:** [Unicode indentation can panic the YAML Test Suite
extractor](#test-suite-extractor-slices-inside-unicode-indentation).

**Remediation:** Parse and validate the documented ASCII style character
explicitly. Slice at the returned character boundary, or return `None` for an
unsupported style, and add a multibyte regression.

### test-suite-extractor-slices-inside-unicode-indentation

Unicode indentation can panic the YAML Test Suite extractor

**moderate** · criterion `11.1` ·
`crates/yamalgam-compare/src/test_case.rs:100-113` · effort: small ·
<img src="assets/sparkline-test-suite-extractor-slices-inside-unicode-indentation.svg" height="14" alt="commit activity for test-suite-extractor-slices-inside-unicode-indentation" />

An explicit indent of four bytes followed by two ASCII spaces and a three-byte
Unicode em space produces `current_indent == 5`. The bounds check passes, but
byte offset 4 lies inside the em space, so slicing panics. Inferred indentation
does not share the bug because its offset comes from a valid `trim_start`
boundary.

```rust crates/yamalgam-compare/src/test_case.rs:100-113
while idx < lines.len() {
    let line = lines[idx];

    if let Some(min_indent) = indent {
        let stripped = line.trim_start();
        let current_indent = line.len() - stripped.len();

        if !line.trim().is_empty() && current_indent < min_indent {
            break;
        }

        if line.len() >= min_indent {
            content_lines.push(&line[min_indent..]);
        } else if line.trim().is_empty() {
```

> Extended whitespace in one external test case can abort the entire suite
> extraction pass.

**Related:** [The public tree-format parser panics on a non-ASCII style
character](#compare-tree-parser-panics-on-nonascii-style).

**Remediation:** Walk `char_indices` over whitespace and choose a verified
UTF-8 boundary, or reject non-ASCII indentation before slicing. Add a focused
explicit-indentation regression.

*Verdict: Both paths are outside the shipped CLI parser, but each is public,
processes external fixture text, and has a concrete one-line panic trigger.*

---

## The Repository Dependency Policy Surface

*The root workspace graph passes dependency policy, while the separately
locked fuzz package sits outside that canonical gate.*

### fuzz-workspace-escapes-dependency-policy

The excluded fuzz graph bypasses dependency policy and retains a stale lockfile

**advisory** · criterion `7.1` · `.justfile:20-22` · effort: small ·
<img src="assets/sparkline-fuzz-workspace-escapes-dependency-policy.svg" height="14" alt="commit activity for fuzz-workspace-escapes-dependency-policy" />

`fuzz` is excluded from the root workspace and owns a separate lockfile, so the
canonical Cargo Deny invocation never evaluates it. A focused fuzz-manifest
check rejects `libfuzzer-sys` because its license expression includes NCSA,
which project policy does not allow, and reports the fuzz package itself as
unlicensed. Its lockfile also retains obsolete `serde_yaml`, `unsafe-libyaml`,
Figment, logging, and configuration edges. This is fuzz-only, but repository
policy does not cover every first-party manifest.

```makefile .justfile:20-22
# Check dependencies for security advisories and license compliance
deny:
    cargo deny --all-features --config .config/deny.toml check
```

**Remediation:** Add the workspace license expression to the excluded fuzz
package. Decide and document whether NCSA is allowed for the fuzz-only
dependency, refresh `fuzz/Cargo.lock`, and extend the canonical dependency gate
with a locked Cargo Deny invocation for `fuzz/Cargo.toml`.

*Verdict: The shipped graph has no active advisory or unused dependency; the
remaining gap is repository-wide policy consistency for the fuzz graph.*

---

## The Scanner Presentation Metadata Surface

*The scanner exposes style, chomp, and content metadata in each atom, but its
shared scalar constructor fills two of those fields with defaults.*

### scanner-never-populates-atom-presentation-metadata

Scanner exposes atom metadata but always returns defaults

**moderate** · criterion `FC-1` ·
`crates/yamalgam-scanner/src/scanner.rs:290-307` · effort: medium ·
<img src="assets/sparkline-scanner-never-populates-atom-presentation-metadata.svg" height="14" alt="commit activity for scanner-never-populates-atom-presentation-metadata" />

ADR-0002 says atom style, chomp, and content flags survive scanning so
consumers can reconstruct presentation and use direct-output paths. All scalar
construction passes through `scalar_token`, which hard-codes the chomp and
flags. The block-scalar scanner computes a chomp indicator but does not pass it
through. Public scanner consumers therefore cannot distinguish clip, strip,
and keep metadata or observe documented flags.

```rust crates/yamalgam-scanner/src/scanner.rs:290-307
/// Build a scalar token with owned or borrowed data and explicit style.
fn scalar_token(
    data: Cow<'input, str>,
    style: ScalarStyle,
    start: yamalgam_core::Mark,
    end: yamalgam_core::Mark,
) -> Token<'input> {
    Token {
        kind: TokenKind::Scalar,
        atom: Atom {
            data,
            span: Span { start, end },
            style,
            chomp: Chomp::default(),
            flags: AtomFlags::empty(),
        },
    }
}
```

**Remediation:** Preserve the computed chomp value when constructing
block-scalar atoms and derive `AtomFlags` from scalar content and scanner
context. Thread the metadata through the documented emitter fast path, with
regressions for `|-`, `|`, `|+`, whitespace, line breaks, and direct-output
flag cases.

*Verdict: Scalar values and styles are preserved, so parsing remains correct;
the gap affects presentation fidelity and the documented fast-path contract.*

---

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---------|---------|----------|--------|--------|
| _The Resource Limit Error Surface_ |  |  |  |  |
| [resource-limit-checks-return-unstructured-strings](#resource-limit-checks-return-unstructured-strings) | moderate | `crates/yamalgam-core/src/loader.rs:224-234` | medium | — |
| _The Comparison Fixture Input Surface_ |  |  |  |  |
| [compare-tree-parser-panics-on-nonascii-style](#compare-tree-parser-panics-on-nonascii-style) | moderate | `crates/yamalgam-compare/src/tree_format.rs:202-215` | trivial | related: [test-suite-extractor-slices-inside-unicode-indentation](#test-suite-extractor-slices-inside-unicode-indentation) |
| [test-suite-extractor-slices-inside-unicode-indentation](#test-suite-extractor-slices-inside-unicode-indentation) | moderate | `crates/yamalgam-compare/src/test_case.rs:100-113` | small | related: [compare-tree-parser-panics-on-nonascii-style](#compare-tree-parser-panics-on-nonascii-style) |
| _The Repository Dependency Policy Surface_ |  |  |  |  |
| [fuzz-workspace-escapes-dependency-policy](#fuzz-workspace-escapes-dependency-policy) | advisory | `.justfile:20-22` | small | — |
| _The Scanner Presentation Metadata Surface_ |  |  |  |  |
| [scanner-never-populates-atom-presentation-metadata](#scanner-never-populates-atom-presentation-metadata) | moderate | `crates/yamalgam-scanner/src/scanner.rs:290-307` | medium | — |

<sub>
Generated 2026-08-08 at commit `f72e662`. Intermediate artifacts:
recon.yaml, findings.yaml, reviewer-verdicts.yaml, and report.html. Provenance:
Codex (GPT-5-generated, independently GPT-5-verified).
</sub>
