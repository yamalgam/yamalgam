---
audit_date: 2026-08-03
commit: 27cf03716d12a1170691b4bcb1108f0a3d25c257
scope: M10 surface — the yamalgam-query crate and the yg binary, as introduced across main...m10-yg-cli (21 commits, ~5k lines of new src, ~1.3k lines of tests)
findings:
  critical: 4
  significant: 5
  moderate: 22
  advisory: 4
  note: 1
---

# Audit: the yamalgam-query crate and the yg binary

M10 is a large, disciplined piece of work — a YAML-native jq engine on jaq 3.0, a metadata-carrying value model, two emitters, twelve native filters, a jq-manual conformance corpus with an individually reasoned allowlist, and a CLI — and the engineering underneath it is good. `YamlVal` clone is genuinely O(1) because `Data` holds `Rc<Vec>`, `Rc<YgMap>` and `Rc<str>`, so jaq's constant cloning costs refcount bumps rather than deep copies; the arithmetic and ordering semantics are jq-faithful and mutually consistent; the dependency tree has no CVEs, no yanked crates, a committed lockfile and a pinned toolchain; and every one of the eleven deviations in the implementation-notes ledger was checked against the code and every one is accurate.

The defects cluster in one place, and it is not the engine. It is the round trip back to YAML text. Three confirmed data-loss paths share the shape *the in-memory value is right, the emitted text is wrong*: a scalar containing a comma is emitted unquoted inside a flow collection and re-parses as two values; a node mutated through an alias keeps its `alias_of` and is re-emitted as `*anchor`, discarding the edit; a string emitted under `--schema 1.1` comes back as a boolean. Separately, `--in-place` persists its temporary file without ever checking that anything was written to it, so a filter yielding no output truncates the user's file to zero bytes and exits 0. Each of these was reproduced against the built binary, not inferred.

`--in-place` is the amplifier throughout. It takes emitter bugs that would otherwise be visible glitches on a terminal and writes them silently over the user's files with a success exit code. One resource bound is missing rather than wrong: the alias-expansion limit counts parser events, and the builder shares subtrees by `Rc`, so a YAML bomb is linear in memory and passes every check — then both emitters walk that DAG as a tree, turning 216 bytes of input into 13.2 MB of JSON with `--strict` offering no protection.

Nothing here is architectural. The value model, the jaq wiring, the conformance discipline and the dependency hygiene are all sound and worth keeping as they are. The remediation list is a set of localized fixes concentrated in the emitter and in `run_in_place`, plus one decision about whether the public API should be readable before M11 starts consuming it.

---

## The YAML Round-Trip Surface

*The query engine computes the right value and the emitter then writes text that does not mean that value — three independent paths where a scalar's content is re-read as document structure or as a different type.*

### flow-context-plain-scalar-injection

**critical** · `crates/yamalgam-query/src/value.rs:705-727` · effort: small

`needs_quotes` takes only a `&str`, so it cannot know whether the scalar will land inside a flow collection. `emit/yaml.rs::string` has the `Position` in hand but consults it only to decide whether a literal block is allowed, never for the quoting decision. Inside a flow collection, `,` `[` `]` `{` `}` are structural, so a bare scalar containing any of them changes the document's shape when it is read back. The flow indicators are tested at position 0 only; there is no interior check and no context parameter at all.

```rust
// crates/yamalgam-query/src/value.rs:705-727
pub(crate) fn needs_quotes(value: &str) -> bool {
    use yamalgam_core::{TagResolution, TagResolver as _, Value};

    if value.is_empty()
        || !matches!(
            TagResolution::Yaml12.resolve_scalar(value),
            Value::String(_)
        )
    {
        return true;
    }

    let first = value.chars().next().unwrap_or(' ');
    let syntactic_first = "!&*-?:,[]{}#|>@`\"'%".contains(first);
    value.starts_with(' ')
        || value.ends_with(' ')
        || syntactic_first
        || value.contains(": ")
        || value.ends_with(':')
        || value.contains(" #")
        || value.contains('\n')
        || value.contains('\t')
}
```

Reproduced against the built binary at 27cf037:

```
$ printf 'tags: [alpha, beta]\n' > t.yml
$ yg -i '.tags[0] = "x,y"' t.yml ; echo $?
0
$ cat t.yml
tags: [x,y, beta]
$ yg --json '.tags | length' t.yml
3
```

A two-element list became a three-element list on disk, with exit 0 and no diagnostic. The value jaq computed was correct — `yg --json '.tags[0] = "x,y"'` renders `["x,y","beta"]` — so the defect is entirely in the text. Flow position is reached two ways: input-preserved flow style, as above, and `-c/--compact-output`, which forces every collection to flow, so an ordinary block-style document becomes injectable the moment a filter constructs a scalar.

> I don't need to control the filter. I need one field in a config file that someone is going to run `yg -i` over in CI. If their document uses flow sequences anywhere — and generated manifests always do — then any value I control that carries a comma stops being a value and starts being a list boundary. The tool reports success. The diff looks plausible. Nobody re-reads a file the formatter just touched.

**Enables:** `in-place-empty-output-truncates-file` · **Related:** `plain-scalar-control-characters`, `emitter-quoting-ignores-active-schema`

**Remediation:** Change the signature to `needs_quotes(value: &str, flow: bool)` — or thread `Position` through — and when `flow` is true, additionally require quoting for any value containing `,` `[` `]` `{` `}`. Have `emit/yaml.rs::string` pass `matches!(position, Position::Flow)`. Separately, tighten the `SingleQuoted` arm, whose guard is `!string.contains('\'')`: when the value does contain an apostrophe it currently falls through to a bare plain scalar rather than single-quoting with `''` escaping. The corpus round-trip test at `tests/emit.rs:190` only exercises `EmitOpts::default()`, so neither the compact nor the flow-nesting path has any round-trip coverage.

### alias-metadata-survives-mutation-dropping-edits

**critical** · `crates/yamalgam-query/src/valt.rs:199-205` · effort: medium

`Meta.alias_of` is documented as "referenced anchor name when this node came from an alias", but nothing binds it to the node's contents. All three in-place mutation paths in `jaq_core::ValT` — `map_index` (valt.rs:159), `map_values` (valt.rs:129) and `map_range` (valt.rs:265) — rewrite `self.data` and leave `self.meta` untouched. The YAML emitter then treats `alias_of` as authoritative: `Context::alias_is_emittable` (emit/yaml.rs:321) returns true once the anchor has been written, and `Context::alias_text` (emit/yaml.rs:328) emits `*name` in place of the node's actual contents.

```rust
// crates/yamalgam-query/src/valt.rs:199-205
match &mut self.data {
    Data::Map(mapping) => {
        use indexmap::map::Entry::{Occupied, Vacant};

        match Rc::make_mut(mapping).0.entry(index.clone()) {
            Occupied(mut entry) => {
                let previous = core::mem::take(entry.get_mut());
```

Reproduced against the built binary at 27cf037:

```
$ printf 'a: &x {p: 1}\nb: *x\n' > al.yml
$ yg --json '.b.p = 2' al.yml
{
  "a": {
    "p": 1
  },
  "b": {
    "p": 2
  }
}
$ yg -i '.b.p = 2' al.yml ; echo $?
0
$ cat al.yml
a: &x {p: 1}
b: *x
```

The engine computed the mutation correctly — `--json` proves it — and the YAML emitter threw it away. Under `-i` that is what gets persisted, so the user's edit vanishes with a success exit and no output. The author already knows the invariant: `YamlVal::set_anchor` (value.rs:305-310) clears `alias_of` when an anchor is set, and `explode_aliases` (value.rs:397-402) clears both. Those are two ad-hoc enforcement sites for a rule the type does not encode, and the mutation paths honour neither.

> The dangerous version of this isn't the one that fails loudly. It's the automation that runs `yg -i '.image.tag = $NEW'` across a manifest tree every deploy. On any file where that key came in through an alias, the write is a no-op and the exit code says it worked. The deploy ships the old tag. The only way to notice is to read the file, which is the thing the automation exists to avoid.

**Enables:** `in-place-empty-output-truncates-file` · **Related:** `stale-raw-metadata-after-string-splice`, `no-public-read-access-to-yamlval`

**Remediation:** Make identity metadata impossible to desynchronize from content rather than adding a third clearing site. Preferred: split identity out of `Meta` — keep presentation-only fields (`tag`, `style`, `raw`) there and move `anchor`/`alias_of` into the `Data` layer, e.g. a `Data::Alias { of: Rc<str>, value: Rc<YamlVal> }` variant — so any mutation that rebuilds `data` structurally cannot leave a stale alias behind. If the flat shape must stay, funnel every write through a single `YamlVal::data_mut()` that clears `meta.anchor` and `meta.alias_of` before handing out `&mut Data`.

### emitter-quoting-ignores-active-schema

**significant** · `crates/yamalgam-query/src/value.rs:705-713` · effort: small

The spec's `r[output.yaml.quoting]` is a MUST: "yg MUST NOT produce output that is ambiguous under the active schema." The emitter has no way to honour it. `EmitOpts` (emit/mod.rs:22-32) carries indent, compact, sort_keys and color — but no schema — and `needs_quotes` hard-codes `TagResolution::Yaml12`. Under `--schema 1.1` the loader resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans on the way in, while the emitter decides quoting with 1.2 rules on the way out, where those words are ordinary strings.

```rust
// crates/yamalgam-query/src/value.rs:705-713
pub(crate) fn needs_quotes(value: &str) -> bool {
    use yamalgam_core::{TagResolution, TagResolver as _, Value};

    if value.is_empty()
        || !matches!(
            TagResolution::Yaml12.resolve_scalar(value),
            Value::String(_)
        )
    {
```

Reproduced against the built binary at 27cf037:

```
$ printf 'a: "YES"\n' > s.yml
$ yg --schema 1.1 '.a |= ascii_downcase' s.yml > s2.yml
$ cat s2.yml
a: yes
$ yg --schema 1.1 '.a | type' s2.yml
boolean
```

A string became a boolean across one round trip. yg does print a warning whenever 1.1 mode is active, which is a partial mitigation, but it fires on every 1.1 invocation and says nothing about this specific transformation, so it cannot distinguish a safe run from a lossy one. This is not in the deviations ledger; note 8 defers only the `# yg-schema:` pragma, not schema-aware emission.

**Enables:** `in-place-empty-output-truncates-file` · **Related:** `flow-context-plain-scalar-injection`

**Remediation:** Add the active `TagResolution` to `EmitOpts`, thread it from `commands/query.rs::loader_config` into `to_yaml`, and have `needs_quotes` resolve with that schema instead of the `Yaml12` constant. Failsafe and json schemas are safe in the other direction — they only over-quote — so 1.1 is the case that needs the fix.

### json-nonstring-keys-collide

**significant** · `crates/yamalgam-query/src/emit/json.rs:76-81` · effort: small

YAML mappings admit any resolved value as a key, and `YamlVal`'s `PartialEq` and `Hash` keep `Data::Int(1)` and `Data::Str("1")` distinct (different hash tags at value.rs:644 and value.rs:646), so the `IndexMap` stores both. The JSON emitter flattens both to the same object key with no detection.

```rust
// crates/yamalgam-query/src/emit/json.rs:76-81
fn key_string(key: &YamlVal) -> String {
    match &key.data {
        Data::Str(string) => string.to_string(),
        _ => key.to_string(),
    }
}
```

Reproduced against the built binary at 27cf037:

```
$ printf '{1: alpha, "1": beta}\n' > k.yml
$ yg --json . k.yml
{
  "1": "alpha",
  "1": "beta"
}
```

RFC 8259 permits duplicate names, but every mainstream parser keeps the last, so `alpha` is gone the moment anything consumes this. The same collapse hits `true`/`"true"`, `null`/`"null"` and `1.0`/`"1.0"`. It is not confined to `--json`: `to_json` also implements the `tojson` native filter (funs.rs:78) and `ValT::into_string` (valt.rs:322), which backs jq string interpolation, so `"\(.)"` over such a mapping is equally lossy.

**Remediation:** Detect the collision instead of allowing it. Build the key strings first and, on a duplicate, either return an error (`to_json` would become fallible, matching how jq rejects non-string keys under `-j`) or disambiguate non-string keys with a reserved encoding. At minimum, warn on stderr when `--json` is asked to render a mapping with non-string keys.

### plain-scalar-control-characters

**moderate** · `crates/yamalgam-query/src/value.rs:717-726` · effort: small

A string with no source style metadata is emitted plain whenever `needs_quotes` returns false, and that predicate rejects only `\n` and `\t` in the control range. `yamalgam-scanner/src/reader.rs:70` documents that a bare `\r` is normalized to a newline, so a carriage return inside a plain scalar becomes a line break on re-parse.

```rust
// crates/yamalgam-query/src/value.rs:717-726
let first = value.chars().next().unwrap_or(' ');
let syntactic_first = "!&*-?:,[]{}#|>@`\"'%".contains(first);
value.starts_with(' ')
    || value.ends_with(' ')
    || syntactic_first
    || value.contains(": ")
    || value.ends_with(':')
    || value.contains(" #")
    || value.contains('\n')
    || value.contains('\t')
```

Confirmed reachable: `printf 'a\rb\n' | yg -R .` emits the CR byte literally, verified with `od -c`. `-R` builds values through `YamlVal::str` with `meta: None`, so no style preservation can intervene, and the same gap applies to any filter-produced string. Two further consequences: `\x1b` reaches an interactive terminal unescaped, so ANSI/OSC sequences in untrusted input become terminal control; and `needs_quotes` does not flag U+0085 (NEL) or U+2028/U+2029, which are line breaks under YAML 1.1 — the schema yg advertises for interop with exactly those consumers.

**Related:** `flow-context-plain-scalar-injection`

**Remediation:** Replace the two trailing `contains` checks with a single scan — `value.chars().any(|c| c.is_control() || c == '\u{85}' || c == '\u{2028}' || c == '\u{2029}')` — keeping the `\n` special case ahead of it so literal blocks are still chosen for newline-only strings. In `quote_double`, widen the escape arm to `c if c.is_control() || ('\u{7f}'..='\u{9f}').contains(&c)` using `\uXXXX`.

### stale-raw-metadata-after-string-splice

**moderate** · `crates/yamalgam-query/src/valt.rs:300-307` · effort: small

`Meta.raw` is documented as "raw scalar text before schema resolution" and the builder populates it from the source event (build.rs:202). `map_range` on a `Data::Str` splices new content in and returns `self` with `meta` untouched — the test at valt.rs:560-573 asserts precisely that metadata survives, so this is intentional for `style` and incidental for `raw`.

```rust
// crates/yamalgam-query/src/valt.rs:300-307
let mut updated = String::with_capacity(
    string.len() - (end_byte - start_byte) + replacement.len(),
);
updated.push_str(&string[..start_byte]);
updated.push_str(&replacement);
updated.push_str(&string[end_byte..]);
*string = Rc::from(updated);
Ok(self)
```

Two public behaviours then read stale text. `YamlVal::raw_text` (value.rs:212) prefers `meta.raw` over the string content, so after `.a[0:1] = "J"` on `a: 'hello'`, the `raw` filter returns `"hello"` for a node whose value is `"Jello"`. `YamlVal::resolve_with` (value.rs:239-249) also prefers `meta.raw`, so `resolve` and `resolve_as` re-resolve the original scalar rather than the edited one. Full replacement is unaffected because it installs a fresh `YamlVal` with `meta: None`, which keeps this narrow — but it is the same root cause as the alias finding.

**Related:** `alias-metadata-survives-mutation-dropping-edits`

**Remediation:** Clear `meta.raw` and `meta.tag` whenever a scalar's content is rewritten. If the `data_mut()` funnel proposed for the alias finding is adopted, this falls out for free.

*The emitter was built as an M10 stopgap ahead of the real serializer in M13, and it shows — but `-i` writes its output to disk, so "stopgap" and "lossy" are not compatible in the same release.*

---

## The In-Place Editing Surface

*`run_in_place` is correctly atomic against crashes and correctly rolls back on error, but it never checks that the filter produced anything before renaming the temporary file over the user's document.*

### in-place-empty-output-truncates-file

**critical** · `crates/yamalgam/src/commands/query.rs:172-203` · effort: small

`run_records` writes only from inside its visit closure, which the engine calls once per filter output. A filter that produces no output writes nothing, and that is not an error — `run_records` returns `Ok(None)`. Nothing between the run and the persist inspects `file_last` or the temp file's length, so the empty file is flushed, synced, chmod'd and renamed over the user's document. `file_last.is_some()` is checked, but only after the persist has already happened.

```rust
// crates/yamalgam/src/commands/query.rs:172-203
let file_last = run_records(
    filter,
    Rc::new(RunCtx::new(config.clone())),
    records,
    args,
    json_output,
    false,
    temporary.as_file_mut(),
)?;
temporary
    .as_file_mut()
    .flush()
    .and_then(|()| temporary.as_file().sync_all())
    .map_err(|error| {
        YgError::Runtime(format!(
            "failed to finish temporary output for {path}: {error}"
        ))
    })?;
temporary
    .as_file()
    .set_permissions(source_metadata.permissions())
    .map_err(|error| {
        YgError::Runtime(format!(
            "failed to preserve permissions for {path}: {error}"
        ))
    })?;
temporary.persist(path).map_err(|error| {
    YgError::Runtime(format!(
        "failed to replace {path} atomically: {}",
        error.error
    ))
})?;
```

Reproduced against the built binary at 27cf037:

```
$ printf 'enabled: false\nname: production\nreplicas: 3\n' > config.yml
$ wc -c < config.yml
44
$ yg -i 'select(.enabled)' config.yml ; echo $?
0
$ wc -c < config.yml
0
```

`select()` is the ordinary jq idiom for conditional output; it yields nothing whenever the predicate is false. So this is document-driven, not merely operator error: a pipeline running `yg -i '.[] | select(.enabled)' services.yaml` destroys the file whenever the input happens to contain no enabled entry. The filter is correct; the input decides whether the file survives. The content is unrecoverable — the rename is the last step and there is no backup.

> The good version of this attack doesn't need me to break anything. It needs me to make a predicate false. If their pipeline filters on a field I populate, I don't corrupt the file — I empty it, and the tool tells them it worked. The next stage reads an empty document and applies whatever its defaults are.

**Enabled by:** `flow-context-plain-scalar-injection`, `alias-metadata-survives-mutation-dropping-edits`, `emitter-quoting-ignores-active-schema` · **Related:** `in-place-multi-file-partial-application`

**Remediation:** Track whether the visit closure ever fired for the current file — `run_records` already threads a `first` flag for exactly this shape — and refuse to persist when it did not: drop the temporary, leave the original untouched, exit non-zero with a message.

```rust
let (file_last, wrote_any) = run_records(/* ... */)?;
if !wrote_any && !args.allow_empty {
    return Err(YgError::Runtime(format!(
        "{path}: filter produced no output; refusing to truncate (use --allow-empty to override)"
    )));
}
```

Also add `conflicts_with = "null_input"` to the `in_place` arg, since `yg -i -n '{}' config.yaml` currently overwrites every named file with the null-input result.

### in-place-replaces-symlinks-and-drops-ownership

**moderate** · `crates/yamalgam/src/commands/query.rs:190-203` · effort: small

`std::fs::metadata(path)` at query.rs:155 follows symlinks, so `source_metadata` describes the link's target — but `temporary.persist(path)` is a `rename(2)` onto the link path itself, which replaces the symlink with a new regular file and leaves the target untouched.

```rust
// crates/yamalgam/src/commands/query.rs:190-203
temporary
    .as_file()
    .set_permissions(source_metadata.permissions())
    .map_err(|error| {
        YgError::Runtime(format!(
            "failed to preserve permissions for {path}: {error}"
        ))
    })?;
temporary.persist(path).map_err(|error| {
    YgError::Runtime(format!(
        "failed to replace {path} atomically: {}",
        error.error
    ))
})?;
```

Reproduced against the built binary at 27cf037:

```
$ printf 'a: 1\n' > shared/real.yml && ln -sf shared/real.yml link.yml
$ yg -i '.a = 2' link.yml ; echo $?
0
$ test -L link.yml || echo "symlink was replaced by a regular file"
symlink was replaced by a regular file
$ cat shared/real.yml
a: 1
```

The edit landed on a new file at the link's path; the file the user believed they were editing is unchanged. The direction here is defensive against the classic symlink attack — yg does not write *through* an attacker-planted link into a privileged path — so the exposure is silent breakage of intentional links, not privilege escalation. Ownership is the second half: `set_permissions` carries mode bits only, so a root process running `yg -i` over a user's checkout leaves root-owned files behind.

**Related:** `in-place-empty-output-truncates-file`

**Remediation:** Call `std::fs::symlink_metadata(path)` first. If it reports a symlink, either resolve with `canonicalize()` and edit the target (documenting that choice) or refuse with a clear message. On Unix, follow `set_permissions` with a `chown` to the source's uid/gid when the process has the privilege. Detect `nlink > 1` and warn that the edit will break the hard link.

### in-place-multi-file-partial-application

**moderate** · `crates/yamalgam/src/commands/query.rs:152-157` · effort: medium

`run_in_place` rewrites files one at a time, persisting each before moving on. Every fallible step inside the loop propagates with `?`, so a failure on file N aborts with files 1..N-1 already replaced and N+1.. untouched.

```rust
// crates/yamalgam/src/commands/query.rs:152-157
let mut last = None;

for (file_index, path) in files.iter().enumerate() {
    let source_metadata = std::fs::metadata(path)
        .map_err(|error| YgError::InputParse(format!("failed to inspect {path}: {error}")))?;
    let mut records = collect_inputs(std::slice::from_ref(path), args, config)?;
```

Reproduced against the built binary at 27cf037:

```
$ printf 'n: 1\n' > f1.yml ; printf 'n: [bad\n' > f2.yml ; printf 'n: 3\n' > f3.yml
$ yg -i '.n = 99' f1.yml f2.yml f3.yml ; echo $?
yg: f2.yml: unexpected StreamEnd, expected FlowEntry (,) or FlowSequenceEnd (]) in flow sequence
5
$ cat f1.yml ; cat f3.yml
n: 99
n: 3
```

The message names only the file that failed. Nothing tells the user that a prefix of their argument list was already modified, so the natural response — fix `f2` and re-run the same command — re-applies the filter to `f1`. For a non-idempotent filter such as `.count |= . + 1`, that double-applies silently.

**Related:** `in-place-empty-output-truncates-file`

**Remediation:** Either stage every file into a temporary first and persist only after all files succeed, or keep the streaming behaviour and make the partial state explicit: on failure, report which paths were modified and which were skipped.

### slurp-silently-per-file-under-in-place

**moderate** · `crates/yamalgam/src/commands/query.rs:154-157` · effort: trivial

The spec's `r[cli.slurp]` says `-s` reads all documents from all files into a single sequence and runs the filter once, and adds "The meaning of this flag MUST NOT diverge from jq." In the normal path that holds. In the in-place path each file is passed to `collect_inputs` on its own, so `yg -i -s '<filter>' a.yml b.yml` runs the filter twice, once per file, each time over a one-file sequence.

```rust
// crates/yamalgam/src/commands/query.rs:154-157
for (file_index, path) in files.iter().enumerate() {
    let source_metadata = std::fs::metadata(path)
        .map_err(|error| YgError::InputParse(format!("failed to inspect {path}: {error}")))?;
    let mut records = collect_inputs(std::slice::from_ref(path), args, config)?;
```

Each file is then rewritten with the result of a filter that silently saw different input than requested, with no warning. The author knows about this — it is item 2 of the Landmines section in `.handoffs/2026-08-02-2342-m10-yg-cli-complete.md` — but it is absent from the implementation-notes ledger, from README.md and from `--help`. A handoff note is not a user-visible document, so this is drift beyond what the ledger admits.

**Related:** `in-place-empty-output-truncates-file`

**Remediation:** Either reject `-i` combined with `-s` with a usage error (exit 2, matching the existing `-i` without files check), or document the per-file semantics in the ledger and in the `-s` help text. Whichever is chosen, record it somewhere a user will read.

### in-place-with-from-file-rejected-by-clap

**moderate** · `crates/yamalgam/src/lib.rs:111-114` · effort: trivial

With `-f`, the first positional binds to `filter` and `files` collects only the remainder, so `yg -i -f prog.jq data.yaml` leaves `files` empty. clap's `requires` tests presence of the referenced argument, and a multi-value positional with zero occurrences is absent, so clap rejects the invocation before `cmd_query` runs.

```rust
// crates/yamalgam/src/lib.rs:111-114
/// Edit files atomically; reserialization loses comments until CST editing lands.
// r[impl cli.inplace]
#[arg(short = 'i', long, requires = "files")]
pub in_place: bool,
```

Reproduced against the built binary at 27cf037:

```
$ yg -i -f prog.jq d.yml
error: the following required arguments were not provided:
  <FILE>...

Usage: yg --in-place --from-file <FILE> <FILTER> <FILE>...

For more information, try '--help'.
```

`filter_and_files` (query.rs:213-229) explicitly handles this shape — it moves `args.filter` into the file list when `from_file` is set — which shows the combination is intended to work. The runtime guard at query.rs:41-44 already produces the correct diagnostic and exit 2 for the genuinely file-less case. So the declarative constraint blocks a supported invocation while duplicating a check implemented more accurately a few lines away, and `tests/yg.rs:268-273` passes either way because both paths exit 2.

**Related:** `in-place-empty-output-truncates-file`

**Remediation:** Drop `requires = "files"` and let the runtime guard own the validation, since it sees the post-normalization file list. Add a test for `yg -i -f prog.jq file.yaml` to pin the supported combination.

*The hard part was done right — temp file beside the target, flush, fsync, permission copy, atomic rename, rollback on error. The gap is the success path, which has no floor.*

---

## The Resource Bounds Surface

*The loader's limits are enforced where the document is built and nowhere else, so a bounded in-memory graph becomes an unbounded walk at emit time and an unbounded allocation at mutation time.*

### yaml-bomb-materialized-by-emitters

**critical** · `crates/yamalgam-query/src/build.rs:154-171` · effort: medium

`Builder::node` charges one unit against `max_alias_expansions` (10,000 under `moderate`, the CLI default) per `Event::Alias`, then resolves the alias by `Rc`-cloning the anchored node. The resulting graph is a DAG with shared subtrees, so a classic billion-laughs document costs O(source length), not O(expanded size), and passes every limit. Both emitters then walk that DAG as if it were a tree.

```rust
// crates/yamalgam-query/src/build.rs:154-171
Event::Alias { name, span } => {
    self.alias_expansions += 1;
    self.config
        .limits
        .check_alias_expansions(self.alias_expansions)
        .map_err(BuildError::Limit)?;

    let mut value = self.anchors.get(name.as_ref()).cloned().ok_or_else(|| {
        BuildError::UnknownAlias {
            name: name.to_string(),
            span,
        }
    })?;
    let meta = value.meta_mut();
    meta.anchor = None;
    meta.alias_of = Some(Rc::from(name.as_ref()));
    Ok(value)
}
```

Reproduced against the built binary at 27cf037, with a 216-byte input and six levels of 9-way aliasing:

```
$ wc -c < bomb.yml
216
$ yg --json . bomb.yml | wc -c
13227921
$ yg --strict --json . bomb.yml > /dev/null ; echo $?
0
```

Roughly 61,000x amplification, and `--strict` does not stop it — the strict profile tightens depth and alias-expansion caps, neither of which this trips. Each additional level multiplies output by 9: measured on the same machine, seven levels (272 bytes) produces 143 MB and eight levels (307 bytes) produces 1.41 GB. `emit/json.rs::write_json` has no alias branch at all, so every alias is materialized unconditionally; `emit/yaml.rs::alias_is_emittable` emits `*name` only when the anchor was already written during this same emission. Default YAML output is not safe either — `yg '.g' bomb.yml` selects a subtree whose descendants alias anchors defined outside it, yielding 34 MB.

> Their loader is careful. It counts my alias tokens and caps them, and it's right that the parse is cheap — the tree really is small in memory. The mistake is assuming that means the *output* is small. I don't have to defeat the limit. I just have to wait until something walks the graph and forgets it's a graph.

**Enabled by:** `fromjson-ignores-configured-limits` · **Related:** `filter-constructed-values-recurse-without-depth-bound`

**Remediation:** Bound emission, not just parsing. Give `to_yaml` and `to_json` an output budget — node count or byte count — threaded from `LoaderConfig::limits`, and fail with a `Limit` error rather than allocating past it. Make `to_json` alias-aware the way `to_yaml` is: track visited `Rc` identities via `Rc::as_ptr` during the walk and error when the same shared node is reached more times than a threshold. Identity tracking also fixes the `--sort-keys` and subtree-selection cases, which the current `anchors_emitted` name list cannot see.

### filter-constructed-values-recurse-without-depth-bound

**significant** · `crates/yamalgam-query/src/emit/json.rs:24-35` · effort: medium

Input documents are depth-limited during construction — `Builder::node` calls `limits.check_depth` (build.rs:93-96), and `LoaderConfig` caps depth at 64/256/1024. No such bound exists for values the filter itself builds. jaq evaluates `reduce` and `foreach` iteratively, so a 100k-deep sequence can be constructed without overflowing during evaluation.

```rust
// crates/yamalgam-query/src/emit/json.rs:24-35
Data::Seq(items) => {
    output.push('[');
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_indent(output, options, depth + 1);
        write_json(output, item, options, depth + 1);
    }
    write_closing_indent(output, options, depth);
    output.push(']');
}
```

Every consumer of that value then recurses once per level with no limit: `write_json` here, `Context::node` in emit/yaml.rs:47-100, `Display::fmt` in value.rs:670-701, `Hash` in value.rs:636-662, `Ord` in value.rs:619-634, and `deep_merge` in ops.rs:29-46. Measured:

```
$ yg -n 'reduce range(100000) as $i (null; [.]) | 1'
thread 'main' (...) has overflowed its stack
fatal runtime error: stack overflow, aborting
$ echo $?
134
```

Exit 134 is SIGABRT — the runtime detects the overflow and aborts, so there is a message, but it is a runtime abort and not a catchable panic: no `YgError`, no exit code from the documented set, no context. The `| 1` matters: it discards the deep value so the filter never emits it, and the abort still happens, which isolates the recursive drop glue as an independent path from the emitters.

**Related:** `map-index-allocates-directly-from-untrusted-index`, `yaml-bomb-materialized-by-emitters`

**Remediation:** Thread a depth counter through `write_json`, `Context::node`, `Display`, `Hash` and `Ord` and return a typed error past a ceiling derived from `LoaderConfig.limits`. Deep drop needs a manual `Drop` for the collection variants that pops children onto an explicit worklist. If the full fix is too large for M10, bound the emitters at minimum — those are the paths reached on every successful run.

### map-index-allocates-directly-from-untrusted-index

**significant** · `crates/yamalgam-query/src/valt.rs:178-187` · effort: small

The only validation on `position` is that it fits in `usize`, which admits every non-negative `i64`.

```rust
// crates/yamalgam-query/src/valt.rs:178-187
self.data = match index.as_int() {
    Some(position) if position >= 0 => {
        let Ok(position) = usize::try_from(position) else {
            let error = Error::str(format_args!("index {index} out of bounds"));
            return opt.fail(self, |_| Exn::from(error));
        };
        let mut items = vec![Self::null(); position];
        items.push(replacement);
        Data::Seq(Rc::new(items))
    }
```

`yg -n '.[1000000000000] = 1'` requests a Vec of 10^12 `YamlVal` — at 32 bytes each, roughly 32 TB. Measured against the built binary at 27cf037: both that and `yg -n '"a" * 1000000000000'` hang indefinitely (killed at 60s) with no panic and no abort. The process wedges allocating and zeroing the request, and under overcommit that drives the OOM killer. This is not a capacity-overflow panic — `str::repeat` raises that only when `len * n` overflows `usize`, which 10^12 on a one-byte string does not. An unbounded hang is arguably worse than a panic: it consumes the machine rather than failing fast. The pattern repeats at valt.rs:240 (`resize`) and ops.rs:126 (`repeat`), both reachable from filter text alone with no input document.

**Related:** `filter-constructed-values-recurse-without-depth-bound`

**Remediation:** Apply a ceiling before allocating. `LoaderConfig.limits` is already carried in `RunCtx`; add a max-collection-size and max-string-length limit and check it in `map_index`, the resize path, and `Mul`-on-string. Failing that, use `try_reserve` and convert allocation failure into an error rather than an abort.

### fromjson-ignores-configured-limits

**moderate** · `crates/yamalgam-query/src/funs.rs:312-328` · effort: trivial

`RunCtx` already carries the operator's `LoaderConfig`, and every other native filter reaches it via `context_value.0.data().context.config` (see `resolve` at funs.rs:154). `fromjson` does not.

```rust
// crates/yamalgam-query/src/funs.rs:312-328
fn fromjson(&self) -> ValR<Self> {
    let Data::Str(source) = &self.data else {
        return Err(Error::typ(self.clone(), "string"));
    };
    let config = yamalgam_core::LoaderConfig::moderate()
        .with_tag_resolution(yamalgam_core::TagResolution::Json);
    let mut documents = crate::build::parse_str_with_config(source, &config)
        .map_err(|error| Error::str(format_args!("fromjson: {error}")))?;
    if documents.len() == 1 {
        Ok(documents.remove(0))
    } else {
        Err(Error::str(format_args!(
            "fromjson: expected 1 document, got {}",
            documents.len()
        )))
    }
}
```

`yg --strict '.payload | fromjson'` parses the embedded string with a 256 MB input cap (yamalgam-core/src/loader.rs:59-64), depth 256 and 10,000 alias expansions, instead of the strict caps the operator asked for. Choosing `--strict` is precisely the case where a doubly-untrusted embedded string most needs the tighter budget. Compounding it, the parse goes through the full YAML parser rather than a JSON subset, so an attacker-supplied string field containing YAML anchors and aliases is accepted and expanded — a second entry point for the bomb above.

**Enables:** `yaml-bomb-materialized-by-emitters`

**Remediation:** Read the config from the run context the way `resolve` does — `context_value.0.data().context.config.clone().with_tag_resolution(TagResolution::Json)` — and pass that through. Separately, decide whether `fromjson` should accept YAML syntax at all.

*The limit vocabulary already exists and is correctly wired into the builder; it simply stops at the builder. Fixing this needs no new mechanism, only the existing one threaded to the places that currently allocate without asking.*

---

## The Process Contract Surface

*Exit codes and stream discipline are the CLI's API for shell pipelines, and several of them report the wrong thing: a successful pipe reports failure, an explicit failure reports success, and a plain `-n` invocation blocks.*

### null-input-still-blocks-on-stdin

**significant** · `crates/yamalgam/src/commands/query.rs:242-249` · effort: small

`cmd_query` calls `collect_inputs` unconditionally at query.rs:53, before `run_filter_with_null_input` decides to drive the filter with a single null. `collect_inputs` has no `null_input` branch: with no file arguments it drains stdin to EOF.

```rust
// crates/yamalgam/src/commands/query.rs:242-249
if files.is_empty() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| {
            YgError::InputParse(format!("failed to read standard input: {error}"))
        })?;
    append_input(&mut records, input, None, 0, args, config);
```

Reproduced against the built binary at 27cf037:

```
$ ( sleep 6 ) | timeout 3 yg -n '1+1'
$ echo $?
124
```

Exit 124 is the timeout — yg produced nothing and blocked. With stdin already closed it returns `2` immediately, which is why the existing test misses it: `assert_cmd` hands the child a closed stdin, so `tests/yg.rs:45` asserts the right output for the wrong reason. `yg -n '1 + 1'` typed at a terminal is the single most common way anyone first exercises a jq-like tool, and it hangs until Ctrl-D. jq's `-n` never touches stdin unless the program calls `input`/`inputs`, and the spec says `r[cli.null-input]` is "Identical to jq". This is not in the deviations ledger.

**Related:** `query-cli-materializes-all-inputs-before-filtering`

**Remediation:** In `collect_inputs`, return an empty record list when `args.null_input && files.is_empty()`, and defer the stdin read until `input`/`inputs` actually pulls. Add a regression test that runs `yg -n '1+1'` with an open stdin and a timeout.

### stdout-epipe-becomes-runtime-error

**moderate** · `crates/yamalgam/src/commands/query.rs:110-122` · effort: small

Rust sets `SIGPIPE` to `SIG_IGN` at startup, so writing to a closed pipe returns `EPIPE` rather than killing the process. All four write sites convert any `io::Error` into `YgError::Runtime`, which `exit_for` maps to 5 — the code the spec's `r[cli.exit.runtime]` reserves for filter type errors and resource-limit violations.

```rust
// crates/yamalgam/src/commands/query.rs:110-122
if json_output {
    writeln!(output, "{}", to_json(&value, &options))
        .map_err(|error| YgError::Runtime(error.to_string()))?;
} else if args.raw_output && value.is_string() {
    writeln!(output, "{}", value.as_str_content().unwrap_or_default())
        .map_err(|error| YgError::Runtime(error.to_string()))?;
} else {
    if !first && !args.no_doc {
        writeln!(output, "---").map_err(|error| YgError::Runtime(error.to_string()))?;
    }
    write!(output, "{}", to_yaml(&value, &options))
        .map_err(|error| YgError::Runtime(error.to_string()))?;
}
```

Reproduced against the built binary at 27cf037: `yg -n 'range(200000)' | head -2` prints `yg: Broken pipe (os error 32)` and yg exits 5. `yg . big.yaml | head -5` is exactly the usage this tool exists for; under `set -o pipefail`, standard in CI, that fails the whole pipeline for a successful query. It also makes exit 5 ambiguous between "the filter failed" and "the consumer stopped reading" — the distinction a pipeline needs most.

**Related:** `exit-code-mapping-diverges-from-spec`

**Remediation:** Treat `ErrorKind::BrokenPipe` as a normal end of output: match on it in a small `write_output` helper wrapping all four sites and return a sentinel that stops iteration and exits 0 without printing. Give genuine write failures their own exit code. Flush the stdout lock explicitly at the end of `cmd_query`.

### exit-code-mapping-diverges-from-spec

**moderate** · `crates/yamalgam/src/commands/query.rs:389-396` · effort: small

```rust
// crates/yamalgam/src/commands/query.rs:389-396
fn exit_for(error: &YgError) -> ExitCode {
    match error {
        YgError::FilterSyntax(_) => ExitCode::from(2),
        YgError::FilterCompile(_) => ExitCode::from(3),
        YgError::InputParse(_) | YgError::Runtime(_) => ExitCode::from(5),
        YgError::Halt(code) => ExitCode::from((*code).clamp(0, 255) as u8),
    }
}
```

Three mismatches against the documented table (spec 8.5). First and worst, halt codes are clamped into `0..=255`, so `halt(-1)` — the idiomatic "fail maximally" spelling, which jq turns into exit 255 via the POSIX `& 0xff` convention — produces success. Confirmed: `yg -n 'halt(-1)'` exits 0. A CI gate written as `yg 'if .replicas < 1 then halt(-1) else . end' d.yaml` passes on exactly the documents it was written to reject. Second, a nonexistent input file exits 5 rather than the usage code; jq exits 2. Third, failures raised before the query pipeline return through anyhow and exit 1 — the code the spec reserves for "`-e` and the last output was false or null".

> If their gate calls `halt(-1)` on the condition I need to slip past, I don't need to touch the gate at all. It already reports success. The author wrote the check, tested that it fired, and never checked what the shell actually saw.

**Related:** `halt-prints-a-spurious-error-line`, `stdout-epipe-becomes-runtime-error`

**Remediation:** Use POSIX truncation instead of a clamp — `ExitCode::from(*code as u8)` wraps -1 to 255 and 256 to 0, matching jq and every shell. Add a test asserting `halt(-1)` exits 255. Split input-availability errors from parse errors and map the former to 2. Give pre-pipeline failures their own code so 1 keeps its documented `-e` meaning.

### halt-prints-a-spurious-error-line

**moderate** · `crates/yamalgam/src/commands/query.rs:380-383` · effort: trivial

```rust
// crates/yamalgam/src/commands/query.rs:380-383
fn report(error: YgError) -> ExitCode {
    eprintln!("yg: {error}");
    exit_for(&error)
}
```

`YgError::Halt` is routed through the same `report()` as genuine errors, and its `Display` is the bare string `halt` (error.rs:43-45). Confirmed: `yg -n 'halt'` writes `yg: halt` to stderr and exits 0. Any wrapper that treats stderr output as a failure signal — a common CI and Makefile pattern — sees a successful run as broken, and the message carries no information: it does not say what code was requested or which document halted. jq's `halt` is silent; only `halt_error` writes to stderr, and it writes the value, not the word.

**Related:** `exit-code-mapping-diverges-from-spec`

**Remediation:** Branch on the variant before printing: return `exit_for(&error)` directly for `Halt` and keep the `eprintln` for the other four variants.

### runtime-errors-drop-document-and-file-identity

**moderate** · `crates/yamalgam-query/src/engine.rs:180-187` · effort: small

```rust
// crates/yamalgam-query/src/engine.rs:180-187
for input in &mut driver {
    let input = input.map_err(|error| E::from(YgError::InputParse(error)))?;
    for output in filter.id.run((jaq_context.clone(), input)) {
        let output: YamlVal = output.map_err(|error| E::from(YgError::runtime(error)))?;
        last = Some(output.as_bool());
        visit(output)?;
    }
}
```

`run_filter_with_null_input` receives `&RunCtx`, whose `input_meta` `RefCell` holds the current filename, file index and document index (engine.rs:48-65), refreshed for every record by the input iterator. The runtime-error arm ignores it, so `yg '.a.b' *.yaml` over fifty files fails with a bare "cannot index string with b" and no indication of which file triggered it. The contrast is visible in the same codebase: parse errors are prefixed with the filename in `append_input` (query.rs:352-355), and `BuildError::UnknownAlias` carries a line and column. The gap is worst under `-i`.

**Related:** `in-place-multi-file-partial-application`

**Remediation:** In the runtime-error arm, borrow `context.input_meta` and prefix the message with the filename and document index in the same form `append_input` already uses. Consider making `YgError::Runtime` a struct with optional filename and document fields so the formatting lives in one place.

*Individually these are small; together they mean a CI script cannot reliably distinguish "your query is wrong" from "the consumer stopped reading" from "the gate deliberately failed".*

---

## The Filter Parameterization Surface

*The compiler accepts global variables and the CLI never supplies any, so the only way to get a runtime value into a filter is to concatenate it into the program text.*

### no-argument-binding-forces-filter-concatenation

**moderate** · `crates/yamalgam-query/src/engine.rs:129-137` · effort: medium

```rust
// crates/yamalgam-query/src/engine.rs:129-137
let variables = variables
    .iter()
    .map(|variable| format!("${variable}"))
    .collect::<Vec<_>>();
compile::Compiler::default()
    .with_funs(crate::funs::all_funs())
    .with_global_vars(variables.iter().map(String::as_str))
    .compile(modules)
    .map_err(|errors| YgError::FilterCompile(render_compile_errors(errors)))
```

`compile_filter` accepts `variables: &[String]` and correctly forwards it to jaq's `with_global_vars`. The sole call site at query.rs:35 passes `&[]`. There is no `--arg`, `--argjson`, `--slurpfile` or `--args` in `QueryArgs` — confirmed against `yg --help` — and jaq's `Vars::new([])` supplies no values either. A filter has no way to receive a value except as literal text inside the program.

> They gave me a language with `env` in it and no way to pass me a parameter. So every script that needs one builds the program with string concatenation. I don't attack the tool — I attack the shell line. `x") | halt_error(0) | ("` closes their literal and the rest of the program is mine.

**Enables:** `env-filter-registered-unconditionally` · **Related:** `exit-code-mapping-diverges-from-spec`

**Remediation:** Add `--arg NAME VALUE` and `--argjson NAME JSON` to `QueryArgs`, collect them into the `variables` slice `compile_filter` already accepts, and bind their values through `Vars` in `run_filter_with_null_input` (currently `Vars::new([])` at engine.rs:171). Until then, document in `--help` that interpolating shell values into the FILTER argument is unsafe.

### env-filter-registered-unconditionally

**advisory** · `crates/yamalgam-query/src/funs.rs:11-22` · effort: medium

```rust
// crates/yamalgam-query/src/funs.rs:11-22
pub fn all_funs() -> impl Iterator<Item = Fun<YgKind>> {
    let run = run::<YgKind>;
    jaq_core::funs::<YgKind>()
        .chain(jaq_std::funs::<YgKind>())
        .chain(
            jaq_std::input::funs::<YgKind>()
                .into_vec()
                .into_iter()
                .map(run),
        )
        .chain(yg_natives().into_vec().into_iter().map(run))
}
```

`jaq-std` is depended on with default features, so `jaq_std::funs()` resolves to `base_funs().chain(extra_funs())` and `extra_funs` includes the `std()` group — `env` and `now`. Confirmed: `SECRET=hunter2 yg -n 'env.SECRET'` prints `hunter2`. This matches jq's behaviour and the operator normally types the filter, so it is not an exposure in ordinary CLI use. It matters because `compile_filter` and `all_funs` are public API on a library crate with no way to opt out, and because it is the payload that makes the concatenation sink above worth exploiting. The rest of this boundary is clean and worth recording: jaq's module loader is left at its rejecting default, so `include`/`import` cannot touch the filesystem; `input`/`inputs` are bounded to the CLI-supplied `RcIter`; and no filter can spawn a process or open a socket.

**Enabled by:** `no-argument-binding-forces-filter-concatenation`

**Remediation:** Add `all_funs_sandboxed()`, or a `sandbox: bool` on `compile_filter`. Note that the obvious implementation is unavailable: in jaq-std 3.0.1, `std()`, `format()`, `math()`, `regex()`, `time()` and `log()` are all private (src/lib.rs:460, 482, 518, 612, 625, 663); only `funs()`, `base_funs()` and `extra_funs()` are `pub`. The implementable options are `base_funs()` alone (too blunt — drops format/math/regex wholesale), filtering the `all_funs()` iterator by `Fun` name to remove `env` and `now` (recommended, surgical, no upstream change), or patching jaq-std.

*Nothing here is wrong; something is absent — and the absence forces users into the one construction that is.*

---

## The Value Model Surface

*The value type is well built — O(1) clone, total and mutually consistent Ord/Eq/Hash, jq-faithful arithmetic — but it is publicly constructible and publicly parseable without being publicly readable.*

### no-public-read-access-to-yamlval

**advisory** · `crates/yamalgam-query/src/value.rs:71-75` · effort: small

```rust
// crates/yamalgam-query/src/value.rs:71-75
#[derive(Clone, Debug, Default)]
pub struct YamlVal {
    pub(crate) data: Data,
    pub(crate) meta: Option<Rc<Meta>>,
}
```

`parse_str` and `parse_str_with_config` are public and return `Vec<YamlVal>`, and there are seven public constructors plus `From` impls. The complete set of public read accessors is `meta()`, `type_name()`, `kind_name()`, `tag_shorthand()`, `style_name()`, `get_str()`, `is_string()`, `as_str_content()` and `keys()`. There is no way for an external caller to obtain a `bool`, an `i64` or an `f64`, to iterate a sequence, or to iterate a mapping's values. `value::Data` and `value::YgMap` are both declared `pub` but neither is re-exported from lib.rs, `YgMap`'s single field is `pub(crate)`, and `YamlVal.data` is `pub(crate)` — public names with zero reachable surface. The in-crate integration tests show the friction: `tests/value.rs:144` verifies numeric contents only by comparing against a re-constructed `YamlVal`.

**Related:** `alias-metadata-survives-mutation-dropping-edits`, `rc-makes-value-model-non-send`

**Remediation:** Decide the public read contract now, while there are no downstream users. Minimal version: re-export `Data` from lib.rs, add `pub fn data(&self) -> &Data`, and give `YgMap` a public iteration and lookup surface. If committing to the `Data` layout is undesirable, the alternative is a typed accessor set — `as_bool`, `as_i64`, `as_f64`, `as_seq() -> Option<&[YamlVal]>`, `iter_entries()` — modelled on `serde_json::Value`. Land it before M11 begins consuming the crate.

### input-meta-refcell-races-with-input-lookahead

**moderate** · `crates/yamalgam/src/commands/query.rs:85-89` · effort: medium

```rust
// crates/yamalgam/src/commands/query.rs:85-89
let iterator_context = Rc::clone(&context);
let inputs = records.into_iter().map(move |record| {
    *iterator_context.input_meta.borrow_mut() = record.metadata;
    record.value
});
```

`RunCtx` exposes `pub input_meta: RefCell<InputMeta>` and the CLI writes it from inside the input iterator's `map` closure. The engine wraps that same iterator in an `RcIter` handed to jaq as `Inputs`, and the native filters `filename`, `file_index` and `document_index` read the `RefCell` back during evaluation. Nothing sequences these: jaq's `input` and `inputs` pull from the same `RcIter` while the current document's filter is still running, and each pull re-runs the closure, overwriting `input_meta` with the *next* record's metadata. This is not a borrow panic today — every `borrow()` in funs.rs is a statement-scoped temporary — but the shape is one shared `RefCell` written by a lazily-driven iterator and read re-entrantly from inside that iterator's consumer, which is exactly the arrangement that becomes a `BorrowMutError` the first time a native filter holds a `Ref` across a pull.

**Related:** `runtime-errors-drop-document-and-file-identity`

**Remediation:** Carry the metadata with the value instead of in ambient mutable state: make the engine's input item `(InputMeta, YamlVal)`. If the `RefCell` stays, make the field private behind accessors so the write point is auditable, and document that `filename` is unspecified in combination with `input`/`inputs`.

### integer-only-indexing-diverges-from-equal-float-values

**moderate** · `crates/yamalgam-query/src/valt.rs:331-336` · effort: small

```rust
// crates/yamalgam-query/src/valt.rs:331-336
const fn as_int(&self) -> Option<i64> {
    match self.data {
        Data::Int(number) => Some(number),
        _ => None,
    }
}
```

`PartialEq` treats `Int(3)` and `Float(3.0)` as equal (value.rs:582-584) and `Hash` gives them the same hash (value.rs:644-645) — they are indistinguishable as mapping keys. But the integer-only rule is encoded in four independent places with no shared helper: `as_int` itself, reached from `map_index` (valt.rs:178, 224) and `range_bounds` (valt.rs:356); `ValT::index` (valt.rs:90), which does not call `as_int` at all but pattern-matches `(Data::Seq(items), Data::Int(position))` inline; and `jaq_std::ValT::as_isize` (valt.rs:405). Confirmed: `.[1]` on `[1,2,3]` returns 2 while `.[1.0]` fails with "cannot index [1, 2, 3] with 1.0". This is reachable from ordinary arithmetic — `Div` returns `Float` whenever the division is inexact (ops.rs:155) — so a computed index is a `Float` whenever the arithmetic that produced it was.

**Remediation:** Pick one contract and encode it once. If integral floats should index — closest to jq — give `as_int` a single implementation that also accepts `Data::Float(f)` where `f.fract() == 0.0 && f.is_finite()`, and have all four sites call it, including the inline match at valt.rs:90. Note this alone does not reach jq parity for slice bounds: jq truncates, so `.[0:(length/2)]` works there on odd-length input where the bound is 1.5, and an `f.fract() == 0.0` test would still reject it. Decide separately whether to truncate non-integral finite floats in `range_bounds`.

### run-filter-signature-overconstrains-callers

**moderate** · `crates/yamalgam-query/src/engine.rs:157-163` · effort: small

```rust
// crates/yamalgam-query/src/engine.rs:157-163
pub fn run_filter_with_null_input<E: From<YgError>>(
    filter: &Filter<YgKind>,
    context: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, String>> + 'static,
    null_input: bool,
    mut visit: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E> {
```

The `'static` bound is an artifact of the boxing on engine.rs:164, whose elided trait-object lifetime defaults to `'static`. jaq's own contract is lifetime-generic — `jaq_std::input::Inputs<'i, V>` is `&'i RcIter<dyn Iterator<Item = Result<V, String>> + 'i>` — so making the function generic over a lifetime would compile unchanged and let callers stream from a borrowed buffer. The `String` error channel is jaq's, but re-exporting it verbatim pushes stringification onto every caller: the engine immediately re-wraps it as `YgError::InputParse(String)`, so a caller's typed `BuildError` is flattened to text and can never be matched on again. The CLI already pays for this at query.rs:346-357, and every test writes `map(Ok::<_, String>)` because inference cannot pick the error type.

**Related:** `input-meta-refcell-races-with-input-lookahead`, `no-public-read-access-to-yamlval`

**Remediation:** Introduce a lifetime parameter and box as `Box<dyn Iterator<Item = _> + 'a>`. Replace the `String` item error with `BuildError` and convert at the boundary. Collapse the two entry points into one — `run_filter` (engine.rs:144) exists only to call `run_filter_with_null_input` with `false`.

### rc-makes-value-model-non-send

**advisory** · `crates/yamalgam-query/src/value.rs:71-75` · effort: small

```rust
// crates/yamalgam-query/src/value.rs:71-75
#[derive(Clone, Debug, Default)]
pub struct YamlVal {
    pub(crate) data: Data,
    pub(crate) meta: Option<Rc<Meta>>,
}
```

`Data` holds `Rc<Vec<YamlVal>>`, `Rc<YgMap>` and `Rc<str>`, and `meta` is `Option<Rc<Meta>>`. That is the right call for the jaq hot path — `Rc`'s non-atomic refcount is measurably cheaper than `Arc`'s, and jaq is single-threaded by construction — so this is a deliberate and correct trade, not an oversight. It is recorded because it is a public API commitment invisible in the signatures: no downstream consumer can move a `YamlVal` across a thread boundary, which forecloses a parallel multi-file mode for yg and any future LSP that wants to parse off the main thread.

**Related:** `no-public-read-access-to-yamlval`

**Remediation:** No action required if single-threaded is the intended long-term contract — in which case document it on `YamlVal` so consumers learn it from the docs rather than from a compile error. If parallelism is wanted later, make the pointer type a crate-level alias now (`type Shared<T> = Rc<T>`) so a future switch is one line, and decide before M12's LSP work rather than during it.

*Nothing here is broken today; the crate has exactly one consumer and it lives inside the workspace. The argument for reshaping now is that the cost is near zero and rises monotonically from here.*

---

## The Emission Performance Surface

*The value model is genuinely fast and the emitter is where the allocations are — a String per node, a String per line of indentation, a copy per scalar inspected, and a linear scan per anchor lookup.*

### yaml-inline-emitter-string-per-node

**moderate** · `crates/yamalgam-query/src/emit/yaml.rs:191`, `:390-392` · effort: small

The block path writes into a shared `&mut String`, which is the right shape. The flow path does not: `fn inline(&mut self, value: &YamlVal) -> String` (emit/yaml.rs:191) builds each nested value into its own String and pushes that into its parent's, so a value at depth *d* is copied *d* times on its way to the output. For a document rendered with `-c`, total copying is O(output_size × depth) rather than O(output_size). Indentation is a separate allocation of the same kind:

```rust
// crates/yamalgam-query/src/emit/yaml.rs:390-392
fn indent(&self, output: &mut String, depth: usize) {
    output.push_str(&" ".repeat(self.options.indent.saturating_mul(depth)));
}
```

Every emitted line allocates and drops a heap buffer to write whitespace. A 100k-line document performs 100k allocations for indentation alone.

**Related:** `needs-quotes-copies-every-scalar-it-inspects`

**Remediation:** Thread the same `&mut String` through `inline()` that block emission already uses. For indentation, keep one preallocated string of spaces on the `Context` and push a slice of it, extending only when a deeper level is first reached.

### needs-quotes-copies-every-scalar-it-inspects

**moderate** · `crates/yamalgam-query/src/value.rs:705-713` · effort: small

```rust
// crates/yamalgam-query/src/value.rs:705-713
pub(crate) fn needs_quotes(value: &str) -> bool {
    use yamalgam_core::{TagResolution, TagResolver as _, Value};

    if value.is_empty()
        || !matches!(
            TagResolution::Yaml12.resolve_scalar(value),
            Value::String(_)
        )
    {
```

`resolve_scalar` returns an owned `yamalgam_core::Value`. For a scalar that resolves as a string — the overwhelmingly common case — that means allocating a String containing the entire scalar, matching on it, and dropping it, purely to learn that it was a string. One allocation and one full copy per scalar emitted, on the default output path, compounding with the per-node String allocation above since both run over the same data.

**Related:** `yaml-inline-emitter-string-per-node`

**Remediation:** Add a borrowing or predicate form to the `TagResolver` trait — `resolves_as_string(&str) -> bool`, or a `resolve_scalar_kind` returning a lightweight enum — and call that from `needs_quotes`. The resolver already computes the discriminant before building the `Value`; this only avoids materializing the payload.

### emitter-anchor-table-is-a-linear-scanned-vec

**moderate** · `crates/yamalgam-query/src/emit/yaml.rs:321-326` · effort: trivial

```rust
// crates/yamalgam-query/src/emit/yaml.rs:321-326
fn alias_is_emittable(&self, value: &YamlVal) -> bool {
    value
        .meta()
        .and_then(|meta| meta.alias_of.as_deref())
        .is_some_and(|alias| self.anchors_emitted.iter().any(|anchor| anchor == alias))
}
```

`anchors_emitted` is a `Vec<String>` scanned linearly on every alias check, and the check runs per node during emission — O(N × A) string comparisons for N nodes and A anchors. Most real documents have a handful of anchors, so this is invisible in practice, but generated documents with thousands (Helm value trees, CI matrices) turn emission quadratic. The same shape appears in `IdentityInventory`, making the `aliases` and `orphan_anchors` filters quadratic in anchor count — and those two filters exist specifically to audit documents with many anchors.

**Related:** `alias-audit-skips-mapping-keys`

**Remediation:** Replace both with a `HashSet<Rc<str>>`, or an `IndexSet` where insertion order matters for output. The anchor names are already `Rc<str>` in `Meta`, so the set can share them without new allocation.

### query-cli-materializes-all-inputs-before-filtering

**moderate** · `crates/yamalgam/src/commands/query.rs:242-249` · effort: medium

```rust
// crates/yamalgam/src/commands/query.rs:242-249
if files.is_empty() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| {
            YgError::InputParse(format!("failed to read standard input: {error}"))
        })?;
    append_input(&mut records, input, None, 0, args, config);
```

`collect_inputs` reads each source to a String and parses every document into a Vec of records before `run_filter` is called. Peak memory is the sum of all input files plus their full parsed representations, even for a filter that inspects one field of the first document. jq streams: it parses one value, runs the filter, emits, and moves on. The practical consequences: `yg '.name' **/*.yaml` over a large repo holds every file in memory simultaneously, and yg cannot be used on a stream that never ends — `kubectl get -w -o yaml | yg .` produces nothing, because `collect_inputs` is still waiting for EOF.

**Related:** `null-input-still-blocks-on-stdin`

**Remediation:** Make the record source a lazy iterator that parses one document at a time. The engine already consumes an `impl Iterator<Item = Result<YamlVal, String>>`, so the interface it needs is in place; the change is confined to `collect_inputs` and its callers. `-s` and `-i` legitimately need the whole set, so those paths keep the eager form. If streaming is not wanted, record the decision in the spec so the memory profile is a stated contract.

*None of these affect correctness, and for typical CLI documents none are user-visible — but this is a workspace with a dedicated benchmark crate and an explicit performance identity, and the emitter is what those benchmarks would show.*

---

## The Spec Conformance Surface

*The eleven documented deviations are all accurate and all genuinely owned; the gaps are the drift the ledger does not admit.*

### alias-audit-skips-mapping-keys

**moderate** · `crates/yamalgam-query/src/value.rs:517-523` · effort: small

```rust
// crates/yamalgam-query/src/value.rs:517-523
Data::Map(mapping) => {
    for (key, value) in &mapping.0 {
        path.push(key.clone());
        self.visit(value, path);
        path.pop();
    }
}
```

`IdentityInventory::visit` descends into sequence items and mapping values but never into the key node itself, even though the builder gives keys full metadata — build.rs:140 builds keys through the same `node()` path, so `&anchored_key: v` and `*alias_key: v` both carry `anchor` and `alias_of`. The consequences are wrong answers rather than errors: an anchor defined on a key is missing from the inventory, and — worse — an alias *used* as a key is not counted as a reference, so `orphan_anchors` reports a genuinely referenced anchor as an orphan. Both filters are advertised in README.md as auditing tools, which is exactly the use case a false positive breaks. The behaviour is inconsistent with `explode_aliases`, which does recurse into keys (value.rs:388-394).

**Related:** `emitter-anchor-table-is-a-linear-scanned-vec`

**Remediation:** Call `self.visit(key, path)` for each key before visiting the value, using a path representation that distinguishes key position from value position. Add a test with `&k key: v` and a second mapping whose key is `*k`.

### regex-is-not-pcre

**moderate** · `docs/spec/yg-query-language-spec-draft-01.md:265-266` · effort: trivial

```markdown
r[filters.regex]
`test`, `match`, `capture`, `scan`, `split`, `sub`, `gsub` — identical to jq. PCRE-compatible patterns.
```

The regex surface is delegated wholesale to jaq-std 3.0.1, which depends on `regex-bites` (Cargo.lock:625-637). `regex-bites` is a finite-automata engine in the Rust `regex` family: no lookahead, no lookbehind, no backreferences, no possessive or atomic groups. jq 1.8 uses Oniguruma, which supports all of them. Every jq-manual example in the corpus passes, so the common cases — named captures, flags, `g` — are fine, but a user porting a real jq script containing `(?=...)`, `(?<=...)` or `\1` gets a runtime error. That is a hard failure rather than silent degradation, which is the right direction; the problem is documentary. The unmatched-optional-capture difference is already allowlisted (`tests/fixtures/manual-allowlist.yml:39-41`), which shows the engine difference is known at the case level but not at the capability level.

**Remediation:** Add a numbered entry to `docs/spec/yg-spec-draft-01-implementation-notes.md` stating that regex support is the regex-bites/RE2-style subset, and have draft 02 replace "PCRE-compatible" with the accurate constraint.

*The conformance discipline behind the ledger and the allowlist is better than most shipped tools manage; these two items are simply missing from it.*

---

## The Supply Chain Surface

*The dependency tree is clean by every automated measure, and the one real gap is that the pre-release gate does not run the policy it thinks it runs.*

### release-check-runs-cargo-deny-without-config

**moderate** · `.justfile:176-180` · effort: trivial

```make
# .justfile:176-180
# Security/license check
echo ""
echo "🔐 Running cargo-deny..."
cargo deny check
echo "✅ Dependencies OK"
```

The project's cargo-deny policy lives at `.config/deny.toml`, not the repo-root default, so every invocation must pass `--config`. The dedicated `deny` recipe at `.justfile:21-22` does — and this branch actually fixed its argument order — but the pre-release recipe above was not updated. Verified against the working tree at 27cf037:

```
$ cargo deny --all-features --config .config/deny.toml check
advisories ok, bans ok, licenses ok, sources ok

$ cargo deny check
error[rejected]: failed to satisfy license requirements
  ... (dozens, including "MIT OR Apache-2.0" on anyhow, anstream, anstyle)
advisories ok, bans ok, licenses FAILED, sources ok
```

Without `--config`, cargo-deny uses built-in defaults with no license allowlist, so it rejects MIT/Apache-2.0 — the pre-release gate fails on a tree whose policy is fully satisfied. The practical outcome is that whoever runs it learns to ignore the failure, which is worse than not running the check: an ignored red gate does not catch the real advisory when one lands.

**Remediation:** Change `.justfile:179` to invoke the existing recipe (`just deny`) rather than duplicating the command, so there is one definition of what the check means. Any command with a required flag should live in exactly one recipe that other recipes call.

### jaq-trait-coupling-outruns-version-constraint

**advisory** · `crates/yamalgam-query/Cargo.toml:12-14` · effort: trivial

```toml
# crates/yamalgam-query/Cargo.toml:12-14
indexmap = "2"
jaq-core = "3.0"
jaq-std = "3.0"
```

`valt.rs` implements `jaq_core::ValT` (valt.rs:22) and, separately, `jaq_std::ValT` (valt.rs:395). Implementing a dependency's traits is a much tighter coupling than calling its functions: a new required method, a changed associated type, or a default-method signature change in any 3.x release is a compile break, and semver permits all of those on a trait that is not sealed. Nothing is broken today — but the caret has already been exercised. `Cargo.lock:614-615` pins jaq-core **3.1.0**, not 3.0.1; only jaq-std is still 3.0.1. A minor bump on the crate whose traits this code implements has already been taken silently under this range.

**Remediation:** Decide deliberately and write it down. Either tighten to `jaq-core = "~3.0"` / `jaq-std = "~3.0"` so a minor bump is an explicit action, or keep the caret and add a comment noting that `valt.rs` implements jaq traits and that jaq upgrades need a compile check. A CI job that periodically builds against the latest permitted jaq would turn a future break into a scheduled notification rather than a surprise.

### unused-dev-dep-pretty-assertions

**note** · `crates/yamalgam-query/Cargo.toml:20-22` · effort: trivial

```toml
# crates/yamalgam-query/Cargo.toml:20-22
[dev-dependencies]
pretty_assertions = "1"
serde = { version = "1", features = ["derive"] }
```

`cargo udeps` flags `pretty_assertions` as unused, and the only occurrence anywhere under `crates/yamalgam-query/` is this declaration. The cost is a small unnecessary compile in the test profile, nothing more.

**Remediation:** Either remove it, or start using it — its diff output is genuinely better than std `assert_eq!` for the multi-line YAML comparisons these tests do, so adopting it is at least as reasonable as dropping it.

*No CVEs, no yanked crates, licenses inside policy, lockfile committed, toolchain pinned, one new build script and zero new proc-macro crates across the whole M10 subtree.*

---

## Remediation Ledger

| Finding | Concern | Location | Effort | Chains |
|---|---|---|---|---|
| **The YAML Round-Trip Surface** | | | | |
| [flow-context-plain-scalar-injection](#flow-context-plain-scalar-injection) | critical | `value.rs:705-727` | small | enables in-place truncation |
| [alias-metadata-survives-mutation-dropping-edits](#alias-metadata-survives-mutation-dropping-edits) | critical | `valt.rs:199-205` | medium | enables in-place truncation |
| [emitter-quoting-ignores-active-schema](#emitter-quoting-ignores-active-schema) | significant | `value.rs:705-713` | small | enables in-place truncation |
| [json-nonstring-keys-collide](#json-nonstring-keys-collide) | significant | `emit/json.rs:76-81` | small | — |
| [plain-scalar-control-characters](#plain-scalar-control-characters) | moderate | `value.rs:717-726` | small | — |
| [stale-raw-metadata-after-string-splice](#stale-raw-metadata-after-string-splice) | moderate | `valt.rs:300-307` | small | shares root with alias finding |
| **The In-Place Editing Surface** | | | | |
| [in-place-empty-output-truncates-file](#in-place-empty-output-truncates-file) | critical | `commands/query.rs:172-203` | small | enabled by all three emitter bugs |
| [in-place-replaces-symlinks-and-drops-ownership](#in-place-replaces-symlinks-and-drops-ownership) | moderate | `commands/query.rs:190-203` | small | — |
| [in-place-multi-file-partial-application](#in-place-multi-file-partial-application) | moderate | `commands/query.rs:152-157` | medium | — |
| [slurp-silently-per-file-under-in-place](#slurp-silently-per-file-under-in-place) | moderate | `commands/query.rs:154-157` | trivial | — |
| [in-place-with-from-file-rejected-by-clap](#in-place-with-from-file-rejected-by-clap) | moderate | `lib.rs:111-114` | trivial | — |
| **The Resource Bounds Surface** | | | | |
| [yaml-bomb-materialized-by-emitters](#yaml-bomb-materialized-by-emitters) | critical | `build.rs:154-171` | medium | enabled by fromjson |
| [filter-constructed-values-recurse-without-depth-bound](#filter-constructed-values-recurse-without-depth-bound) | significant | `emit/json.rs:24-35` | medium | — |
| [map-index-allocates-directly-from-untrusted-index](#map-index-allocates-directly-from-untrusted-index) | significant | `valt.rs:178-187` | small | — |
| [fromjson-ignores-configured-limits](#fromjson-ignores-configured-limits) | moderate | `funs.rs:312-328` | trivial | enables YAML bomb |
| **The Process Contract Surface** | | | | |
| [null-input-still-blocks-on-stdin](#null-input-still-blocks-on-stdin) | significant | `commands/query.rs:242-249` | small | — |
| [stdout-epipe-becomes-runtime-error](#stdout-epipe-becomes-runtime-error) | moderate | `commands/query.rs:110-122` | small | — |
| [exit-code-mapping-diverges-from-spec](#exit-code-mapping-diverges-from-spec) | moderate | `commands/query.rs:389-396` | small | — |
| [halt-prints-a-spurious-error-line](#halt-prints-a-spurious-error-line) | moderate | `commands/query.rs:380-383` | trivial | — |
| [runtime-errors-drop-document-and-file-identity](#runtime-errors-drop-document-and-file-identity) | moderate | `engine.rs:180-187` | small | — |
| **The Filter Parameterization Surface** | | | | |
| [no-argument-binding-forces-filter-concatenation](#no-argument-binding-forces-filter-concatenation) | moderate | `engine.rs:129-137` | medium | enables env exposure |
| [env-filter-registered-unconditionally](#env-filter-registered-unconditionally) | advisory | `funs.rs:11-22` | medium | enabled by concatenation |
| **The Value Model Surface** | | | | |
| [no-public-read-access-to-yamlval](#no-public-read-access-to-yamlval) | advisory | `value.rs:71-75` | small | blocks M11/M12 |
| [input-meta-refcell-races-with-input-lookahead](#input-meta-refcell-races-with-input-lookahead) | moderate | `commands/query.rs:85-89` | medium | — |
| [integer-only-indexing-diverges-from-equal-float-values](#integer-only-indexing-diverges-from-equal-float-values) | moderate | `valt.rs:331-336` | small | — |
| [run-filter-signature-overconstrains-callers](#run-filter-signature-overconstrains-callers) | moderate | `engine.rs:157-163` | small | — |
| [rc-makes-value-model-non-send](#rc-makes-value-model-non-send) | advisory | `value.rs:71-75` | small | — |
| **The Emission Performance Surface** | | | | |
| [yaml-inline-emitter-string-per-node](#yaml-inline-emitter-string-per-node) | moderate | `emit/yaml.rs:191, 390-392` | small | — |
| [needs-quotes-copies-every-scalar-it-inspects](#needs-quotes-copies-every-scalar-it-inspects) | moderate | `value.rs:705-713` | small | — |
| [emitter-anchor-table-is-a-linear-scanned-vec](#emitter-anchor-table-is-a-linear-scanned-vec) | moderate | `emit/yaml.rs:321-326` | trivial | — |
| [query-cli-materializes-all-inputs-before-filtering](#query-cli-materializes-all-inputs-before-filtering) | moderate | `commands/query.rs:242-249` | medium | — |
| **The Spec Conformance Surface** | | | | |
| [alias-audit-skips-mapping-keys](#alias-audit-skips-mapping-keys) | moderate | `value.rs:517-523` | small | — |
| [regex-is-not-pcre](#regex-is-not-pcre) | moderate | `spec draft-01:265-266` | trivial | — |
| **The Supply Chain Surface** | | | | |
| [release-check-runs-cargo-deny-without-config](#release-check-runs-cargo-deny-without-config) | moderate | `.justfile:176-180` | trivial | — |
| [jaq-trait-coupling-outruns-version-constraint](#jaq-trait-coupling-outruns-version-constraint) | advisory | `yamalgam-query/Cargo.toml:12-14` | trivial | — |
| [unused-dev-dep-pretty-assertions](#unused-dev-dep-pretty-assertions) | note | `yamalgam-query/Cargo.toml:20-22` | trivial | — |

### Suggested order

The four criticals are not independent work. Three of them are emitter bugs and the fourth is what makes all three destructive, so the cheapest sequencing is:

1. **`in-place-empty-output-truncates-file`** — small, self-contained, and it is the amplifier. Landing this first bounds the damage from everything else while the emitter work proceeds.
2. **`flow-context-plain-scalar-injection`** and **`emitter-quoting-ignores-active-schema`** — both are changes to the same predicate and its call site; do them together.
3. **`alias-metadata-survives-mutation-dropping-edits`** — the `data_mut()` funnel also resolves `stale-raw-metadata-after-string-splice` for free.
4. **`yaml-bomb-materialized-by-emitters`** — independent of the above and the only one that needs new plumbing (an emit budget plus `Rc`-identity tracking).

The `--in-place` composition findings (symlinks, multi-file, `-s`, `-f`) are small and cluster naturally with step 1. The value-model decision is not urgent but should be made before M11 starts, not during it.
