---
audit_date: 2026-04-08
project: yamalgam
commit: 61160528c570bd5845d15703e3f502136ea48e7d
scope: Full workspace — all 9 Rust crates plus xtask
auditor: claude-opus-4-6 via cased + crustoleum (4 parallel agents)
findings:
  critical: 0
  significant: 8
  moderate: 14
  advisory: 7
  note: 0
---

# Audit: yamalgam

yamalgam is a well-structured, zero-unsafe YAML 1.2 parser across 9
crates and ~20K lines of Rust, with clean clippy output and disciplined
workspace organization. **The Untrusted Input Surface** has two
depth-underflow paths in the serde deserializer that can panic on
malformed event streams — the most immediately actionable findings.
**The Error Architecture Surface** loses precision at each pipeline
stage, with the scanner's stringly-typed errors propagating opacity
downstream. **The Hot Path Surface** reveals that benchmarks
undercount release-build costs because bench profiles enable LTO but
release profiles do not, and the parser's dynamic dispatch prevents
cross-crate inlining. **The Supply Chain Surface** ships deprecated
serde_yaml transitively through figment. **The API Design Surface**
is sound but misses standard trait implementations. Fix the depth
underflows, add LTO to release, and drop the figment yaml feature —
those three changes address the audit's most significant findings
with minimal effort.

---

## The Untrusted Input Surface

*The parser's defense-in-depth is compromised by unsigned arithmetic
edge cases in the serde layer, where depth counters can underflow on
malformed event streams — debug panics, release infinite loops.*

### skip_value depth counter underflows on leading end-events

**significant** &middot; `crates/yamalgam-serde/src/de.rs:748-765` &middot; effort: trivial

If the first event consumed by `skip_value` is a `SequenceEnd` or
`MappingEnd` — possible via `from_tokens` with a custom iterator or
a parser bug — `depth` decrements from 0. In debug builds this panics
on unsigned underflow. In release builds it wraps to `u32::MAX` and
the function loops until the event stream is exhausted, creating an
infinite-loop DoS.

```rust crates/yamalgam-serde/src/de.rs:748-765
fn skip_value(&mut self) -> Result<(), Error> {
    let mut depth: u32 = 0;
    loop {
        let event = self.next_event()?;
        match &event {
            Event::SequenceEnd { .. } | Event::MappingEnd { .. } => {
                depth -= 1;    // underflows if first event is an end
            }
            // ...
        }
        if depth == 0 { return Ok(()); }
    }
}
```

> The depth starts at zero. Feed me a `MappingEnd` as the first event
> and I get `u32::MAX`. Now I will happily consume every remaining
> event in the stream before this function returns.

Related: [serde-drain-depth-underflow](#seqaccessdrain-and-mapaccessdrain-miss-cross-type-end-event-guard).

**Remediation:** Use `depth.checked_sub(1)` and return
`Error::Unexpected` on underflow. Three call sites.

<!-- whitespace is important -->
<div>&hairsp;</div>

### SeqAccess::drain and MapAccess::drain miss cross-type end-event guard

**significant** &middot; `crates/yamalgam-serde/src/de.rs:888-949` &middot; effort: trivial

`SeqAccess::drain` has a depth==0 guard for `SequenceEnd` (line 894)
but lets `MappingEnd` fall through to the decrement. `MapAccess::drain`
has the mirror bug — guards `MappingEnd` at depth 0 but not
`SequenceEnd`. A stray cross-type end event at depth 0 causes `u32`
underflow.

```rust crates/yamalgam-serde/src/de.rs:888-902
// SeqAccess::drain — MappingEnd at depth 0 hits the fallthrough:
Event::SequenceEnd { .. } | Event::MappingEnd { .. } => depth -= 1,

// MapAccess::drain — SequenceEnd at depth 0 hits the fallthrough:
Event::SequenceEnd { .. } | Event::MappingEnd { .. } => depth -= 1,
```

Related: [serde-skip-value-depth-underflow](#skip_value-depth-counter-underflows-on-leading-end-events).

**Remediation:** Add a depth==0 guard for the other end-event type, or
use `checked_sub`.

<!-- whitespace is important -->
<div>&hairsp;</div>

### CST insert_whitespace_before indexes source without clamping

**moderate** &middot; `crates/yamalgam-cst/src/lib.rs:488-491` &middot; effort: trivial

Unlike `source_text()` which clamps end with `.min(self.source.len())`,
`insert_whitespace_before` uses `span.start.offset` directly as the
slice bound. If a parser event carries an out-of-bounds offset, the
slice operation panics.

```rust crates/yamalgam-cst/src/lib.rs:488-491
if span.start.offset > self.last_offset {
    let ws_start = self.last_offset;
    let ws_end = span.start.offset;
    let ws_text = &self.source[ws_start..ws_end];  // no clamp
```

**Remediation:** Clamp `ws_end` with `.min(self.source.len())`,
matching the pattern already used in `source_text()`.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Scanner casts u32 column to i32 without overflow check

**advisory** &middot; `crates/yamalgam-scanner/src/scanner.rs:1020` &middot; effort: trivial

`Mark.column` is `u32`; the indent stack uses `i32` with -1 as
sentinel. Lines exceeding 2^31 characters wrap to negative, corrupting
indent comparisons. Theoretically reachable with
`ResourceLimits::none()` (the default constructor).

```rust crates/yamalgam-scanner/src/scanner.rs:1020
let col = mark.column as i32;
```

**Remediation:** Use `i32::try_from(column).unwrap_or(i32::MAX)`.

*Verdict: Two depth-underflow paths in the serde deserializer are the
most actionable findings in this audit. Both are trivial fixes — three
`checked_sub` calls + error returns. The CST indexing and column cast
follow the same pattern of trusting upstream invariants with arithmetic
rather than with Result. All four are small, contained fixes.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## The Error Architecture Surface

*Error types lose precision at each pipeline stage — scanner errors are
stringly-typed, compose errors lack spans, and serde errors discard
position information that users need for debugging.*

### ScanError is a stringly-typed bag with no variants

**significant** &middot; `crates/yamalgam-scanner/src/scanner.rs:34-47` &middot; effort: medium

`ScanError` has a single `message: String` field. 51 distinct error
sites construct it with ad-hoc string literals. Callers cannot
programmatically distinguish error kinds — "tab character used for
indentation" vs "unterminated quoted scalar" — without string matching.
`ParseError` wraps `ScanError`, so the opacity propagates through the
entire pipeline.

```rust crates/yamalgam-scanner/src/scanner.rs:34-47
pub struct ScanError {
    /// Human-readable error message.
    pub message: String,
}
```

> Every error looks the same to me. I get a `String`. Whether
> the document has a tab where it shouldn't or an unterminated
> string — it's all just prose I'd have to regex to classify.

Enables [compose-error-stringly-typed](#composeerror-variants-carry-only-string-no-structured-data),
[serde-error-no-span](#serde-errorunexpected-passes-none-span-at-15-call-sites).

**Remediation:** Add a `ScanErrorKind` enum alongside the message. Keep
the message for `Display` but give callers variants for match arms.
51 error sites need variant classification.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Diagnostic lacks Display and std::error::Error impls

**significant** &middot; `crates/yamalgam-core/src/diagnostic.rs:49-62` &middot; effort: trivial

`Diagnostic` is used as the `E` type in six public `Result` signatures
across `Input`'s API but implements neither `Display` nor
`std::error::Error`. Callers cannot use `?` to propagate through
`anyhow::Error` or `Box<dyn Error>`.

```rust crates/yamalgam-core/src/diagnostic.rs:49-62
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub span: Option<Span>,
    pub labels: Vec<Label>,
}
```

**Remediation:** Implement `Display` (formatting code + message) and
`Error` for `Diagnostic`. Non-breaking addition.

<!-- whitespace is important -->
<div>&hairsp;</div>

### ComposeError variants carry only String, no structured data

**moderate** &middot; `crates/yamalgam-compose/src/lib.rs:19-30` &middot; effort: small

`UnexpectedEvent` and `LimitExceeded` use `format!("{event:?}")`
strings. `UndefinedAlias` carries the name but no `Span`. Callers
cannot distinguish error sub-kinds without parsing the message text.

```rust crates/yamalgam-compose/src/lib.rs:19-30
pub enum ComposeError {
    Resolve(ResolveError),
    UndefinedAlias(String),
    UnexpectedEvent(String),
    LimitExceeded(String),
}
```

Enabled by [scanner-error-stringly-typed](#scanerror-is-a-stringly-typed-bag-with-no-variants).

**Remediation:** Add span fields and structured sub-data to variants.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Serde Error::Unexpected passes None span at 15+ call sites

**moderate** &middot; `crates/yamalgam-serde/src/error.rs:17-38` &middot; effort: medium

The `Unexpected` variant has an `Option<Span>` field but it is
populated with `None` at 21 call sites in `de.rs`.
`Custom(String)` from `serde::de::Error::custom` never carries
position information. Users debugging type mismatches get no
indication of where in the YAML the error occurred.

```rust crates/yamalgam-serde/src/error.rs:17-38
Unexpected {
    expected: &'static str,
    found: String,
    span: Option<Span>,    // has the field...
},
Custom(String),            // ...but Custom never does
```

Enabled by [scanner-error-stringly-typed](#scanerror-is-a-stringly-typed-bag-with-no-variants).

**Remediation:** Thread the most recently consumed event's span through
to error construction sites. The `Event` already carries a span.

<!-- whitespace is important -->
<div>&hairsp;</div>

### CST handle_error discards ParseError content

**advisory** &middot; `crates/yamalgam-cst/src/lib.rs:545` &middot; effort: trivial

The `ParseError` is received as a function argument but dropped with
`let _ = error; // consumed`. Callers can see where an error occurred
but not what went wrong.

```rust crates/yamalgam-cst/src/lib.rs:545
let _ = error; // consumed
```

**Remediation:** Store the `ParseError` or its `Display` string in the
error node.

*Verdict: ParseError (in the parser) is the model — typed variants with
spans. ScanError at the pipeline's foundation is a bare String, and
that opacity ripples through compose and serde. Fixing ScanError is the
highest-leverage error architecture change. The Diagnostic finding is a
quick win — two trait impls.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## The Panic Discipline Surface

*Library code relies on implicit invariants backed by expect/unwrap
rather than Result, creating 40+ panic sites on paths reachable from
the token stream.*

### Parser uses .expect("peeked") after peek_token — 40+ sites

**moderate** &middot; `crates/yamalgam-parser/src/parser.rs:292` &middot; effort: small

Over 40 locations follow the pattern: `peek_token()` confirms `Some`,
then `next_token()?.expect("peeked")` consumes it. The invariant is
correct — peek fills the lookahead buffer, next drains it — but it
depends on no interleaving calls between peek and next.

```rust crates/yamalgam-parser/src/parser.rs:292
let t = self.next_token()?.expect("peeked");
```

**Remediation:** Add a `consume_peeked()` helper returning `Result`
that encapsulates peek-then-take. Eliminates 40+ panic sites in one
abstraction.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Deserializer peek methods unwrap replay_buffer.front()

**moderate** &middot; `crates/yamalgam-serde/src/de.rs:268-301` &middot; effort: trivial

Four `.unwrap()` calls on `replay_buffer.front()` — each structurally
safe (post `push_front` or confirmed via if-let) but the invariant is
not local. A refactor removing the `push_front` turns these into panics.

```rust crates/yamalgam-serde/src/de.rs:274-279
let event = self.next_raw_event()?;
self.replay_buffer.push_front(event);
Ok(self.replay_buffer.front().unwrap())
```

**Remediation:** Return the pushed reference directly, or use `expect`
with a descriptive message.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Composer::peek_event double-unwraps via action-integer pattern

**advisory** &middot; `crates/yamalgam-compose/src/lib.rs:384` &middot; effort: small

Both `unwrap()` and `unwrap_err()` on a single line. Structurally
guaranteed by the preceding `peek()` match, but the action-integer
encoding obscures the invariant.

```rust crates/yamalgam-compose/src/lib.rs:384
let err = self.events.next().unwrap().unwrap_err();
```

**Remediation:** Restructure to use explicit match on
`self.events.next()` directly, eliminating the action-integer pattern.

*Verdict: The invariants behind each unwrap are correct in the current
code. The concern is not that they are wrong today but that they are
fragile — each depends on a guarantee from another method with no
compile-time enforcement. A `consume_peeked()` abstraction in the
parser would eliminate the largest cluster with a mechanical refactor.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## The Hot Path Surface

*The multi-crate parsing pipeline pays vtable dispatch, missing
inlining, and per-event allocation costs that benchmarks undercount
because bench profiles enable LTO but release profiles do not.*

### Parser uses Box&lt;dyn Iterator&gt; for token source

**significant** &middot; `crates/yamalgam-parser/src/parser.rs:72` &middot; effort: medium

Every call to `self.tokens.next()` goes through vtable dispatch. The
concrete type is always `Scanner` in the normal case, but the
indirection prevents inlining `Scanner::next()` into the parser's hot
loop. Combined with no LTO in release builds, this is an unrecoverable
optimization barrier.

```rust crates/yamalgam-parser/src/parser.rs:72
pub struct Parser<'input> {
    tokens: Box<dyn Iterator<Item = Result<Token<'input>, ScanError>> + 'input>,
```

> I'm paying a vtable call on every single token. The compiler can see
> the concrete type at every call site, but the `Box<dyn>` erases that
> information. If LTO is off, there's nothing it can do.

Related: [no-inline-cross-crate](#zero-inline-annotations-on-hot-readerscanner-functions),
[release-no-lto](#release-profile-has-no-lto).

**Remediation:** Make `Parser` generic over the token iterator:
`Parser<'input, I: Iterator<...>>`. Provide a type alias for the
common `Scanner` case. The `from_tokens()` constructor still accepts
any iterator — the concrete type just becomes visible to the compiler.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Zero #[inline] annotations on hot Reader/Scanner functions

**significant** &middot; `crates/yamalgam-scanner/src/reader.rs:40-90` &middot; effort: trivial

Without `#[inline]`, cross-crate calls rely on LTO. The release
profile has no LTO, meaning release builds get no inlining of
per-character functions. Reader's `peek()`, `peek_at()`, `advance()`,
`mark()`, and `is_eof()` are called on every character of input.

```rust crates/yamalgam-scanner/src/reader.rs:40-51
// no #[inline] — called on every character
pub fn peek(&self) -> Option<char> { ... }
pub fn peek_at(&self, n: usize) -> Option<char> { ... }
pub fn advance(&mut self) -> Option<char> { ... }
pub const fn mark(&self) -> Mark { ... }
pub const fn is_eof(&self) -> bool { ... }
```

Related: [parser-dyn-iterator-dispatch](#parser-uses-boxdyn-iterator-for-token-source),
[release-no-lto](#release-profile-has-no-lto).

**Remediation:** Add `#[inline]` to ~10 functions in reader.rs.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Reader::peek_at() is O(n) per call via chars().nth(n)

**significant** &middot; `crates/yamalgam-scanner/src/reader.rs:49-51` &middot; effort: small

`chars().nth(n)` iterates from the current offset through `n`
characters each call. The scanner calls `peek_at()` ~46 times, often
with sequential small indices — `is_document_start()` does `peek()` +
`peek_at(1)` + `peek_at(2)` + `peek_at(3)`, each rescanning from
offset. For ASCII-dominated YAML the constant factors are small, but
for CJK/emoji content the rescanning is genuinely wasteful.

```rust crates/yamalgam-scanner/src/reader.rs:49-51
pub fn peek_at(&self, n: usize) -> Option<char> {
    self.input[self.offset..].chars().nth(n)
}
```

**Remediation:** Cache the current char in `Reader`. For small
lookahead, a fixed-size char buffer eliminates rescanning. Or add
`peek_byte()` for ASCII-range checks.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Release profile has no LTO

**moderate** &middot; `Cargo.toml:57-58` &middot; effort: trivial

Bench profile adds `lto = "thin"` but release does not. Cross-crate
inlining happens in benchmarks but not in `cargo install` or release
builds. Users get measurably worse performance than benchmarks report.

```toml Cargo.toml:57-63
[profile.release]
debug = "line-tables-only"
# no lto setting — defaults to false

[profile.bench]
inherits = "release"
lto = "thin"   # bench gets LTO, release does not
```

Enables: [no-inline-cross-crate](#zero-inline-annotations-on-hot-readerscanner-functions),
[parser-dyn-iterator-dispatch](#parser-uses-boxdyn-iterator-for-token-source).

**Remediation:** Add `lto = "thin"` to `[profile.release]`. One line.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Resolver::on_event allocates Vec per event on passthrough path

**moderate** &middot; `crates/yamalgam-parser/src/resolve.rs:93-96` &middot; effort: small

The `NoopResolver` (used in all default paths) allocates a `Vec` per
event in the stream. For a 1000-event document, that is 1000 heap
allocations producing no value. ADR-0007 originally specified
`SmallVec<[Event; 1]>` for this reason.

```rust crates/yamalgam-parser/src/resolve.rs:93-103
pub trait Resolver<'input> {
    fn on_event(&mut self, event: Event<'input>)
        -> Result<Vec<Event<'input>>, ResolveError>;
}

impl<'input> Resolver<'input> for NoopResolver {
    fn on_event(&mut self, event: Event<'input>)
        -> Result<Vec<Event<'input>>, ResolveError> {
        Ok(vec![event])
    }
}
```

**Remediation:** Adopt `SmallVec<[Event; 1]>` per the ADR, or a
`None/One/Many` enum that inlines the single-event case.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Serde deserializer clones Event on every peek-then-consume

**moderate** &middot; `crates/yamalgam-serde/src/de.rs:375` &middot; effort: small

The pattern `peek_event()?.clone()` followed by `next_event()?`
appears in `deserialize_any` — the most common serde dispatch path.
Cloning `Event::Scalar` with `Cow::Owned` value triggers a `String`
allocation + memcpy. Every YAML value goes through this path.

```rust crates/yamalgam-serde/src/de.rs:375
let event = self.peek_event()?.clone();
match event {
    Event::Scalar { value, style, .. } => {
        let _ = self.next_event()?;  // discards the same event
```

**Remediation:** Extract the discriminant (variant + style) from the
peek, drop the borrow, then call `next_event()` to get the owned
event. Eliminates the clone.

<!-- whitespace is important -->
<div>&hairsp;</div>

### purge_stale_simple_keys allocates Vec on every token fetch

**moderate** &middot; `crates/yamalgam-scanner/src/scanner.rs:591-615` &middot; effort: small

Called from `fetch_next_token()` on every iteration. `Vec::new()`
starts at zero capacity (no alloc until push), so the cost is zero in
the common case. The allocation path is rare but avoidable.

```rust crates/yamalgam-scanner/src/scanner.rs:591-595
fn purge_stale_simple_keys(&mut self) {
    let mut purged_root_ids: Vec<u64> = Vec::new();
    self.simple_keys.retain(|sk| {
        // ...
```

**Remediation:** Use a `bool` flag during retain, then iterate again
for the queue_position lookup. Or `SmallVec<[u64; 2]>`.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Anchored events cloned twice during anchor buffering

**moderate** &middot; `crates/yamalgam-serde/src/de.rs:196-222` &middot; effort: small

When an anchored collection is encountered, all sub-events are
collected into a `Vec`, cloned for the anchor registry, and the
original drained into the replay buffer. Every `Event` is cloned
including scalars with `Cow::Owned` strings — doubling memory for
every anchored subtree.

```rust crates/yamalgam-serde/src/de.rs:200-206
let mut buffer = vec![strip_anchor(event)];
// ... loop buffering sub-events ...
self.register_anchor(anchor_name, buffer.clone());
for ev in buffer {
    self.replay_buffer.push_back(ev);
}
```

**Remediation:** Move `buffer` into `register_anchor`, then replay
from the anchors map. Or use `Rc<[Event]>` to share.

*Verdict: The three significant findings — dyn dispatch, missing
inline, and release-without-LTO — compound multiplicatively. Fix any
one and the others matter less; fix all three and the entire cross-crate
call chain becomes inlineable. The one-line LTO fix is the lowest-effort
highest-impact change. The per-event allocations (Resolver Vec, serde
clone, anchor buffer) are independent costs that add up linearly with
document size.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## The Supply Chain Surface

*The dependency tree ships the very library this project aims to
replace, and carries unused dependencies across multiple crates that
inflate compile times without contributing functionality.*

### figment yaml feature pulls deprecated serde_yaml into every crate

**significant** &middot; `crates/yamalgam-core/Cargo.toml:37` &middot; effort: trivial

figment's `yaml` feature enables deprecated `serde_yaml v0.9.34` as a
transitive dependency. serde_yaml pulls `unsafe-libyaml` (C code
compiled via cc) and `indexmap` into every crate that depends on
`yamalgam-core` — which is all of them.

```toml crates/yamalgam-core/Cargo.toml:37
figment = { version = "0.10", features = ["toml", "yaml", "json"] }
```

> A YAML parser project transitively depending on a deprecated YAML
> parser. The irony writes itself.

**Remediation:** Drop the `yaml` feature:
`figment = { version = "0.10", features = ["toml", "json"] }`. YAML
config support can be restored via yamalgam's own parser when figment
is replaced.

<!-- whitespace is important -->
<div>&hairsp;</div>

### 7 unused dependencies across 4 crates

**moderate** &middot; `crates/yamalgam-{scanner,compare,serde,bench}/Cargo.toml` &middot; effort: trivial

Seven declared dependencies are unused per cargo-machete and source
grep. Most are leftovers from the C baseline removal (PR #79) or
initial scaffolding: `thiserror` in scanner, compare, and serde
(all three implement errors manually); `serde_json` and
`yamalgam-core` in compare; `serde` (derive) and `yamalgam-core` in
bench.

**Remediation:** Remove the unused entries from each `Cargo.toml`.

<!-- whitespace is important -->
<div>&hairsp;</div>

### pretty_assertions unused in 4 crates

**advisory** &middot; `crates/yamalgam-{scanner,compare,compose,serde}/Cargo.toml` &middot; effort: trivial

Dev-only — does not affect release builds. Adds compile time during
`cargo test`.

**Remediation:** Remove from `[dev-dependencies]` unless planned for
future tests.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Stale deny.toml advisory ignore

**advisory** &middot; `deny.toml:61` &middot; effort: trivial

The ignore entry for `RUSTSEC-2023-0051` (dlopen_derive via yamlstar)
is never matched because yamlstar is optional and not compiled by
default. Creates noise in `cargo deny` output.

**Remediation:** Remove the stale entry from `deny.toml`.

*Verdict: The figment finding is the headline — a one-line fix that
removes a deprecated YAML parser, its C dependency, and several
transitive crates from the dependency tree. The unused dependencies
are housekeeping, mostly leftovers from the C baseline removal.
No known CVEs in any production dependency. The supply chain posture
is healthy once the unused weight is trimmed.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## The API Design Surface

*Public types are well-designed with correct lifetime propagation and
zero-copy Cow usage, but miss standard trait implementations that
idiomatic Rust consumers expect from map-like containers.*

### Mapping lacks IntoIterator impls

**moderate** &middot; `crates/yamalgam-core/src/value.rs:54-112` &middot; effort: trivial

`Mapping` provides `iter()` but does not implement `IntoIterator`.
Callers must write `for (k, v) in mapping.iter()` instead of
`for (k, v) in &mapping`. Consuming iteration is impossible without
accessing the private `entries` field.

```rust crates/yamalgam-core/src/value.rs:54-55
pub struct Mapping {
    entries: Vec<(Value, Value)>,
}
```

**Remediation:** Implement `IntoIterator` for `&Mapping`, `&mut Mapping`,
and `Mapping`. Non-breaking additions.

<!-- whitespace is important -->
<div>&hairsp;</div>

### Value::get(&str) heap-allocates on every lookup

**moderate** &middot; `crates/yamalgam-core/src/value.rs:129-133` &middot; effort: trivial

Every call constructs a `Value::String` via `key.to_owned()`,
allocating on the heap, solely to linear-scan and compare. The
allocation is dropped immediately after the scan.

```rust crates/yamalgam-core/src/value.rs:129-133
pub fn get(&self, key: &str) -> Option<&Self> {
    match self {
        Self::Mapping(m) => m.get(&Self::String(key.to_owned())),
        _ => None,
    }
}
```

**Remediation:** Add `Mapping::get_by_str` that compares `String`
variants directly against `&str` without allocating.

<!-- whitespace is important -->
<div>&hairsp;</div>

### ResourceLimits is Clone but not Copy despite qualifying

**advisory** &middot; `crates/yamalgam-core/src/loader.rs` &middot; effort: trivial

`ResourceLimits` has 7 `Option<usize>` fields — all `Copy`. The serde
crate's `from_str_with_limits` takes it by value, requiring callers to
`.clone()` for reuse. Inconsistent with `from_str_with_config` which
takes `&LoaderConfig` by reference.

**Remediation:** Derive `Copy` on `ResourceLimits`.

*Verdict: The API findings are all non-breaking additions. The
underlying design — streaming Iterator-based parser, `Cow<'input, str>`
zero-copy, layered crates — is sound. Lifetimes are well-applied
throughout the codebase. These are polish items for a pre-1.0 library.*

<!-- whitespace is important -->
<div>&nbsp;</div>

---

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---------|---------|----------|--------|--------|
| | | **Untrusted Input** | | |
| [skip-value-underflow](#skip_value-depth-counter-underflows-on-leading-end-events) | significant | `de.rs:748-765` | trivial | related: drain-underflow |
| [drain-underflow](#seqaccessdrain-and-mapaccessdrain-miss-cross-type-end-event-guard) | significant | `de.rs:888-949` | trivial | related: skip-value-underflow |
| [cst-unclamped-index](#cst-insert_whitespace_before-indexes-source-without-clamping) | moderate | `cst/lib.rs:488` | trivial | -- |
| [column-i32-cast](#scanner-casts-u32-column-to-i32-without-overflow-check) | advisory | `scanner.rs:1020` | trivial | -- |
| | | **Error Architecture** | | |
| [scan-error-stringly](#scanerror-is-a-stringly-typed-bag-with-no-variants) | significant | `scanner.rs:34-47` | medium | enables: compose-error, serde-span |
| [diagnostic-no-display](#diagnostic-lacks-display-and-stderrorderror-impls) | significant | `diagnostic.rs:49-62` | trivial | -- |
| [compose-error-string](#composeerror-variants-carry-only-string-no-structured-data) | moderate | `compose/lib.rs:19-30` | small | enabled by: scan-error |
| [serde-no-span](#serde-errorunexpected-passes-none-span-at-15-call-sites) | moderate | `serde/error.rs:17-38` | medium | enabled by: scan-error |
| [cst-discards-error](#cst-handle_error-discards-parseerror-content) | advisory | `cst/lib.rs:545` | trivial | -- |
| | | **Panic Discipline** | | |
| [expect-peeked-40x](#parser-uses-expectpeeked-after-peek_token--40-sites) | moderate | `parser.rs:292` | small | -- |
| [replay-buffer-unwrap](#deserializer-peek-methods-unwrap-replay_bufferfront) | moderate | `de.rs:268-301` | trivial | -- |
| [compose-double-unwrap](#composerpeek_event-double-unwraps-via-action-integer-pattern) | advisory | `compose/lib.rs:384` | small | -- |
| | | **Hot Path** | | |
| [dyn-iterator-dispatch](#parser-uses-boxdyn-iterator-for-token-source) | significant | `parser.rs:72` | medium | related: no-inline, no-lto |
| [no-inline-cross-crate](#zero-inline-annotations-on-hot-readerscanner-functions) | significant | `reader.rs:40-90` | trivial | related: dyn-dispatch, no-lto |
| [peek-at-linear-scan](#readerpeek_at-is-on-per-call-via-charsnthn) | significant | `reader.rs:49-51` | small | -- |
| [release-no-lto](#release-profile-has-no-lto) | moderate | `Cargo.toml:57-58` | trivial | enables: no-inline, dyn-dispatch |
| [resolver-vec-per-event](#resolveron_event-allocates-vec-per-event-on-passthrough-path) | moderate | `resolve.rs:93-96` | small | -- |
| [serde-peek-clone](#serde-deserializer-clones-event-on-every-peek-then-consume) | moderate | `de.rs:375` | small | -- |
| [purge-vec-alloc](#purge_stale_simple_keys-allocates-vec-on-every-token-fetch) | moderate | `scanner.rs:591-615` | small | -- |
| [anchor-buffer-clone](#anchored-events-cloned-twice-during-anchor-buffering) | moderate | `de.rs:196-222` | small | -- |
| | | **Supply Chain** | | |
| [figment-serde-yaml](#figment-yaml-feature-pulls-deprecated-serde_yaml-into-every-crate) | significant | `core/Cargo.toml:37` | trivial | -- |
| [unused-deps-7x](#7-unused-dependencies-across-4-crates) | moderate | `*/Cargo.toml` | trivial | -- |
| [unused-dev-deps](#pretty_assertions-unused-in-4-crates) | advisory | `*/Cargo.toml` | trivial | -- |
| [stale-deny-ignore](#stale-denytoml-advisory-ignore) | advisory | `deny.toml:61` | trivial | -- |
| | | **API Design** | | |
| [mapping-no-intoiter](#mapping-lacks-intoiterator-impls) | moderate | `value.rs:54-112` | trivial | -- |
| [value-get-allocs](#valuegetstr-heap-allocates-on-every-lookup) | moderate | `value.rs:129-133` | trivial | -- |
| [limits-not-copy](#resourcelimits-is-clone-but-not-copy-despite-qualifying) | advisory | `loader.rs` | trivial | -- |

---

<sub>
Generated 2026-04-08 at commit 6116052.
Intermediate artifacts: recon.yaml, findings.yaml.
</sub>
