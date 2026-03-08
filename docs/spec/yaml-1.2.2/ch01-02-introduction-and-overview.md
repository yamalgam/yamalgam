# Chapters 1-2. Introduction and Language Overview

> Marked-up copy of YAML 1.2.2 specification Chapters 1-2, with tracey requirement markers.
> Only normative requirements and formal definitions are marked. Informational prose is summarized or omitted.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

# Chapter 1. Introduction to YAML

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->

## 1.1. Goals

The design goals for YAML are, in decreasing priority:

y[intro.goals.human-readable]
1. YAML should be easily readable by humans.

y[intro.goals.portable]
1. YAML data should be portable between programming languages.

y[intro.goals.native-match]
1. YAML should match the [native data structures] of dynamic languages.

y[intro.goals.consistent-model]
1. YAML should have a consistent model to support generic tools.

y[intro.goals.one-pass]
1. YAML should support one-pass processing.

y[intro.goals.expressive]
1. YAML should be expressive and extensible.

y[intro.goals.easy-impl]
1. YAML should be easy to implement and use.


## 1.2. YAML History

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 1.3. Terminology

y[intro.terminology.rfc2119]
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED",  "MAY", and "OPTIONAL" in this document are to be
interpreted as described in RFC 2119.

<!-- Remaining chapter layout description omitted — see vendor/yaml-spec/spec-1.2.2.md -->


# Chapter 2. Language Overview

<!-- Introductory prose omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 2.1. Collections

y[overview.collections.block-indent]
YAML's [block collections] use [indentation] for scope and begin each entry on
its own line.

y[overview.collections.block-seq-indicator]
[Block sequences] indicate each entry with a dash and space ("`- `").

y[overview.collections.mapping-indicator]
[Mappings] use a colon and space ("`: `") to mark each [key/value pair].

y[overview.collections.comment-indicator]
[Comments] begin with an octothorpe (also called a "hash", "sharp", "pound" or
"number sign" - "`#`").

<!-- Examples 2.1 through 2.4 omitted — see vendor/yaml-spec/spec-1.2.2.md -->

y[overview.collections.flow-styles]
YAML also has [flow styles], using explicit [indicators] rather than
[indentation] to denote scope.

y[overview.collections.flow-seq-syntax]
The [flow sequence] is written as a [comma] separated list within [square]
[brackets].

y[overview.collections.flow-map-syntax]
In a similar manner, the [flow mapping] uses [curly] [braces].

<!-- Examples 2.5 through 2.6 omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 2.2. Structures

y[overview.structures.doc-start-marker]
YAML uses three dashes ("`---`") to separate [directives] from [document]
[content].
This also serves to signal the start of a document if no [directives] are
present.

y[overview.structures.doc-end-marker]
Three dots ( "`...`") indicate the end of a document without starting a new
one, for use in communication channels.

<!-- Examples 2.7 through 2.8 omitted — see vendor/yaml-spec/spec-1.2.2.md -->

y[overview.structures.anchor-alias]
Repeated [nodes] (objects) are first [identified] by an [anchor] (marked with
the ampersand - "`&`") and are then [aliased] (referenced with an asterisk -
"`*`") thereafter.

<!-- Examples 2.9 through 2.10 omitted — see vendor/yaml-spec/spec-1.2.2.md -->

y[overview.structures.complex-key]
A question mark and space ("`? `") indicate a complex [mapping] [key].

y[overview.structures.compact-notation]
Within a [block collection], [key/value pairs] can start immediately following
the [dash], [colon] or [question mark].

<!-- Examples 2.11 through 2.12 omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 2.3. Scalars

y[overview.scalars.literal-style]
[Scalar content] can be written in [block] notation, using a [literal style]
(indicated by "`|`") where all [line breaks] are significant.

y[overview.scalars.folded-style]
Alternatively, they can be written with the [folded style] (denoted by "`>`")
where each [line break] is [folded] to a [space] unless it ends an [empty] or a
[more-indented] line.

<!-- Examples 2.13 through 2.16 omitted — see vendor/yaml-spec/spec-1.2.2.md -->

y[overview.scalars.flow-plain]
YAML's [flow scalars] include the [plain style] (most examples thus far) and
two quoted styles.

y[overview.scalars.double-quoted-escapes]
The [double-quoted style] provides [escape sequences].

y[overview.scalars.single-quoted-no-escape]
The [single-quoted style] is useful when [escaping] is not needed.

y[overview.scalars.flow-multiline-fold]
All [flow scalars] can span multiple lines; [line breaks] are always [folded].

<!-- Examples 2.17 through 2.18 omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 2.4. Tags

y[overview.tags.untagged-nodes]
In YAML, [untagged nodes] are given a type depending on the [application].

<!-- Examples 2.19 through 2.22 omitted — see vendor/yaml-spec/spec-1.2.2.md -->

y[overview.tags.explicit-tag-indicator]
Explicit typing is denoted with a [tag] using the exclamation point ("`!`")
symbol.

y[overview.tags.global-tags-uri+3]
[Global tags] are URIs and may be specified in a [tag shorthand] notation using
a [handle].

y[overview.tags.local-tags+3]
[Application]\-specific [local tags] may also be used.

<!-- Examples 2.23 through 2.28 omitted — see vendor/yaml-spec/spec-1.2.2.md -->


## 2.5. Full Length Example

<!-- Informational content omitted — see vendor/yaml-spec/spec-1.2.2.md -->
