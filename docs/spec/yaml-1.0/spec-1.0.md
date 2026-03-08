# YAML Ain't Markup Language (YAML™) 1.0

## Final Draft 2004-JAN-29

### Oren Ben-Kiki

<[oren@ben-kiki.org](mailto:oren@ben-kiki.org)>

### Clark Evans

<[cce@clarkevans.com](mailto:cce@clarkevans.com)>

### Brian Ingerson

<[ingy@ttul.org](mailto:ingy@ttul.org)>

Copyright © 2001-2004 Oren Ben-Kiki, Clark Evans, Brian Ingerson

This document may be freely copied provided it is not modified.

**Abstract**

YAML™ (rhymes with "camel") is a human friendly, cross language, unicode based data serialization language designed around the common native structures of agile programming languages. It is broadly useful for programming needs ranging from configuration files to Internet messaging to object persistence to data auditing. Together with the [Unicode](http://www.unicode.org/) standard for characters, this specification provides all the information necessary to understand YAML Version 1.0 and to construct programs that process YAML information.

---

## Table of Contents

[1. Introduction](#introduction)
[1.1. Goals](#goals)
[1.2. Prior Art](#prior-art)
[1.3. Relation to XML](#relation-to-xml)
[1.4. Terminology](#terminology)

[2. Preview](#preview)
[2.1. Collections](#collections)
[2.2. Structures](#structures)
[2.3. Scalars](#scalars)
[2.4. Tags](#tags)
[2.5. Full Length Example](#full-length-example)

[3. Processing YAML Information](#processing-yaml-information)
[3.1. Processes](#processes)
[3.1.1. Represent](#represent)
[3.1.2. Serialize](#serialize)
[3.1.3. Present](#present)
[3.1.4. Parse](#parse)
[3.1.5. Compose](#compose)
[3.1.6. Construct](#construct)

[3.2. Information Models](#information-models)
[3.2.1. Node Graph Representation](#node-graph-representation)
[3.2.2. Event / Tree Serialization](#event--tree-serialization)
[3.2.3. Character Stream Presentation](#character-stream-presentation)

[3.3. Completeness](#completeness)
[3.3.1. Well-Formed](#well-formed)
[3.3.2. Resolved](#resolved)
[3.3.3. Recognized and Valid](#recognized-and-valid)
[3.3.4. Available](#available)

[4. Syntax](#syntax)
[4.1. Characters](#characters)
[4.1.1. Character Set](#character-set)
[4.1.2. Encoding](#encoding)
[4.1.3. Indicators](#indicators)
[4.1.4. Line Breaks](#line-breaks)
[4.1.5. Miscellaneous](#miscellaneous)

[4.2. Space Processing](#space-processing)
[4.2.1. Indentation](#indentation)
[4.2.2. Throwaway comments](#throwaway-comments)

[4.3. YAML Stream](#yaml-stream)
[4.3.1. Document](#document)
[4.3.2. Directive](#directive)
[4.3.3. Presentation Node](#presentation-node)
[4.3.4. Node Property](#node-property)
[4.3.5. Tag](#tag)
[4.3.6. Anchor](#anchor)

[4.4. Alias](#alias)
[4.5. Collection](#collection)
[4.5.1. Sequence](#sequence)
[4.5.2. Mapping](#mapping)

[4.6. Scalar](#scalar)
[4.6.1. End Of line Normalization](#end-of-line-normalization)
[4.6.2. Block Modifiers](#block-modifiers)
[4.6.3. Explicit Indentation](#explicit-indentation)
[4.6.4. Chomping](#chomping)
[4.6.5. Literal](#literal)
[4.6.6. Folding](#folding)
[4.6.7. Folded](#folded)
[4.6.8. Single Quoted](#single-quoted)
[4.6.9. Escaping](#escaping)
[4.6.10. Double Quoted](#double-quoted)
[4.6.11. Plain](#plain)

[A. Tag Repository](#tag-repository)
[A.1. Sequence](#sequence-1)
[A.2. Mapping](#mapping-1)
[A.3. String](#string)

[B. YAML Terms](#yaml-terms)

## Chapter 1. Introduction

"YAML Ain't Markup Language" (abbreviated YAML) is a data serialization language designed to be human friendly and work well with modern programming languages for common everyday tasks. This specification is both an introduction to the YAML language and the concepts supporting it; and also a complete reference of the information needed to develop applications for processing YAML.

Open, interoperable and readily understandable tools have advanced computing immensely. YAML was designed from the start to be useful and friendly to the people working with data. It uses printable unicode characters, some of which provide structural information and the rest representing the data itself. YAML achieves a unique cleanness by minimizing the amount of structural characters, and allowing the data to show itself in a natural and meaningful way. For example, indentation is used for structure, colons separate pairs, and dashes are used for bulleted lists.

There are myriad flavors of data structures, but they can all be adequately represented with three basic primitives: mappings (hashes/dictionaries), sequences (arrays/lists) and scalars (strings/numbers). YAML leverages these primitives and adds a simple typing system and aliasing mechanism to form a complete language for encoding any data structure. While most programming languages can use YAML for data serialization, YAML excels in those languages that are fundamentally built around the three basic primitives. These include the new wave of agile languages such as Perl, Python, PHP, Ruby and Javascript.

There are hundreds of different languages for programming, but only a handful of languages for storing and transferring data. Even though its potential is virtually boundless, YAML was specifically created to work well for common use cases such as: configuration files, log files, interprocess messaging, cross-langauge data sharing, object persistence and debugging of complex data structures. When data is well organized and easy to understand, programming becomes a simpler task.

## 1.1. Goals


y[intro.goals.consistent-model]

y[intro.goals.easy-impl]

y[intro.goals.expressive]

y[intro.goals.human-readable]

y[intro.goals.native-match]

y[intro.goals.one-pass]

y[intro.goals.portable]

The design goals for YAML are:

1. YAML documents are easily readable by humans.
2. YAML uses the native data structures of agile languages.
3. YAML data is portable between programming languages.
4. YAML has a consistent model to support generic tools.
5. YAML enables stream-based processing.
6. YAML is expressive and extensible.
7. YAML is easy to implement and use.

## 1.2. Prior Art

YAML's initial direction was set by the data serialization and markup language discussions among [SML-DEV](http://www.docuverse.com/smldev/) members. Later on it directly incorporated experience from Brian Ingerson's Perl module [Data::Denter](http://search.cpan.org/doc/INGY/Data-Denter-0.13/Denter.pod). Since then YAML has matured through ideas and support from its user community.

YAML integrates and builds upon concepts described by [C](http://cm.bell-labs.com/cm/cs/cbook/index.html), [Java](http://java.sun.com/), [Perl](http://www.perl.org/), [Python](http://www.python.org/), [Ruby](http://www.ruby-lang.org/), [RFC0822](http://www.ietf.org/rfc/rfc0822.txt) (MAIL), [RFC1866](http://www.ics.uci.edu/pub/ietf/html/rfc1866.txt) (HTML), [RFC2045](http://www.ietf.org/rfc/rfc2045.txt) (MIME), [RFC2396](http://www.ietf.org/rfc/rfc2396.txt) (URI), [XML](http://www.w3.org/TR/REC-xml.html), [SAX](http://www.saxproject.org/) and [SOAP](http://www.w3.org/TR/SOAP).

The syntax of YAML was motivated by Internet Mail (RFC0822) and remains partially compatible with that standard. Further, YAML borrows the idea of having multiple documents from MIME (RFC2045). YAML's top-level production is a stream of independent documents; ideal for message-based distributed processing systems.

YAML's indentation based block scoping is similar to Python's (without the ambiguities caused by tabs). Indented blocks facilitate easy inspection of a document's structure. YAML's literal scalar leverages this by enabling formatted text to be cleanly mixed within an indented structure without troublesome escaping.

YAML's double quoted scalar uses familar C-style escape sequences. This enables ASCII representation of non-printable or 8-bit (ISO 8859-1) characters such as `\x3B`. 16-bit Unicode and 32-bit (ISO/IEC 10646) characters are supported with escape sequences such as `\u003B` and `\U0000003B`.

Motivated by HTML's end-of-line normalization, YAML's folded scalar employs an intuitive method of handling white space. In YAML, single line breaks may be folded into a single space, while empty lines represent line break characters. This technique allows for paragraphs to be word-wrapped without affecting the canonical form of the content.

YAML's core type system is based on the requirements of Perl, Python and Ruby. YAML directly supports both collection (hash, array) values and scalar (string) values. Support for common types enables programmers to use their language's native data constructs for YAML manipulation, instead of requiring a special document object model (DOM).

Like XML's SOAP, YAML supports serializing native graph structures through a rich alias mechanism. Also like SOAP, YAML provides for application-defined types. This allows YAML to encode rich data structures required for modern distributed computing. YAML provides unique global type names using a namespace mechanism inspired by Java's DNS based package naming convention and XML's URI based namespaces.

YAML was designed to have an incremental interface that includes both a pull-style input stream and a push-style (SAX-like) output stream interfaces. Together this enables YAML to support the processing of large documents, such as a transaction log, or continuous streams, such as a feed from a production machine.

## 1.3. Relation to XML

Newcomers to YAML often search for its correlation to the eXtensible Markup Language (XML). While the two languages may actually compete in several application domains, there is no direct correlation between them.

YAML is primarily a data serialization language. XML was designed to be backwards compatible with the Standard Generalized Markup Language (SGML) and thus had many design constraints placed on it that YAML does not share. Inheriting SGML's legacy, XML is designed to support structured documents, where YAML is more closely targeted at messaging and native data structures. Where XML is a pioneer in many domains, YAML is the result of lessons learned from XML and other technologies.

It should be mentioned that there are ongoing efforts to define standard XML/YAML mappings. This generally requires that a subset of each language be used. For more information on using both XML and YAML, please visit [https://yaml.org/xml/](/xml/).

## 1.4. Terminology


y[intro.terminology.rfc2119]

This specification uses key words in accordance with [RFC2119](http://www.ietf.org/rfc/rfc2119.txt) to indicate requirement level. In particular, the following words are used to describe the actions of a YAML processor:

*may*
This word, or the adjective "*optional*", mean that conformant YAML processors are permitted, but need not behave as described.

*should*
This word, or the adjective "*recommended*", mean that there could be reasons for a YAML processor to deviate from the behavior described, but that such deviation could hurt interoperability and should therefore be advertised with appropriate notice.

*must*
This word, or the term "*required*" or "*shall*", mean that the behavior described is an absolute requirement of the specification.

## Chapter 2. Preview

This section provides a quick glimpse into the expressive power of YAML. It is not expected that the first-time reader grok all of the examples. Rather, these selections are used as motivation for the remainder of the specification.

## 2.1. Collections


y[overview.collections.block-indent]

y[overview.collections.block-seq-indicator]

y[overview.collections.comment-indicator]

y[overview.collections.flow-map-syntax]

y[overview.collections.flow-seq-syntax]

y[overview.collections.flow-styles]

y[overview.collections.mapping-indicator]

YAML's block collections use indentation for scope and begin each member on its own line. Block sequences indicate each member with a dash ("**-**"). Block mappings use a colon to mark each (key: value) pair.

**Example 2.1. Sequence of scalars (ball players)**

```
- Mark McGwire
- Sammy Sosa
- Ken Griffey
```

**Example 2.2. Mapping of scalars to scalars (player statistics)**

```
hr: 65
avg: 0.278
rbi: 147
```

**Example 2.3. Mapping of scalars to sequences (ball clubs in each league)**

```
american:
 - Boston Red Sox
 - Detroit Tigers
 - New York Yankees
national:
 - New York Mets
 - Chicago Cubs
 - Atlanta Braves
```

**Example 2.4. Sequence of mappings (players' statistics)**

```
-
 name: Mark McGwire
 hr: 65
 avg: 0.278
-
 name: Sammy Sosa
 hr: 63
 avg: 0.288
```

YAML also has in-line flow styles for compact notation. The flow sequence is written as a comma separated list within square brackets. In a similar manner, the flow mapping uses curley braces. In YAML, the space after the "**-**" and "**:**" and "**:**" is mandatory.

**Example 2.5. Sequence of sequences**

```
- [name        , hr, avg  ]
- [Mark McGwire, 65, 0.278]
- [Sammy Sosa  , 63, 0.288]
```

**Example 2.6. Mapping of mappings**

```
Mark McGwire: {hr: 65, avg: 0.278}
Sammy Sosa: {     hr: 63,     avg: 0.288   }
```

## 2.2. Structures


y[overview.structures.anchor-alias]

y[overview.structures.compact-notation]

y[overview.structures.complex-key]

y[overview.structures.doc-end-marker]

y[overview.structures.doc-start-marker]

YAML uses three dashes ("**---**") to separate documents within a stream. Comment lines begin with the pound sign ("**#**"). Three dots ("**...**") indicate the end of a document without starting a new one, for use in communication channels.

Repeated nodes are first marked with the ampersand ("**&**") and then referenced with an asterisk ("*****") thereafter.

**Example 2.7. Two documents in a stream each with a leading comment**

```
# Ranking of 1998 home runs
---
- Mark McGwire
- Sammy Sosa
- Ken Griffey

# Team ranking
---
- Chicago Cubs
- St Louis Cardinals
```

**Example 2.8. Play by play feed from a game**

```
---
time: 20:03:20
player: Sammy Sosa
action: strike (miss)
...
---
time: 20:03:47
player: Sammy Sosa
action: grand slam
...
```

**Example 2.9. Single document with two comments**

```
---
hr: # 1998 hr ranking
 - Mark McGwire
 - Sammy Sosa
rbi:
 # 1998 rbi ranking
 - Sammy Sosa
 - Ken Griffey
```

**Example 2.10. Node for "Sammy Sosa" appears twice in this document**

```
---
hr:
 - Mark McGwire
 # Following node labeled SS
 - &SS Sammy Sosa
rbi:
 - *SS # Subsequent occurance
 - Ken Griffey
```

The question mark indicates a complex key. Within a block sequence, mapping pairs can start immediately following the dash.

**Example 2.11. Mapping between sequences**

```
? # PLAY SCHEDULE
  - Detroit Tigers
  - Chicago Cubs
:
  - 2001-07-23
? [ New York Yankees,     Atlanta Braves ]
: [ 2001-07-02, 2001-08-12,     2001-08-14 ]
```

**Example 2.12. Sequence key shortcut**

```
---
# products purchased
- item    : Super Hoop
  quantity: 1
- item    : Basketball
  quantity: 4
- item    : Big Shoes
  quantity: 1
```

## 2.3. Scalars


y[overview.scalars.double-quoted-escapes]

y[overview.scalars.flow-multiline-fold]

y[overview.scalars.flow-plain]

y[overview.scalars.folded-style]

y[overview.scalars.literal-style]

y[overview.scalars.single-quoted-no-escape]

Scalar values can be written in block form using a literal style ("**|**") where all new lines count. Or they can be written with the folded style ("**>**") for content that can be word wrapped. In the folded style, newlines are treated as a space unless they are part of a blank or indented line.

**Example 2.13. In literals, newlines are preserved**

```
# ASCII Art
--- |
 \//||\/||
 // || ||__
```

**Example 2.14. In the plain scalar, newlines are treated as a space**

```
---
 Mark McGwire's
 year was crippled
 by a knee injury.
```

**Example 2.15. Folded newlines preserved for indented and blank lines**

```
--- >
 Sammy Sosa completed another
 fine season with great stats.

 63 Home Runs
 0.288 Batting Average

 What a year!
```

**Example 2.16. Indentation determines scope**

```
name: Mark McGwire
accomplishment: >
 Mark set a major league
 home run record in 1998.
stats: |
 65 Home Runs
 0.278 Batting Average
```

YAML's flow scalars include the plain style (most examples thus far) and quoted styles. The double quoted style provides escape sequences. Single quoted style is useful when escaping is not needed. All flow scalars can span multiple lines; intermediate whitespace is trimmed to a single space.

**Example 2.17. Quoted scalars**

```
unicode: "Sosa did fine.\u263A"
control: "\b1998\t1999\t2000\n"
hexesc:  "\x13\x10 is \r\n"

single: '"Howdy!" he cried.'
quoted: ' # not a ''comment''.'
tie-fighter: '|\-*-/|'
```

**Example 2.18. Multiline flow scalars**

```
plain:   This unquoted scalar
   spans many lines.

quoted: "So does this
   quoted scalar.\n"
```

## 2.4. Tags


y[overview.tags.explicit-tag-indicator]

y[overview.tags.untagged-nodes]

In YAML, plain (unquoted) scalars are given an implicit type depending on the application. The examples in this specification use types from YAML's tag repository, which includes types like integers, floating point values, timestamps, null, boolean, and string values.

**Example 2.19. Integers**

```
canonical: 12345
decimal: +12,345
sexagecimal: 3:25:45
octal: 014
hexadecimal: 0xC
```

**Example 2.20. Floating point**

```
canonical: 1.23015e+3
exponential: 12.3015e+02
sexagecimal: 20:30.15
fixed: 1,230.15
negative infinity: (-inf)
not a number: (NaN)
```

**Example 2.21. Miscellaneous**

```
null: ~
true: y
false: n
string: '12345'
```

**Example 2.22. Timestamps**

```
canonical: 2001-12-15T02:59:43.1Z
iso8601: 2001-12-14t21:59:43.10-05:00
spaced: 2001-12-14 21:59:43.10 -05:00
date: 2002-12-14
```

Explicit typing is denoted with a tag using the bang ("**!**") symbol. Application tags should include a domain name and may use the caret ("**^**") to abbreviate subsequent tags.

**Example 2.23. Various explicit tags**

```
---
not-date: !str 2002-04-28

picture: !binary |
 R0lGODlhDAAMAIQAAP//9/X
 17unp5WZmZgAAAOfn515eXv
 Pz7Y6OjuDg4J+fn5OTk6enp
 56enmleECcgggoBADs=

application specific tag: !!something |
 The semantics of the tag
 above may be different for
 different documents.
```

**Example 2.24. Application specific tag**

```
# Establish a tag prefix
---
!clarkevans.com,2002/graph/^shape

# Use the prefix: shorthand for
# !clarkevans.com,2002/graph/circle
- !^circle
  center: &ORIGIN {x: 73, y: 129}
  radius: 7
- !^line
  start: *ORIGIN
  finish: { x: 89, y: 102 }
- !^label
  start: *ORIGIN
  color: 0xFFEEBB
  value: Pretty vector drawing.
```

**Example 2.25. Unorderd set**

```
# sets are represented as a
# mapping where each key is
# associated with the empty string
---
!set
? Mark McGwire
? Sammy Sosa
? Ken Griff
```

**Example 2.26. Ordered mappings**

```
# ordered maps are represented as
# a sequence of mappings, with
# each mapping having one key
---
!omap
- Mark McGwire: 65
- Sammy Sosa: 63
- Ken Griffy: 58
```

## 2.5. Full Length Example

Below are two full-length examples of YAML. On the left is a sample invoice; on the right is a sample log file.

**Example 2.27. Invoice**

```
---
!clarkevans.com,2002/^invoice
invoice: 34843
date   : 2001-01-23
bill-to: &id001
    given  : Chris
    family : Dumars
    address:
        lines: |
            458 Walkman Dr.
            Suite #292
        city    : Royal Oak
        state   : MI
        postal  : 48046
ship-to: *id001
product:
    - sku         : BL394D
      quantity    : 4
      description : Basketball
      price       : 450.00
    - sku         : BL4438H
      quantity    : 1
      description : Super Hoop
      price       : 2392.00
tax  : 251.42
total: 4443.52
comments:
    Late afternoon is best.
    Backup contact is Nancy
    Billsmer @ 338-4338.
```

**Example 2.28. Log file**

```
---
Time: 2001-11-23 15:01:42 -05:00
User: ed
Warning:
  This is an error message
  for the log file
---
Time: 2001-11-23 15:02:31 -05:00
User: ed
Warning:
  A slightly different error
  message.
---
Date: 2001-11-23 15:03:17 -05:00
User: ed
Fatal:
  Unknown variable "bar"
Stack:
  - file: TopClass.py
    line: 23
    code: |
      x = MoreObject("345\n")
  - file: MoreClass.py
    line: 58
    code: |-
      foo = bar
```

## Chapter 3. Processing YAML Information

YAML is both a text format and a method for representing native language data structures in this format. This specification defines two concepts: a class of data objects called YAML representations, and a syntax for encoding YAML representations as a series of characters, called a YAML stream. A YAML processor is a tool for converting information between these complementary views. It is assumed that a YAML processor does its work on behalf of another module, called an application. This chapter describes the information structures a processor must provide to or obtain from the application.

YAML information is used in two ways: for machine processing, and for human consumption. The challange of reconciling these two perspectives is best done in three distinct translation stages: representation, serialization, and presentation. Representation addresses how YAML views native language data structures to achieve portability between programming environments. Serialization concerns itself with turning a YAML representation into a serial form, that is, a form with sequential access constraints. Presentation deals with the formatting of a YAML serialization as a stream of characters, in a manner friendly to humans.

A processor need not expose the serialization or representation stages. It may translate directly between native objects and a character stream and ("dump" and "load" in the diagram above). However, such a direct translation should take place so that the native objects are constructed only from information available in the representation.

## 3.1. Processes


y[model.process.dump.present]

y[model.process.dump.represent]

y[model.process.dump.represent.tag]

y[model.process.dump.serialize]

y[model.process.load.compose]

y[model.process.load.construct]

y[model.process.load.parse]

y[model.process.no-serial-details]

This section details the processes shown in the diagram above.

### 3.1.1. Represent

YAML representations model the data constructs from agile programming languages, such as Perl, Python, or Ruby. YAML representations view native language data objects in a generic manner, allowing data to be portable between various programming languages and implementations. Strings, arrays, hashes, and other user-defined types are supported. This specification formalizes what it means to be a YAML representatation and suggests how native language objects can be viewed as a YAML representation.

YAML representations are constructed with three primitives: the sequence, the mapping and the scalar. By sequence we mean an ordered collection, by mapping we mean an unordered association of unique keys to values, and by scalar we mean any object with opaque structure yet expressable as a series of unicode characters. When used generatively, these primitives construct directed graph structures. These primitives were chosen beacuse they are both powerful and familiar: the sequence corresponds to a Perl array and a Python list, the mapping corresponds to a Perl hashtable and a Python dictionary. The scalar represents strings, integers, dates and other atomic data types.

YAML represents any native language data object as one of these three primitives, together with a type specifier called a tag. Type specifiers are either global, using a syntax based on the domain name and registration date, or private in scope. For example, an integer is represented in YAML with a scalar plus a globally scoped **tag:yaml.org,2002/int** tag. Similarly, an invoice object, particular to a given organization, could be represented as a mapping together with a **tag:private.yaml.org,2002:invoice** tag. This simple model, based on the sequence and mapping and scalar together with a type specifier, can represent any data structure independent of programming language.

### 3.1.2. Serialize

For sequential access mediums, such as an event callback API, a YAML representation must be serialized to an ordered tree. Serialization is necessary since nodes in a YAML representation may be referenced more than once (more than one incoming arrow) and since mapping keys are unordered. Serialization is accomplished by imposing an ordering on mapping keys and by replacing the second and subsequent references to a given node with place holders called aliases. The result of this process, the YAML serialization tree, can then be traversed to produce a series of event calls for one-pass processing of YAML data.

### 3.1.3. Present

YAML character streams (or documents) encode YAML representations into a series of characters. Some of the characters in a YAML stream represent the content of the source information, while other characters are used for presentation style. Not only must YAML character streams store YAML representations, they must do so in a manner which is human friendly.

To address human presentation, the YAML syntax has a rich set of stylistic options which go far beyond the needs of data serialization. YAML has two approaches for expressing a node's nesting, one that uses indentation to designate depth in the serialization tree and another which uses begin and end delimiters. Depending upon escaping and how line breaks should be treated, YAML scalars may be written with many different styles. YAML syntax also has a comment mechanism for annotations othogonal to the "content" of a YAML representation. These presentation level details provide sufficient variety of expression.

In a similar manner, for human readable text, it is frequently desirable to omit data typing information which is often obvious to the human reader and not needed. This is especially true if the information is created by hand, expecting humans to bother with data typing detail is optimistic. Implicit type information may be restored using a data schema or similar mechanisms.

### 3.1.4. Parse

Parsing is the inverse process of presentation, it takes a stream of characters and produces a series of events.

### 3.1.5. Compose

Composing takes a series of events and produces a node graph representation. See completeness for more detail on the constraints composition must follow. When composing, one must deal with broken aliases and anchors, and other things of this sort.

### 3.1.6. Construct

Construction converts construct YAML representations into native language objects.

## 3.2. Information Models


y[model.loading.complete-representation]

y[model.loading.error-recovery]

y[model.loading.failure-points]

y[model.loading.identified-alias]

y[model.loading.non-specific-tag]

y[model.loading.partial-representation]

y[model.loading.reject-ill-formed]

y[model.loading.tag-resolution-alias]

y[model.loading.tag-resolution-app-specific]

y[model.loading.tag-resolution-bang-convention]

y[model.loading.tag-resolution-no-other-content]

y[model.loading.tag-resolution-no-presentation]

y[model.loading.tag-resolution-no-sibling]

y[model.loading.tag-resolution-override]

y[model.loading.tag-resolution-params]

y[model.loading.tag-resolution-required]

y[model.loading.unavailable-tag]

y[model.loading.unrecognized-collection]

y[model.loading.unrecognized-scalar]

y[model.loading.unresolved-tags]

y[model.loading.valid-node]

y[model.loading.well-formed]

y[model.present.comments-no-effect]

y[model.present.comments-not-in-scalars]

y[model.present.detail-not-content]

y[model.present.directives-defined]

y[model.present.directives-not-in-model]

y[model.present.directives-per-document]

y[model.present.format-canonical]

y[model.present.format-definition]

y[model.present.format-not-in-model]

y[model.present.multi-document]

y[model.present.scalar-styles]

y[model.present.stream-definition]

y[model.present.style-groups]

y[model.present.style-not-in-model]

y[model.repr.canonical-form]

y[model.repr.collection-vs-scalar]

y[model.repr.equality]

y[model.repr.equality.cyclic]

y[model.repr.equality.mapping]

y[model.repr.equality.recursive]

y[model.repr.equality.scalar]

y[model.repr.equality.scalar-identity]

y[model.repr.equality.sequence]

y[model.repr.equality.tag-comparison]

y[model.repr.graph-definition]

y[model.repr.node-definition]

y[model.repr.node.mapping]

y[model.repr.node.scalar]

y[model.repr.node.sequence]

y[model.repr.serial-detail-not-content]

y[model.repr.tag-definition]

y[model.repr.tag.global]

y[model.repr.tag.local]

y[model.repr.tag.local-uri-charset]

y[model.repr.tag.no-substring-relationship]

y[model.repr.tag.node-kind]

y[model.repr.tag.scalar-canonical]

y[model.repr.uniqueness]

y[model.serial.alias-resolution]

y[model.serial.anchor-definition]

y[model.serial.anchor-discard]

y[model.serial.anchor-no-alias-required]

y[model.serial.key-order.no-representation-order]

y[model.serial.key-order.serialization-detail]

y[model.serial.key-order.use-sequence]

y[model.serial.no-key-order-or-anchor-names]

y[model.serial.tree-definition]

This section has the formal details of the results of the processes.

To maximize data portability between programming languages and implementations, users of YAML should be mindful of the distinction between serialization or presentation properties and those which are part of the YAML representation. While imposing a order on mapping keys is necessary for flattening YAML representations to a sequential access medium, the specific ordering of a mapping should not be used to convey application level information. In a similar manner, while indentation technique or the specific scalar style is needed for character level human presentation, this syntax detail is not part of a YAML serialization nor a YAML representation. By carefully separating properties needed for serialization and presentation, YAML representations of native language information will be consistent and portable between various programming environments.

### 3.2.1. Node Graph Representation

In YAML's view, native data is represented as a directed graph of tagged nodes. Nodes that are defined in terms of other nodes are collections and nodes that are defined independent of any other nodes are scalars. YAML supports two kinds of collection nodes, sequence and mappings. Mapping nodes are somewhat tricky beacuse its keys are considered to be unordered and unique.

#### 3.2.1.1. Nodes

A YAML representation is a rooted, connected, directed graph. By "directed graph" we mean a set of nodes and arrows, where arrows connect one node to another. Note that the YAML graph may include cycles, and a node may have more than one incoming arrow.

YAML nodes have a tag and can be of one of three kinds: scalar, sequence, or mapping. The node's tag serves to restrict the set of possible values which the node can have.

*scalar*
A scalar is a series of zero or more Unicode characters. YAML places no restriction on the length or content of the series.

*sequence*
A sequence is a series of zero or more nodes. In particular, a sequence may contain the same node more than once or it could even contain itself (directly or indirectly).

*mapping*
A mapping is an unordered set of key/value node pairs, with the restriction that each of the keys is unique. This restriction has non-trivial implications detailed below. YAML places no further restrictions on the nodes. In particular, keys may be arbitrary nodes, the same node may be used as a value in several pairs, and a mapping could even contain itself as a key or a value (directly or indirectly).

When appropriate, it is convient to consider sequences and mappings together, as a collection. In this view, sequences are treated as mappings with integer keys starting at zero. Having a unified collections view for sequences and mappings is helpful for both constructing practical YAML tools and APIs and for theoretical analysis.

YAML allows several representations to be encoded to the same character stream. Representations appearing in the same character stream are independent. That is, a given node may not appear in more than one representation graph.

#### 3.2.1.2. Tags

YAML represents type information of native objects with a simple identifier, called a tag. These identifiers are URIs, using a subset of the "tag" URI scheme. YAML tags use only the domain based form, **tag:**domain**,**date**:**identifier, for example, **tag:yaml.org,2002:str**. YAML presentations provide several mechanisms to make this less verbose. Tags may be minted by those who own the domain at the specified date. The day must be omitted if it is the 1st of the month, and the month and day must be omitted for January 1st. The year is never omitted. Thus, each YAML tag has a single globally unique representation. More information on this URI scheme can be found at [http://www.taguri.org](http://www.taguri.org).

YAML tags can be either globally unique, or private to a single representation graph. Private tags start with **tag:private.yaml.org,2002:**. Clearly private tags are not globally unique, since the domain name and the date are fixed.

YAML does not mandate any special relationship between different tags that begin with the same substring. Tags ending URI fragments (containing "**#**") are no exception. Tags that share the same base URI but differ in their fragment part are considered to be different, independent tags. By convention, fragments are used to identify different "versions" of a tag, while "**/**" is used to define nested tag "namespace" hierarchies. However, this is merely a convention, and each tag may employ its own rules. For example, **tag:perl.yaml.org,2002:** tags use "**::**" to express namespace hierarchies, **tag:java.yaml.org,2002:** tags use "**.**", etc.

YAML tags are used to associate meta information with each node. In particular, each tag is required to specify a the kind (scalar, sequence, or mapping) it applies to. Scalar tags must also provide mechanism for converting values to a canonical form for supporting equality testing. Furthermore, a tag may provide additional information such as the set of allowed values for validation, a mechanism for implicit typing, or any other data that is applicable to all of the tag's nodes.

#### 3.2.1.3. Equality

Since YAML mappings require key uniqueness, representations must include a mechanism for testing the equality of nodes. This is non-trivial since YAML presentations allow various ways to write a given scalar. For example, the integer ten can be written as **10** or **0xA** (hex). If both forms are used as a key in the same mapping, only a YAML processor which "knows" about integer tags and their presentation formats would correctly flag the duplicate key as an error.

*canonical form*
YAML supports the need for scalar equality by requiring that every scalar tag have a mechanism to produce a canonical form of its scalars. By canonical form, we mean a Unicode character string which represents the scalar's content and can be used for equality testing. While this requirement is stronger than a well defined equality operator, it has other uses, such as the production of digital signatures.

*equality*
Two nodes must have the same tag and value to be equal. Since each tag applies to exactly one kind, this implies that the two nodes must have the same kind to be equal. Two scalar nodes are equal only when their canonical values are character-by-character equivalent. Equality of collections is defined recursively. Two sequences are equal only when they have the same length and each node in one sequence is equal to the corresponding node in the other sequence. Two mappings are equal only when they have equal sets of keys, and each key in this set is associated with equal values in both mappings.

*identity*
Node equality should not be confused with node identity. Two nodes are identical only when they represent the same native object. Typically, this corresponds to a single memory address. During serialization, equal scalar nodes may be treated as if they were identical. In contrast, the seperate identity of two distinct, but equal, collection nodes must be preserved.

### 3.2.2. Event / Tree Serialization

To express a YAML representation using a serial API, it necessary to impose an order on mapping keys and employ alias nodes to indicate a subsequent occurence of a previously encountered node. The result of this serialization process is a tree structure, where each branch has an ordered set of children. This tree can be traversed for a serial event based API. Construction of native structures from the serial interface should not use key order or anchors for the preservation of important data.

#### 3.2.2.1. Key Order

In the representation model, keys in a mapping do not have order. To serialize a mapping, it is necessary to impose an ordering on its keys. This order should not be used when composing a representation graph from serialized events.

In every case where node order is significant, a sequence must be used. For example, an ordered mapping can be represented by a sequence of mappings, where each mapping is a single key/value pair. YAML presentations provide convient shorthand syntax for this case.

#### 3.2.2.2. Aliases

In the representation model, a node may appear in more than one context. When serializing such nodes, the first occurance of the node is serialized with an anchor and subsequent occurances are serialized as an alias which specifies the same anchor. Anchors need not be unique within a serialization. When composing a representation graph from serialized events, alias nodes refer to the most recent node in the serialization having the specified anchor.

An anchored node need not have an alias referring to it. It is therefore possible to provide an anchor for all nodes in serialization. After composing a representation graph, the anchors are discarded. Hence, anchors must not be used for encoding application data.

### 3.2.3. Character Stream Presentation

YAML presentations make use of styles, comments, directives and other syntactical details. Although the processor may provide this information, these features should not be used when constructing native structures.

#### 3.2.3.1. Styles

In the syntax, each node has an additional style property, depending on its node. There are two types of styles, block and flow. Block styles use indentation to denote nesting and scope within the presentation. In contrast, flow styles rely on explicit markers to denote nesting and scope.

YAML provides several shorthand forms for collection styles, allowing for compact nesting of collections in common cases. For compact set notation, null mapping values may be omitted. For compact ordered mapping notation, a mapping with a single key:value pair may be specified directly inside a flow sequence collection. Also, simple block collections may begin in-line rather than the next line.

YAML provides a rich set of scalar style variants. Scalar block styles include the literal and folded styles; scalar flow styles include the plain, single quoted and double quoted styles. These styles offer a range of tradeoffs between expressive power and readability.

#### 3.2.3.2. Comments

The syntax allows optional comment blocks to be interleaved with the node blocks. Comment blocks may appear before or after any node block. A comment block can't appear inside a scalar node value.

#### 3.2.3.3. Directives

Each document may be associated with a set of directives. A directive is a key:value pair where both the key and the value are simple strings. Directives are instructions to the YAML processor, allowing for extending YAML in the future. This version of YAML defines a single directive, "**YAML**". Additional directives may be added in future versions of YAML. A processor should ignore unknown directives with an appropriate warning. There is no provision for specifying private directives. This is intentional.

The "**YAML**" directive specifies the version of YAML the document adheres to. This specification defines version **1.0**. A version 1.0 processor should accept documents with an explicit "**%YAML:1.0**" directive, as well as documents lacking a "**YAML**" directive. Documents with a directive specifying a higher minor version (e.g. "**%YAML:1.1**") should be processed with an appropriate warning. Documents with a directive specifying a higher major version (e.g. "**%YAML:2.0**") should be rejected with an appropriate error message.

## 3.3. Completeness

The process of converting YAML information from a character stream presentation to a native data structure has several potential failure points. The character stream may be ill-formed, implicit tags may be unresolvable, tags may be unrecognized, the content may be invalid, and a native type may be unavailable. Each of these failures results with an incomplete conversion.

A partial representation need not specify the tag of each node, and the canonical form of scalar values need not be available. This weaker representation is useful for cases of incomplete knowledge of tags used in the document.

### 3.3.1. Well-Formed

A well-formed character stream must match the productions specified in the next chapter. A YAML processor should reject ill-formed input. A processor may recover from syntax errors, but it must provide a mechanism for reporting such errors.

### 3.3.2. Resolved

It is not required that all tags in a complete YAML representation be explicitly specified in the character stream presentation. In this case, these implicit tags must be resolved.

When resolving tags, a YAML processor must only rely upon representation details, with one notable exception. It may consider whether a scalar was written in the plain style when resolving the scalar's tag. Other than this exception, the processor must not rely upon presentation or serialization details. In particular, it must not consider key order, anchors, styles, spacing, indentation or comments.

The plain scalar style exception allows unquoted values to signify numbers, dates, or other typed data, while quoted values are treated as generic strings. With this exception, a processor may match plain scalars against a set of regular expressions, to provide automatic resolution of such types without an explict tag.

If a document contains unresolved nodes, the processor is unable to compose a complete representation graph. However, the processor may compose a partial representation, based on each node's kind (mapping, sequence, scalar) and allowing for unresolved tags.

### 3.3.3. Recognized and Valid

To be valid, a node must have a tag which is recognized by the processor and its value must satisfy the constraints imposed by its tag. If a document contains a scalar node with an unrecognized tag or an invalid value, only a partial representation may be composed. In constrast, a processor can always compose a complete YAML representation for an unrecognized or an invalid collection, since collection equality does not depend upon the collection's data type.

### 3.3.4. Available

In a given processing environment, there may not be an available native type corresponding to a given tag. If a node's tag is unavailable, a YAML processor will not be able to construct a native data structure for it. In this case, a complete YAML representation may still be composed, and an application may wish to use this representation directly.

## Chapter 4. Syntax

y[syntax.naming.b-prefix]

y[syntax.naming.c-prefix]

y[syntax.naming.e-prefix]

y[syntax.naming.l-prefix]

y[syntax.naming.nb-prefix]

y[syntax.naming.ns-prefix]

y[syntax.naming.plus-suffix]

y[syntax.naming.prefix-convention]

y[syntax.naming.s-prefix]

y[syntax.naming.xy-prefix]

y[syntax.params.chomping]

y[syntax.params.context]

y[syntax.params.definition]

y[syntax.params.indentation]

y[syntax.production.alternation]

y[syntax.production.atomic]

y[syntax.production.concatenation]

y[syntax.production.definition]

y[syntax.production.lookaround]

y[syntax.production.parenthesized]

y[syntax.production.precedence]

y[syntax.production.quantified]

y[syntax.production.special]

Following are the BNF productions defining the syntax of YAML character streams. The productions introduce the relevant character classes, describe the processing of white space, and then follow with the decomposition of the stream into logical chunks. To make this chapter easier to follow, production names use Hungarian-style notation:

**c-**
a production matching a single special character

**b-**
a production matching a single line break

**nb-**
a production matching a single non-break character

**s-**
a production matching a single non-break space character

**ns-**
a production matching a single non-break non-space character

**i-**
a production matching indentation spaces

X **-**Y **-**
a production matching a sequence of characters, starting with an X**-** production and ending with a Y**-** production

**l-**
a production matching a single line (shorthand for **i-b-**)

## 4.1. Characters


y[char.b-as-line-feed]

y[char.b-break]

y[char.b-carriage-return]

y[char.b-non-content]

y[char.c-alias]

y[char.c-anchor]

y[char.c-byte-order-mark]

y[char.c-collect-entry]

y[char.c-comment]

y[char.c-directive]

y[char.c-double-quote]

y[char.c-escape]

y[char.c-flow-indicator]

y[char.c-folded]

y[char.c-indicator]

y[char.c-literal]

y[char.c-mapping-end]

y[char.c-mapping-key]

y[char.c-mapping-start]

y[char.c-mapping-value]

y[char.c-reserved]

y[char.c-sequence-end]

y[char.c-sequence-entry]

y[char.c-sequence-start]

y[char.c-single-quote]

y[char.encoding.ascii-first]

y[char.encoding.bom-detection]

y[char.encoding.not-content]

y[char.encoding.same-encoding]

y[char.escape.must-escape]

y[char.escape.not-content]

y[char.escape.parse-to-unicode]

y[char.line-break.format-not-content]

y[char.line-break.normalize]

y[char.line-break.parse-as-lf]

y[char.misc.uri-no-expand]

y[char.nb-char]

y[char.ns-ascii-letter]

y[char.ns-char]

y[char.ns-dec-digit]

y[char.ns-esc-16-bit]

y[char.ns-esc-32-bit]

y[char.ns-esc-8-bit]

y[char.ns-esc-backslash]

y[char.ns-esc-backspace]

y[char.ns-esc-bell]

y[char.ns-esc-carriage-return]

y[char.ns-esc-double-quote]

y[char.ns-esc-escape]

y[char.ns-esc-form-feed]

y[char.ns-esc-line-feed]

y[char.ns-esc-line-separator]

y[char.ns-esc-next-line]

y[char.ns-esc-non-breaking-space]

y[char.ns-esc-null]

y[char.ns-esc-paragraph-separator]

y[char.ns-esc-space]

y[char.ns-esc-vertical-tab]

y[char.ns-hex-digit]

y[char.ns-uri-char]

y[char.ns-word-char]

y[char.s-space]

y[char.s-tab]

y[char.s-white]

y[char.set.escape-outside]

y[char.set.input-accept]

y[char.set.output-produce]

### 4.1.1. Character Set

YAML streams use a subset of the Unicode character set. On input, a YAML processor must accept all printable ASCII characters, the space, tab, line break, and all Unicode characters beyond 0x9F. On output, a YAML processor must only produce those acceptable characters, and should also escape all non-printable Unicode characters.

| [1] | c-printable | ::= | #x9 \| #xA \| #xD \| [#x20-#x7E] \| #x85 \| [#xA0-#xD7FF] \| [#xE000-#xFFFD] \| [#x10000-#x10FFFF] | /* characters as defined by the Unicode standard, excluding most control characters and the surrogate blocks */ |

This character range explicitly excludes the surrogate block **[#xD800-#xDFFF]**, DEL **0x7F**, the C0 control block **[#x0-#x1F]**, the C1 control block **[#x80-#x9F]**, **#xFFFE** and **#xFFFF**. Note that in UTF-16, characters above **#xFFFF** are represented with a surrogate pair. When present, DEL and characters in the C0 and C1 control block must be represented in a YAML stream using escape sequences.

### 4.1.2. Encoding

A YAML processor must support the UTF-16 and UTF-8 character encodings. If an input stream does not begin with a byte order mark, the encoding shall be UTF-8. UTF-16 (LE or BE) or UTF-8, as signaled by the byte order mark. Since YAML files may only contain printable characters, this does not raise any ambiguities. For more information about the byte order mark and the Unicode character encoding schemes see the Unicode [FAQ](http://www.unicode.org/unicode/faq/utf_bom.html).

| [2] | c-byte-order-mark | ::= | #xFEFF | /* unicode BOM */ |

### 4.1.3. Indicators

Indicators are special characters that are used to describe the structure of a YAML document. In general, they cannot be used as the first character of a plain scalar.

| [3] | c-sequence-start | ::= | "[" | /* starts a flow sequence collection */ |
| [4] | c-sequence-end | ::= | "]" | /* ends a flow sequence collection */ |
| [5] | c-mapping-start | ::= | "{" | /* starts a flow mapping collection */ |
| [6] | c-mapping-end | ::= | "}" | /* ends a flow mapping collection */ |
| [7] | c-sequence-entry | ::= | "-" | /* indicates a sequence entry */ |
| [8] | c-mapping-entry | ::= | ":" | /* separates a key from its value */ |
| [9] | c-collect-entry | ::= | "," | /* separates flow collection entries */ |
| [10] | c-complex-key | ::= | "?" | /* a complex key */ |
| [11] | c-tag | ::= | "!" | /* indicates a tag property */ |
| [12] | c-anchor | ::= | "&" | /* an anchor property */ |
| [13] | c-alias | ::= | "*" | /* an alias node */ |
| [14] | c-literal | ::= | "\|" | /* a literal scalar */ |
| [15] | c-folded | ::= | ">" | /* a folded scalar */ |
| [16] | c-single-quote | ::= | "'" | /* a single quoted scalar */ |
| [17] | c-double-quote | ::= | """ | /* a double quoted scalar */ |
| [18] | c-throwaway | ::= | "#" | /* a throwaway comment */ |
| [19] | c-directive | ::= | "%" | /* a directive */ |
| [20] | c-reserved | ::= | "@" \| "`" | /* reserved for future use */ |
| [21] | c-indicators | ::= | ["["](#c-sequence-start) \| ["\]"](#c-sequence-end) \| ["{"](#c-mapping-start) \| ["}"](#c-mapping-end) \| ["-"](#c-sequence-entry) \| [":"](#c-mapping-entry) \| ["?"](#c-complex-key) \| [","](#c-collect-entry) \| ["!"](#c-tag) \| ["*"](#c-alias) \| ["&"](#c-anchor) \| ["\|"](#c-literal) \| [">"](#c-folded) \| ["'"](#c-single-quote) \| ["""](#c-double-quote) \| ["#"](#c-throwaway) \| ["%"](#c-directive) \| ["@" \| "`"](#c-reserved) | /* indicator characters */ |

### 4.1.4. Line Breaks

The Unicode standard defines several line break characters. These line breaks can be grouped into two categories. Specific line breaks have well-defined semantics for breaking text into lines and paragraphs. Generic line breaks are not given meaning beyond "ending a line".

| [22] | b-line-feed | ::= | #xA | /* aSCII line feed (LF) */ |
| [23] | b-carriage-return | ::= | #xD | /* aSCII carriage return (CR) */ |
| [24] | b-next-line | ::= | #x85 | /* unicode next line (NEL) */ |
| [25] | b-line-separator | ::= | #x2028 | /* unicode line separator (LS) */ |
| [26] | b-paragraph-separator | ::= | #x2029 | /* unicode paragraph separator (PS) */ |
| [27] | b-char | ::= | [b-line-feed](#b-line-feed) \| [b-carriage-return](#b-carriage-return) \| [b-next-line](#b-next-line) \| [b-line-separator](#b-line-separator) \| [b-paragraph-separator](#b-paragraph-separator) | /* line break characters */ |
| [28] | b-generic | ::= | ( [b-carriage-return](#b-carriage-return) [b-line-feed](#b-line-feed) ) \| [b-carriage-return](#b-carriage-return) \| [b-line-feed](#b-line-feed) \| [b-next-line](#b-next-line) | /* line break with non-specific semantics */ |
| [29] | b-specific | ::= | [b-line-separator](#b-line-separator) \| [b-paragraph-separator](#b-paragraph-separator) | /* line break with specific semantics */ |
| [30] | b-any | ::= | [b-generic](#b-generic) \| [b-specific](#b-specific) | /* any non-content line break */ |

Outside scalar text content, YAML allows any line break to be used to terminate lines, and in most cases also allows such line breaks to be preceded by trailing comment characters. On output, a YAML processor is free to emit such line breaks using whatever convention is most appropriate. YAML output should avoid using trailing line spaces.

### 4.1.5. Miscellaneous

This section includes several common character range definitions.

| [31] | nb-char | ::= | [c-printable](#c-printable) - [b-char](#b-char) | /* characters valid in a line */ |
| [32] | s-char | ::= | #x9 \| #x20 | /* white space valid in a line */ |
| [33] | ns-char | ::= | [nb-char](#nb-char) - [s-char](#s-char) | /* non-space characters valid in a line */ |
| [34] | ns-ascii-letter | ::= | [#x41-#x5A] \| [#x61-#x7A] | /* aSCII letters, A-Z or a-z */ |
| [35] | ns-decimal-digit | ::= | [#x30-#x39] | /* 0-9 */ |
| [36] | ns-hex-digit | ::= | [ns-decimal-digit](#ns-decimal-digit) \| [#x41-#x46] \| [#x61-#x66] | /* 0-9, A-F or a-f */ |
| [37] | ns-word-char | ::= | [ns-decimal-digit](#ns-decimal-digit) \| [ns-ascii-letter](#ns-ascii-letter) \| "-" | /* characters valid in a word */ |

## 4.2. Space Processing


y[struct.anchor.not-content]

y[struct.b-as-space]

y[struct.b-comment]

y[struct.b-l-folded]

y[struct.b-l-trimmed]

y[struct.c-nb-comment-text]

y[struct.c-ns-anchor-property]

y[struct.c-ns-properties]

y[struct.comment.json-compat-final-break]

y[struct.comment.not-content]

y[struct.comment.separated-by-whitespace]

y[struct.comment.should-terminate-with-break]

y[struct.directive.not-content]

y[struct.flow-folding.spaces-not-content]

y[struct.global-tag-prefix.must-be-valid-uri]

y[struct.global-tag-prefix.same-semantics]

y[struct.indent.node-deeper-than-parent]

y[struct.indent.not-content]

y[struct.indent.siblings-same-level]

y[struct.indent.tab-forbidden]

y[struct.l-comment]

y[struct.l-directive]

y[struct.l-empty]

y[struct.line-prefix.not-content]

y[struct.ns-anchor-name]

y[struct.ns-directive-name]

y[struct.ns-directive-parameter]

y[struct.ns-reserved-directive]

y[struct.ns-yaml-version]

y[struct.s-b-comment]

y[struct.s-block-line-prefix]

y[struct.s-flow-folded]

y[struct.s-flow-line-prefix]

y[struct.s-indent]

y[struct.s-indent-less-or-equal]

y[struct.s-indent-less-than]

y[struct.s-l-comments]

y[struct.s-line-prefix]

y[struct.s-separate]

y[struct.s-separate-in-line]

y[struct.s-separate-lines]

y[struct.separation.indented-after-comments]

y[struct.separation.not-content]

y[struct.yaml-directive.at-most-once]

y[struct.yaml-directive.must-accept-current]

y[struct.yaml-directive.should-reject-higher-major]

y[struct.yaml-directive.should-warn-higher-minor]

YAML streams use lines and spaces to convey structure. This requires special processing rules for white space (space and tab).

### 4.2.1. Indentation

In a YAML character stream, structure is often determined from indentation, where indentation is defined as a line break character followed by zero or more space characters. With one notable exception, a node must be more indented than its parent node. All sibling nodes must use the exact same indentation level, however the content of each such node could be further indented. Indentation is used exclusively to delineate structure and is otherwise ignored; in particular, indentation characters must never be considered part of the document's content.

Tab characters are not allowed in indentation since different systems treat tabs differently. To maintain portability, YAML's tab policy is conservative; they shall not be used. Note that most modern editors may be configured so that pressing the tab key results in the insertion of an appropriate number of spaces.

| [38] | i-spaces(n) | ::= | #x20 x n | /* specific level of indentation */ |

Since the YAML stream depends upon indentation level to delineate blocks, many productions are a function of an integer, based on the [**i-spaces(n)**](#i-spaces(n)) production above. In some cases, the notations **production(<n)**, **production(≤n)** and **production(>n)** are used; these are shorthands for "**production(m)** for some specific m such that m is less than/less than or equal/greater than n", respectively. The notation **production(any)** is a shorthand for "**production(m)** for some specific value of m such that m ≥ 0".

The "**-**" sequence entry, "**?**" complex key and "**:**" mapping entry indicators are perceived by people to be part of the indentation. Hence the indentation rules are slightly more flexible when dealing with these indicators. First, a block sequence need not be indented relative to its parent node, unless that node is a block sequence entry. For example:

**Example 4.1.**

```
a key in a mapping at indentation level 0:
# The value for this key is a block sequence.
- This sequence is also at indentation level 0.
-   Another entry in the sequence.
- # The value of this entry is a nested sequence.
 - This nested sequence must be
  indented at least to level 1.
 - Another entry in the nested sequence.
- Last entry in block sequence at indentation level 0.
second key in mapping: at indentation level 0.
```

In addition, in the special case when the value of a sequence entry or complex key:value pair is a block collection, and neither the nested block collection nor its first entry have any properties specified (tag or anchor), then this first entry may be specified in the same line as the indicator of the containing sequence entry. In this case both the indicator and any following spaces are counted as part of the indentation. For example:

**Example 4.2.**

```
- This sequence is not indented.
-   inline-map: further indented by four.
    this key: is also further indented by four.
    ? - nested sequence used as key
      - indented by eight spaces
    : nested map: used as value
      indented by: six spaces
-  - inline-seq; further indented by three.
   -    second entry in nested sequence.
- Last entry in top sequence.
```

### 4.2.2. Throwaway comments

Throwaway comments have no effect whatsoever on the document's representation graph. The usual purpose of a comment is to communicate between the human maintainers of the file. A typical example is comments in a configuration file.

A throwaway comment is marked by a ["**#**"](#c-throwaway) indicator and always spans to the end of a line. Comments can be indented on their own line, or may, in some cases, follow other syntax elements with leading spaces.

Outside text content, empty lines or lines containing only white space are taken to be implicit throwaway comment lines. Lines containing indentation followed by "**#**" and comment characters are taken to be explicit throwaway comment lines.

A throwaway comment may appear before a document's top level node or following any node. It may not appear inside a scalar node, but may precede or follow it.

| [39] | c-nb-throwaway-comment | ::= | ["#"](#c-throwaway) [nb-char](#nb-char)* | /* comment trailing a line */ |
| [40] | l-comment(n) | ::= | [l-empty-comment(n)](#l-empty-comment(n)) \| [l-text-comment(n)](#l-text-comment(n)) | /* types of comment lines */ |
| [41] | l-empty-comment(n) | ::= | [i-spaces(≤n)](#i-spaces(n)) [b-any](#b-any) | /* empty throwaway comment line */ |
| [42] | l-text-comment(n) | ::= | [i-spaces(<n)](#i-spaces(n)) [c-nb-throwaway-comment](#c-nb-throwaway-comment) [b-any](#b-any) | /* explicit throwaway comment line */ |
| [43] | s-b-trailing-comment | ::= | ( [s-char](#s-char)+ [c-nb-throwaway-comment](#c-nb-throwaway-comment)? )? [b-any](#b-any) | /* trailing non-content spaces, comment and line break */ |

**Example 4.3.**

```
###The first tree lines of this stream

## are comments (the second one is empty).
this: |   # Comments may trail block indicators.
    contains three lines of text.
    The third one starts with a
    # character. This isn't a comment.

# The last three lines of this stream
# are comments (the first line is empty).
```

## 4.3. YAML Stream

A sequence of bytes is a YAML stream if, taken as a whole, it complies with the following production. Note that an empty stream is a valid YAML stream containing no documents.

Encoding is assumed to be UTF-8 unless explicitly specified by including a byte order mark as the first character of the stream. While a byte order mark may also appear before additional document headers, the same encoding must be used for all documents contained in a YAML stream.

### 4.3.1. Document

A YAML stream may contain several independent YAML documents. A document header line may be used to start a document and must be used to separate documents within a stream. This line must start with a document separator: ["**---**"](#ns-ns-document-start) followed by a line break or a sequence of space characters. If no explicit header line is specified at the start of the stream, the processor should behave as if a header line containing an unadorned "**---**" was specified.

When YAML is used as the format for a communication stream, it is useful to be able to indicate the end of a document independent of starting the next one without closing the data stream. Lacking such a marker, the YAML processor reading the stream would be forced to wait for the header of the next document (that may be long time in coming) in order to detect the end of the previous document. To support this scenario, a YAML document may be terminated by a ["**...**"](#ns-ns-document-end) line. Nothing but throwaway comments may appear between this line and the (mandatory) header line of the following document.

Since "**---**" and "**...**" indicate document boundaries, these character strings are forbidden as content lines unless they are indented.

**Example 4.4.**

```
--- >
This YAML stream contains a single text value.
The next stream is a log file - a sequence of
log entries. Adding an entry to the log is a
simple matter of appending it at the end.
```

**Example 4.5.**

```
---
at: 2001-08-12 09:25:00.00 Z
type: GET
HTTP: '1.0'
url: '/index.html'
---
at: 2001-08-12 09:25:10.00 Z
type: GET
HTTP: '1.0'
url: '/toc.html'
```

**Example 4.6.**

```
# This stream is an example of a top-level mapping.
invoice : 34843
date    : 2001-01-23
total   : 4443.52
```

**Example 4.7.**

```
# A one-line alternative syntax for the above document.
{ invoice: 34843, date: 2001-01-23, total: 4443.52 }
```

**Example 4.8.**

```
# The following is a stream of three documents.
# The first is an empty mapping, the second an
# empty sequence, and the last an empty string.
--- {}
--- [ ]
--- ''
```

**Example 4.9.**

```
# A communication channel based on a YAML stream.
---
sent at: 2002-06-06 11:46:25.10 Z
payload: Whatever
# Receiver can process this as soon as the following is sent:
...
# Even if the next message is sent long after:
---
sent at: 2002-06-06 12:05:53.47 Z
payload: Whatever
...
```

### 4.3.2. Directive

Directives are instructions to the YAML processor. Like throwaway comments, directives are not reflected in the document's representation graph. Directives apply to a single document. It is an error for the same directive to be specified more than once for the same document.

### 4.3.3. Presentation Node

A presentation node begins at a particular level of indentation, n, and its content is indented at some level > n. A presentation node can be a collection, a scalar or an alias.

A YAML document is a normal node. However a document can't be an alias (there is nothing it may refer to). Also if the header line is omitted the first document must be a collection.

### 4.3.4. Node Property

Each presentation node may have anchor and tag properties. These properties are specified in a properties list appearing before the node value itself. For a root node (a document), the properties appear in the document header line, following the directives (if any). It is an error for the same property to be specified more than once for the same node.

### 4.3.5. Tag

Tags can be presented in the character stream with the tag indicator, ["**!**"](#c-tag). Unlike anchors, tags are part of the document's representation graph. The YAML processor is responsible for resolving tags which are not present in the character stream.

**Example 4.10.**

```
a string: '12'
another string: "12"
explicit string: !str 12
explicit integer: !int 12
implicit integer: 12
```

#### 4.3.5.1. Shorthands

To increase readability, YAML does not use the full URI notation in the character stream. Instead, it provides several shorthand notations for different groups of tags. If a tag may be written using more than one shorthand, the shortest format must be used. A processor need not expand shorthand tags to a full URI form. However, in such a case the processor must still perform escaping. These rules ensure that each tag's shorthand is a globally unique.

- If a tag property is of the form **!**foo, it is a shorthand for the private tag URI **tag:private.yaml.org,2002:**foo.

**Example 4.11.**

```
# Both examples below make use of the
# 'tag:private.yaml.org,2002:ball'
# tag, but with different semantics.
---
pool: !!ball { number: 8 }
---
bearing: !!ball { material: steel }
```

- If a tag property foo contains neither "**:**" nor "**/**" characters, it is a shorthand for the tag URI **tag:yaml.org,2002:**foo. The **yaml.org** domain is used to define the core and universal YAML data types.

**Example 4.12.**

```
# The URI is 'tag:yaml.org,2002:str'
- !str is a Unicode string
```

- If the tag property is of the form vocabulary**/**foo where vocabulary is a single word, it is a shorthand for the tag URI **tag:**vocabulary**.yaml.org,2002:**foo. Each domain vocabulary**.yaml.org** is used for tags specific to the given vocabulary, such as a particular programming language.

**Example 4.13.**

```
# The URI is 'tag:perl.yaml.org,2002:Text::Tabs'
- !perl/Text::Tabs {}
```

- Otherwise, the tag property must be of the form domain**,**date**/**foo, which is a shorthand for the tag URI **tag:**domain**,**date**/**foo. To ensure uniqueness, the day must be omitted if it is the 1st of the month, and the month and day must be omitted for January 1st. Such tags may be freely minted by the owners of the domain at the specified date.

**Example 4.14.**

```
# The URI is 'tag:clarkevans.com,2003-02:timesheet'
- !clarkevans.com,2003-02/timesheet
```

Following are several examples which are not valid tag shorthands.

**Example 4.15. Invalid Shorthands**

```
- !http://www.yaml.org/bing invalid
- !tag:yaml.org,2002:str
```

Only the tag shorthand is allowed in a character stream, URIs, including the taguri is forbidden.

#### 4.3.5.2. Escaping

YAML allows arbitrary Unicode characters to be used in a tag with escape sequences. The processor must expand such escape sequences before reporting the tag's shorthand or URI to the application.

Sometimes it may be helpful for a YAML tag to be expanded to its full URI form. A YAML processor may provide a mechanism to perform such expansion. Since URIs support a limited ASCII-based character set, this expansion requires all characters outside this set to be encoded in UTF-8 and the resulting bytes to be encoded using "**%**" notation with upper-case hexadecimal digits. Further details on the URI encoding requirements are given in [RFC2396](http://www.ietf.org/rfc/rfc2396.txt).

**Example 4.16.**

```
# The following values have the same tag URI:
# 'tag:domain.tld,2002/a%3C%0A%25b'.
- !domain.tld,2002/a<\n%b value
- !domain.tld,2002/a\x3c\x0A%b value
```

#### 4.3.5.3. Prefixing

YAML provides a convenient prefix mechanism for the common case where a node and (most of) its descendents have globally unique tags, whose shorthand forms share a common prefix. If a node's tag property is of the form prefix**^**suffix, the "**^**" character is discarded from the tag. If a descendent node's tag property is of the form **^**foo, it is treated as if it was written prefixfoo where prefix comes from the most recent ancestor that established a prefix. Note that this mechanism is purely syntactical and does not imply any additional semantics. In particular, the prefix must not be assumed to be an identifier for anything. It is possible to include a "**^**" character in a tag by escaping it. It is an error for a node's tag property to contain more than one unescaped "**^**" character, or for the tag property to begin with "**^**" unless the node is a descendent of an ancestor that established a tag prefix.

**Example 4.17.**

```
# 'tag:domain.tld,2002:invoice' is some tag.
invoice: !domain.tld,2002/^invoice
  # 'seq' is shorthand for 'tag:yaml.org,2002:seq'.
  # This does not effect '^customer' below
  # because it is does not specify a prefix.
  customers: !seq
    # '^customer' is shorthand for the full notation
    # '!domain.tld,2002/customer' that stands for the
    # URI 'tag:domain.tld,2002:customer'.
    - !^customer
      given : Chris
      family : Dumars
```

### 4.3.6. Anchor

An anchor is a property that can be used to mark a node for future reference. An alias node can then be used to indicate additional inclusions of an anchored node by specifying the node's anchor.

## 4.4. Alias


y[flow.alias.error-undefined-anchor]

y[flow.alias.must-anchor-first]

y[flow.alias.must-not-specify-properties]

An alias node is a place holder for subsequent occurrences of a previously serialized node. The first occurence of the node must be marked by an anchor to allow subsequent occurences to be represented as alias nodes.

An alias refers to the most recent preceding node having the same anchor. It is an error to have an alias use an anchor that does not occur previously in the serialization of the documeht. It is not an error to have an anchor that is not used by any alias node.

**Example 4.18.**

```
anchor : &A001 This scalar has an anchor.
override : &A001 The alias node below is a repeated use of this value.
alias : *A001
```

## 4.5. Collection


y[block.b-l-spaced]

y[block.b-nb-literal-next]

y[block.c-b-block-header]

y[block.c-chomping-indicator]

y[block.c-indentation-indicator]

y[block.c-l-block-map-explicit-entry]

y[block.c-l-block-map-explicit-key]

y[block.c-l-block-map-implicit-value]

y[block.c-l-block-seq-entry]

y[block.c-l-folded]

y[block.c-l-literal]

y[block.chomping.not-content]

y[block.explicit-key-separate-value]

y[block.flow-indent-requirement]

y[block.header.comment-no-follow]

y[block.implicit-key-restrictions]

y[block.indent.emit-explicit]

y[block.indent.leading-empty-error]

y[block.indent.non-empty-line-error]

y[block.l-block-map-explicit-value]

y[block.l-block-mapping]

y[block.l-block-sequence]

y[block.l-chomped-empty]

y[block.l-folded-content]

y[block.l-keep-empty]

y[block.l-literal-content]

y[block.l-nb-diff-lines]

y[block.l-nb-folded-lines]

y[block.l-nb-literal-text]

y[block.l-nb-same-lines]

y[block.l-nb-spaced-lines]

y[block.l-strip-empty]

y[block.l-trail-comments]

y[block.ns-l-block-map-entry]

y[block.ns-l-block-map-implicit-entry]

y[block.ns-l-compact-mapping]

y[block.ns-l-compact-sequence]

y[block.ns-s-block-map-implicit-key]

y[block.properties-indent]

y[block.s-l-block-collection]

y[block.s-l-block-in-block]

y[block.s-l-block-indented]

y[block.s-l-block-node]

y[block.s-l-block-scalar]

y[block.s-l-flow-in-block]

y[block.s-nb-folded-text]

y[block.s-nb-spaced-text]

y[block.seq-space]

y[block.seq.dash-separated]

y[block.trail-comment.indent]

y[block.value-not-adjacent]

y[flow.c-double-quoted]

y[flow.c-flow-mapping]

y[flow.c-flow-sequence]

y[flow.c-ns-alias-node]

y[flow.c-ns-flow-map-empty-key-entry]

y[flow.c-quoted-quote]

y[flow.c-single-quoted]

y[flow.double-quoted.continuation-must-contain-non-space]

y[flow.e-node]

y[flow.e-scalar]

y[flow.implicit-key.must-single-line]

y[flow.nb-double-char]

y[flow.nb-double-multi-line]

y[flow.nb-double-one-line]

y[flow.nb-double-text]

y[flow.nb-ns-double-in-line]

y[flow.nb-ns-plain-in-line]

y[flow.nb-ns-single-in-line]

y[flow.nb-single-char]

y[flow.nb-single-multi-line]

y[flow.nb-single-one-line]

y[flow.nb-single-text]

y[flow.ns-double-char]

y[flow.ns-flow-content]

y[flow.ns-flow-map-entry]

y[flow.ns-flow-map-explicit-entry]

y[flow.ns-flow-map-implicit-entry]

y[flow.ns-flow-map-yaml-key-entry]

y[flow.ns-flow-node]

y[flow.ns-flow-pair]

y[flow.ns-flow-pair-entry]

y[flow.ns-flow-pair-yaml-key-entry]

y[flow.ns-flow-seq-entry]

y[flow.ns-flow-yaml-content]

y[flow.ns-flow-yaml-node]

y[flow.ns-plain]

y[flow.ns-plain-multi-line]

y[flow.ns-plain-one-line]

y[flow.ns-s-flow-map-entries]

y[flow.ns-s-flow-seq-entries]

y[flow.ns-s-implicit-yaml-key]

y[flow.ns-single-char]

y[flow.plain.continuation-must-contain-non-space]

y[flow.plain.must-not-be-empty]

y[flow.plain.must-not-begin-with-indicators]

y[flow.plain.must-not-contain-colon-space-space-hash]

y[flow.s-double-break]

y[flow.s-double-escaped]

y[flow.s-double-next-line]

y[flow.s-ns-plain-next-line]

y[flow.s-single-next-line]

y[flow.scalar-style.must-not-convey-content]

y[flow.single-quoted.continuation-must-contain-non-space]

Collection nodes come in two kinds, sequence and mapping. Each kind has two styles, block and flow. Block styles begin on the next line and use indentation for internal structure. Flow collection styles start on the current line, may span multiple lines, and rely on indicators to represent internal structure.

To enable line spanning in flow collections, wherever tokens may be separated by white space it is possible to end the line (with an optional throwaway comment) and continue the collection in the next line.

### 4.5.1. Sequence

A sequence node is an ordered collection of sub-nodes, where each subordinate node has a higher indentation level. A flow style is available for short, simple sequences. For syntax compactness, if a sub-sequence node has no properties, and its first entry is specified without any properties, the sub-sequence may immediately follow the sequence entry indicator.

**Example 4.19.**

```
empty: []
flow: [ one, two, three # May span lines,
         , four,        # indentation is
           five ]       # mostly ignored.
block:
- Note indicator is not indented.
-
 - Subordinate sequence entry (note must be indented).
 - Another entry in subordinate sequence
- - Another way to write a sub-sequence
  - Another entry in sub-sequence
- >
 A folded sequence entry (fifth entry)
```

### 4.5.2. Mapping

A mapping node is an unordered association of unique keys with values. It is an error for two equal key entries to appear in the same mapping node. In such a case the processor may continue, ignoring the second key and issuing an appropriate warning. This strategy preserves a consistent information model for streaming and random access applications.

A flow form is available for short, simple mapping nodes. For syntax compactness, if a mapping node has no properties, and its first key is specified as a flow scalar without any properties, this first key may immediately follow the sequence entry indicator.

**Example 4.20.**

```
empty: {}
null values: { one, two }
flow: { one: 1, two: 2 }
two equal maps in a sequence: [ key: value, { "key" : value } ]
spanning: { one: 1,
   two: 2 }
block:
 key : value
 nested mapping:
  key: Subordinate mapping
 nested sequence:
  - Subordinate sequence
!float 12 : This key is a float.
"\a" : This key had to be escaped.
? '?'
: This key had to be quoted.
? >
 This is a multi
 line folded key
: Whose value is
  also multi-line.
? This key has implicit null value
?
 - This key
 - is a sequence
: - With a sequence value.
? This: key
  is a: mapping
:
 with a: mapping value.
---
- A key: value pair in a sequence.
  A second: key:value pair.
- The previous entry is equal to the following one.
-
 A key:
     value pair in a sequence.
 A second:
     key:value pair.
```

## 4.6. Scalar

While most of the document productions are fairly strict, the scalar production is generous. It offers three flow style variants and two block style variants to choose from, depending upon the readability requirements.

Additionally, Throwaway comments may follow a scalar node, but may not appear inside one. The comment lines following a block scalar node must be less indented than the block scalar value. Empty lines in a scalar node that are followed by a non-empty content line are interpreted as content rather than as implicit comments. Such lines may be less indented than the text content.

### 4.6.1. End Of line Normalization

Inside all scalar nodes, a compliant YAML processor must translate the two-character combination CR LF, any CR that is not followed by an LF, and any NEL into a single LF (this does not apply to escaped characters). LS and PS characters are preserved. These rules are compatible with Unicode's newline guidelines.

On output, a YAML processor is free to serialize end of line markers using whatever convention is most appropriate, though again LS and PS must be preserved.

### 4.6.2. Block Modifiers

Each block scalar may have explicit indentation and chomping modifiers. These modifiers are specified following the block style indicator. It is an error for the same modifier to be specified more than once for the same node.

### 4.6.3. Explicit Indentation

Typically the indentation level of a block scalar node is detected from its first non-empty content line. This detection fails when this first non-empty line contains leading white space characters. Note that content lines, including the first non-empty content line, may begin with a "**#**" character.

When the first non-empty content line begins with spaces, YAML requires that the indentation level for the scalar node text content be given explicitly. This level is specified as the integer number of the additional indentation spaces used for the text content.

If the block scalar begins with lines containing only spaces, and no explicit indentation is given, the processor assumes such lines are empty lines. It is an error for any such leading empty line to contain more spaces than the indentation level that is deduced from the first non-empty content line.

The indentation level is always non-zero, except for the top level node of each document. This node is commonly indented by zero spaces (not indented). When the content is not indented, all lines up to the next document separator, document terminator, or end of the stream are assumed to be content lines, even if they begin with a "**#**" character. Note that in this case, all lines up to the next document seperator are assumed to be content lines, even if they begin with a "**#**" character.

It is always valid to specify an explicit indentation level, though a YAML processor should only do so in cases where detection fails. It is an error for detection to fail when there is no explicit indentation specified.

**Example 4.21.**

```
# Explicit indentation must be given
# in both the following cases.
leading spaces: |2
      This value starts with four spaces.

leading spaces after empty lines: |2

      This value starts with four spaces.

# The following is valid:
leading comment indicator: |

  # Content line starts with a '#'
  character, and follows empty lines.

# This is a comment because it is not
# more indented than the base level.
# Since blocks may not contain comments,
# this ends the block and the following
# empty line is not a content line.

# Explicit indentation may
# also be given when it is
# not required.
redundant: |2
  This value is indented 2 spaces.

# Indentation applies to top level nodes.
--- |
Usually top level nodes are not indented.
--- |
  This text is indented two spaces.
  It contains no leading spaces.
--- |0
  This text contains two leading spaces.
---
This text is not indented, so
# this is a content line and
--- |
  However, this is indented two spaces
# So this is a comment ending the block.
```

### 4.6.4. Chomping

Typically the final line break of a block scalar is considered to be a part of its value, and any trailing empty lines are taken to be comment lines. This default *clip* chomping behavior can be overriden by specifying a chomp control modifier.

*strip* ("**-**")
The "**-**" chomp control specifies that the final line break character of the block scalar should be stripped from its value.

*keep* ("**+**")
The "**+**" chomp control specifies that any trailing empty lines following the block scalar should be considered to be a part of its value. If this modifier is not specified, such lines are considered to be empty throwaway comment lines and are ignored.

**Example 4.22.**

```
clipped: |
    This has one newline.

same as "clipped" above: "This has one newline.\n"

stripped: |-
    This has no newline.

same as "stripped" above: "This has no newline."

kept: |+
    This has two newlines.

same as "kept" above: "This has two newlines.\n\n"
```

### 4.6.5. Literal

A literal scalar is the simplest scalar style. No processing is performed on literal scalar characters aside from end of line normalization and stripping away the indentation. Indentation is detected from the first non-empty content line. Explicit indentation must be specified in case this yields the wrong result.

Since escaping is not done, the literal style is restricted to printable characters and long lines cannot be wrapped. In exchange for these restrictions, literal scalars are the most readable format for source code or other text values with significant use of indicators, quotes, escape sequences, and line breaks.

**Example 4.23.**

```
empty: |

literal: |
 The \ ' " characters may be
 freely used. Leading white
    space is significant.

 Line breaks are significant. Thus this value
 contains one empty line and ends with a single
 line break, but does not start with one.

is equal to: "The \\ ' \" characters may \
 be\nfreely used. Leading white\n   space \
 is significant.\n\nLine breaks are \
 significant. Thus this value\ncontains \
 one empty line and ends with a single\nline \
 break, but does not start with one.\n"

# Comments may follow a block scalar value.
# They must be less indented.

# Modifiers may be combined in any order.
indented and chomped: |2-
    This has no newline.

also written as: |-2
    This has no newline.

both are equal to: "  This has no newline."
```

### 4.6.6. Folding

Folding supports scenarios where word-wrapping is useful for presentation, where the serialized content does not contain line breaks at convenient places.

When folding is done, a single normalized line feed is converted to a single space (**#x20**). When two or more consecutive (possibly indented) normalized line feeds are encountered, the processor does not convert them into spaces. Instead, the parser ignores the first line feed and preserves the rest. Thus a single line feed can be serialized as two, two line feeds can be serialized as three, etc. In this process, specific line breaks are preserved and may be safely used to convey text structure.

Since scalars come in both a block and flow variants, folding behavior must be defined in both contexts.

#### 4.6.6.1. Folding in a block context

When folding block scalars, space conversion only applies to line feeds separating text lines having a non-space starting character. Hence, folding does not apply to leading line feeds, line feeds surrounding a specific line break, or line feeds adjacent to a text line that starts with a space character.

The combined effect of the processing rules above is that each "paragraph" is interpreted as a single line, empty lines are used to represent a line feed, and "more indented" lines are preserved. Also, specific line breaks may be safely used to indicate text structure.

#### 4.6.6.2. Folding flow scalars

When folding is applied in a flow context, the process is somewhat different. Flow scalars depend on explicit indicators to convey structure, rather than indentation. Hence, in such scalars, all line space preceding or following a line break is not considered to be part of the scalar value. Hence folding flow scalars provides a more relaxed, less powerful semantics. In flow scalars, folding strips all leading and trailing white space, further, all generic line breaks are folded, even if the line was "more indented".

The combined effect of these processing rules is that each "paragraph" is interpreted as a single line, empty lines are used to represent a line feed, and text can be freely "indented" without affecting the scalar value. Again, specific line breaks may be safely used to indicate text structure.

### 4.6.7. Folded

A folded scalar is similar to a literal scalar. However, unlike a literal scalar, a folded scalar is subject to (block) line folding. This allows long lines to be broken anywhere a space character (**#x20**) appears, at the cost of requiring an empty line to represent each line feed character.

**Example 4.24.**

```
empty: >

one paragraph: >
 Line feeds are converted to spaces,
 so this value contains no line
 breaks except for the final one.

multiple paragraphs: >2

  An empty line, either at
  the start or in the value:

  Is interpreted as a line
  break. Thus this value
  contains three line breaks.

indented text: >
    This is a folded paragraph
    followed by a list:
     * first entry
     * second entry
    Followed by another folded
    paragraph, another list:

     * first entry

     * second entry

    And a final folded
    paragraph.

above is equal to: |
    This is a folded paragraph followed by a list:
     * first entry
     * second entry
    Followed by another folded paragraph, another list:

     * first entry

     * second entry

    And a final folded paragraph.
```

### 4.6.8. Single Quoted

The single quoted flow scalar style is indicated by surrounding ["**'**"](#c-single-quote) characters. Therefore, within a single quoted scalar such characters need to be escaped. No other form of escaping is done, limiting single quoted scalars to printable characters.

Single quoted scalars are subject to (flow) folding. This allows empty lines to be broken everywhere a single space character (**#x20**) separates non-space characters, at the cost of requiring an empty line to represent each line feed character.

**Example 4.25.**

```
empty: ''
second:
  '! : \ etc. can be used freely.'
third: 'a single quote '' must be escaped.'
span: 'this contains
      six spaces

      and one
      line break'
is same as: "this contains six spaces\nand one line break"
```

### 4.6.9. Escaping

Escaping allows YAML scalar nodes to specify arbitrary Unicode characters, using C-style escape codes. Non-escaped nodes are restricted to printable Unicode characters.

An escaped line break is completely ignored.

### 4.6.10. Double Quoted

The double quoted style variant adds escaping to the single quoted style variant. This is indicated by surrounding ["**"**"](#c-double-quote) characters. Escaping allows arbitrary Unicode characters to be specified at the cost of some verbosity: escaping the printable ["**\**"](#c-escape) and ["**"**"](#c-double-quote) characters. It is an error for a double quoted value to contain invalid escape sequences.

Like single quoted scalars, double quoted scalars may span multiple lines, resulting in a single space content character for each line break. If the line break is escaped, any white space preceding it is preserved, and the line break and any leading white space in the continuation line are discarded.

**Example 4.26.**

```
empty: ""
second: "! : etc. can be used freely."
third: "a \" or a \\ must be escaped."
fourth:
  "this value ends with an LF.\n"
span: "this contains
  four  \
      spaces"
is equal to: "this contains four  spaces"
```

### 4.6.11. Plain

The plain style variant is a restricted form of the single quoted style variant. As it has no identifying markers, it may not start or end with white space characters, may not start with most indicators, and may not contain certain indicators. Also, a plain scalar is subject to implicit typing. This can be avoided by providing a explicit tag property.

Since it lacks identifying markers, the restrictions on a plain scalar depends on the context. There are three different such contexts, with increasing restrictions. Top level plain values are the least restricted plain scalar format. While they can't start with most indicators, they may contain any indicator except " **#**" and ["**:** "](#c-mapping-entry). Plain scalars used in flow collections are further restricted not to contain flow indicators. Finally, plain keys are further restricted to a single line.

**Example 4.27.**

```
first: There is no unquoted empty string.
second: 12          ## This is an integer.
boolean: n          ## This is false.
third: !str 12      ## This is a string.
span: this contains
      six spaces

      and one
      line break

indicators: this has no comments.
            #:foo and bar# are
            both text.
flow: [ can span
           lines, # comment
             like
           this ]
note: { one-line keys: but
        multi-line values }
```

## Appendix A. Tag Repository

Following is a description of the three mandatory core tags. YAML requires support for the seq, map and str tags. YAML also provides a set of universal tags, that are not mandatory, in the YAML tag repository available at [https://yaml.org/spec/type.html](/spec/type.html). These tags represent native data types in most programming languages, or are useful in a wide range of applications. Therefore, applications are strongly encouraged to make use of them whenever they are appropriate, in order to improve interoperability between YAML systems.

## A.1. Sequence

URI: **tag:yaml.org,2002:seq**

Shorthand: **!seq**

Kind: Sequence.

Definition: Collections indexed by sequential integers starting with zero. Example bindings include the Perl array, Python's list or tuple, and Java's array or vector.

Resolution and Validation: This tag accepts all sequence values. It is is typically used as the fallback tag for sequence nodes.

**Example A.1.**

```
# The following are equal seqs
# with different identities.
flow: [ one, two ]
spanning: [ one,
     two ]
block:
  - one
  - two
```

## A.2. Mapping

URI: **tag:yaml.org,2002:map**

Shorthand: **!map**

Kind: Mapping.

Definition: Associative container, where each key is unique in the association and mapped to exactly one value. Example bindings include the Perl hash, Python's dictionary, and Java's Hashtable.

Resolution and Validation: This tag accepts all mapping values. It is is typically used as the fallback tag for mapping nodes.

**Example A.2.**

```
# The following are equal maps
# with different identities.
flow: { one: 1, two: 2 }
block:
    one: 1
    two: 2
```

## A.3. String

URI: **tag:yaml.org,2002:str**

Shorthand: **!str**

Kind: Scalar.

Definition: Unicode strings, a sequence of zero or more Unicode characters. This type is usually bound to the native language's string or character array construct. Note that generic YAML tools should have an immutable (const) interface to such constructs even when the language default is mutable (such as in C/C++).

Canonical Format: N/A (single format).

Resolution and Validation: This tag accepts all scalar values. It is is typically used as the fallback tag for scalar nodes.

**Example A.3.**

```
# Assuming an application
# using implicit integers.
- 12     # An integer
# The following scalars
# are loaded to the
# string value '1' '2'.
- !str 12
- '12'
- "12"
- "\
 1\
 2"
# Otherwise, everything is a string:
- /foo/bar
- 192.168.1.1
```

## Appendix B. YAML Terms

YAML defines a special meaning to the following terms:

### A

alias, anchor, application, available

### B

block

### C

canonical form, character streams, clip, collection, comment

### D

directive

### E

equality

### F

flow

### I

identity, ill-formed, implicit tags, invalid

### K

keep

### M

mapping, may, must

### O

optional

### P

partial representation, processor

### R

recognized, recommended, required, resolved

### S

scalar, sequence, shall, should, strip, style

### T

tag

### U

unavailable, unrecognized, unresolved

### V

valid

### W

well-formed
