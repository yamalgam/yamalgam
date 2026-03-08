# Chapter 9. Document Stream Productions

> Marked-up copy of YAML 1.2.2 specification Chapter 9, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

## 9.1. Documents

A YAML character [stream] may contain several _documents_.
Each document is completely independent from the rest.


### 9.1.1. Document Prefix

A document may be preceded by a _prefix_ specifying the [character encoding]
and optional [comment] lines.

Note that all [documents] in a stream must use the same [character encoding].
However it is valid to re-specify the [encoding] using a [byte order mark] for
each [document] in the stream.

The existence of the optional prefix does not necessarily indicate the
existence of an actual [document].

```
[#] l-document-prefix ::=
  c-byte-order-mark?
  l-comment*
```


**Example #. Document Prefix**

```
⇔# Comment
# lines
Document
```

```
"Document"
```

**Legend:**
* [l-document-prefix] <!-- 1 2 -->


### 9.1.2. Document Markers

Using [directives] creates a potential ambiguity.
It is valid to have a "`%`" character at the start of a line (e.g. as the first
character of the second line of a [plain scalar]).
How, then, to distinguish between an actual [directive] and a [content] line
that happens to start with a "`%`" character?

The solution is the use of two special _marker_ lines to control the processing
of [directives], one at the start of a [document] and one at the end.

At the start of a [document], lines beginning with a "`%`" character are
assumed to be [directives].
The (possibly empty) list of [directives] is terminated by a _directives end
marker_ line.
Lines following this marker can safely use "`%`" as the first character.

At the end of a [document], a _document end marker_ line is used to signal the
[parser] to begin scanning for [directives] again.

The existence of this optional _document suffix_ does not necessarily indicate
the existence of an actual following [document].

Obviously, the actual [content] lines are therefore forbidden to begin with
either of these markers.

```
[#] c-directives-end ::= "---"
```

```
[#] c-document-end ::=
  "..."    # (not followed by non-ws char)
```

```
[#] l-document-suffix ::=
  c-document-end
  s-l-comments
```

```
[#] c-forbidden ::=
  <start-of-line>
  (
      c-directives-end
    | c-document-end
  )
  (
      b-char
    | s-white
    | <end-of-input>
  )
```


**Example #. Document Markers**

```
%YAML 1.2
---
Document
... # Suffix
```

```
"Document"
```

**Legend:**
* [c-directives-end] <!-- 2 -->
* [l-document-suffix] <!-- 4 -->
* [c-document-end] <!-- 4:1,3 -->


### 9.1.3. Bare Documents

A _bare document_ does not begin with any [directives] or [marker] lines.
Such documents are very "clean" as they contain nothing other than the
[content].

In this case, the first non-comment line may not start with a "`%`" first
character.

Document [nodes] are [indented] as if they have a parent [indented] at -1
[spaces].
Since a [node] must be more [indented] than its parent [node], this allows the
document's [node] to be [indented] at zero or more [spaces].

```
[#] l-bare-document ::=
  s-l+block-node(-1,BLOCK-IN)
  /* Excluding c-forbidden content */
```


**Example #. Bare Documents**

```
Bare
document
...
# No document
...
|
%!PS-Adobe-2.0 # Not the first line
```

```
"Bare document"
---
"%!PS-Adobe-2.0\n"
```

**Legend:**
* [l-bare-document] <!-- 1 2 6 7 -->


### 9.1.4. Explicit Documents

An _explicit document_ begins with an explicit [directives end marker] line but
no [directives].
Since the existence of the [document] is indicated by this [marker], the
[document] itself may be [completely empty].

```
[#] l-explicit-document ::=
  c-directives-end
  (
      l-bare-document
    | (
        e-node    # ""
        s-l-comments
      )
  )
```


**Example #. Explicit Documents**

```
---
{ matches
% : 20 }
...
---
# Empty
...
```

```
{ "matches %": 20 }
---
null
```

**Legend:**
* [l-explicit-document] <!-- 1 2 3 5 6 -->


### 9.1.5. Directives Documents

A _directives document_ begins with some [directives] followed by an explicit
[directives end marker] line.

```
[#] l-directive-document ::=
  l-directive+
  l-explicit-document
```


**Example #. Directives Documents**

```
%YAML 1.2
--- |
%!PS-Adobe-2.0
...
%YAML 1.2
---
# Empty
...
```

```
"%!PS-Adobe-2.0\n"
---
null
```

**Legend:**
* [l-explicit-document] <!-- 1 2 3 5 6 7 -->


## 9.2. Streams

A YAML _stream_ consists of zero or more [documents].

Subsequent [documents] require some sort of separation [marker] line.
If a [document] is not terminated by a [document end marker] line, then the
following [document] must begin with a [directives end marker] line.

```
[#] l-any-document ::=
    l-directive-document
  | l-explicit-document
  | l-bare-document
```

```
[#] l-yaml-stream ::=
  l-document-prefix*
  l-any-document?
  (
      (
        l-document-suffix+
        l-document-prefix*
        l-any-document?
      )
    | c-byte-order-mark
    | l-comment
    | l-explicit-document
  )*
```


**Example #. Stream**

```
Document
---
# Empty
...
%YAML 1.2
---
matches %: 20
```

```
"Document"
---
null
---
{ "matches %": 20 }
```

**Legend:**
* [l-any-document] <!-- 1 2 3 -->
* [l-document-suffix] <!-- 4 -->
* [l-explicit-document] <!-- 5 6 7 -->


A sequence of bytes is a _well-formed stream_ if, taken as a whole, it complies
with the above `l-yaml-stream` production.
