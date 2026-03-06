---
name: idiomatic-rust
description: Idiomatic Rust patterns for libfyaml C port. Use when writing or reviewing Rust code ported from C. Don't write C in Rust - the goal is correct behavior, not line-by-line translation.
---

# Idiomatic Rust: Don't port C patterns

This is a Rust port of libfyaml (C), but **do not write C in Rust**. The goal is correct behavior, not line-by-line translation.

## Put logic where the data lives

If a struct has all the information needed to compute something, make it a method:

```rust
// BAD: C-style — caller extracts fields just to pass them back in
// (mirrors fy_atom_format() taking 8 separate atom fields)
fn atom_format(
    has_lb: bool, has_ws: bool, starts_with_ws: bool,
    style: u8, chomp: u8, storage_hint: usize,
) -> String

// GOOD: The type already knows its properties
impl Atom {
    fn format(&self) -> String {
        // self.has_lb, self.style, self.chomp are all right here
    }
}
```

**Rationale:** Boolean flags are:
- Easy to pass in wrong order (`format(true, false, true, 0, 1, 64)` — which bool is which?)
- Require the caller to extract properties just to pass them back in
- Duplicate knowledge that the type already has

## Use newtypes for domain concepts

```rust
// BAD: Bare primitives everywhere — easy to mix up line, column, byte offset
fn mark(line: i32, col: i32, pos: usize) -> Mark

// GOOD: The type system catches mistakes
struct Line(u32);
struct Column(u32);
struct ByteOffset(usize);

struct Mark {
    line: Line,
    column: Column,
    offset: ByteOffset,
}
```

## Prefer methods over standalone functions

When you find yourself writing `fy_token_get_text(token, ...)`, make it `token.text()` instead. This:
- Groups related functionality
- Enables IDE autocomplete on the type
- Makes the relationship between data and operations explicit

```rust
// BAD: C-style free functions (mirrors fy_token_get_text, fy_token_get_type, etc.)
fn token_get_text(token: &Token) -> &str
fn token_get_type(token: &Token) -> TokenType

// GOOD: Methods on the type
impl Token {
    fn text(&self) -> &str { ... }
    fn token_type(&self) -> TokenType { ... }
}
```

## Enums over boolean flags and tagged unions

libfyaml uses integer enums + union structs. Rust enums with data are strictly better:

```rust
// BAD: C-style tagged union (mirrors struct fy_event + enum fy_event_type)
struct Event {
    event_type: u32,
    // caller must match type before accessing the right field
    scalar_data: Option<ScalarData>,
    mapping_data: Option<MappingData>,
    sequence_data: Option<SequenceData>,
}

// GOOD: Rust enum — impossible to access wrong variant
enum Event {
    StreamStart,
    StreamEnd,
    DocumentStart { implicit: bool },
    DocumentEnd { implicit: bool },
    Scalar { anchor: Option<Anchor>, tag: Option<Tag>, value: ScalarValue },
    SequenceStart { anchor: Option<Anchor>, tag: Option<Tag>, style: CollectionStyle },
    SequenceEnd,
    MappingStart { anchor: Option<Anchor>, tag: Option<Tag>, style: CollectionStyle },
    MappingEnd,
    Alias { anchor: Anchor },
}
```

```rust
// BAD: C-style node type + union (mirrors struct fy_node)
struct Node {
    node_type: u32,  // FYNT_SCALAR=0, FYNT_SEQUENCE=1, FYNT_MAPPING=2
    scalar: Option<Token>,
    sequence: Option<Vec<Node>>,
    mapping: Option<Vec<NodePair>>,
}

// GOOD: Impossible states are unrepresentable
enum NodeData {
    Scalar(ScalarValue),
    Sequence(Vec<Node>),
    Mapping(Vec<NodePair>),
}
```

## Replace C memory patterns with Rust ownership

| libfyaml C pattern | Rust equivalent |
|---------------------|-----------------|
| `struct fy_token *` with `refs` field | `Rc<Token>` or owned `Token` |
| Intrusive linked lists (`list_head`) | `Vec<T>` or `VecDeque<T>` |
| Recycled object pools | `Vec` reuse or typed arena |
| `goto err_out` / `goto cleanup` | `Result<T, E>` with `?` |
| `alloca` / stack temporaries | Local variables, `SmallVec` |
| Direct pointer into mmap'd buffer | `&'input [u8]` with lifetime |
| Callback function pointers | Trait objects or closures |
| Bitflags (`FYPCF_*`) | `bitflags` crate |

## Zero-copy with lifetimes, not raw pointers

libfyaml's `fy_atom` points directly into the input buffer for zero-copy. In Rust, use lifetimes:

```rust
// BAD: Raw pointer mimicking C (mirrors fy_atom's fyi + start_mark)
struct Atom {
    data_ptr: *const u8,
    len: usize,
}

// GOOD: Borrow from input with lifetime, fall back to owned when needed
struct Atom<'input> {
    data: Cow<'input, str>,
    start: Mark,
    end: Mark,
    style: ScalarStyle,
}
```

## The C code is a spec, not a template

When porting C:
1. Understand what the C code **does** (behavior)
2. Understand **why** it does it (intent)
3. Implement that behavior idiomatically in Rust

The `// cref:` comments link to C for reference, but the Rust should stand on its own.
