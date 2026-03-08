# Chapters 3-4. Processes, Models, and Syntax Conventions

> Marked-up copy of YAML 1.2.2 specification Chapters 3-4, with tracey requirement markers.
> Focus on data model definitions and normative requirements.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

## 3.1. Processes

Translating between [native data structures] and a character [stream] is done
in several logically distinct stages, each with a well defined input and output
data model, as shown in the following diagram:

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

A YAML processor need not expose the [serialization] or [representation]
stages.
It may translate directly between [native data structures] and a character
[stream] ([dump] and [load] in the diagram above).
However, such a direct translation should take place so that the [native data
structures] are [constructed] only from information available in the
[representation].
In particular, [mapping key order], [comments] and [tag handles] should not be
referenced during [construction].


### 3.1.1. Dump

_Dumping_ native data structures to a character [stream] is done using the
following three stages:

Representing Native Data Structures
:
YAML _represents_ any _native data structure_ using three [node kinds]:
[sequence] - an ordered series of entries; [mapping] - an unordered association
of [unique] [keys] to [values]; and [scalar] - any datum with opaque structure
presentable as a series of Unicode characters.

Each YAML [node] requires, in addition to its [kind] and [content], a [tag]
specifying its data type.
Type specifiers are either [global] URIs or are [local] in scope to a single
[application].

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

Serializing the Representation Graph
:
For sequential access mediums, such as an event callback API, a YAML
[representation] must be _serialized_ to an ordered tree.
Since in a YAML [representation], [mapping keys] are unordered and [nodes] may
be referenced more than once (have more than one incoming "arrow"), the
serialization process is required to impose an [ordering] on the [mapping keys]
and to replace the second and subsequent references to a given [node] with
place holders called [aliases].
YAML does not specify how these _serialization details_ are chosen.
It is up to the YAML [processor] to come up with human-friendly [key order] and
[anchor] names, possibly with the help of the [application].
The result of this process, a YAML [serialization tree], can then be traversed
to produce a series of event calls for one-pass processing of YAML data.

Presenting the Serialization Tree
:
The final output process is _presenting_ the YAML [serializations] as a
character [stream] in a human-friendly manner.
To maximize human readability, YAML offers a rich set of stylistic options
which go far beyond the minimal functional needs of simple data storage.
Therefore the YAML [processor] is required to introduce various _presentation
details_ when creating the [stream], such as the choice of [node styles], how
to [format scalar content], the amount of [indentation], which [tag handles] to
use, the [node tags] to leave [unspecified], the set of [directives] to provide
and possibly even what [comments] to add.
While some of this can be done with the help of the [application], in general
this process should be guided by the preferences of the user.


### 3.1.2. Load

_Loading_ [native data structures] from a character [stream] is done using the
following three stages:

Parsing the Presentation Stream
:
_Parsing_ is the inverse process of [presentation], it takes a [stream] of
characters and produces a [serialization tree].
Parsing discards all the [details] introduced in the [presentation] process,
reporting only the [serialization tree].
Parsing can fail due to [ill-formed] input.

Composing the Representation Graph
:
_Composing_ takes a [serialization tree] and produces a [representation graph].
Composing discards all the [details] introduced in the [serialization] process,
producing only the [representation graph].
Composing can fail due to any of several reasons, detailed [below].

Constructing Native Data Structures
:
The final input process is _constructing_ [native data structures] from the
YAML [representation].
Construction must be based only on the information available in the
[representation] and not on additional [serialization] or [presentation
details] such as [comments], [directives], [mapping key order], [node styles],
[scalar content format], [indentation] levels etc.
Construction can fail due to the [unavailability] of the required [native data
types].


## 3.2. Information Models

This section specifies the formal details of the results of the above
processes.
To maximize data portability between programming languages and implementations,
users of YAML should be mindful of the distinction between [serialization] or
[presentation] properties and those which are part of the YAML
[representation].

Thus, while imposing a [order] on [mapping keys] is necessary for flattening
YAML [representations] to a sequential access medium, this [serialization
detail] must not be used to convey [application] level information.

In a similar manner, while [indentation] technique and a choice of a [node
style] are needed for the human readability, these [presentation details] are
neither part of the YAML [serialization] nor the YAML [representation].

By carefully separating properties needed for [serialization] and
[presentation], YAML [representations] of [application] information will be
consistent and portable between various programming environments.

<!-- Figure: Information Models omitted — see vendor/yaml-spec/spec-1.2.2.md -->


### 3.2.1. Representation Graph

YAML's _representation_ of [native data structure] is a rooted, connected,
directed graph of [tagged] [nodes].
By "directed graph" we mean a set of [nodes] and directed edges ("arrows"),
where each edge connects one [node] to another (see a formal directed graph
definition[^digraph]).
All the [nodes] must be reachable from the _root node_ via such edges.
Note that the YAML graph may include cycles and a [node] may have more than one
incoming edge.

[Nodes] that are defined in terms of other [nodes] are [collections]; [nodes]
that are independent of any other [nodes] are [scalars].
YAML supports two [kinds] of [collection nodes]: [sequences] and [mappings].
[Mapping nodes] are somewhat tricky because their [keys] are unordered and must
be [unique].

<!-- Figure: Representation Model omitted — see vendor/yaml-spec/spec-1.2.2.md -->


#### 3.2.1.1. Nodes

A YAML _node_ [represents] a single [native data structure].
Such nodes have _content_ of one of three _kinds_: scalar, sequence or mapping.
In addition, each node has a [tag] which serves to restrict the set of possible
values the content can have.

Scalar
:
The content of a _scalar_ node is an opaque datum that can be [presented] as a
series of zero or more Unicode characters.

Sequence
:
The content of a _sequence_ node is an ordered series of zero or more nodes.
In particular, a sequence may contain the same node more than once.
It could even contain itself.

Mapping
:
The content of a _mapping_ node is an unordered set of _key/value_ node
_pairs_, with the restriction that each of the keys is [unique].
YAML places no further restrictions on the nodes.
In particular, keys may be arbitrary nodes, the same node may be used as the
value of several key/value pairs and a mapping could even contain itself as a
key or a value.


#### 3.2.1.2. Tags

YAML [represents] type information of [native data structures] with a simple
identifier, called a _tag_.

_Global tags_ are URIs and hence globally unique across all [applications].
The "`tag:`" URI scheme[^tag-uri] is recommended for all global YAML tags.

In contrast, _local tags_ are specific to a single [application].
Local tags start with "`!`", are not URIs and are not expected to be globally
unique.

YAML provides a "`TAG`" directive to make tag notation less verbose; it also
offers easy migration from local to global tags.
To ensure this, local tags are restricted to the URI character set and use URI
character [escaping].

YAML does not mandate any special relationship between different tags that
begin with the same substring.
Tags ending with URI fragments (containing "`#`") are no exception; tags that
share the same base URI but differ in their fragment part are considered to be
different, independent tags.

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

YAML tags are used to associate meta information with each [node].
In particular, each tag must specify the expected [node kind] ([scalar],
[sequence] or [mapping]).

[Scalar] tags must also provide a mechanism for converting [formatted content]
to a [canonical form] for supporting [equality] testing.

Furthermore, a tag may provide additional information such as the set of
allowed [content] values for validation, a mechanism for [tag resolution] or
any other data that is applicable to all of the tag's [nodes].


#### 3.2.1.3. Node Comparison

Since YAML [mappings] require [key] uniqueness, [representations] must include
a mechanism for testing the equality of [nodes].
This is non-trivial since YAML allows various ways to [format scalar content].

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

Canonical Form
:
YAML supports the need for [scalar] equality by requiring that every [scalar]
[tag] must specify a mechanism for producing the _canonical form_ of any
[formatted content].
This form is a Unicode character string which also [presents] the same
[content] and can be used for equality testing.

Equality
:
Two [nodes] must have the same [tag] and [content] to be _equal_.
Since each [tag] applies to exactly one [kind], this implies that the two
[nodes] must have the same [kind] to be equal.

Two [scalars] are equal only when their [tags] and canonical forms are equal
character-by-character.

Equality of [collections] is defined recursively.

Two [sequences] are equal only when they have the same [tag] and length and
each [node] in one [sequence] is equal to the corresponding [node] in the other
[sequence].

Two [mappings] are equal only when they have the same [tag] and an equal set of
[keys] and each [key] in this set is associated with equal [values] in both
[mappings].

Different URI schemes may define different rules for testing the equality of
URIs.
Since a YAML [processor] cannot be reasonably expected to be aware of them all,
it must resort to a simple character-by-character comparison of [tags] to
ensure consistency.
This also happens to be the comparison method defined by the "`tag:`" URI
scheme.
[Tags] in a YAML stream must therefore be [presented] in a canonical way so
that such comparison would yield the correct results.

If a node has itself as a descendant (via an alias), then determining the
equality of that node is implementation-defined.

A YAML [processor] may treat equal [scalars] as if they were identical.

Uniqueness
:
A [mapping's] [keys] are _unique_ if no two keys are equal to each other.
Obviously, identical nodes are always considered equal.


### 3.2.2. Serialization Tree

To express a YAML [representation] using a serial API, it is necessary to
impose an [order] on [mapping keys] and employ [alias nodes] to indicate a
subsequent occurrence of a previously encountered [node].
The result of this process is a _serialization tree_, where each [node] has an
ordered set of children.
This tree can be traversed for a serial event-based API.

[Construction] of [native data structures] from the serial interface should not
use [key order] or [anchor names] for the preservation of [application] data.

<!-- Figure: Serialization Model omitted — see vendor/yaml-spec/spec-1.2.2.md -->


#### 3.2.2.1. Mapping Key Order

In the [representation] model, [mapping keys] do not have an order.

To [serialize] a [mapping], it is necessary to impose an _ordering_ on its
[keys].
This order is a [serialization detail] and should not be used when [composing]
the [representation graph] (and hence for the preservation of [application]
data).

In every case where [node] order is significant, a [sequence] must be used.
For example, an ordered [mapping] can be [represented] as a [sequence] of
[mappings], where each [mapping] is a single [key/value pair].
YAML provides convenient [compact notation] for this case.


#### 3.2.2.2. Anchors and Aliases

In the [representation graph], a [node] may appear in more than one
[collection].
When [serializing] such data, the first occurrence of the [node] is
_identified_ by an _anchor_.
Each subsequent occurrence is [serialized] as an [alias node] which refers back
to this anchor.

Otherwise, anchor names are a [serialization detail] and are discarded once
[composing] is completed.

When [composing] a [representation graph] from [serialized] events, an alias
event refers to the most recent event in the [serialization] having the
specified anchor.
Therefore, anchors need not be unique within a [serialization].

In addition, an anchor need not have an alias node referring to it.


### 3.2.3. Presentation Stream

A YAML _presentation_ is a [stream] of Unicode characters making use of
[styles], [scalar content formats], [comments], [directives] and other
[presentation details] to [present] a YAML [serialization] in a human readable
way.

YAML allows several [serialization trees] to be contained in the same YAML
presentation stream, as a series of [documents] separated by [markers].

<!-- Figure: Presentation Model omitted — see vendor/yaml-spec/spec-1.2.2.md -->


#### 3.2.3.1. Node Styles

Each [node] is presented in some _style_, depending on its [kind].
The node style is a [presentation detail] and is not reflected in the
[serialization tree] or [representation graph].

There are two groups of styles.
[Block styles] use [indentation] to denote structure.
In contrast, [flow styles] rely on explicit [indicators].

YAML provides a rich set of _scalar styles_.
[Block scalar] styles include the [literal style] and the [folded style].
[Flow scalar] styles include the [plain style] and two quoted styles, the
[single-quoted style] and the [double-quoted style].
These styles offer a range of trade-offs between expressive power and
readability.

Normally, [block sequences] and [mappings] begin on the next line.
In some cases, YAML also allows nested [block] [collections] to start in-line
for a more [compact notation].
In addition, YAML provides a [compact notation] for [flow mappings] with a
single [key/value pair], nested inside a [flow sequence].
These allow for a natural "ordered mapping" notation.

<!-- Figure: Kind/Style Combinations omitted — see vendor/yaml-spec/spec-1.2.2.md -->


#### 3.2.3.2. Scalar Formats

YAML allows [scalars] to be [presented] in several _formats_.
For example, the integer "`11`" might also be written as "`0xB`".

[Tags] must specify a mechanism for converting the formatted content to a
[canonical form] for use in [equality] testing.

Like [node style], the format is a [presentation detail] and is not reflected
in the [serialization tree] and [representation graph].


#### 3.2.3.3. Comments

[Comments] are a [presentation detail] and must not have any effect on the
[serialization tree] or [representation graph].
In particular, comments are not associated with a particular [node].

The usual purpose of a comment is to communicate between the human maintainers
of a file.
A typical example is comments in a configuration file.

Comments must not appear inside [scalars], but may be interleaved with such
[scalars] inside [collections].


#### 3.2.3.4. Directives

Each [document] may be associated with a set of [directives].
A directive has a name and an optional sequence of parameters.

Directives are instructions to the YAML [processor] and like all other
[presentation details] are not reflected in the YAML [serialization tree] or
[representation graph].

This version of YAML defines two directives, "`YAML`" and "`TAG`".
All other directives are [reserved] for future versions of YAML.


## 3.3. Loading Failure Points

The process of [loading] [native data structures] from a YAML [stream] has
several potential _failure points_.
The character [stream] may be [ill-formed], [aliases] may be [unidentified],
[unspecified tags] may be [unresolvable], [tags] may be [unrecognized], the
[content] may be [invalid], [mapping] [keys] may not be [unique] and a native
type may be [unavailable].
Each of these failures results with an incomplete loading.

A _partial representation_ need not [resolve] the [tag] of each [node] and the
[canonical form] of [formatted scalar content] need not be available.
This weaker representation is useful for cases of incomplete knowledge of the
types used in the [document].

In contrast, a _complete representation_ specifies the [tag] of each [node] and
provides the [canonical form] of [formatted scalar content], allowing for
[equality] testing.
A complete representation is required in order to [construct] [native data
structures].

<!-- Figure: Loading Failure Points omitted — see vendor/yaml-spec/spec-1.2.2.md -->


### 3.3.1. Well-Formed Streams and Identified Aliases

A [well-formed] character [stream] must match the BNF productions specified in
the following chapters.

Successful loading also requires that each [alias] shall refer to a previous
[node] [identified] by the [anchor].

A YAML [processor] should reject _ill-formed streams_ and _unidentified
aliases_.

A YAML [processor] may recover from syntax errors, possibly by ignoring certain
parts of the input, but it must provide a mechanism for reporting such errors.


### 3.3.2. Resolved Tags

Typically, most [tags] are not explicitly specified in the character [stream].
During [parsing], [nodes] lacking an explicit [tag] are given a _non-specific
tag_: "`!`" for non-[plain scalars] and "`?`" for all other [nodes].

[Composing] a [complete representation] requires each such non-specific tag to
be _resolved_ to a _specific tag_, be it a [global tag] or a [local tag].

Resolving the [tag] of a [node] must only depend on the following three
parameters: (1) the non-specific tag of the [node], (2) the path leading from
the [root] to the [node] and (3) the [content] (and hence the [kind]) of the
[node].

When a [node] has more than one occurrence (using [aliases]), tag resolution
must depend only on the path to the first ([anchored]) occurrence of the
[node].

Note that resolution must not consider [presentation details] such as
[comments], [indentation] and [node style].

Also, resolution must not consider the [content] of any other [node], except
for the [content] of the [key nodes] directly along the path leading from the
[root] to the resolved [node].

Finally, resolution must not consider the [content] of a sibling [node] in a
[collection] or the [content] of the [value node] associated with a [key node]
being resolved.

These rules ensure that tag resolution can be performed as soon as a [node] is
first encountered in the [stream], typically before its [content] is [parsed].
Also, tag resolution only requires referring to a relatively small number of
previously parsed [nodes].
Thus, in most cases, tag resolution in one-pass [processors] is both possible
and practical.

YAML [processors] should resolve [nodes] having the "`!`" non-specific tag as
"`tag:yaml.org,2002:seq`", "`tag:yaml.org,2002:map`" or
"`tag:yaml.org,2002:str`" depending on their [kind].
This _tag resolution convention_ allows the author of a YAML character [stream]
to effectively "disable" the tag resolution process.
By explicitly specifying a "`!`" non-specific [tag property], the [node] would
then be resolved to a "vanilla" [sequence], [mapping] or string, according to
its [kind].

[Application] specific tag resolution rules should be restricted to resolving
the "`?`" non-specific tag, most commonly to resolving [plain scalars].
These may be matched against a set of regular expressions to provide automatic
resolution of integers, floats, timestamps and similar types.
An [application] may also match the [content] of [mapping nodes] against sets
of expected [keys] to automatically resolve points, complex numbers and similar
types.
Resolved [sequence node] types such as the "ordered mapping" are also possible.

That said, tag resolution is specific to the [application].
YAML [processors] should therefore provide a mechanism allowing the
[application] to override and expand these default tag resolution rules.

If a [document] contains _unresolved tags_, the YAML [processor] is unable to
[compose] a [complete representation] graph.
In such a case, the YAML [processor] may [compose] a [partial representation],
based on each [node's kind] and allowing for non-specific tags.


### 3.3.3. Recognized and Valid Tags

To be _valid_, a [node] must have a [tag] which is _recognized_ by the YAML
[processor] and its [content] must satisfy the constraints imposed by this
[tag].

If a [document] contains a [scalar node] with an _unrecognized tag_ or _invalid
content_, only a [partial representation] may be [composed].

In contrast, a YAML [processor] can always [compose] a [complete
representation] for an unrecognized or an invalid [collection], since
[collection] [equality] does not depend upon knowledge of the [collection's]
data type.
However, such a [complete representation] cannot be used to [construct] a
[native data structure].


### 3.3.4. Available Tags

In a given processing environment, there need not be an _available_ native type
corresponding to a given [tag].
If a [node's tag] is _unavailable_, a YAML [processor] will not be able to
[construct] a [native data structure] for it.
In this case, a [complete representation] may still be [composed] and an
[application] may wish to use this [representation] directly.


# Chapter 4. Syntax Conventions

The following chapters formally define the syntax of YAML character [streams],
using parameterized BNF productions.
Each BNF production is both named and numbered for easy reference.
Whenever possible, basic structures are specified before the more complex
structures using them in a "bottom up" fashion.

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 4.1. Production Syntax

Productions are defined using the syntax `production-name ::= term`, where a
term is either:

An atomic term
:
* A quoted string (`"abc"`), which matches that concatenation of characters. A
  single character is usually written with single quotes (`'a'`).
* A hexadecimal number (`x0A`), which matches the character at that Unicode
  code point.
* A range of hexadecimal numbers (`[x20-x7E]`), which matches any character
  whose Unicode code point is within that range.
* The name of a production (`c-printable`), which matches that production.

A lookaround
:
* `[ lookahead = term ]`, which matches the empty string if `term` would match.
* `[ lookahead ≠ term ]`, which matches the empty string if `term` would not
  match.
* `[ lookbehind = term ]`, which matches the empty string if `term` would match
  beginning at any prior point on the line and ending at the current position.

A special production
:
* `<start-of-line>`, which matches the empty string at the beginning of a line.
* `<end-of-input>`, matches the empty string at the end of the input.
* `<empty>`, which (always) matches the empty string.

A parenthesized term
:
Matches its contents.

A concatenation
:
Is `term-one term-two`, which matches `term-one` followed by `term-two`.

A alternation
:
Is `term-one | term-two`, which matches the `term-one` if possible, or
`term-two` otherwise.

A quantified term:
:
* `term?`, which matches `(term | <empty>)`.
* `term*`, which matches `(term term* | <empty>)`.
* `term+`, which matches `(term term*)`.

> Note: Quantified terms are always greedy.

The order of precedence is parenthesization, then quantification, then
concatenation, then alternation.

Some lines in a production definition might have a comment like:

```
production-a ::=
  production-b      # clarifying comment
```

These comments are meant to be informative only.
For instance a comment that says `# not followed by non-ws char` just means
that you should be aware that actual production rules will behave as described
even though it might not be obvious from the content of that particular
production alone.


## 4.2. Production Parameters

Some productions have parameters in parentheses after the name, such as
[`s-line-prefix(n,c)`](#rule-s-line-prefix).
A parameterized production is shorthand for a (infinite) series of productions,
each with a fixed value for each parameter.

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

The parameters are as follows:

Indentation: `n` or `m`
:
May be any natural number, including zero. `n` may also be -1.

Context: `c`
:
This parameter allows productions to tweak their behavior according to their
surrounding.
YAML supports two groups of _contexts_, distinguishing between [block styles]
and [flow styles].
:
May be any of the following values:
:
* `BLOCK-IN` -- inside block context
* `BLOCK-OUT` -- outside block context
* `BLOCK-KEY` -- inside block key context
* `FLOW-IN` -- inside flow context
* `FLOW-OUT` -- outside flow context
* `FLOW-KEY` -- inside flow key context

(Block) Chomping: `t`
:
The [line break] chomping behavior for flow scalars.
May be any of the following values:

* `STRIP` -- remove all trailing newlines
* `CLIP` -- remove all trailing newlines except the first
* `KEEP` -- retain all trailing newlines


## 4.3. Production Naming Conventions

To make it easier to follow production combinations, production names use a
prefix-style naming convention.
Each production is given a prefix based on the type of characters it begins and
ends with.

`e-`
:
A production matching no characters.

`c-`
:
A production starting and ending with a special character.

`b-`
:
A production matching a single [line break].

`nb-`
:
A production starting and ending with a non-[break] character.

`s-`
:
A production starting and ending with a [white space] character.

`ns-`
:
A production starting and ending with a non-[space] character.

`l-`
:
A production matching complete line(s).

`X-Y-`
:
A production starting with an `X-` character and ending with a `Y-` character,
where `X-` and `Y-` are any of the above prefixes.

`X+`, `X-Y+`
:
A production as above, with the additional property that the matched content
[indentation] level is greater than the specified `n` parameter.
