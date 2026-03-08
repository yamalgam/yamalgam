# Chapter 8. Block Style Productions

> Marked-up copy of YAML 1.2.2 specification Chapter 8, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

YAML's _block styles_ employ [indentation] rather than [indicators] to denote
structure.
This results in a more human readable (though less compact) notation.


## 8.1. Block Scalar Styles

YAML provides two _block scalar styles_, [literal] and [folded].
Each provides a different trade-off between readability and expressive power.


### 8.1.1. Block Scalar Headers

[Block scalars] are controlled by a few [indicators] given in a _header_
preceding the [content] itself.
This header is followed by a non-content [line break] with an optional
[comment].

This is the only case where a [comment] must not be followed by additional
[comment] lines.

> Note: See [Production Parameters] for the definition of the `t` variable.

```
[#] c-b-block-header(t) ::=
  (
      (
        c-indentation-indicator
        c-chomping-indicator(t)
      )
    | (
        c-chomping-indicator(t)
        c-indentation-indicator
      )
  )
  s-b-comment
```


**Example #. Block Scalar Header**

```
- | # Empty header↓
 literal
- >1 # Indentation indicator↓
 ·folded
- |+ # Chomping indicator↓
 keep

- >1- # Both indicators↓
 ·strip
```

```
[ "literal\n",
  " folded\n",
  "keep\n\n",
  " strip" ]
```

**Legend:**
* [c-b-block-header(t)] <!-- _#_Empty_header↓ 01_#_Indentation_indicator↓ +_#_Chomping_indicator↓ 01-_#_Both_indicators↓ -->


#### 8.1.1.1. Block Indentation Indicator

Every block scalar has a _content indentation level_.
The content of the block scalar excludes a number of leading [spaces] on each
line up to the content indentation level.

If a block scalar has an _indentation indicator_, then the content indentation
level of the block scalar is equal to the indentation level of the block scalar
plus the integer value of the indentation indicator character.

If no indentation indicator is given, then the content indentation level is
equal to the number of leading [spaces] on the first non-[empty line] of the
contents.
If there is no non-[empty line] then the content indentation level is equal to
the number of spaces on the longest line.

It is an error if any non-[empty line] does not begin with a number of spaces
greater than or equal to the content indentation level.

It is an error for any of the leading [empty lines] to contain more [spaces]
than the first non-[empty line].

A YAML [processor] should only emit an explicit indentation indicator for cases
where detection will fail.

```
[#] c-indentation-indicator ::=
  [x31-x39]    # 1-9
```


**Example #. Block Indentation Indicator**

```
- |°
·detected
- >°
·
··
··# detected
- |1
··explicit
- >°
·→
·detected
```

```
[ "detected\n",
  "\n\n# detected\n",
  " explicit\n",
  "\t\ndetected\n" ]
```

**Legend:**
* [c-indentation-indicator] <!-- ° 7:4 -->
* [s-indent(n)] <!-- ·· · -->


**Example #. Invalid Block Scalar Indentation Indicators**

```
- |
··
·text
- >
··text
·text
- |2
·text
```
<!-- 2:2 -->
<!-- 6:1 -->
<!-- 8:1 -->

```
ERROR:
- A leading all-space line must
  not have too many spaces.
- A following text line must
  not be less indented.
- The text is less indented
  than the indicated level.
```
<!-- spaces -->
<!-- 5:10,13 -->
<!-- 6:15,13 -->


#### 8.1.1.2. Block Chomping Indicator

_Chomping_ controls how final [line breaks] and trailing [empty lines] are
interpreted.
YAML provides three chomping methods:


Strip
:
_Stripping_ is specified by the "`-`" chomping indicator.
In this case, the final [line break] and any trailing [empty lines] are
excluded from the [scalar's content].


Clip
:
_Clipping_ is the default behavior used if no explicit chomping indicator is
specified.
In this case, the final [line break] character is preserved in the [scalar's
content].
However, any trailing [empty lines] are excluded from the [scalar's content].


Keep
:
_Keeping_ is specified by the "`+`" chomping indicator.
In this case, the final [line break] and any trailing [empty lines] are
considered to be part of the [scalar's content].
These additional lines are not subject to [folding].

The chomping method used is a [presentation detail] and must not be used to
convey [content] information.

```
[#]
c-chomping-indicator(STRIP) ::= '-'
c-chomping-indicator(KEEP)  ::= '+'
c-chomping-indicator(CLIP)  ::= ""
```


The interpretation of the final [line break] of a [block scalar] is controlled
by the chomping indicator specified in the [block scalar header].

```
[#]
b-chomped-last(STRIP) ::= b-non-content  | <end-of-input>
b-chomped-last(CLIP)  ::= b-as-line-feed | <end-of-input>
b-chomped-last(KEEP)  ::= b-as-line-feed | <end-of-input>
```


**Example #. Chomping Final Line Break**

```
strip: |-
  text↓
clip: |
  text↓
keep: |+
  text↓
```

```
{ "strip": "text",
  "clip": "text\n",
  "keep": "text\n" }
```

**Legend:**
* [b-non-content] <!-- 2:7 -->
* [b-as-line-feed] <!-- 4:7 6:7 -->


The interpretation of the trailing [empty lines] following a [block scalar] is
also controlled by the chomping indicator specified in the [block scalar
header].

```
[#]
l-chomped-empty(n,STRIP) ::= l-strip-empty(n)
l-chomped-empty(n,CLIP)  ::= l-strip-empty(n)
l-chomped-empty(n,KEEP)  ::= l-keep-empty(n)
```

```
[#] l-strip-empty(n) ::=
  (
    s-indent-less-or-equal(n)
    b-non-content
  )*
  l-trail-comments(n)?
```

```
[#] l-keep-empty(n) ::=
  l-empty(n,BLOCK-IN)*
  l-trail-comments(n)?
```


Explicit [comment] lines may follow the trailing [empty lines].

To prevent ambiguity, the first such [comment] line must be less [indented]
than the [block scalar content].
Additional [comment] lines, if any, are not so restricted.
This is the only case where the [indentation] of [comment] lines is
constrained.

```
[#] l-trail-comments(n) ::=
  s-indent-less-than(n)
  c-nb-comment-text
  b-comment
  l-comment*
```


**Example #. Chomping Trailing Lines**

```
# Strip
  # Comments:
strip: |-
  # text↓
··⇓
·# Clip
··# comments:
↓
clip: |
  # text↓
·↓
·# Keep
··# comments:
↓
keep: |+
  # text↓
↓
·# Trail
··# comments.
```

```
{ "strip": "# text",
  "clip": "# text\n",
  "keep": "# text\n\n" }
```

**Legend:**
* [l-strip-empty(n)] <!-- 5 6 7 8 11 12 13 14 -->
* [l-keep-empty(n)] <!-- 17 18 19 -->
* [l-trail-comments(n)] <!-- 6 7 8 12 13 14 18 19 -->

If a [block scalar] consists only of [empty lines], then these lines are
considered as trailing lines and hence are affected by chomping.


**Example #. Empty Scalar Chomping**

```
strip: >-
↓
clip: >
↓
keep: |+
↓
```

```
{ "strip": "",
  "clip": "",
  "keep": "\n" }
```

**Legend:**
* [l-strip-empty(n)] <!-- 2 4 -->
* [l-keep-empty(n)] <!-- 6 -->


### 8.1.2. Literal Style

The _literal style_ is denoted by the "`|`" indicator.
It is the simplest, most restricted and most readable [scalar style].

```
[#] c-l+literal(n) ::=
  c-literal                # '|'
  c-b-block-header(t)
  l-literal-content(n+m,t)
```


**Example #. Literal Scalar**

```
|↓
·literal↓
·→text↓
↓
```

```
"literal\n\ttext\n"
```

**Legend:**
* [c-l+literal(n)] <!-- 1 2 3 4 -->


Inside literal scalars, all ([indented]) characters are considered to be
[content], including [white space] characters.
Note that all [line break] characters are [normalized].
In addition, [empty lines] are not [folded], though final [line breaks] and
trailing [empty lines] are [chomped].

There is no way to escape characters inside literal scalars.
This restricts them to [printable] characters.
In addition, there is no way to break a long literal line.

```
[#] l-nb-literal-text(n) ::=
  l-empty(n,BLOCK-IN)*
  s-indent(n) nb-char+
```

```
[#] b-nb-literal-next(n) ::=
  b-as-line-feed
  l-nb-literal-text(n)
```

```
[#] l-literal-content(n,t) ::=
  (
    l-nb-literal-text(n)
    b-nb-literal-next(n)*
    b-chomped-last(t)
  )?
  l-chomped-empty(n,t)
```


**Example #. Literal Content**

```
|
·
··
··literal↓
···↓
··
··text↓
↓
·# Comment
```

```
"\n\nliteral\n·\n\ntext\n"
```

**Legend:**
* [l-nb-literal-text(n)] <!-- 2 3 4:1,9 5:1,3 6 7:1,6 -->
* [b-nb-literal-next(n)] <!-- 4:10 5:1,3 5:4 6 7:1,6 -->
* [b-chomped-last(t)] <!-- 7:7 -->
* [l-chomped-empty(n,t)] <!-- 9 -->


### 8.1.3. Folded Style

The _folded style_ is denoted by the "`>`" indicator.
It is similar to the [literal style]; however, folded scalars are subject to
[line folding].

```
[#] c-l+folded(n) ::=
  c-folded                 # '>'
  c-b-block-header(t)
  l-folded-content(n+m,t)
```


**Example #. Folded Scalar**

```
>↓
·folded↓
·text↓
↓
```

```
"folded text\n"
```

**Legend:**
* [c-l+folded(n)] <!-- 1 2 3 4 -->


[Folding] allows long lines to be broken anywhere a single [space] character
separates two non-[space] characters.

```
[#] s-nb-folded-text(n) ::=
  s-indent(n)
  ns-char
  nb-char*
```

```
[#] l-nb-folded-lines(n) ::=
  s-nb-folded-text(n)
  (
    b-l-folded(n,BLOCK-IN)
    s-nb-folded-text(n)
  )*
```


**Example #. Folded Lines**

```
>

·folded↓
·line↓
↓
·next
·line↓
   * bullet

   * list
   * lines

·last↓
·line↓

# Comment
```

```
"\nfolded line\nnext line\n  \
* bullet\n \n  * list\n  \
* lines\n\nlast line\n"
```

**Legend:**
* [l-nb-folded-lines(n)] <!-- 3:1,7 4:1,5 6 7:1,5 13:1,5 14:1,5 -->
* [s-nb-folded-text(n)] <!-- 3 4 6 7:1,5 13 14:1,5 -->


(The following three examples duplicate this example, each highlighting
different productions.)

Lines starting with [white space] characters (_more-indented_ lines) are not
[folded].

```
[#] s-nb-spaced-text(n) ::=
  s-indent(n)
  s-white
  nb-char*
```

```
[#] b-l-spaced(n) ::=
  b-as-line-feed
  l-empty(n,BLOCK-IN)*
```

```
[#] l-nb-spaced-lines(n) ::=
  s-nb-spaced-text(n)
  (
    b-l-spaced(n)
    s-nb-spaced-text(n)
  )*
```


**Example #. More Indented Lines**

```
>

 folded
 line

 next
 line
···* bullet↓
↓
···* list↓
···* lines↓

 last
 line

# Comment
```

```
"\nfolded line\nnext line\n  \
* bullet\n \n  * list\n  \
* lines\n\nlast line\n"
```

**Legend:**
* [l-nb-spaced-lines(n)] <!-- 8 9 10 11:1,10 -->
* [s-nb-spaced-text(n)] <!-- 8:1,11 10:1,9 11:1,10 -->


[Line breaks] and [empty lines] separating folded and more-indented lines are
also not [folded].

```
[#] l-nb-same-lines(n) ::=
  l-empty(n,BLOCK-IN)*
  (
      l-nb-folded-lines(n)
    | l-nb-spaced-lines(n)
  )
```

```
[#] l-nb-diff-lines(n) ::=
  l-nb-same-lines(n)
  (
    b-as-line-feed
    l-nb-same-lines(n)
  )*
```


**Example #. Empty Separation Lines**

```
>
↓
 folded
 line↓
↓
 next
 line↓
   * bullet

   * list
   * lines↓
↓
 last
 line

# Comment
```

```
"\nfolded line\nnext line\n  \
* bullet\n \n  * list\n  \
* lines\n\nlast line\n"
```

**Legend:**
* [b-as-line-feed] <!-- 4:6 7:6 11:11 -->
* (separation) [l-empty(n,c)] <!-- 2 5 12 -->


The final [line break] and trailing [empty lines] if any, are subject to
[chomping] and are never [folded].

```
[#] l-folded-content(n,t) ::=
  (
    l-nb-diff-lines(n)
    b-chomped-last(t)
  )?
  l-chomped-empty(n,t)
```


**Example #. Final Empty Lines**

```
>

 folded
 line

 next
 line
   * bullet

   * list
   * lines

 last
 line↓
↓
# Comment
```

```
"\nfolded line\nnext line\n  \
* bullet\n \n  * list\n  \
* lines\n\nlast line\n"
```

**Legend:**
* [b-chomped-last(t)] <!-- 14:6 -->
* [l-chomped-empty(n,t)] <!-- 15 16 -->


## 8.2. Block Collection Styles

For readability, _block collections styles_ are not denoted by any [indicator].
Instead, YAML uses a lookahead method, where a block collection is
distinguished from a [plain scalar] only when a [key/value pair] or a [sequence
entry] is seen.


### 8.2.1. Block Sequences

A _block sequence_ is simply a series of [nodes], each denoted by a leading
"`-`" indicator.

The "`-`" indicator must be [separated] from the [node] by [white space].
This allows "`-`" to be used as the first character in a [plain scalar] if
followed by a non-space character (e.g. "`-42`").

```
[#] l+block-sequence(n) ::=
  (
    s-indent(n+1+m)
    c-l-block-seq-entry(n+1+m)
  )+
```

```
[#] c-l-block-seq-entry(n) ::=
  c-sequence-entry    # '-'
  [ lookahead ≠ ns-char ]
  s-l+block-indented(n,BLOCK-IN)
```


**Example #. Block Sequence**

```
block sequence:
··- one↓
  - two : three↓
```

```
{ "block sequence": [
    "one",
    { "two": "three" } ] }
```

**Legend:**
* [c-l-block-seq-entry(n)] <!-- 2:3, 3:3, -->
* auto-detected [s-indent(n)] <!-- 2:1,2 -->


The entry [node] may be either [completely empty], be a nested [block node] or
use a _compact in-line notation_.
The compact notation may be used when the entry is itself a nested [block
collection].
In this case, both the "`-`" indicator and the following [spaces] are
considered to be part of the [indentation] of the nested [collection].
Note that it is not possible to specify [node properties] for such a
[collection].

```
[#] s-l+block-indented(n,c) ::=
    (
      s-indent(m)
      (
          ns-l-compact-sequence(n+1+m)
        | ns-l-compact-mapping(n+1+m)
      )
    )
  | s-l+block-node(n,c)
  | (
      e-node    # ""
      s-l-comments
    )
```

```
[#] ns-l-compact-sequence(n) ::=
  c-l-block-seq-entry(n)
  (
    s-indent(n)
    c-l-block-seq-entry(n)
  )*
```


**Example #. Block Sequence Entry Types**

```
-° # Empty
- |
 block node
-·- one # Compact
··- two # sequence
- one: two # Compact mapping
```

```
[ null,
  "block node\n",
  [ "one", "two" ],
  { "one": "two" } ]
```

**Legend:**
* Empty <!-- °_#_Empty -->
* [s-l+block-node(n,c)] <!-- 2:2, block_node -->
* [ns-l-compact-sequence(n)] <!-- ·-_one_#_Compact ··-_two_#_sequence -->
* [ns-l-compact-mapping(n)] <!-- _one:_two_#_Compact_mapping -->


### 8.2.2. Block Mappings

A _Block mapping_ is a series of entries, each [presenting] a [key/value pair].

```
[#] l+block-mapping(n) ::=
  (
    s-indent(n+1+m)
    ns-l-block-map-entry(n+1+m)
  )+
```


**Example #. Block Mappings**

```
block mapping:
·key: value↓
```

```
{ "block mapping": {
    "key": "value" } }
```

**Legend:**
* [ns-l-block-map-entry(n)] <!-- 2:2, -->
* auto-detected [s-indent(n)] <!-- 2:1 -->


If the "`?`" indicator is specified, the optional value node must be specified
on a separate line, denoted by the "`:`" indicator.
Note that YAML allows here the same [compact in-line notation] described above
for [block sequence] entries.

```
[#] ns-l-block-map-entry(n) ::=
    c-l-block-map-explicit-entry(n)
  | ns-l-block-map-implicit-entry(n)
```

```
[#] c-l-block-map-explicit-entry(n) ::=
  c-l-block-map-explicit-key(n)
  (
      l-block-map-explicit-value(n)
    | e-node                        # ""
  )
```

```
[#] c-l-block-map-explicit-key(n) ::=
  c-mapping-key                     # '?' (not followed by non-ws char)
  s-l+block-indented(n,BLOCK-OUT)
```

```
[#] l-block-map-explicit-value(n) ::=
  s-indent(n)
  c-mapping-value                   # ':' (not followed by non-ws char)
  s-l+block-indented(n,BLOCK-OUT)
```


**Example #. Explicit Block Mapping Entries**

```
? explicit key # Empty value↓°
? |
  block key↓
:·- one # Explicit compact
··- two # block value↓
```

```
{ "explicit key": null,
  "block key\n": [
    "one",
    "two" ] }
```

**Legend:**
* [c-l-block-map-explicit-key(n)] <!-- 1:1,29 2 3 -->
* [l-block-map-explicit-value(n)] <!-- 4 5 -->
* [e-node] <!-- 1:30 -->

<!-- REVIEW value should be null above -->

If the "`?`" indicator is omitted, [parsing] needs to see past the
[implicit key], in the same way as in the [single key/value pair] [flow
mapping].
Hence, such [keys] are subject to the same restrictions; they are limited to a
single line and must not span more than 1024 Unicode characters.

```
[#] ns-l-block-map-implicit-entry(n) ::=
  (
      ns-s-block-map-implicit-key
    | e-node    # ""
  )
  c-l-block-map-implicit-value(n)
```

```
[#] ns-s-block-map-implicit-key ::=
    c-s-implicit-json-key(BLOCK-KEY)
  | ns-s-implicit-yaml-key(BLOCK-KEY)
```


In this case, the [value] may be specified on the same line as the [implicit
key].

Note however that in block mappings the [value] must never be adjacent to the
"`:`", as this greatly reduces readability and is not required for [JSON
compatibility] (unlike the case in [flow mappings]).

There is no compact notation for in-line [values].
Also, while both the [implicit key] and the [value] following it may be empty,
the "`:`" indicator is mandatory.
This prevents a potential ambiguity with multi-line [plain scalars].

```
[#] c-l-block-map-implicit-value(n) ::=
  c-mapping-value           # ':' (not followed by non-ws char)
  (
      s-l+block-node(n,BLOCK-OUT)
    | (
        e-node    # ""
        s-l-comments
      )
  )
```


**Example #. Implicit Block Mapping Entries**

```
plain key: in-line value
°:° # Both empty
"quoted key":
- entry
```

```
{ "plain key": "in-line value",
  null: null,
  "quoted key": [ "entry" ] }
```

**Legend:**
* [ns-s-block-map-implicit-key] <!-- 1:1,9 2:1 3:1,12 -->
* [c-l-block-map-implicit-value(n)] <!-- 1:10, 2:2, 3:13 4 -->


A [compact in-line notation] is also available.
This compact notation may be nested inside [block sequences] and explicit block
mapping entries.
Note that it is not possible to specify [node properties] for such a nested
mapping.

```
[#] ns-l-compact-mapping(n) ::=
  ns-l-block-map-entry(n)
  (
    s-indent(n)
    ns-l-block-map-entry(n)
  )*
```


**Example #. Compact Block Mappings**

```
- sun: yellow↓
- ? earth: blue↓
  : moon: white↓
```

```
[ { "sun": "yellow" },
  { { "earth": "blue" }:
      { "moon": "white" } } ]
```

**Legend:**
* [ns-l-compact-mapping(n)] <!-- 1:3, 2:3, 3 -->


### 8.2.3. Block Nodes

YAML allows [flow nodes] to be embedded inside [block collections] (but not
vice-versa).

[Flow nodes] must be [indented] by at least one more [space] than the parent
[block collection].
Note that [flow nodes] may begin on a following line.

It is at this point that [parsing] needs to distinguish between a [plain
scalar] and an [implicit key] starting a nested [block mapping].

```
[#] s-l+block-node(n,c) ::=
    s-l+block-in-block(n,c)
  | s-l+flow-in-block(n)
```

```
[#] s-l+flow-in-block(n) ::=
  s-separate(n+1,FLOW-OUT)
  ns-flow-node(n+1,FLOW-OUT)
  s-l-comments
```


**Example #. Block Node Types**

```
-↓
··"flow in block"↓
-·>
 Block scalar↓
-·!!map # Block collection
  foo : bar↓
```

```
[ "flow in block",
  "Block scalar\n",
  { "foo": "bar" } ]
```

**Legend:**
* [s-l+flow-in-block(n)] <!-- 1:2 2 -->
* [s-l+block-in-block(n,c)] <!-- 3:3 4 5:3, 6 -->


The block [node's properties] may span across several lines.

In this case, they must be [indented] by at least one more [space] than the
[block collection], regardless of the [indentation] of the [block collection]
entries.

```
[#] s-l+block-in-block(n,c) ::=
    s-l+block-scalar(n,c)
  | s-l+block-collection(n,c)
```

```
[#] s-l+block-scalar(n,c) ::=
  s-separate(n+1,c)
  (
    c-ns-properties(n+1,c)
    s-separate(n+1,c)
  )?
  (
      c-l+literal(n)
    | c-l+folded(n)
  )
```


**Example #. Block Scalar Nodes**

```
literal: |2
··value
folded:↓
···!foo
··>1
·value
```

```
{ "literal": "value",
  "folded": !<!foo> "value" }
```

**Legend:**
* [c-l+literal(n)] <!-- 1:10, 2:1,7 -->
* [c-l+folded(n)] <!-- 3:8 4 5 6 -->


Since people perceive the "`-`" indicator as [indentation], nested [block
sequences] may be [indented] by one less [space] to compensate, except, of
course, if nested inside another [block sequence] ([`BLOCK-OUT` context] versus
[`BLOCK-IN` context]).

```
[#] s-l+block-collection(n,c) ::=
  (
    s-separate(n+1,c)
    c-ns-properties(n+1,c)
  )?
  s-l-comments
  (
      seq-space(n,c)
    | l+block-mapping(n)
  )
```

```
[#] seq-space(n,BLOCK-OUT) ::= l+block-sequence(n-1)
    seq-space(n,BLOCK-IN)  ::= l+block-sequence(n)
```


**Example #. Block Collection Nodes**

```
sequence: !!seq
- entry
- !!seq
 - nested
mapping: !!map
 foo: bar
```

```
{ "sequence": [
    "entry",
    [ "nested" ] ],
  "mapping": { "foo": "bar" } }
```

**Legend:**
* [s-l+block-collection(n,c)] <!-- 1:10, 2 3 4 5:9, 6 -->
* [l+block-sequence(n)] <!-- 2 3 4 -->
* [l+block-mapping(n)] <!-- 6 -->
