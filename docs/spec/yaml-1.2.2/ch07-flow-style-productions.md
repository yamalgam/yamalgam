# Chapter 7. Flow Style Productions

> Marked-up copy of YAML 1.2.2 specification Chapter 7, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

YAML's _flow styles_ can be thought of as the natural extension of JSON to
cover [folding] long content lines for readability, [tagging] nodes to control
[construction] of [native data structures] and using [anchors] and [aliases] to
reuse [constructed] object instances.


## 7.1. Alias Nodes

Subsequent occurrences of a previously [serialized] node are [presented] as
_alias nodes_.

y[flow.alias.must-anchor-first]
The first occurrence of the [node] must be marked by an [anchor] to allow
subsequent occurrences to be [presented] as alias nodes.

An alias node is denoted by the "`*`" indicator.
The alias refers to the most recent preceding [node] having the same [anchor].

y[flow.alias.error-undefined-anchor]
It is an error for an alias node to use an [anchor] that does not previously
occur in the [document].

It is not an error to specify an [anchor] that is not used by any alias node.

y[flow.alias.must-not-specify-properties]
Note that an alias node must not specify any [properties] or [content], as
these were already specified at the first occurrence of the [node].

y[flow.c-ns-alias-node]
```
[#] c-ns-alias-node ::=
  c-alias           # '*'
  ns-anchor-name
```


**Example #. Alias Nodes**

```
First occurrence: &anchor Foo
Second occurrence: *anchor
Override anchor: &anchor Bar
Reuse anchor: *anchor
```

```
{ "First occurrence": &A "Foo",
  "Override anchor": &B "Bar",
  "Second occurrence": *A,
  "Reuse anchor": *B }
```

**Legend:**
* [c-ns-alias-node] <!-- 2:20,7 4:15,7 -->
* [ns-anchor-name] <!-- 1:20,6 2:21,6 3:19,6 4:16,6 -->


## 7.2. Empty Nodes

YAML allows the [node content] to be omitted in many cases.
[Nodes] with empty [content] are interpreted as if they were [plain scalars]
with an empty value.
Such [nodes] are commonly resolved to a "`null`" value.

y[flow.e-scalar]
```
[#] e-scalar ::= ""
```


In the examples, empty [scalars] are sometimes displayed as the glyph "`°`" for
clarity.
Note that this glyph corresponds to a position in the characters [stream]
rather than to an actual character.


**Example #. Empty Content**

```
{
  foo : !!str°,
  !!str° : bar,
}
```

```
{ "foo": "",
  "": "bar" }
```

**Legend:**
* [e-scalar] <!-- ° -->


Both the [node's properties] and [node content] are optional.
This allows for a _completely empty node_.
Completely empty nodes are only valid when following some explicit indication
for their existence.

y[flow.e-node]
```
[#] e-node ::=
  e-scalar    # ""
```


**Example #. Completely Empty Flow Nodes**

```
{
  ? foo :°,
  °: bar,
}
```

```
{ "foo": null,
  null : "bar" }
```

**Legend:**
* [e-node] <!-- ° -->


## 7.3. Flow Scalar Styles

YAML provides three _flow scalar styles_: [double-quoted], [single-quoted] and
[plain] (unquoted).
Each provides a different trade-off between readability and expressive power.

y[flow.scalar-style.must-not-convey-content]
The [scalar style] is a [presentation detail] and must not be used to convey
[content] information, with the exception that [plain scalars] are
distinguished for the purpose of [tag resolution].


### 7.3.1. Double-Quoted Style

The _double-quoted style_ is specified by surrounding "`"`" indicators.
This is the only [style] capable of expressing arbitrary strings, by using
"`\`" [escape sequences].
This comes at the cost of having to escape the "`\`" and "`"`" characters.

y[flow.nb-double-char]
```
[#] nb-double-char ::=
    c-ns-esc-char
  | (
        nb-json
      - c-escape          # '\'
      - c-double-quote    # '"'
    )
```

y[flow.ns-double-char]
```
[#] ns-double-char ::=
  nb-double-char - s-white
```


Double-quoted scalars are restricted to a single line when contained inside an
[implicit key].

y[flow.c-double-quoted]
```
[#] c-double-quoted(n,c) ::=
  c-double-quote         # '"'
  nb-double-text(n,c)
  c-double-quote         # '"'
```

y[flow.nb-double-text]
```
[#]
nb-double-text(n,FLOW-OUT)  ::= nb-double-multi-line(n)
nb-double-text(n,FLOW-IN)   ::= nb-double-multi-line(n)
nb-double-text(n,BLOCK-KEY) ::= nb-double-one-line
nb-double-text(n,FLOW-KEY)  ::= nb-double-one-line
```

y[flow.nb-double-one-line]
```
[#] nb-double-one-line ::=
  nb-double-char*
```


**Example #. Double Quoted Implicit Keys**

```
"implicit block key" : [
  "implicit flow key" : value,
 ]
```

```
{ "implicit block key":
  [ { "implicit flow key": "value" } ] }
```

**Legend:**
* [nb-double-one-line] <!-- 1:2,18 2:4,17 -->
* [c-double-quoted(n,c)] <!-- 1:1,20 2:3,19 -->


In a multi-line double-quoted scalar, [line breaks] are subject to [flow line
folding], which discards any trailing [white space] characters.
It is also possible to _escape_ the [line break] character.
In this case, the escaped [line break] is excluded from the [content] and any
trailing [white space] characters that precede the escaped line break are
preserved.
Combined with the ability to [escape] [white space] characters, this allows
double-quoted lines to be broken at arbitrary positions.

y[flow.s-double-escaped]
```
[#] s-double-escaped(n) ::=
  s-white*
  c-escape         # '\'
  b-non-content
  l-empty(n,FLOW-IN)*
  s-flow-line-prefix(n)
```

y[flow.s-double-break]
```
[#] s-double-break(n) ::=
    s-double-escaped(n)
  | s-flow-folded(n)
```


**Example #. Double Quoted Line Breaks**

```
"folded·↓
to a space,→↓
·↓
to a line feed, or·→\↓
·\·→non-content"
```

```
"folded to a space,\nto a line feed, or \t \tnon-content"
```

**Legend:**
* [s-flow-folded(n)] <!-- ·↓ →↓ -->
* [s-double-escaped(n)] <!-- ·→\↓ 5:1 -->


All leading and trailing [white space] characters on each line are excluded
from the [content].

y[flow.double-quoted.continuation-must-contain-non-space]
Each continuation line must therefore contain at least one non-[space]
character.

Empty lines, if any, are consumed as part of the [line folding].

y[flow.nb-ns-double-in-line]
```
[#] nb-ns-double-in-line ::=
  (
    s-white*
    ns-double-char
  )*
```

y[flow.s-double-next-line]
```
[#] s-double-next-line(n) ::=
  s-double-break(n)
  (
    ns-double-char nb-ns-double-in-line
    (
        s-double-next-line(n)
      | s-white*
    )
  )?
```

y[flow.nb-double-multi-line]
```
[#] nb-double-multi-line(n) ::=
  nb-ns-double-in-line
  (
      s-double-next-line(n)
    | s-white*
  )
```


**Example #. Double Quoted Lines**

```
"·1st non-empty↓
↓
·2nd non-empty·
→3rd non-empty·"
```

```
" 1st non-empty\n2nd non-empty 3rd non-empty "
```

**Legend:**
* [nb-ns-double-in-line] <!-- 1:2,14 3:2,13 4:2,13 -->
* [s-double-next-line(n)] <!-- ↓ 3 4:1,14 -->


### 7.3.2. Single-Quoted Style

The _single-quoted style_ is specified by surrounding "`'`" indicators.
Therefore, within a single-quoted scalar, such characters need to be repeated.
This is the only form of _escaping_ performed in single-quoted scalars.
In particular, the "`\`" and "`"`" characters may be freely used.
This restricts single-quoted scalars to [printable] characters.
In addition, it is only possible to break a long single-quoted line where a
[space] character is surrounded by non-[spaces].

y[flow.c-quoted-quote]
```
[#] c-quoted-quote ::= "''"
```

y[flow.nb-single-char]
```
[#] nb-single-char ::=
    c-quoted-quote
  | (
        nb-json
      - c-single-quote    # "'"
    )
```

y[flow.ns-single-char]
```
[#] ns-single-char ::=
  nb-single-char - s-white
```


**Example #. Single Quoted Characters**

```
'here''s to "quotes"'
```

```
"here's to \"quotes\""
```

**Legend:**
* [c-quoted-quote] <!-- '' -->


Single-quoted scalars are restricted to a single line when contained inside a
[implicit key].

y[flow.c-single-quoted]
```
[#] c-single-quoted(n,c) ::=
  c-single-quote    # "'"
  nb-single-text(n,c)
  c-single-quote    # "'"
```

y[flow.nb-single-text]
```
[#]
nb-single-text(FLOW-OUT)  ::= nb-single-multi-line(n)
nb-single-text(FLOW-IN)   ::= nb-single-multi-line(n)
nb-single-text(BLOCK-KEY) ::= nb-single-one-line
nb-single-text(FLOW-KEY)  ::= nb-single-one-line
```

y[flow.nb-single-one-line]
```
[#] nb-single-one-line ::=
  nb-single-char*
```


**Example #. Single Quoted Implicit Keys**

```
'implicit block key' : [
  'implicit flow key' : value,
 ]
```

```
{ "implicit block key":
  [ { "implicit flow key": "value" } ] }
```

**Legend:**
* [nb-single-one-line] <!-- 1:2,18 2:4,17 -->
* [c-single-quoted(n,c)] <!-- 1:1,20 2:3,19 -->


All leading and trailing [white space] characters are excluded from the
[content].

y[flow.single-quoted.continuation-must-contain-non-space]
Each continuation line must therefore contain at least one non-[space]
character.

Empty lines, if any, are consumed as part of the [line folding].

y[flow.nb-ns-single-in-line]
```
[#] nb-ns-single-in-line ::=
  (
    s-white*
    ns-single-char
  )*
```

y[flow.s-single-next-line]
```
[#] s-single-next-line(n) ::=
  s-flow-folded(n)
  (
    ns-single-char
    nb-ns-single-in-line
    (
        s-single-next-line(n)
      | s-white*
    )
  )?
```

y[flow.nb-single-multi-line]
```
[#] nb-single-multi-line(n) ::=
  nb-ns-single-in-line
  (
      s-single-next-line(n)
    | s-white*
  )
```


**Example #. Single Quoted Lines**

```
'·1st non-empty↓
↓
·2nd non-empty·
→3rd non-empty·'
```

```
" 1st non-empty\n2nd non-empty 3rd non-empty "
```

**Legend:**
* [nb-ns-single-in-line(n)] <!-- 1:2,14 3:2,13 4:2,13 -->
* [s-single-next-line(n)] <!-- 1:16 2 3 4:1,14 -->


### 7.3.3. Plain Style

The _plain_ (unquoted) style has no identifying [indicators] and provides no
form of escaping.
It is therefore the most readable, most limited and most [context] sensitive
[style].

y[flow.plain.must-not-be-empty]
In addition to a restricted character set, a plain scalar must not be empty or
contain leading or trailing [white space] characters.

It is only possible to break a long plain line where a [space] character is
surrounded by non-[spaces].

y[flow.plain.must-not-begin-with-indicators]
Plain scalars must not begin with most [indicators], as this would cause
ambiguity with other YAML constructs.

However, the "`:`", "`?`" and "`-`" [indicators] may be used as the first
character if followed by a non-[space] "safe" character, as this causes no
ambiguity.

y[flow.ns-plain-first+4]
```
[#] ns-plain-first(c) ::=
    (
        ns-char
      - c-indicator
    )
  | (
      (
          c-mapping-key       # '?'
        | c-mapping-value     # ':'
        | c-sequence-entry    # '-'
      )
      [ lookahead = ns-plain-safe(c) ]
    )
```


y[flow.plain.must-not-contain-colon-space-space-hash]
Plain scalars must never contain the "`: `" and "` #`" character combinations.
Such combinations would cause ambiguity with [mapping] [key/value pairs] and
[comments].

y[flow.plain.must-not-contain-flow-indicators+3]
In addition, inside [flow collections], or when used as [implicit keys], plain
scalars must not contain the "`[`", "`]`", "`{`", "`}`" and "`,`" characters.
These characters would cause ambiguity with [flow collection] structures.

y[flow.ns-plain-safe+4]
```
[#]
ns-plain-safe(FLOW-OUT)  ::= ns-plain-safe-out
ns-plain-safe(FLOW-IN)   ::= ns-plain-safe-in
ns-plain-safe(BLOCK-KEY) ::= ns-plain-safe-out
ns-plain-safe(FLOW-KEY)  ::= ns-plain-safe-in
```

y[flow.ns-plain-safe-out+4]
```
[#] ns-plain-safe-out ::=
  ns-char
```

y[flow.ns-plain-safe-in+4]
```
[#] ns-plain-safe-in ::=
  ns-char - c-flow-indicator
```

y[flow.ns-plain-char+4]
```
[#] ns-plain-char(c) ::=
    (
        ns-plain-safe(c)
      - c-mapping-value    # ':'
      - c-comment          # '#'
    )
  | (
      [ lookbehind = ns-char ]
      c-comment          # '#'
    )
  | (
      c-mapping-value    # ':'
      [ lookahead = ns-plain-safe(c) ]
    )
```


**Example #. Plain Characters**

```
# Outside flow collection:
- ::vector
- ": - ()"
- Up, up, and away!
- -123
- https://example.com/foo#bar
# Inside flow collection:
- [ ::vector,
  ": - ()",
  "Up, up and away!",
  -123,
  https://example.com/foo#bar ]
```

```
[ "::vector",
  ": - ()",
  "Up, up, and away!",
  -123,
  "https://example.com/foo#bar",
  [ "::vector",
    ": - ()",
    "Up, up, and away!",
    -123,
    "https://example.com/foo#bar" ] ]
```

**Legend:**
* [ns-plain-first(c)] <!-- 2:3 5:3 8:5 11:3 -->
* [ns-plain-char(c)] <!-- 2:4 4:5 6:7 6:25 8:6 12:7 12:25 -->
* Not ns-plain-first(c) <!-- 3:4 9:4 -->
* Not ns-plain-char(c) <!-- 10:6 -->


Plain scalars are further restricted to a single line when contained inside an
[implicit key].

y[flow.ns-plain]
```
[#]
ns-plain(n,FLOW-OUT)  ::= ns-plain-multi-line(n,FLOW-OUT)
ns-plain(n,FLOW-IN)   ::= ns-plain-multi-line(n,FLOW-IN)
ns-plain(n,BLOCK-KEY) ::= ns-plain-one-line(BLOCK-KEY)
ns-plain(n,FLOW-KEY)  ::= ns-plain-one-line(FLOW-KEY)
```

y[flow.nb-ns-plain-in-line]
```
[#] nb-ns-plain-in-line(c) ::=
  (
    s-white*
    ns-plain-char(c)
  )*
```

y[flow.ns-plain-one-line]
```
[#] ns-plain-one-line(c) ::=
  ns-plain-first(c)
  nb-ns-plain-in-line(c)
```


**Example #. Plain Implicit Keys**

```
implicit block key : [
  implicit flow key : value,
 ]
```

```
{ "implicit block key":
  [ { "implicit flow key": "value" } ] }
```

**Legend:**
* [ns-plain-one-line(c)] <!-- 1:1,18 2:3,17 -->


All leading and trailing [white space] characters are excluded from the
[content].

y[flow.plain.continuation-must-contain-non-space]
Each continuation line must therefore contain at least one non-[space]
character.

Empty lines, if any, are consumed as part of the [line folding].

y[flow.s-ns-plain-next-line]
```
[#] s-ns-plain-next-line(n,c) ::=
  s-flow-folded(n)
  ns-plain-char(c)
  nb-ns-plain-in-line(c)
```

y[flow.ns-plain-multi-line]
```
[#] ns-plain-multi-line(n,c) ::=
  ns-plain-one-line(c)
  s-ns-plain-next-line(n,c)*
```


**Example #. Plain Lines**

```
1st non-empty↓
↓
·2nd non-empty·
→3rd non-empty
```

```
"1st non-empty\n2nd non-empty 3rd non-empty"
```

**Legend:**
* [nb-ns-plain-in-line(c)] <!-- 1:1,13 3:2,13 4:2, -->
* [s-ns-plain-next-line(n,c)] <!-- 1:14 2 3 4 -->


## 7.4. Flow Collection Styles

A _flow collection_ may be nested within a [block collection] ([`FLOW-OUT`
context]), nested within another flow collection ([`FLOW-IN` context]) or be a
part of an [implicit key] ([`FLOW-KEY` context] or [`BLOCK-KEY` context]).
Flow collection entries are terminated by the "`,`" indicator.
The final "`,`" may be omitted.
This does not cause ambiguity because flow collection entries can never be
[completely empty].

y[flow.in-flow+3]
```
[#]
in-flow(n,FLOW-OUT)  ::= ns-s-flow-seq-entries(n,FLOW-IN)
in-flow(n,FLOW-IN)   ::= ns-s-flow-seq-entries(n,FLOW-IN)
in-flow(n,BLOCK-KEY) ::= ns-s-flow-seq-entries(n,FLOW-KEY)
in-flow(n,FLOW-KEY)  ::= ns-s-flow-seq-entries(n,FLOW-KEY)
```


### 7.4.1. Flow Sequences

_Flow sequence content_ is denoted by surrounding "`[`" and "`]`" characters.

y[flow.c-flow-sequence]
```
[#] c-flow-sequence(n,c) ::=
  c-sequence-start    # '['
  s-separate(n,c)?
  in-flow(n,c)?
  c-sequence-end      # ']'
```


Sequence entries are separated by a "`,`" character.

y[flow.ns-s-flow-seq-entries]
```
[#] ns-s-flow-seq-entries(n,c) ::=
  ns-flow-seq-entry(n,c)
  s-separate(n,c)?
  (
    c-collect-entry     # ','
    s-separate(n,c)?
    ns-s-flow-seq-entries(n,c)?
  )?
```


**Example #. Flow Sequence**

```
- [ one, two, ]
- [three ,four]
```

```
[ [ "one",
    "two" ],
  [ "three",
    "four" ] ]
```

**Legend:**
* [c-sequence-start] [c-sequence-end] <!-- [ ] -->
* [ns-flow-seq-entry(n,c)] <!-- one two three four -->


Any [flow node] may be used as a flow sequence entry.
In addition, YAML provides a [compact notation] for the case where a flow
sequence entry is a [mapping] with a [single key/value pair].

y[flow.ns-flow-seq-entry]
```
[#] ns-flow-seq-entry(n,c) ::=
  ns-flow-pair(n,c) | ns-flow-node(n,c)
```


**Example #. Flow Sequence Entries**

```
[
"double
 quoted", 'single
           quoted',
plain
 text, [ nested ],
single: pair,
]
```

```
[ "double quoted",
  "single quoted",
  "plain text",
  [ "nested" ],
  { "single": "pair" } ]
```

**Legend:**
* [ns-flow-node(n,c)] <!-- 2 3:1,8 3:11, 4:1,18 5 6:1,5 6:8,10 -->
* [ns-flow-pair(n,c)] <!-- 7:1,12 -->


### 7.4.2. Flow Mappings

_Flow mappings_ are denoted by surrounding "`{`" and "`}`" characters.

y[flow.c-flow-mapping]
```
[#] c-flow-mapping(n,c) ::=
  c-mapping-start       # '{'
  s-separate(n,c)?
  ns-s-flow-map-entries(n,in-flow(c))?
  c-mapping-end         # '}'
```


Mapping entries are separated by a "`,`" character.

y[flow.ns-s-flow-map-entries]
```
[#] ns-s-flow-map-entries(n,c) ::=
  ns-flow-map-entry(n,c)
  s-separate(n,c)?
  (
    c-collect-entry     # ','
    s-separate(n,c)?
    ns-s-flow-map-entries(n,c)?
  )?
```


**Example #. Flow Mappings**

```
- { one : two , three: four , }
- {five: six,seven : eight}
```

```
[ { "one": "two",
    "three": "four" },
  { "five": "six",
    "seven": "eight" } ]
```

**Legend:**
* [c-mapping-start] [c-mapping-end] <!-- { } -->
* [ns-flow-map-entry(n,c)] <!-- one_:_two three:_four five:_six seven_:_eight -->


If the optional "`?`" mapping key indicator is specified, the rest of the entry
may be [completely empty].

y[flow.ns-flow-map-entry]
```
[#] ns-flow-map-entry(n,c) ::=
    (
      c-mapping-key    # '?' (not followed by non-ws char)
      s-separate(n,c)
      ns-flow-map-explicit-entry(n,c)
    )
  | ns-flow-map-implicit-entry(n,c)
```

y[flow.ns-flow-map-explicit-entry]
```
[#] ns-flow-map-explicit-entry(n,c) ::=
    ns-flow-map-implicit-entry(n,c)
  | (
      e-node    # ""
      e-node    # ""
    )
```


**Example #. Flow Mapping Entries**

```
{
? explicit: entry,
implicit: entry,
?°°
}
```

```
{ "explicit": "entry",
  "implicit": "entry",
  null: null }
```

**Legend:**
* [ns-flow-map-explicit-entry(n,c)] <!-- explicit:_entry -->
* [ns-flow-map-implicit-entry(n,c)] <!-- implicit:_entry -->
* [e-node] <!-- ° -->


Normally, YAML insists the "`:`" mapping value indicator be [separated] from
the [value] by [white space].
A benefit of this restriction is that the "`:`" character can be used inside
[plain scalars], as long as it is not followed by [white space].
This allows for unquoted URLs and timestamps.
It is also a potential source for confusion as "`a:1`" is a [plain scalar] and
not a [key/value pair].

Note that the [value] may be [completely empty] since its existence is
indicated by the "`:`".

y[flow.ns-flow-map-implicit-entry]
```
[#] ns-flow-map-implicit-entry(n,c) ::=
    ns-flow-map-yaml-key-entry(n,c)
  | c-ns-flow-map-empty-key-entry(n,c)
  | c-ns-flow-map-json-key-entry(n,c)
```

y[flow.ns-flow-map-yaml-key-entry]
```
[#] ns-flow-map-yaml-key-entry(n,c) ::=
  ns-flow-yaml-node(n,c)
  (
      (
        s-separate(n,c)?
        c-ns-flow-map-separate-value(n,c)
      )
    | e-node    # ""
  )
```

y[flow.c-ns-flow-map-empty-key-entry]
```
[#] c-ns-flow-map-empty-key-entry(n,c) ::=
  e-node    # ""
  c-ns-flow-map-separate-value(n,c)
```

y[flow.c-ns-flow-map-separate-value+4]
```
[#] c-ns-flow-map-separate-value(n,c) ::=
  c-mapping-value    # ':'
  [ lookahead ≠ ns-plain-safe(c) ]
  (
      (
        s-separate(n,c)
        ns-flow-node(n,c)
      )
    | e-node    # ""
  )
```


**Example #. Flow Mapping Separate Values**

```
{
unquoted·:·"separate",
https://foo.com,
omitted value:°,
°:·omitted key,
}
```

```
{ "unquoted": "separate",
  "https://foo.com": null,
  "omitted value": null,
  null: "omitted key" }
```

**Legend:**
* [ns-flow-yaml-node(n,c)] <!-- unquoted https://foo.com omitted_value -->
* [e-node] <!-- :·"separate" 4:14,2 :·omitted_key -->
* [c-ns-flow-map-separate-value(n,c)] <!-- 4:15 5:1 -->


y[flow.json-key.should-separate-value+3]
To ensure [JSON compatibility], if a [key] inside a flow mapping is
[JSON-like], YAML allows the following [value] to be specified adjacent to the
"`:`".
This causes no ambiguity, as all [JSON-like] [keys] are surrounded by
[indicators].
However, as this greatly reduces readability, YAML [processors] should
[separate] the [value] from the "`:`" on output, even in this case.

y[flow.c-ns-flow-map-json-key-entry+3]
```
[#] c-ns-flow-map-json-key-entry(n,c) ::=
  c-flow-json-node(n,c)
  (
      (
        s-separate(n,c)?
        c-ns-flow-map-adjacent-value(n,c)
      )
    | e-node    # ""
  )
```

y[flow.c-ns-flow-map-adjacent-value+3]
```
[#] c-ns-flow-map-adjacent-value(n,c) ::=
  c-mapping-value          # ':'
  (
      (
        s-separate(n,c)?
        ns-flow-node(n,c)
      )
    | e-node    # ""
  )
```


**Example #. Flow Mapping Adjacent Values**

```
{
"adjacent":value,
"readable":·value,
"empty":°
}
```

```
{ "adjacent": "value",
  "readable": "value",
  "empty": null }
```

**Legend:**
* [c-flow-json-node(n,c)] <!-- "adjacent" "readable" "empty" -->
* [e-node] <!-- ° -->
* [c-ns-flow-map-adjacent-value(n,c)] <!-- value -->


A more compact notation is usable inside [flow sequences], if the [mapping]
contains a _single key/value pair_.
This notation does not require the surrounding "`{`" and "`}`" characters.
Note that it is not possible to specify any [node properties] for the [mapping]
in this case.


**Example #. Single Pair Flow Mappings**

```
[
foo: bar
]
```

```
[ { "foo": "bar" } ]
```

**Legend:**
* [ns-flow-pair(n,c)] <!-- foo:_bar -->


If the "`?`" indicator is explicitly specified, [parsing] is unambiguous and
the syntax is identical to the general case.

y[flow.ns-flow-pair]
```
[#] ns-flow-pair(n,c) ::=
    (
      c-mapping-key     # '?' (not followed by non-ws char)
      s-separate(n,c)
      ns-flow-map-explicit-entry(n,c)
    )
  | ns-flow-pair-entry(n,c)
```


**Example #. Single Pair Explicit Entry**

```
[
? foo
 bar : baz
]
```

```
[ { "foo bar": "baz" } ]
```

**Legend:**
* [ns-flow-map-explicit-entry(n,c)] <!-- foo bar_:_baz -->


If the "`?`" indicator is omitted, [parsing] needs to see past the _implicit
key_ to recognize it as such.

y[flow.implicit-key.must-limit-1024+3]
To limit the amount of lookahead required, the "`:`" indicator must appear at
most 1024 Unicode characters beyond the start of the [key].

y[flow.implicit-key.must-single-line]
In addition, the [key] is restricted to a single line.

Note that YAML allows arbitrary [nodes] to be used as [keys].
In particular, a [key] may be a [sequence] or a [mapping].
Thus, without the above restrictions, practical one-pass [parsing] would have
been impossible to implement.

y[flow.ns-flow-pair-entry]
```
[#] ns-flow-pair-entry(n,c) ::=
    ns-flow-pair-yaml-key-entry(n,c)
  | c-ns-flow-map-empty-key-entry(n,c)
  | c-ns-flow-pair-json-key-entry(n,c)
```

y[flow.ns-flow-pair-yaml-key-entry]
```
[#] ns-flow-pair-yaml-key-entry(n,c) ::=
  ns-s-implicit-yaml-key(FLOW-KEY)
  c-ns-flow-map-separate-value(n,c)
```

y[flow.c-ns-flow-pair-json-key-entry+3]
```
[#] c-ns-flow-pair-json-key-entry(n,c) ::=
  c-s-implicit-json-key(FLOW-KEY)
  c-ns-flow-map-adjacent-value(n,c)
```

y[flow.ns-s-implicit-yaml-key]
```
[#] ns-s-implicit-yaml-key(c) ::=
  ns-flow-yaml-node(0,c)
  s-separate-in-line?
  /* At most 1024 characters altogether */
```

y[flow.c-s-implicit-json-key+3]
```
[#] c-s-implicit-json-key(c) ::=
  c-flow-json-node(0,c)
  s-separate-in-line?
  /* At most 1024 characters altogether */
```


**Example #. Single Pair Implicit Entries**

```
- [ YAML·: separate ]
- [ °: empty key entry ]
- [ {JSON: like}:adjacent ]
```

```
[ [ { "YAML": "separate" } ],
  [ { null: "empty key entry" } ],
  [ { { "JSON": "like" }: "adjacent" } ] ]
```

**Legend:**
* [ns-s-implicit-yaml-key] <!-- YAML· -->
* [e-node] <!-- ° -->
* [c-s-implicit-json-key] <!-- {JSON:_like} -->
* Value <!-- :_separate :_empty_key_entry :adjacent -->


**Example #. Invalid Implicit Keys**

```
[ foo
 bar: invalid,
 "foo_...>1K characters..._bar": invalid ]
```
<!-- 1:3,3 2:1,4 -->
<!-- 3:2,30 -->

```
ERROR:
- The foo bar key spans multiple lines
- The foo...bar key is too long
```
<!-- 2:7,7 -->
<!-- 3:7,9 -->


## 7.5. Flow Nodes

_JSON-like_ [flow styles] all have explicit start and end [indicators].
The only [flow style] that does not have this property is the [plain scalar].
Note that none of the "JSON-like" styles is actually acceptable by JSON.
Even the [double-quoted style] is a superset of the JSON string format.

y[flow.ns-flow-yaml-content]
```
[#] ns-flow-yaml-content(n,c) ::=
  ns-plain(n,c)
```

y[flow.c-flow-json-content+3]
```
[#] c-flow-json-content(n,c) ::=
    c-flow-sequence(n,c)
  | c-flow-mapping(n,c)
  | c-single-quoted(n,c)
  | c-double-quoted(n,c)
```

y[flow.ns-flow-content]
```
[#] ns-flow-content(n,c) ::=
    ns-flow-yaml-content(n,c)
  | c-flow-json-content(n,c)
```


**Example #. Flow Content**

```
- [ a, b ]
- { a: b }
- "a"
- 'b'
- c
```

```
[ [ "a", "b" ],
  { "a": "b" },
  "a",
  "b",
  "c" ]
```

**Legend:**
* [c-flow-json-content(n,c)] <!-- [_a,_b_] {_a:_b_} "a" 'b' -->
* [ns-flow-yaml-content(n,c)] <!-- 5:3 -->


A complete [flow] [node] also has optional [node properties], except for [alias
nodes] which refer to the [anchored] [node properties].

y[flow.ns-flow-yaml-node]
```
[#] ns-flow-yaml-node(n,c) ::=
    c-ns-alias-node
  | ns-flow-yaml-content(n,c)
  | (
      c-ns-properties(n,c)
      (
          (
            s-separate(n,c)
            ns-flow-yaml-content(n,c)
          )
        | e-scalar
      )
    )
```

y[flow.c-flow-json-node+3]
```
[#] c-flow-json-node(n,c) ::=
  (
    c-ns-properties(n,c)
    s-separate(n,c)
  )?
  c-flow-json-content(n,c)
```

y[flow.ns-flow-node]
```
[#] ns-flow-node(n,c) ::=
    c-ns-alias-node
  | ns-flow-content(n,c)
  | (
      c-ns-properties(n,c)
      (
        (
          s-separate(n,c)
          ns-flow-content(n,c)
        )
        | e-scalar
      )
    )
```


**Example #. Flow Nodes**

```
- !!str "a"
- 'b'
- &anchor "c"
- *anchor
- !!str°
```

```
[ "a",
  "b",
  "c",
  "c",
  "" ]
```

**Legend:**
* [c-flow-json-node(n,c)] <!-- !!str_"a" 'b' &anchor_"c" -->
* [ns-flow-yaml-node(n,c)] <!-- *anchor !!str° -->
