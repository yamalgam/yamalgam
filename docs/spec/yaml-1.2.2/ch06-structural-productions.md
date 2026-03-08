# Chapter 6. Structural Productions

> Marked-up copy of YAML 1.2.2 specification Chapter 6, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

## 6.1. Indentation Spaces

In YAML [block styles], structure is determined by _indentation_.
In general, indentation is defined as a zero or more [space] characters at the
start of a line.

y[struct.indent.tab-forbidden]
To maintain portability, [tab] characters must not be used in indentation,
since different systems treat [tabs] differently.
Note that most modern editors may be configured so that pressing the [tab] key
results in the insertion of an appropriate number of [spaces].

y[struct.indent.not-content]
The amount of indentation is a [presentation detail] and must not be used to
convey [content] information.

y[struct.s-indent]
```
[#]
s-indent(0) ::=
  <empty>

# When n>=0
s-indent(n+1) ::=
  s-space s-indent(n)
```


A [block style] construct is terminated when encountering a line which is less
indented than the construct.
The productions use the notation "`s-indent-less-than(n)`" and
"`s-indent-less-or-equal(n)`" to express this.

y[struct.s-indent-less-than]
```
[#]
s-indent-less-than(1) ::=
  <empty>

# When n>=1
s-indent-less-than(n+1) ::=
  s-space s-indent-less-than(n)
  | <empty>
```

y[struct.s-indent-less-or-equal]
```
[#]
s-indent-less-or-equal(0) ::=
  <empty>

# When n>=0
s-indent-less-or-equal(n+1) ::=
  s-space s-indent-less-or-equal(n)
  | <empty>
```


y[struct.indent.node-deeper-than-parent]
Each [node] must be indented further than its parent [node].

y[struct.indent.siblings-same-level]
All sibling [nodes] must use the exact same indentation level.
However the [content] of each sibling [node] may be further indented
independently.


**Example #. Indentation Spaces**

```
··# Leading comment line spaces are
···# neither content nor indentation.
····
Not indented:
·By one space: |
····By four
······spaces
·Flow style: [    # Leading spaces
···By two,        # in flow style
··Also by two,    # are neither
··→Still by two   # content nor
····]             # indentation.
```

```
{ "Not indented": {
    "By one space": "By four\n  spaces\n",
    "Flow style": [
      "By two",
      "Also by two",
      "Still by two" ] } }
```

**Legend:**
* [s-indent(n)] <!-- 5:1 6:1,4 7:1,4 8:1 9:1,2 10:1,2 11:1,2 12:1,2 -->
* Content <!-- 7:5,2 -->
* Neither content nor indentation <!-- 1:1,2 2:1,3 3 9:3 12:3,2 -->


The "`-`", "`?`" and "`:`" characters used to denote [block collection] entries
are perceived by people to be part of the indentation.
This is handled on a case-by-case basis by the relevant productions.


**Example #. Indentation Indicators**

```
?·a
:·-→b
··-··-→c
·····-·d
```

```
{ "a":
  [ "b",
    [ "c",
      "d" ] ] }
```

**Legend:**
* Total Indentation <!-- 1:1 2:1,3 3:1,6 4:1,6 -->
* [s-indent(n)] <!-- 2:2 3:1,2 3:4,2 4:1,5 -->
* Indicator as indentation <!-- 1:1 2:1 2:3 3:3 3:6 4:6 -->


## 6.2. Separation Spaces

Outside [indentation] and [scalar content], YAML uses [white space] characters
for _separation_ between tokens within a line.
Note that such [white space] may safely include [tab] characters.

y[struct.separation.not-content]
Separation spaces are a [presentation detail] and must not be used to convey
[content] information.

y[struct.s-separate-in-line]
```
[#] s-separate-in-line ::=
    s-white+
  | <start-of-line>
```


**Example #. Separation Spaces**

```
-·foo:→·bar
- -·baz
  -→baz
```

```
[ { "foo": "bar" },
  [ "baz",
    "baz" ] ]
```

**Legend:**
* [s-separate-in-line] <!-- →· · → -->


## 6.3. Line Prefixes

Inside [scalar content], each line begins with a non-[content] _line prefix_.
This prefix always includes the [indentation].
For [flow scalar styles] it additionally includes all leading [white space],
which may contain [tab] characters.

y[struct.line-prefix.not-content]
Line prefixes are a [presentation detail] and must not be used to convey
[content] information.

y[struct.s-line-prefix]
```
[#]
s-line-prefix(n,BLOCK-OUT) ::= s-block-line-prefix(n)
s-line-prefix(n,BLOCK-IN)  ::= s-block-line-prefix(n)
s-line-prefix(n,FLOW-OUT)  ::= s-flow-line-prefix(n)
s-line-prefix(n,FLOW-IN)   ::= s-flow-line-prefix(n)
```

y[struct.s-block-line-prefix]
```
[#] s-block-line-prefix(n) ::=
  s-indent(n)
```

y[struct.s-flow-line-prefix]
```
[#] s-flow-line-prefix(n) ::=
  s-indent(n)
  s-separate-in-line?
```


**Example #. Line Prefixes**

```
plain: text
··lines
quoted: "text
··→lines"
block: |
··text
···→lines
```

```
{ "plain": "text lines",
  "quoted": "text lines",
  "block": "text\n \tlines\n" }
```

**Legend:**
* [s-flow-line-prefix(n)] <!-- 2:1,2 4:1,3 -->
* [s-block-line-prefix(n)] <!-- 6:1,2 7:1,2 -->
* [s-indent(n)] <!-- 2:1 4:1 6:1,2 7:1,2 -->


## 6.4. Empty Lines

An _empty line_ line consists of the non-[content] [prefix] followed by a [line
break].

y[struct.l-empty]
```
[#] l-empty(n,c) ::=
  (
      s-line-prefix(n,c)
    | s-indent-less-than(n)
  )
  b-as-line-feed
```

The semantics of empty lines depend on the [scalar style] they appear in.
This is handled on a case-by-case basis by the relevant productions.


**Example #. Empty Lines**

```
Folding:
  "Empty line
···→
  as a line feed"
Chomping: |
  Clipped empty lines
·
```

```
{ "Folding": "Empty line\nas a line feed",
  "Chomping": "Clipped empty lines\n" }
```

**Legend:**
* [l-empty(n,c)] <!-- 3 7 -->


## 6.5. Line Folding

_Line folding_ allows long lines to be broken for readability, while retaining
the semantics of the original long line.
If a [line break] is followed by an [empty line], it is _trimmed_; the first
[line break] is discarded and the rest are retained as [content].

y[struct.b-l-trimmed]
```
[#] b-l-trimmed(n,c) ::=
  b-non-content
  l-empty(n,c)+
```


Otherwise (the following line is not [empty]), the [line break] is converted to
a single [space] (`x20`).

y[struct.b-as-space]
```
[#] b-as-space ::=
  b-break
```


A folded non-[empty line] may end with either of the above [line breaks].

y[struct.b-l-folded]
```
[#] b-l-folded(n,c) ::=
  b-l-trimmed(n,c) | b-as-space
```


**Example #. Line Folding**

```
>-
  trimmed↓
··↓
·↓
↓
  as↓
  space
```

```
"trimmed\n\n\nas space"
```

**Legend:**
* [b-l-trimmed(n,c)] <!-- 2:10 3 4 5 -->
* [b-as-space] <!-- 6:5 -->


The above rules are common to both the [folded block style] and the [scalar
flow styles].
Folding does distinguish between these cases in the following way:


Block Folding
:
In the [folded block style], the final [line break] and trailing [empty lines]
are subject to [chomping] and are never folded.
In addition, folding does not apply to [line breaks] surrounding text lines
that contain leading [white space].
Note that such a [more-indented] line may consist only of such leading [white
space].
:
The combined effect of the _block line folding_ rules is that each "paragraph"
is interpreted as a line, [empty lines] are interpreted as a line feed and the
formatting of [more-indented] lines is preserved.


**Example #. Block Folding**

```
>
··foo·↓
·↓
··→·bar↓
↓
··baz↓
```

```
"foo \n\n\t bar\n\nbaz\n"
```

**Legend:**
* [b-l-folded(n,c)] <!-- 2:7 3:1,2 4:8 5:1 -->
* Non-content spaces <!-- 2:1,2 4:1,2 6:1,2 -->
* Content spaces <!-- 2:6 4:3,2 -->


Flow Folding
:
Folding in [flow styles] provides more relaxed semantics.
[Flow styles] typically depend on explicit [indicators] rather than
[indentation] to convey structure.

y[struct.flow-folding.spaces-not-content]
Hence spaces preceding or following the text in a line are a [presentation
detail] and must not be used to convey [content] information.

Once all such spaces have been discarded, all [line breaks] are folded without
exception.
:
The combined effect of the _flow line folding_ rules is that each "paragraph"
is interpreted as a line, [empty lines] are interpreted as line feeds and text
can be freely [more-indented] without affecting the [content] information.

y[struct.s-flow-folded]
```
[#] s-flow-folded(n) ::=
  s-separate-in-line?
  b-l-folded(n,FLOW-IN)
  s-flow-line-prefix(n)
```


**Example #. Flow Folding**

```
"↓
··foo·↓
·↓
··→·bar↓
↓
··baz↓ "
```

```
" foo\nbar\nbaz "
```

**Legend:**
* [s-flow-folded(n)] <!-- 1:2 2:1,2 2:6,2 3:1,2 4:1,4 4:8 5:1 6:1,2 6:6 -->
* Non-content spaces <!-- 2:1,2 2:6 3:1 4:1,4 6:1,2 -->


## 6.6. Comments

An explicit _comment_ is marked by a "`#`" indicator.

y[struct.comment.not-content]
Comments are a [presentation detail] and must not be used to convey [content]
information.

y[struct.comment.separated-by-whitespace]
Comments must be [separated] from other tokens by [white space] characters.

y[struct.comment.json-compat-final-break]
> Note: To ensure [JSON compatibility], YAML [processors] must allow for the
omission of the final comment [line break] of the input [stream].

y[struct.comment.should-terminate-with-break]
However, as this confuses many tools, YAML [processors] should terminate the
[stream] with an explicit [line break] on output.

y[struct.c-nb-comment-text]
```
[#] c-nb-comment-text ::=
  c-comment    # '#'
  nb-char*
```

y[struct.b-comment]
```
[#] b-comment ::=
    b-non-content
  | <end-of-input>
```

y[struct.s-b-comment]
```
[#] s-b-comment ::=
  (
    s-separate-in-line
    c-nb-comment-text?
  )?
  b-comment
```


**Example #. Separated Comment**

```
key:····# Comment↓
  value_eof_
```

```
{ "key": "value" }
```

**Legend:**
* [c-nb-comment-text] <!-- 1:9,9 -->
* [b-comment] <!-- ↓ 2:8,5 -->
* [s-b-comment] <!-- 1:5, 2:8,5 -->


Outside [scalar content], comments may appear on a line of their own,
independent of the [indentation] level.
Note that outside [scalar content], a line containing only [white space]
characters is taken to be a comment line.

y[struct.l-comment]
```
[#] l-comment ::=
  s-separate-in-line
  c-nb-comment-text?
  b-comment
```


**Example #. Comment Lines**

```
··# Comment↓
···↓
↓
```

```
# This stream contains no
# documents, only comments.
```

**Legend:**
* [s-b-comment] <!-- 1:3, 2:4 3 -->
* [l-comment] <!-- 1 2 3 -->


In most cases, when a line may end with a comment, YAML allows it to be
followed by additional comment lines.
The only exception is a comment ending a [block scalar header].

y[struct.s-l-comments]
```
[#] s-l-comments ::=
  (
      s-b-comment
    | <start-of-line>
  )
  l-comment*
```


**Example #. Multi-Line Comments**

```
key:····# Comment↓
········# lines↓
  value↓
↓
```

```
{ "key": "value" }
```

**Legend:**
* [s-b-comment] <!-- 1:5, 3:8 -->
* [l-comment] <!-- 2 4 -->
* [s-l-comments] <!-- 1:5, 2 3:8 4 -->


## 6.7. Separation Lines

[Implicit keys] are restricted to a single line.
In all other cases, YAML allows tokens to be separated by multi-line (possibly
empty) [comments].

y[struct.separation.indented-after-comments]
Note that structures following multi-line comment separation must be properly
[indented], even though there is no such restriction on the separation
[comment] lines themselves.

y[struct.s-separate]
```
[#]
s-separate(n,BLOCK-OUT) ::= s-separate-lines(n)
s-separate(n,BLOCK-IN)  ::= s-separate-lines(n)
s-separate(n,FLOW-OUT)  ::= s-separate-lines(n)
s-separate(n,FLOW-IN)   ::= s-separate-lines(n)
s-separate(n,BLOCK-KEY) ::= s-separate-in-line
s-separate(n,FLOW-KEY)  ::= s-separate-in-line
```

y[struct.s-separate-lines]
```
[#] s-separate-lines(n) ::=
    (
      s-l-comments
      s-flow-line-prefix(n)
    )
  | s-separate-in-line
```


**Example #. Separation Spaces**

```
{·first:·Sammy,·last:·Sosa·}:↓
# Statistics:
··hr:··# Home runs
·····65
··avg:·# Average
···0.278
```

```
{ { "first": "Sammy",
    "last": "Sosa" }: {
    "hr": 65,
    "avg": 0.278 } }
```

**Legend:**
* [s-separate-in-line] <!-- 1:2 1:9 1:16 1:22 1:27 -->
* [s-separate-lines(n)] <!-- 1:30 2 3:1,2 3:6, 4:1,5 5:7, 6:1,3 -->
* [s-indent(n)] <!-- 3:1,2 4:1,3 5:1,2 6:1,3 -->


## 6.8. Directives

_Directives_ are instructions to the YAML [processor].
This specification defines two directives, "`YAML`" and "`TAG`", and _reserves_
all other directives for future use.
There is no way to define private directives.
This is intentional.

y[struct.directive.not-content]
Directives are a [presentation detail] and must not be used to convey [content]
information.

y[struct.l-directive]
```
[#] l-directive ::=
  c-directive            # '%'
  (
      ns-yaml-directive
    | ns-tag-directive
    | ns-reserved-directive
  )
  s-l-comments
```


Each directive is specified on a separate non-[indented] line starting with the
"`%`" indicator, followed by the directive name and a list of parameters.
The semantics of these parameters depends on the specific directive.

y[struct.directive.ignore-unknown+2]
A YAML [processor] should ignore unknown directives with an appropriate
warning.

y[struct.ns-reserved-directive]
```
[#] ns-reserved-directive ::=
  ns-directive-name
  (
    s-separate-in-line
    ns-directive-parameter
  )*
```

y[struct.ns-directive-name]
```
[#] ns-directive-name ::=
  ns-char+
```

y[struct.ns-directive-parameter]
```
[#] ns-directive-parameter ::=
  ns-char+
```


**Example #. Reserved Directives**

```
%FOO  bar baz # Should be ignored
               # with a warning.
--- "foo"
```

```
"foo"
```

**Legend:**
* [ns-reserved-directive] <!-- 1:2,12 -->
* [ns-directive-name] <!-- 1:2,3 -->
* [ns-directive-parameter] <!-- 1:7,3 1:11,3 -->


### 6.8.1. "`YAML`" Directives

The "`YAML`" directive specifies the version of YAML the [document] conforms
to.
This specification defines version "`1.2`", including recommendations for _YAML
1.1 processing_.

y[struct.yaml-directive.must-accept-current]
A version 1.2 YAML [processor] must accept [documents] with an explicit "`%YAML
1.2`" directive, as well as [documents] lacking a "`YAML`" directive.
Such [documents] are assumed to conform to the 1.2 version specification.

y[struct.yaml-directive.should-warn-higher-minor]
[Documents] with a "`YAML`" directive specifying a higher minor version (e.g.
"`%YAML 1.3`") should be processed with an appropriate warning.

y[struct.yaml-directive.should-reject-higher-major]
[Documents] with a "`YAML`" directive specifying a higher major version (e.g.
"`%YAML 2.0`") should be rejected with an appropriate error message.

y[struct.yaml-directive.must-accept-prior+2]
A version 1.2 YAML [processor] must also accept [documents] with an explicit
"`%YAML 1.1`" directive.
Note that version 1.2 is mostly a superset of version 1.1, defined for the
purpose of ensuring _JSON compatibility_.

y[struct.yaml-directive.should-process-prior-as-current+2]
Hence a version 1.2 [processor] should process version 1.1 [documents] as if
they were version 1.2, giving a warning on points of incompatibility (handling
of [non-ASCII line breaks], as described [above]).

y[struct.ns-yaml-directive+2]
```
[#] ns-yaml-directive ::=
  "YAML"
  s-separate-in-line
  ns-yaml-version
```

y[struct.ns-yaml-version]
```
[#] ns-yaml-version ::=
  ns-dec-digit+
  '.'
  ns-dec-digit+
```


**Example #. "`YAML`" directive**

```
%YAML 1.3 # Attempt parsing
           # with a warning
---
"foo"
```

```
"foo"
```

**Legend:**
* [ns-yaml-directive] <!-- 1:2,8 -->
* [ns-yaml-version] <!-- 1:7,3 -->


y[struct.yaml-directive.at-most-once]
It is an error to specify more than one "`YAML`" directive for the same
document, even if both occurrences give the same version number.


**Example #. Invalid Repeated YAML directive**

```
%YAML 1.2
%YAML 1.1
foo
```
<!-- 2:2,4 -->

```
ERROR:
The YAML directive must only be
given at most once per document.
```
<!-- 2:5,4 -->


### 6.8.2. "`TAG`" Directives

The "`TAG`" directive establishes a [tag shorthand] notation for specifying
[node tags].
Each "`TAG`" directive associates a [handle] with a [prefix].
This allows for compact and readable [tag] notation.

y[struct.ns-tag-directive+2]
```
[#] ns-tag-directive ::=
  "TAG"
  s-separate-in-line
  c-tag-handle
  s-separate-in-line
  ns-tag-prefix
```


**Example #. "`TAG`" directive**

```
%TAG !yaml! tag:yaml.org,2002:
---
!yaml!str "foo"
```

```
"foo"
```

**Legend:**
* [ns-tag-directive] <!-- 1:2, -->
* [c-tag-handle] <!-- 1:6,6 -->
* [ns-tag-prefix] <!-- 1:13, -->


y[struct.tag-directive.at-most-once-per-handle+2]
It is an error to specify more than one "`TAG`" directive for the same [handle]
in the same document, even if both occurrences give the same [prefix].


**Example #. Invalid Repeated TAG directive**

```
%TAG ! !foo
%TAG ! !foo
bar
```
<!-- 2:6 -->

```
ERROR:
The TAG directive must only
be given at most once per
handle in the same document.
```
<!-- 4:1,6 -->


#### 6.8.2.1. Tag Handles

The _tag handle_ exactly matches the prefix of the affected [tag shorthand].
There are three tag handle variants:

y[struct.c-tag-handle+2]
```
[#] c-tag-handle ::=
    c-named-tag-handle
  | c-secondary-tag-handle
  | c-primary-tag-handle
```


Primary Handle
:
The _primary tag handle_ is a single "`!`" character.
This allows using the most compact possible notation for a single "primary"
name space.
By default, the prefix associated with this handle is "`!`".
Thus, by default, [shorthands] using this handle are interpreted as [local
tags].
:
It is possible to override the default behavior by providing an explicit
"`TAG`" directive, associating a different prefix for this handle.
This provides smooth migration from using [local tags] to using [global tags]
by the simple addition of a single "`TAG`" directive.

y[struct.c-primary-tag-handle+2]
```
[#] c-primary-tag-handle ::= '!'
```


**Example #. Primary Tag Handle**

```
# Private
!foo "bar"
...
# Global
%TAG ! tag:example.com,2000:app/
---
!foo "bar"
```

```
!<!foo> "bar"
---
!<tag:example.com,2000:app/foo> "bar"
```

**Legend:**
* [c-primary-tag-handle] <!-- ! -->


Secondary Handle
:
The _secondary tag handle_ is written as "`!!`".
This allows using a compact notation for a single "secondary" name space.
By default, the prefix associated with this handle is "`tag:yaml.org,2002:`".
:
It is possible to override this default behavior by providing an explicit
"`TAG`" directive associating a different prefix for this handle.

y[struct.c-secondary-tag-handle+2]
```
[#] c-secondary-tag-handle ::= "!!"
```


**Example #. Secondary Tag Handle**

```
%TAG !! tag:example.com,2000:app/
---
!!int 1 - 3 # Interval, not integer
```

```
!<tag:example.com,2000:app/int> "1 - 3"
```

**Legend:**
* [c-secondary-tag-handle] <!-- !! -->


Named Handles
:
A _named tag handle_ surrounds a non-empty name with "`!`" characters.

y[struct.named-tag-handle.must-be-declared+2]
A handle name must not be used in a [tag shorthand] unless an explicit "`TAG`"
directive has associated some prefix with it.

y[struct.named-tag-handle.not-content+2]
:
The name of the handle is a [presentation detail] and must not be used to
convey [content] information.
In particular, the YAML [processor] need not preserve the handle name once
[parsing] is completed.

y[struct.c-named-tag-handle+2]
```
[#] c-named-tag-handle ::=
  c-tag            # '!'
  ns-word-char+
  c-tag            # '!'
```


**Example #. Tag Handles**

```
%TAG !e! tag:example.com,2000:app/
---
!e!foo "bar"
```

```
!<tag:example.com,2000:app/foo> "bar"
```

**Legend:**
* [c-named-tag-handle] <!-- !e! -->


#### 6.8.2.2. Tag Prefixes

There are two _tag prefix_ variants:

y[struct.ns-tag-prefix+2]
```
[#] ns-tag-prefix ::=
  c-ns-local-tag-prefix | ns-global-tag-prefix
```


Local Tag Prefix
:
If the prefix begins with a "`!`" character, [shorthands] using the [handle]
are expanded to a [local tag].
Note that such a [tag] is intentionally not a valid URI and its semantics are
specific to the [application].
In particular, two [documents] in the same [stream] may assign different
semantics to the same [local tag].

y[struct.c-ns-local-tag-prefix+2]
```
[#] c-ns-local-tag-prefix ::=
  c-tag           # '!'
  ns-uri-char*
```


**Example #. Local Tag Prefix**

```
%TAG !m! !my-
--- # Bulb here
!m!light fluorescent
...
%TAG !m! !my-
--- # Color here
!m!light green
```

```
!<!my-light> "fluorescent"
---
!<!my-light> "green"
```

**Legend:**
* [c-ns-local-tag-prefix] <!-- !my- -->


Global Tag Prefix
:

y[struct.global-tag-prefix.must-be-valid-uri]
If the prefix begins with a character other than "`!`", it must be a valid URI
prefix, and should contain at least the scheme.

[Shorthands] using the associated [handle] are expanded to globally unique URI
tags and their semantics is consistent across [applications].

y[struct.global-tag-prefix.same-semantics]
In particular, every [document] in every [stream] must assign the same
semantics to the same [global tag].

y[struct.ns-global-tag-prefix+2]
```
[#] ns-global-tag-prefix ::=
  ns-tag-char
  ns-uri-char*
```


**Example #. Global Tag Prefix**

```
%TAG !e! tag:example.com,2000:app/
---
- !e!foo "bar"
```

```
- !<tag:example.com,2000:app/foo> "bar"
```

**Legend:**
* [ns-global-tag-prefix] <!-- tag:example.com,2000:app/ -->


## 6.9. Node Properties

Each [node] may have two optional _properties_, [anchor] and [tag], in addition
to its [content].
Node properties may be specified in any order before the [node's content].
Either or both may be omitted.

y[struct.c-ns-properties]
```
[#] c-ns-properties(n,c) ::=
    (
      c-ns-tag-property
      (
        s-separate(n,c)
        c-ns-anchor-property
      )?
    )
  | (
      c-ns-anchor-property
      (
        s-separate(n,c)
        c-ns-tag-property
      )?
    )
```


**Example #. Node Properties**

```
!!str &a1 "foo":
  !!str bar
&a2 baz : *a1
```

```
{ &B1 "foo": "bar",
  "baz": *B1 }
```

**Legend:**
* [c-ns-properties(n,c)] <!-- 1:1,9 2:3,5 3:1,3 -->
* [c-ns-anchor-property] <!-- 1:7,3 3:1,3 -->
* [c-ns-tag-property] <!-- 1:1,5 2:3,5 -->


### 6.9.1. Node Tags

The _tag property_ identifies the type of the [native data structure]
[presented] by the [node].
A tag is denoted by the "`!`" indicator.

y[struct.c-ns-tag-property+2]
```
[#] c-ns-tag-property ::=
    c-verbatim-tag
  | c-ns-shorthand-tag
  | c-non-specific-tag
```


Verbatim Tags
:
A tag may be written _verbatim_ by surrounding it with the "`<`" and "`>`"
characters.

y[struct.verbatim-tag.deliver-as-is+2]
In this case, the YAML [processor] must deliver the verbatim tag as-is to the
[application].
In particular, verbatim tags are not subject to [tag resolution].

y[struct.verbatim-tag.must-be-local-or-uri+2]
A verbatim tag must either begin with a "`!`" (a [local tag]) or be a valid URI
(a [global tag]).

y[struct.c-verbatim-tag+2]
```
[#] c-verbatim-tag ::=
  "!<"
  ns-uri-char+
  '>'
```


**Example #. Verbatim Tags**

```
!<tag:yaml.org,2002:str> foo :
  !<!bar> baz
```

```
{ "foo": !<!bar> "baz" }
```

**Legend:**
* [c-verbatim-tag] <!-- !<tag:yaml.org,2002:str> !<!bar> -->


**Example #. Invalid Verbatim Tags**

```
- !<!> foo
- !<$:?> bar
```
<!-- 1:5 -->
<!-- 2:5,3 -->

```
ERROR:
- Verbatim tags aren't resolved,
  so ! is invalid.
- The $:? tag is neither a global
  URI tag nor a local tag starting
  with '!'.
```
<!-- 3:6 -->
<!-- 4:7,3 -->


Tag Shorthands
:
A _tag shorthand_ consists of a valid [tag handle] followed by a non-empty
suffix.

y[struct.shorthand-tag.handle-must-have-prefix+2]
The [tag handle] must be associated with a [prefix], either by default or by
using a "`TAG`" directive.

y[struct.shorthand-tag.result-must-be-local-or-uri+2]
The resulting [parsed] [tag] is the concatenation of the [prefix] and the
suffix and must either begin with "`!`" (a [local tag]) or be a valid URI (a
[global tag]).

y[struct.shorthand-tag.handle-not-content+2]
:
The choice of [tag handle] is a [presentation detail] and must not be used to
convey [content] information.
In particular, the [tag handle] may be discarded once [parsing] is completed.

y[struct.shorthand-tag.suffix-no-bang+2]
:
The suffix must not contain any "`!`" character.
This would cause the tag shorthand to be interpreted as having a [named tag
handle].

y[struct.shorthand-tag.suffix-no-flow-chars+3]
In addition, the suffix must not contain the "`[`", "`]`", "`{`", "`}`" and
"`,`" characters.
These characters would cause ambiguity with [flow collection] structures.

y[struct.shorthand-tag.suffix-escape+2]
If the suffix needs to specify any of the above restricted characters, they
must be [escaped] using the "`%`" character.
This behavior is consistent with the URI character escaping rules
(specifically, section 2.3 of URI RFC).

y[struct.c-ns-shorthand-tag+2]
```
[#] c-ns-shorthand-tag ::=
  c-tag-handle
  ns-tag-char+
```


**Example #. Tag Shorthands**

```
%TAG !e! tag:example.com,2000:app/
---
- !local foo
- !!str bar
- !e!tag%21 baz
```

```
[ !<!local> "foo",
  !<tag:yaml.org,2002:str> "bar",
  !<tag:example.com,2000:app/tag!> "baz" ]
```

**Legend:**
* [c-ns-shorthand-tag] <!-- !local !!str !e!tag%21 -->


**Example #. Invalid Tag Shorthands**

```
%TAG !e! tag:example,2000:app/
---
- !e! foo
- !h!bar baz
```
<!-- 3:3,3 -->
<!-- 4:3,3 -->

```
ERROR:
- The !e! handle has no suffix.
- The !h! handle wasn't declared.
```
<!-- 2:7,3 -->
<!-- 3:7,3 -->


Non-Specific Tags
:
If a [node] has no tag property, it is assigned a [non-specific tag] that needs
to be [resolved] to a [specific] one.
This [non-specific tag] is "`!`" for non-[plain scalars] and "`?`" for all
other [nodes].
This is the only case where the [node style] has any effect on the [content]
information.
:
It is possible for the tag property to be explicitly set to the "`!`"
non-specific tag.
By [convention], this "disables" [tag resolution], forcing the [node] to be
interpreted as "`tag:yaml.org,2002:seq`", "`tag:yaml.org,2002:map`" or
"`tag:yaml.org,2002:str`", according to its [kind].
:
There is no way to explicitly specify the "`?`" non-specific tag.
This is intentional.

y[struct.c-non-specific-tag+2]
```
[#] c-non-specific-tag ::= '!'
```


**Example #. Non-Specific Tags**

```
# Assuming conventional resolution:
- "12"
- 12
- ! 12
```

```
[ "12",
  12,
  "12" ]
```

**Legend:**
* [c-non-specific-tag] <!-- ! -->


### 6.9.2. Node Anchors

An anchor is denoted by the "`&`" indicator.
It marks a [node] for future reference.
An [alias node] can then be used to indicate additional inclusions of the
anchored [node].
An anchored [node] need not be referenced by any [alias nodes]; in particular,
it is valid for all [nodes] to be anchored.

y[struct.c-ns-anchor-property]
```
[#] c-ns-anchor-property ::=
  c-anchor          # '&'
  ns-anchor-name
```


Note that as a [serialization detail], the anchor name is preserved in the
[serialization tree].

y[struct.anchor.not-content]
However, it is not reflected in the [representation] graph and must not be used
to convey [content] information.
In particular, the YAML [processor] need not preserve the anchor name once the
[representation] is [composed].

y[struct.anchor.no-flow-chars+3]
Anchor names must not contain the "`[`", "`]`", "`{`", "`}`" and "`,`"
characters.
These characters would cause ambiguity with [flow collection] structures.

y[struct.ns-anchor-char+3]
```
[#] ns-anchor-char ::=
    ns-char - c-flow-indicator
```

y[struct.ns-anchor-name]
```
[#] ns-anchor-name ::=
  ns-anchor-char+
```


**Example #. Node Anchors**

```
First occurrence: &anchor Value
Second occurrence: *anchor
```

```
{ "First occurrence": &A "Value",
  "Second occurrence": *A }
```

**Legend:**
* [c-ns-anchor-property] <!-- 1:19,7 -->
* [ns-anchor-name] <!-- 1:20,6 2:21,6 -->
