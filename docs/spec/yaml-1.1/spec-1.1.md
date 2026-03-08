::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.book lang="en"}
::::::::::::::::::: titlepage
:::::::::::::::::: {}
::: {}
# []{#id664002} YAML Ain't Markup Language ([YAML]{.trademark}™) Version 1.1 {#yaml-aint-markup-language-yaml-version-1.1 .title}
:::

::: {}
## Final Draft \-- 2005-01-18 {#final-draft----2005-01-18 .subtitle}
:::

::::::: {}
:::::: authorgroup
::: author
### [Oren]{.firstname} [Ben-Kiki]{.surname} {#oren-ben-kiki .author}

`<`{.email}[`oren@ben-kiki.org`{.email}](mailto:oren@ben-kiki.org)`>`{.email}
:::

::: author
### [Clark]{.firstname} [Evans]{.surname} {#clark-evans .author}

`<`{.email}[`cce@clarkevans.com`{.email}](mailto:cce@clarkevans.com)`>`{.email}
:::

::: author
### [Ingy]{.firstname} [döt Net]{.surname} {#ingy-döt-net .author}

`<`{.email}[`ingy@ingy.net`{.email}](mailto:ingy@ingy.net)`>`{.email}
:::
::::::
:::::::

::: {}
[*This version:*]{.emphasis} [html](/spec/cvs/current.html){target="_top"}, [ps](/spec/cvs/current.ps){target="_top"}, [pdf](/spec/cvs/current.pdf){target="_top"}.\
[*Latest version:*]{.emphasis} [html](/spec/current.html){target="_top"}, [ps](/spec/current.ps){target="_top"}, [pdf](/spec/current.pdf){target="_top"}.
:::

::: {}
Copyright © 2001-2008 Oren Ben-Kiki, Clark Evans, Ingy döt Net
:::

:::: {}
::: legalnotice
[]{#id838350} This document may be freely copied, provided it is not modified.
:::
::::

:::: {}
::: abstract
**Status of this Document**

This specification is a draft reflecting consensus reached by members of the [yaml-core mailing list](http://lists.sourceforge.net/lists/listinfo/yaml-core){target="_top"}. Any questions regarding this draft should be raised on this list. We expect all further changes to be strictly limited to wording corrections and fixing production bugs.

We wish to thank implementers, who have tirelessly tracked earlier versions of this specification, as well as our fabulous user community whose feedback has both validated and clarified our direction.
:::
::::

:::: {}
::: abstract
**Abstract**

[YAML]{.trademark}™ (rhymes with "[camel]{.quote}") is a human-friendly, cross language, Unicode based data serialization language designed around the common native data structures of agile programming languages. It is broadly useful for programming needs ranging from configuration files to Internet messaging to object persistence to data auditing. Together with the [Unicode standard for characters](http://www.unicode.org/){target="_top"}, this specification provides all the information necessary to understand YAML Version 1.1 and to create programs that process YAML information.
:::
::::
::::::::::::::::::

------------------------------------------------------------------------
:::::::::::::::::::

::: toc
**Table of Contents**

[ [1. Introduction](#id838426) ]{.chapter}

[ [1.1. Goals](#id838638) ]{.sect1}

[ [1.2. Prior Art](#id838686) ]{.sect1}

[ [1.3. Relation to XML](#id856927) ]{.sect1}

[ [1.4. Terminology](#id856957) ]{.sect1}

[ [2. Preview](#id857168) ]{.chapter}

[ [2.1. Collections](#id857181) ]{.sect1}

[ [2.2. Structures](#id857577) ]{.sect1}

[ [2.3. Scalars](#id858081) ]{.sect1}

[ [2.4. Tags](#id858600) ]{.sect1}

[ [2.5. Full Length Example](#id859040) ]{.sect1}

[ [3. Processing YAML Information](#id859109) ]{.chapter}

[ [3.1. Processes](#id859458) ]{.sect1}

[ [3.1.1. Represent](#id859497) ]{.sect2}

[ [3.1.2. Serialize](#id859873) ]{.sect2}

[ [3.1.3. Present](#id860109) ]{.sect2}

[ [3.1.4. Parse](#id860341) ]{.sect2}

[ [3.1.5. Compose](#id860452) ]{.sect2}

[ [3.1.6. Construct](#id860557) ]{.sect2}

[ [3.2. Information Models](#id860735) ]{.sect1}

[ [3.2.1. Representation Graph](#id861060) ]{.sect2}

[ [3.2.1.1. Nodes](#id861435) ]{.sect3}

[ [3.2.1.2. Tags](#id861700) ]{.sect3}

[ [3.2.1.3. Nodes Comparison](#id862121) ]{.sect3}

[ [3.2.2. Serialization Tree](#id862929) ]{.sect2}

[ [3.2.2.1. Keys Order](#id863110) ]{.sect3}

[ [3.2.2.2. Anchors and Aliases](#id863390) ]{.sect3}

[ [3.2.3. Presentation Stream](#id863676) ]{.sect2}

[ [3.2.3.1. Node Styles](#id863975) ]{.sect3}

[ [3.2.3.2. Scalar Formats](#id864510) ]{.sect3}

[ [3.2.3.3. Comments](#id864687) ]{.sect3}

[ [3.2.3.4. Directives](#id864824) ]{.sect3}

[ [3.3. Loading Failure Points](#id864977) ]{.sect1}

[ [3.3.1. Well-Formed and Identified](#id865423) ]{.sect2}

[ [3.3.2. Resolved](#id865585) ]{.sect2}

[ [3.3.3. Recognized and Valid](#id866900) ]{.sect2}

[ [3.3.4. Available](#id867229) ]{.sect2}

[ [4. Productions Conventions](#id867381) ]{.chapter}

[ [4.1. Production Prefixes](#id867562) ]{.sect1}

[ [4.2. Production Parameters](#id867808) ]{.sect1}

[ [5. Characters](#id868518) ]{.chapter}

[ [5.1. Character Set](#id868524) ]{.sect1}

[ [5.2. Character Encoding](#id868742) ]{.sect1}

[ [5.3. Indicator Characters](#id868988) ]{.sect1}

[ [5.4. Line Break Characters](#id871136) ]{.sect1}

[ [5.5. Miscellaneous Characters](#id871856) ]{.sect1}

[ [5.6. Escape Sequences](#id872840) ]{.sect1}

[ [6. Syntax Primitives](#id891745) ]{.chapter}

[ [6.1. Indentation Spaces](#id891751) ]{.sect1}

[ [6.2. Comments](#id892353) ]{.sect1}

[ [6.3. Separation Spaces](#id893014) ]{.sect1}

[ [6.4. Ignored Line Prefix](#id893482) ]{.sect1}

[ [6.5. Line Folding](#id894136) ]{.sect1}

[ [7. YAML Character Stream](#id895107) ]{.chapter}

[ [7.1. Directives](#id895217) ]{.sect1}

[ [7.1.1. "[**`YAML`**]{.quote}" Directive](#id895631) ]{.sect2}

[ [7.1.2. "[**`TAG`**]{.quote}" Directive](#id896044) ]{.sect2}

[ [7.1.2.1. Tag Prefixes](#id896411) ]{.sect3}

[ [7.1.2.2. Tag Handles](#id896876) ]{.sect3}

[ [7.2. Document Boundary Markers](#id897596) ]{.sect1}

[ [7.3. Documents](#id898031) ]{.sect1}

[ [7.4. Complete Stream](#id898785) ]{.sect1}

[ [8. Nodes](#id899622) ]{.chapter}

[ [8.1. Node Anchors](#id899912) ]{.sect1}

[ [8.2. Node Tags](#id900262) ]{.sect1}

[ [8.3. Node Content](#id901659) ]{.sect1}

[ [8.4. Alias Nodes](#id902561) ]{.sect1}

[ [8.5. Complete Nodes](#id902919) ]{.sect1}

[ [8.5.1. Flow Nodes](#id902924) ]{.sect2}

[ [8.5.2. Block Nodes](#id903421) ]{.sect2}

[ [9. Scalar Styles](#id903915) ]{.chapter}

[ [9.1. Flow Scalar Styles](#id904158) ]{.sect1}

[ [9.1.1. Double Quoted](#id904245) ]{.sect2}

[ [9.1.2. Single Quoted](#id905860) ]{.sect2}

[ [9.1.3. Plain](#id907281) ]{.sect2}

[ [9.2. Block Scalar Header](#id926597) ]{.sect1}

[ [9.2.1. Block Style Indicator](#id926836) ]{.sect2}

[ [9.2.2. Block Indentation Indicator](#id927035) ]{.sect2}

[ [9.2.3. Block Chomping Indicator](#id927557) ]{.sect2}

[ [9.3. Block Scalar Styles](#id928806) ]{.sect1}

[ [9.3.1. Literal](#id928909) ]{.sect2}

[ [9.3.2. Folded](#id929764) ]{.sect2}

[ [10. Collection Styles](#id930798) ]{.chapter}

[ [10.1. Sequence Styles](#id931088) ]{.sect1}

[ [10.1.1. Flow Sequences](#id931285) ]{.sect2}

[ [10.1.2. Block Sequences](#id931893) ]{.sect2}

[ [10.2. Mapping Styles](#id932806) ]{.sect1}

[ [10.2.1. Flow Mappings](#id933010) ]{.sect2}

[ [10.2.2. Block Mappings](#id934537) ]{.sect2}

[ [Index](#id935693) ]{.appendix}
:::

:::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id838426}Chapter 1. Introduction {#chapter-1.-introduction .title}
:::
::::
:::::

"[YAML Ain't Markup Language]{.quote}" (abbreviated YAML) is a data serialization language designed to be human-friendly and work well with modern programming languages for common everyday tasks. This specification is both an introduction to the YAML language and the concepts supporting it; it is also a complete reference of the information needed to develop []{#id838445 .indexterm}[applications](#application/) for processing YAML.

Open, interoperable and readily understandable tools have advanced computing immensely. YAML was designed from the start to be useful and friendly to people working with data. It uses Unicode []{#id838465 .indexterm}[printable](#printable%20character/) characters, some of which provide structural information and the rest containing the data itself. YAML achieves a unique cleanness by minimizing the amount of structural characters and allowing the data to show itself in a natural and meaningful way. For example, []{#id838485 .indexterm}[indentation](#indentation%20space/) may be used for structure, colons separate "[[]{#id838503 .indexterm}[mapping key:](#key/information%20model) []{#id838518 .indexterm}[value](#value/information%20model)]{.quote}" pairs, and dashes are used to create "[bullet]{.quote}" lists.

There are myriad flavors of data structures, but they can all be adequately []{#id838545 .indexterm}[represented](#represent/) with three basic primitives: []{#id838557 .indexterm}[mappings](#mapping/information%20model) (hashes/dictionaries), []{#id838576 .indexterm}[sequences](#sequence/information%20model) (arrays/lists) and []{#id838592 .indexterm}[scalars](#scalar/information%20model) (strings/numbers). YAML leverages these primitives and adds a simple typing system and []{#id838612 .indexterm}[aliasing](#alias/information%20model) mechanism to form a complete language for serializing any data structure. While most programming languages can use YAML for data serialization, YAML excels in working with those languages that are fundamentally built around the three basic primitives. These include the new wave of agile languages such as Perl, Python, PHP, Ruby, and Javascript.

There are hundreds of different languages for programming, but only a handful of languages for storing and transferring data. Even though its potential is virtually boundless, YAML was specifically created to work well for common use cases such as: configuration files, log files, interprocess messaging, cross-language data sharing, object persistence, and debugging of complex data structures. When data is easy to view and understand, programming becomes a simpler task.

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id838638}1.1. Goals {#goals .title style="clear: both"}
:::
::::
:::::

The design goals for YAML are:

::: orderedlist
1.  YAML is easily readable by humans.
2.  YAML matches the native data structures of agile languages.
3.  YAML data is portable between programming languages.
4.  YAML has a consistent model to support generic tools.
5.  YAML supports one-pass processing.
6.  YAML is expressive and extensible.
7.  YAML is easy to implement and use.
:::
:::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id838686}1.2. Prior Art {#prior-art .title style="clear: both"}
:::
::::
:::::

YAML's initial direction was set by the data serialization and markup language discussions among [SML-DEV members](http://www.docuverse.com/smldev/){target="_top"}. Later on, it directly incorporated experience from Brian Ingerson's Perl module [Data::Denter](http://search.cpan.org/doc/INGY/Data-Denter-0.13/Denter.pod){target="_top"}. Since then, YAML has matured through ideas and support from its user community.

YAML integrates and builds upon concepts described by [C](http://cm.bell-labs.com/cm/cs/cbook/index.html){target="_top"}, [Java](http://java.sun.com/){target="_top"}, [Perl](http://www.perl.org/){target="_top"}, [Python](http://www.python.org/){target="_top"}, [Ruby](http://www.ruby-lang.org/){target="_top"}, [RFC0822](http://www.ietf.org/rfc/rfc0822.txt){target="_top"} (MAIL), [RFC1866](http://www.ics.uci.edu/pub/ietf/html/rfc1866.txt){target="_top"} (HTML), [RFC2045](http://www.ietf.org/rfc/rfc2045.txt){target="_top"} (MIME), [RFC2396](http://www.ietf.org/rfc/rfc2396.txt){target="_top"} (URI), [XML](http://www.w3.org/TR/REC-xml.html){target="_top"}, [SAX](http://www.saxproject.org/){target="_top"} and [SOAP](http://www.w3.org/TR/SOAP){target="_top"}.

The syntax of YAML was motivated by Internet Mail (RFC0822) and remains partially compatible with that standard. Further, borrowing from MIME (RFC2045), YAML's top-level production is a []{#id838704 .indexterm}[stream](#stream/information%20model) of independent []{#id838821 .indexterm}[documents](#document/information%20model); ideal for message-based distributed processing systems.

YAML's []{#id838828 .indexterm}[indentation](#indentation%20space/)-based scoping is similar to Python's (without the ambiguities caused by []{#id838859 .indexterm}[tabs](#tab/)). []{#id856349 .indexterm}[Indented blocks](#block%20style/information%20model) facilitate easy inspection of the data's structure. YAML's []{#id856368 .indexterm}[literal style](#literal%20style/information%20model) leverages this by enabling formatted text to be cleanly mixed within an []{#id856385 .indexterm}[indented](#indentation%20space/) structure without troublesome []{#id856400 .indexterm}[escaping](#escaping%20in%20double-quoted%20style/). YAML also allows the use of traditional []{#id856418 .indexterm}[indicator](#indicator/)-based scoping similar to Perl's. Such []{#id856431 .indexterm}[flow content](#flow%20style/information%20model) can be freely nested inside []{#id856448 .indexterm}[indented blocks](#block%20style/information%20model).

YAML's []{#id856456 .indexterm}[double-quoted style](#double-quoted%20style/information%20model) uses familiar C-style []{#id856488 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/). This enables ASCII encoding of non-[]{#id856500 .indexterm}[printable](#printable%20character/) or 8-bit (ISO 8859-1) characters such as ["[**`\x3B`**]{.quote}"](#ns-esc-8-bit). Non-[]{#id856526 .indexterm}[printable](#printable%20character/) 16-bit Unicode and 32-bit (ISO/IEC 10646) characters are supported with []{#id856541 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/) such as ["[**`\u003B`**]{.quote}"](#ns-esc-16-bit) and ["[**`\U0000003B`**]{.quote}"](#ns-esc-32-bit).

Motivated by HTML's end-of-line normalization, YAML's []{#id856583 .indexterm}[line folding](#line%20folding/) employs an intuitive method of handling []{#id856600 .indexterm}[line breaks](#line%20break%20character/). A single []{#id856614 .indexterm}[line break](#line%20break%20character/) is []{#id856628 .indexterm}[folded](#line%20folding/) into a single space, while []{#id856641 .indexterm}[empty lines](#empty%20line/) are interpreted as []{#id856655 .indexterm}[line break](#line%20break%20character/) characters. This technique allows for paragraphs to be word-wrapped without affecting the []{#id856670 .indexterm}[canonical form](#canonical%20form/) of the []{#id856683 .indexterm}[content](#content/information%20model).

YAML's core type system is based on the requirements of agile languages such as Perl, Python, and Ruby. YAML directly supports both []{#id856711 .indexterm}[collection](#collection/information%20model) ([]{#id856728 .indexterm}[mapping](#mapping/information%20model), []{#id856742 .indexterm}[sequence](#sequence/information%20model)) and []{#id856758 .indexterm}[scalar content](#scalar/information%20model). Support for common types enables programmers to use their language's native data structures for YAML manipulation, instead of requiring a special document object model (DOM).

Like XML's SOAP, YAML supports []{#id856780 .indexterm}[serializing](#serialize/) native graph data structures through an []{#id856793 .indexterm}[aliasing](#alias/information%20model) mechanism. Also like SOAP, YAML provides for []{#id856812 .indexterm}[application](#application/)-defined []{#id856822 .indexterm}[types](#tag/information%20model). This allows YAML to []{#id856839 .indexterm}[represent](#represent/) rich data structures required for modern distributed computing. YAML provides globally unique []{#id856853 .indexterm}[type names](#global%20tag/) using a namespace mechanism inspired by Java's DNS-based package naming convention and XML's URI-based namespaces.

YAML was designed to support incremental interfaces that include both input ("[**`getNextEvent()`**]{.quote}") and output "[**`sendNextEvent()`**]{.quote}") one-pass interfaces. Together, these enable YAML to support the processing of large []{#id856894 .indexterm}[documents](#document/information%20model) (e.g. transaction logs) or continuous []{#id856909 .indexterm}[streams](#stream/information%20model) (e.g. feeds from a production machine).
::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id856927}1.3. Relation to XML {#relation-to-xml .title style="clear: both"}
:::
::::
:::::

Newcomers to YAML often search for its correlation to the eXtensible Markup Language (XML). Although the two languages may actually compete in several application domains, there is no direct correlation between them.

YAML is primarily a data serialization language. XML was designed to be backwards compatible with the Standard Generalized Markup Language (SGML) and thus had many design constraints placed on it that YAML does not share. Inheriting SGML's legacy, XML is designed to support structured documentation, where YAML is more closely targeted at data structures and messaging. Where XML is a pioneer in many domains, YAML is the result of lessons learned from XML and other technologies.

It should be mentioned that there are ongoing efforts to define standard XML/YAML mappings. This generally requires that a subset of each language be used. For more information on using both XML and YAML, please visit [https://yaml.org/xml/index.html](/xml/index.html){target="_top"}.
::::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id856957}1.4. Terminology {#terminology .title style="clear: both"}
:::
::::
:::::

This specification uses key words based on [RFC2119](http://www.ietf.org/rfc/rfc2119.txt){target="_top"} to indicate requirement level. In particular, the following words are used to describe the actions of a YAML []{#id856974 .indexterm}[processor](#processor/):

::: variablelist

[May]{.term}
:   The word []{#id856999 .indexterm}[]{#may/}*may*, or the adjective []{#id857013 .indexterm}[]{#optional/}*optional*, mean that conforming YAML []{#id857027 .indexterm}[processors](#processor/) are permitted, but []{#id857040 .indexterm}[]{#need not/}*need not* behave as described.

[Should]{.term}
:   The word []{#id857065 .indexterm}[]{#should/}*should*, or the adjective []{#id857079 .indexterm}[]{#recommended/}*recommended*, mean that there could be reasons for a YAML []{#id857093 .indexterm}[processor](#processor/) to deviate from the behavior described, but that such deviation could hurt interoperability and should therefore be advertised with appropriate notice.

[Must]{.term}
:   The word []{#id857120 .indexterm}[]{#must/}*must*, or the term []{#id857133 .indexterm}[]{#required/}*required* or []{#id857147 .indexterm}[]{#shall/}*shall*, mean that the behavior described is an absolute requirement of the specification.
:::
:::::::
::::::::::::::::::::::::

:::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id857168}Chapter 2. Preview {#chapter-2.-preview .title}
:::
::::
:::::

This section provides a quick glimpse into the expressive power of YAML. It is not expected that the first-time reader grok all of the examples. Rather, these selections are used as motivation for the remainder of the specification.

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id857181}2.1. Collections {#collections .title style="clear: both"}
:::
::::
:::::

YAML's []{#id857190 .indexterm}[block collections](#block%20collection%20style/information%20model) use []{#id857209 .indexterm}[indentation](#indentation%20space/) for scope and begin each entry on its own line. []{#id857222 .indexterm}[Block sequences](#block%20sequence%20style/information%20model) indicate each entry with a dash and space ( []{#id857230 .indexterm}["[**`-`**]{.quote}"](#-%20block%20sequence%20entry/)). []{#id857262 .indexterm}[Mappings](#mapping/information%20model) use a colon and space ([]{#id857279 .indexterm}["[**`: `**]{.quote}"](#:%20mapping%20value/)) to mark each []{#id857298 .indexterm}[mapping key](#key/information%20model): []{#id857312 .indexterm}[value](#value/information%20model) pair.

+-----------------------------------------------+---------------------------------------------+
| ::: example                                   | ::: example                                 |
| []{#id857339}                                 | []{#id857364}                               |
|                                               |                                             |
| **Example 2.1.  Sequence of Scalars\          | **Example 2.2.  Mapping Scalars to Scalars\ |
| (ball players)**                              | (player statistics)**                       |
|                                               |                                             |
| ``` programlisting                            | ``` programlisting                          |
| - Mark McGwire                                | hr:  65    # Home runs                      |
| - Sammy Sosa                                  | avg: 0.278 # Batting average                |
| - Ken Griffey                                 | rbi: 147   # Runs Batted In                 |
| ```                                           | ```                                         |
| :::                                           | :::                                         |
+-----------------------------------------------+---------------------------------------------+
| ::: example                                   | ::: example                                 |
| []{#id857390}                                 | []{#id857416}                               |
|                                               |                                             |
| **Example 2.3.  Mapping Scalars to Sequences\ | **Example 2.4.  Sequence of Mappings\       |
| (ball clubs in each league)**                 | (players' statistics)**                     |
|                                               |                                             |
| ``` programlisting                            | ``` programlisting                          |
| american:                                     | -                                           |
|   - Boston Red Sox                            |   name: Mark McGwire                        |
|   - Detroit Tigers                            |   hr:   65                                  |
|   - New York Yankees                          |   avg:  0.278                               |
| national:                                     | -                                           |
|   - New York Mets                             |   name: Sammy Sosa                          |
|   - Chicago Cubs                              |   hr:   63                                  |
|   - Atlanta Braves                            |   avg:  0.288                               |
| ```                                           | ```                                         |
| :::                                           | :::                                         |
+-----------------------------------------------+---------------------------------------------+

YAML also has []{#id857442 .indexterm}[flow styles](#flow%20style/information%20model), using explicit []{#id857460 .indexterm}[indicators](#indicator/) rather than []{#id857471 .indexterm}[indentation](#indentation%20space/) to denote scope. The []{#id857485 .indexterm}[flow sequence](#flow%20sequence%20style/information%20model) is written as a comma separated list within square brackets. In a similar manner, the []{#id857505 .indexterm}[flow mapping](#flow%20mapping%20style/information%20model) uses curly braces.

+----------------------------------------+--------------------------------------+
| ::: example                            | ::: example                          |
| []{#id857532}                          | []{#id857555}                        |
|                                        |                                      |
| **Example 2.5. Sequence of Sequences** | **Example 2.6. Mapping of Mappings** |
|                                        |                                      |
| ``` programlisting                     | ``` programlisting                   |
| - [name        , hr, avg  ]            | Mark McGwire: {hr: 65, avg: 0.278}   |
| - [Mark McGwire, 65, 0.278]            | Sammy Sosa: {                        |
| - [Sammy Sosa  , 63, 0.288]            |     hr: 63,                          |
|                                        |     avg: 0.288                       |
| ```                                    |   }                                  |
| :::                                    | ```                                  |
|                                        | :::                                  |
+----------------------------------------+--------------------------------------+
::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id857577}2.2. Structures {#structures .title style="clear: both"}
:::
::::
:::::

YAML uses three dashes ([]{#id857587 .indexterm}["[**`---`**]{.quote}"](#document%20boundary%20marker/)) to separate []{#id857607 .indexterm}[documents](#document/information%20model) within a []{#id857621 .indexterm}[stream](#stream/information%20model). Three dots ( []{#id857629 .indexterm}["[**`...`**]{.quote}"](#document%20boundary%20marker/)) indicate the end of a document without starting a new one, for use in communication channels. []{#id857658 .indexterm}[Comment](#comment/information%20model) lines begin with the Octothorpe (also called "[hash]{.quote}", "[sharp]{.quote}", or "[number sign]{.quote}" - []{#id857687 .indexterm}["[**`#`**]{.quote}"](##%20comment/)).

+--------------------------------------------+------------------------------------+
| ::: example                                | ::: example                        |
| []{#id857714}                              | []{#id857738}                      |
|                                            |                                    |
| **Example 2.7.  Two Documents in a Stream\ | **Example 2.8.  Play by Play Feed\ |
| (each with a leading comment)**            | from a Game**                      |
|                                            |                                    |
| ``` programlisting                         | ``` programlisting                 |
| # Ranking of 1998 home runs                | ---                                |
| ---                                        | time: 20:03:20                     |
| - Mark McGwire                             | player: Sammy Sosa                 |
| - Sammy Sosa                               | action: strike (miss)              |
| - Ken Griffey                              | ...                                |
|                                            | ---                                |
| # Team ranking                             | time: 20:03:47                     |
| ---                                        | player: Sammy Sosa                 |
| - Chicago Cubs                             | action: grand slam                 |
| - St Louis Cardinals                       | ...                                |
| ```                                        | ```                                |
| :::                                        | :::                                |
+--------------------------------------------+------------------------------------+

Repeated []{#id857766 .indexterm}[nodes](#node/information%20model) are first []{#id857785 .indexterm}[identified](#identified/) by an []{#id857797 .indexterm}[anchor](#anchor/information%20model) (marked with the ampersand - []{#id857816 .indexterm}["[**`&`**]{.quote}"](#&%20anchor/)), and are then []{#id857834 .indexterm}[aliased](#alias/information%20model) (referenced with an asterisk - []{#id857852 .indexterm}["[**`*`**]{.quote}"](#*%20alias/)) thereafter.

+---------------------------------------+---------------------------------------------------------+
| ::: example                           | ::: example                                             |
| []{#id857879}                         | []{#id857905}                                           |
|                                       |                                                         |
| **Example 2.9.  Single Document with\ | **Example 2.10.  Node for "[**`Sammy Sosa`**]{.quote}"\ |
| Two Comments**                        | appears twice in this document**                        |
|                                       |                                                         |
| ``` programlisting                    | ``` programlisting                                      |
| ---                                   | ---                                                     |
| hr: # 1998 hr ranking                 | hr:                                                     |
|   - Mark McGwire                      |   - Mark McGwire                                        |
|   - Sammy Sosa                        |   # Following node labeled SS                           |
| rbi:                                  |   - &SS Sammy Sosa                                      |
|   # 1998 rbi ranking                  | rbi:                                                    |
|   - Sammy Sosa                        |   - *SS # Subsequent occurrence                         |
|   - Ken Griffey                       |   - Ken Griffey                                         |
| ```                                   | ```                                                     |
| :::                                   | :::                                                     |
+---------------------------------------+---------------------------------------------------------+

A question mark and space []{#id857930 .indexterm}[("[**`? `**]{.quote}"](#?%20mapping%20key/)) indicate a complex []{#id857962 .indexterm}[mapping key](#key/information%20model). Within a []{#id857977 .indexterm}[block collection](#block%20collection%20style/information%20model), []{#id857993 .indexterm}[key:](#key/information%20model) []{#id858010 .indexterm}[value](#value/information%20model) pairs can start immediately following the dash, colon, or question mark.

+---------------------------------------------+------------------------------------------+
| ::: example                                 | ::: example                              |
| []{#id858035}                               | []{#id858058}                            |
|                                             |                                          |
| **Example 2.11. Mapping between Sequences** | **Example 2.12. In-Line Nested Mapping** |
|                                             |                                          |
| ``` programlisting                          | ``` programlisting                       |
| ? - Detroit Tigers                          | ---                                      |
|   - Chicago cubs                            | # products purchased                     |
| :                                           | - item    : Super Hoop                   |
|   - 2001-07-23                              |   quantity: 1                            |
|                                             | - item    : Basketball                   |
| ? [ New York Yankees,                       |   quantity: 4                            |
|     Atlanta Braves ]                        | - item    : Big Shoes                    |
| : [ 2001-07-02, 2001-08-12,                 |   quantity: 1                            |
|     2001-08-14 ]                            | ```                                      |
| ```                                         | :::                                      |
| :::                                         |                                          |
+---------------------------------------------+------------------------------------------+
::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id858081}2.3. Scalars {#scalars .title style="clear: both"}
:::
::::
:::::

[]{#id858089 .indexterm}[Scalar content](#scalar/information%20model) can be written in []{#id858105 .indexterm}[block](#block%20style/information%20model) form, using a []{#id858121 .indexterm}[literal style](#literal%20style/information%20model) ([]{#id858137 .indexterm}["[**`|`**]{.quote}"](#%7C%20literal%20style/)) where all []{#id858158 .indexterm}[line breaks](#line%20break%20character/) are significant. Alternatively, they can be written with the []{#id858173 .indexterm}[folded style](#folded%20style/information%20model) []{#id858190 .indexterm}[("[**`>`**]{.quote}"](#%3E%20folded%20style/)) where each []{#id858210 .indexterm}[line break](#line%20break%20character/) is []{#id858222 .indexterm}[folded](#line%20folding/) to a space unless it ends an []{#id858234 .indexterm}[empty](#empty%20line/) or a []{#id858247 .indexterm}["[more indented]{.quote}" line](#more%20indented%20line/).

+-------------------------------------------------+------------------------------------------------+
| ::: example                                     | ::: example                                    |
| []{#id858273}                                   | []{#id858298}                                  |
|                                                 |                                                |
| **Example 2.13.  In literals,\                  | **Example 2.14.  In the plain scalar,\         |
| newlines are preserved**                        | newlines become spaces**                       |
|                                                 |                                                |
| ``` programlisting                              | ``` programlisting                             |
| # ASCII Art                                     | ---                                            |
| --- |                                           |   Mark McGwire's                               |
|   \//||\/||                                     |   year was crippled                            |
|   // ||  ||__                                   |   by a knee injury.                            |
| ```                                             | ```                                            |
| :::                                             | :::                                            |
+-------------------------------------------------+------------------------------------------------+
| ::: example                                     | ::: example                                    |
| []{#id858323}                                   | []{#id858350}                                  |
|                                                 |                                                |
| **Example 2.15.  Folded newlines are preserved\ | **Example 2.16.  Indentation determines scope\ |
| for \"more indented\" and blank lines**         |  **                                            |
|                                                 |                                                |
| ``` programlisting                              | ``` programlisting                             |
| >                                               | name: Mark McGwire                             |
|  Sammy Sosa completed another                   | accomplishment: >                              |
|  fine season with great stats.                  |   Mark set a major league                      |
|                                                 |   home run record in 1998.                     |
|    63 Home Runs                                 | stats: |                                       |
|    0.288 Batting Average                        |   65 Home Runs                                 |
|                                                 |   0.278 Batting Average                        |
|  What a year!                                   | ```                                            |
| ```                                             | :::                                            |
| :::                                             |                                                |
+-------------------------------------------------+------------------------------------------------+

YAML's []{#id858382 .indexterm}[flow scalars](#flow%20scalar%20style/information%20model) include the []{#id858402 .indexterm}[plain style](#plain%20style/information%20model) (most examples thus far) and []{#id858420 .indexterm}[quoted styles](#quoted%20style/information%20model). The []{#id858436 .indexterm}[double-quoted style](#double-quoted%20style/information%20model) provides []{#id858453 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/). The []{#id858467 .indexterm}[single-quoted style](#single-quoted%20style/information%20model) is useful when []{#id858486 .indexterm}[escaping](#escaping%20in%20double-quoted%20style/) is not needed. All []{#id858499 .indexterm}[flow scalars](#flow%20scalar%20style/information%20model) can span multiple lines; []{#id858516 .indexterm}[line breaks](#line%20break%20character/) are always []{#id858529 .indexterm}[folded](#line%20folding/).

+-----------------------------------+-------------------------------------------+
| ::: example                       | ::: example                               |
| []{#id858553}                     | []{#id858577}                             |
|                                   |                                           |
| **Example 2.17. Quoted Scalars**  | **Example 2.18. Multi-line Flow Scalars** |
|                                   |                                           |
| ``` programlisting                | ``` programlisting                        |
| unicode: "Sosa did fine.\u263A"   | plain:                                    |
| control: "\b1998\t1999\t2000\n"   |   This unquoted scalar                    |
| hexesc:  "\x13\x10 is \r\n"       |   spans many lines.                       |
|                                   |                                           |
| single: '"Howdy!" he cried.'      | quoted: "So does this                     |
| quoted: ' # not a ''comment''.'   |   quoted scalar.\n"                       |
| tie-fighter: '|\-*-/|'            | ```                                       |
| ```                               | :::                                       |
| :::                               |                                           |
+-----------------------------------+-------------------------------------------+
::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id858600}2.4. Tags {#tags .title style="clear: both"}
:::
::::
:::::

In YAML, []{#id858608 .indexterm}[untagged nodes](#non-specific%20tag/) are given an type depending on the []{#id858622 .indexterm}[application](#application/). The examples in this specification generally use the ["[**`seq`**]{.quote}"](/type/seq.html){target="_top"}, ["[**`map`**]{.quote}"](/type/map.html){target="_top"} and ["[**`str`**]{.quote}"](/type/str.html){target="_top"} types from the [YAML tag repository](/type/index.html){target="_top"}. A few examples also use the ["[**`int`**]{.quote}"](/type/int.html){target="_top"} and ["[**`float`**]{.quote}"](/type/float.html){target="_top"} types. The repository includes additional types such as ["[**`null`**]{.quote}"](/type/null.html){target="_top"}, ["[**`bool`**]{.quote}"](/type/bool.html){target="_top"}, ["[**`set`**]{.quote}"](/type/set.html){target="_top"} and others.

+-----------------------------------+---------------------------------------+
| ::: example                       | ::: example                           |
| []{#id858734}                     | []{#id858757}                         |
|                                   |                                       |
| **Example 2.19. Integers**        | **Example 2.20. Floating Point**      |
|                                   |                                       |
| ``` programlisting                | ``` programlisting                    |
| canonical: 12345                  | canonical: 1.23015e+3                 |
| decimal: +12,345                  | exponential: 12.3015e+02              |
| sexagesimal: 3:25:45              | sexagesimal: 20:30.15                 |
| octal: 014                        | fixed: 1,230.15                       |
| hexadecimal: 0xC                  | negative infinity: -.inf              |
| ```                               | not a number: .NaN                    |
| :::                               | ```                                   |
|                                   | :::                                   |
+-----------------------------------+---------------------------------------+
| ::: example                       | ::: example                           |
| []{#id858780}                     | []{#id858801}                         |
|                                   |                                       |
| **Example 2.21. Miscellaneous**   | **Example 2.22. Timestamps**          |
|                                   |                                       |
| ``` programlisting                | ``` programlisting                    |
| null: ~                           | canonical: 2001-12-15T02:59:43.1Z     |
| true: y                           | iso8601: 2001-12-14t21:59:43.10-05:00 |
| false: n                          | spaced: 2001-12-14 21:59:43.10 -5     |
| string: '12345'                   | date: 2002-12-14                      |
| ```                               | ```                                   |
| :::                               | :::                                   |
+-----------------------------------+---------------------------------------+

Explicit typing is denoted with a []{#id858826 .indexterm}[tag](#tag/information%20model) using the exclamation point ([]{#id858842 .indexterm}["[**`!`**]{.quote}"](#!%20tag%20indicator/)) symbol. []{#id858862 .indexterm}[Global tags](#global%20tag/) are URIs and may be specified in a []{#id858873 .indexterm}[shorthand](#tag%20shorthand/) form using a []{#id858888 .indexterm}[handle](#tag%20handle/). []{#id858901 .indexterm}[Application](#application/)-specific []{#id858913 .indexterm}[local tags](#local%20tag/) may also be used.

+-----------------------------------------+-------------------------------------+
| ::: example                             | ::: example                         |
| []{#id858936}                           | []{#id858961}                       |
|                                         |                                     |
| **Example 2.23. Various Explicit Tags** | **Example 2.24. Global Tags**       |
|                                         |                                     |
| ``` programlisting                      | ``` programlisting                  |
| ---                                     | %TAG ! tag:clarkevans.com,2002:     |
| not-date: !!str 2002-04-28              | --- !shape                          |
|                                         |   # Use the ! handle for presenting |
| picture: !!binary |                     |   # tag:clarkevans.com,2002:circle  |
|  R0lGODlhDAAMAIQAAP//9/X                | - !circle                           |
|  17unp5WZmZgAAAOfn515eXv                |   center: &ORIGIN {x: 73, y: 129}   |
|  Pz7Y6OjuDg4J+fn5OTk6enp                |   radius: 7                         |
|  56enmleECcgggoBADs=                    | - !line                             |
|                                         |   start: *ORIGIN                    |
| application specific tag: !something |  |   finish: { x: 89, y: 102 }         |
|  The semantics of the tag               | - !label                            |
|  above may be different for             |   start: *ORIGIN                    |
|  different documents.                   |   color: 0xFFEEBB                   |
| ```                                     |   text: Pretty vector drawing.      |
| :::                                     | ```                                 |
|                                         | :::                                 |
+-----------------------------------------+-------------------------------------+

+------------------------------------+------------------------------------+
| ::: example                        | ::: example                        |
| []{#id858993}                      | []{#id859017}                      |
|                                    |                                    |
| **Example 2.25. Unordered Sets**   | **Example 2.26. Ordered Mappings** |
|                                    |                                    |
| ``` programlisting                 | ``` programlisting                 |
| # sets are represented as a        | # ordered maps are represented as  |
| # mapping where each key is        | # a sequence of mappings, with     |
| # associated with the empty string | # each mapping having one key      |
| --- !!set                          | --- !!omap                         |
| ? Mark McGwire                     | - Mark McGwire: 65                 |
| ? Sammy Sosa                       | - Sammy Sosa: 63                   |
| ? Ken Griff                        | - Ken Griffy: 58                   |
| ```                                | ```                                |
| :::                                | :::                                |
+------------------------------------+------------------------------------+
::::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id859040}2.5. Full Length Example {#full-length-example .title style="clear: both"}
:::
::::
:::::

Below are two full-length examples of YAML. On the left is a sample invoice; on the right is a sample log file.

+----------------------------------------+-----------------------------------+
| ::: example                            | ::: example                       |
| []{#id859060}                          | []{#id859081}                     |
|                                        |                                   |
| **Example 2.27. Invoice**              | **Example 2.28. Log File**        |
|                                        |                                   |
| ``` programlisting                     | ``` programlisting                |
| --- !<tag:clarkevans.com,2002:invoice> | ---                               |
| invoice: 34843                         | Time: 2001-11-23 15:01:42 -5      |
| date   : 2001-01-23                    | User: ed                          |
| bill-to: &id001                        | Warning:                          |
|     given  : Chris                     |   This is an error message        |
|     family : Dumars                    |   for the log file                |
|     address:                           | ---                               |
|         lines: |                       | Time: 2001-11-23 15:02:31 -5      |
|             458 Walkman Dr.            | User: ed                          |
|             Suite #292                 | Warning:                          |
|         city    : Royal Oak            |   A slightly different error      |
|         state   : MI                   |   message.                        |
|         postal  : 48046                | ---                               |
| ship-to: *id001                        | Date: 2001-11-23 15:03:17 -5      |
| product:                               | User: ed                          |
|     - sku         : BL394D             | Fatal:                            |
|       quantity    : 4                  |   Unknown variable "bar"          |
|       description : Basketball         | Stack:                            |
|       price       : 450.00             |   - file: TopClass.py             |
|     - sku         : BL4438H            |     line: 23                      |
|       quantity    : 1                  |     code: |                       |
|       description : Super Hoop         |       x = MoreObject("345\n")     |
|       price       : 2392.00            |   - file: MoreClass.py            |
| tax  : 251.42                          |     line: 58                      |
| total: 4443.52                         |     code: |-                      |
| comments:                              |       foo = bar                   |
|     Late afternoon is best.            |                                   |
|     Backup contact is Nancy            |                                   |
|     Billsmer @ 338-4338.               | ```                               |
| ```                                    | :::                               |
| :::                                    |                                   |
+----------------------------------------+-----------------------------------+
::::::
::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id859109}Chapter 3. Processing YAML Information {#chapter-3.-processing-yaml-information .title}
:::
::::
:::::

YAML is both a text format and a method for []{#id859118 .indexterm}[presenting](#present/) any data structure in this format. Therefore, this specification defines two concepts: a class of data objects called YAML []{#id859132 .indexterm}[representations](#representation/), and a syntax for []{#id859145 .indexterm}[presenting](#present/) YAML []{#id859158 .indexterm}[representations](#representation/) as a series of characters, called a YAML []{#id859171 .indexterm}[stream](#stream/information%20model). A YAML []{#id859187 .indexterm}[]{#processor/}*processor* is a tool for converting information between these complementary views. It is assumed that a YAML processor does its work on behalf of another module, called an []{#id859203 .indexterm}[]{#application/}*application*. This chapter describes the information structures a YAML processor must provide to or obtain from the application.

YAML information is used in two ways: for machine processing, and for human consumption. The challenge of reconciling these two perspectives is best done in three distinct translation stages: []{#id859226 .indexterm}[representation](#representation/), []{#id859238 .indexterm}[serialization](#serialization/), and []{#id859251 .indexterm}[presentation](#presentation/). []{#id859264 .indexterm}[Representation](#representation/) addresses how YAML views native data structures to achieve portability between programming environments. []{#id859279 .indexterm}[Serialization](#serialization/) concerns itself with turning a YAML []{#id859292 .indexterm}[representation](#representation/) into a serial form, that is, a form with sequential access constraints. []{#id859305 .indexterm}[Presentation](#presentation/) deals with the formatting of a YAML []{#id859318 .indexterm}[serialization](#serialization/) as a series of characters in a human-friendly manner.

:::: figure
[]{#id859333}

**Figure 3.1. Processing Overview**

::: mediaobject
![Processing Overview](overview2.png)
:::
::::

A YAML processor need not expose the []{#id859358 .indexterm}[serialization](#serialization/) or []{#id859370 .indexterm}[representation](#representation/) stages. It may translate directly between native data structures and a character []{#id859384 .indexterm}[stream](#stream/information%20model) ([]{#id859400 .indexterm}[]{#dump/}*dump* and []{#id859414 .indexterm}[]{#load/}*load* in the diagram above). However, such a direct translation should take place so that the native data structures are []{#id859430 .indexterm}[constructed](#construct/) only from information available in the []{#id859444 .indexterm}[representation](#representation/).

:::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id859458}3.1. Processes {#processes .title style="clear: both"}
:::
::::
:::::

This section details the processes shown in the diagram above. Note a YAML []{#id859467 .indexterm}[processor](#processor/) need not provide all these processes. For example, a YAML library may provide only YAML input ability, for loading configuration files, or only output ability, for sending data to other []{#id859483 .indexterm}[applications](#application/).

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id859497}3.1.1. Represent {#represent .title}
:::
::::
:::::

YAML []{#id859505 .indexterm}[]{#represent/}*represents* any native data structure using three []{#id859519 .indexterm}[node kinds](#kind/): []{#id859533 .indexterm}[sequence](#sequence/information%20model) - an ordered series of entries; []{#id859551 .indexterm}[mapping](#mapping/information%20model) - an unordered association of []{#id859566 .indexterm}[unique](#equality/) []{#id859579 .indexterm}[keys](#key/information%20model) to []{#id859595 .indexterm}[values](#value/information%20model); and []{#id859610 .indexterm}[scalar](#scalar/information%20model) - any datum with opaque structure []{#id859629 .indexterm}[presentable](#present/) as a series of Unicode characters. Combined, these primitives generate directed graph structures. These primitives were chosen because they are both powerful and familiar: the []{#id859643 .indexterm}[sequence](#sequence/information%20model) corresponds to a Perl array and a Python list, the []{#id859659 .indexterm}[mapping](#mapping/information%20model) corresponds to a Perl hash table and a Python dictionary. The []{#id859676 .indexterm}[scalar](#scalar/information%20model) represents strings, integers, dates, and other atomic data types.

Each YAML []{#id859696 .indexterm}[node](#node/information%20model) requires, in addition to its []{#id859714 .indexterm}[kind](#kind/) and []{#id859725 .indexterm}[content](#content/information%20model), a []{#id859741 .indexterm}[tag](#tag/information%20model) specifying its data type. Type specifiers are either []{#id859757 .indexterm}[global](#global%20tag/) URIs, or are []{#id859772 .indexterm}[local](#local%20tag/) in scope to a single []{#id859785 .indexterm}[application](#application/). For example, an integer is represented in YAML with a []{#id859797 .indexterm}[scalar](#scalar/information%20model) plus the []{#id859813 .indexterm}[global tag](#global%20tag/) "[**`tag:yaml.org,2002:int`**]{.quote}". Similarly, an invoice object, particular to a given organization, could be represented as a []{#id859833 .indexterm}[mapping](#mapping/information%20model) together with the []{#id859852 .indexterm}[local tag](#local%20tag/) "[**`!invoice`**]{.quote}". This simple model can represent any data structure independent of programming language.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id859873}3.1.2. Serialize {#serialize .title}
:::
::::
:::::

For sequential access mediums, such as an event callback API, a YAML []{#id859882 .indexterm}[representation](#representation/) must be []{#id859895 .indexterm}[]{#serialize/}*serialized* to an ordered tree. Since in a YAML []{#id859909 .indexterm}[representation](#representation/), []{#id859921 .indexterm}[mapping keys](#key/information%20model) are unordered and []{#id859937 .indexterm}[nodes](#node/information%20model) may be referenced more than once (have more than one incoming "[arrow]{.quote}"), the serialization process is required to impose an []{#id859960 .indexterm}[ordering](#key%20order/) on the []{#id859972 .indexterm}[mapping keys](#key/information%20model) and to replace the second and subsequent references to a given []{#id859989 .indexterm}[node](#node/information%20model) with place holders called []{#id860007 .indexterm}[aliases](#alias/information%20model). YAML does not specify how these []{#id860022 .indexterm}[]{#serialization detail/}*serialization details* are chosen. It is up to the YAML []{#id860039 .indexterm}[processor](#processor/) to come up with human-friendly []{#id860050 .indexterm}[key order](#key%20order/) and []{#id860062 .indexterm}[anchor](#anchor/information%20model) names, possibly with the help of the []{#id860081 .indexterm}[application](#application/). The result of this process, a YAML []{#id860092 .indexterm}[serialization tree](#serialization/), can then be traversed to produce a series of event calls for one-pass processing of YAML data.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id860109}3.1.3. Present {#present .title}
:::
::::
:::::

The final output process is []{#id860117 .indexterm}[]{#present/}*presenting* the YAML []{#id860130 .indexterm}[serializations](#serialization/) as a character []{#id860143 .indexterm}[stream](#stream/information%20model) in a human-friendly manner. To maximize human readability, YAML offers a rich set of stylistic options which go far beyond the minimal functional needs of simple data storage. Therefore the YAML []{#id860165 .indexterm}[processor](#processor/) is required to introduce various []{#id860176 .indexterm}[]{#presentation detail/}*presentation details* when creating the []{#id860193 .indexterm}[stream](#stream/information%20model), such as the choice of []{#id860208 .indexterm}[node styles](#style/), how to []{#id860221 .indexterm}[format content](#format/), the amount of []{#id860234 .indexterm}[indentation](#indentation%20space/), which []{#id860248 .indexterm}[tag handles](#tag%20handle/) to use, the []{#id860261 .indexterm}[node tags](#tag/information%20model) to leave []{#id860277 .indexterm}[unspecified](#non-specific%20tag/), the set of []{#id860292 .indexterm}[directives](#directive/information%20model) to provide and possibly even what []{#id860308 .indexterm}[comments](#comment/information%20model) to add. While some of this can be done with the help of the []{#id860326 .indexterm}[application](#application/), in general this process should be guided by the preferences of the user.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id860341}3.1.4. Parse {#parse .title}
:::
::::
:::::

[]{#id860348 .indexterm}[]{#parse/}*Parsing* is the inverse process of []{#id860362 .indexterm}[presentation](#present/), it takes a []{#id860375 .indexterm}[stream](#stream/information%20model) of characters and produces a series of events. Parsing discards all the []{#id860394 .indexterm}[details](#presentation%20detail/) introduced in the []{#id860409 .indexterm}[presentation](#present/) process, reporting only the []{#id860420 .indexterm}[serialization](#serialization/) events. Parsing can fail due to []{#id860433 .indexterm}[ill-formed](#ill-formed%20stream/) input.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id860452}3.1.5. Compose {#compose .title}
:::
::::
:::::

[]{#id860459 .indexterm}[]{#compose/}*Composing* takes a series of []{#id860471 .indexterm}[serialization](#serialization/) events and produces a []{#id860484 .indexterm}[representation graph](#representation/). Composing discards all the []{#id860497 .indexterm}[serialization details](#serialization%20detail/) introduced in the []{#id860512 .indexterm}[serialization](#serialize/) process, producing only the []{#id860525 .indexterm}[representation graph](#representation/). Composing can fail due to any of several reasons, detailed []{#id860539 .indexterm}[below](#load%20failure%20point/).
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id860557}3.1.6. Construct {#construct .title}
:::
::::
:::::

The final input process is []{#id860565 .indexterm}[]{#construct/}*constructing* native data structures from the YAML []{#id860579 .indexterm}[representation](#representation/). Construction must be based only on the information available in the []{#id860593 .indexterm}[representation](#representation/), and not on additional []{#id860606 .indexterm}[serialization](#serialization/) or []{#id860618 .indexterm}[presentation details](#presentation%20detail/) such as []{#id860632 .indexterm}[comments](#comment/information%20model), []{#id860648 .indexterm}[directives](#directive/information%20model), []{#id860665 .indexterm}[mapping key order](#key%20order/), []{#id860676 .indexterm}[node styles](#style/), []{#id860688 .indexterm}[content format](#format/), []{#id860700 .indexterm}[indentation](#indentation%20space/) levels etc. Construction can fail due to the []{#id860715 .indexterm}[unavailability](#unavailable%20tag/) of the required native data types.
::::::
::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id860735}3.2. Information Models {#information-models .title style="clear: both"}
:::
::::
:::::

This section specifies the formal details of the results of the above processes. To maximize data portability between programming languages and implementations, users of YAML should be mindful of the distinction between []{#id860747 .indexterm}[serialization](#serialization/) or []{#id860758 .indexterm}[presentation](#presentation/) properties and those which are part of the YAML []{#id860771 .indexterm}[representation](#representation/). Thus, while imposing a []{#id860784 .indexterm}[order](#key%20order/) on []{#id860796 .indexterm}[mapping keys](#key/information%20model) is necessary for flattening YAML []{#id860813 .indexterm}[representations](#representation/) to a sequential access medium, this []{#id860825 .indexterm}[serialization detail](#serialization%20detail/) must not be used to convey []{#id860841 .indexterm}[application](#application/) level information. In a similar manner, while []{#id860854 .indexterm}[indentation](#indentation%20space/) technique and a choice of a []{#id860869 .indexterm}[node style](#style/) are needed for the human readability, these []{#id860881 .indexterm}[presentation details](#presentation%20detail/) are neither part of the YAML []{#id860894 .indexterm}[serialization](#serialization/) nor the YAML []{#id860906 .indexterm}[representation](#representation/). By carefully separating properties needed for []{#id860920 .indexterm}[serialization](#serialization/) and []{#id860932 .indexterm}[presentation](#presentation/), YAML []{#id860945 .indexterm}[representations](#representation/) of []{#id860957 .indexterm}[application](#application/) information will be consistent and portable between various programming environments.

The following diagram summarizes the three information models. Full arrows denote composition, hollow arrows denote inheritance, "[**`1`**]{.quote}" and "[**`*`**]{.quote}" denote "[one]{.quote}" and "[many]{.quote}" relationships. A single "[**`+`**]{.quote}" denotes []{#id861005 .indexterm}[serialization](#serialization/) details, a double "[**`++`**]{.quote}" denotes []{#id861025 .indexterm}[presentation](#presentation/) details.

:::: figure
[]{#id861038}

**Figure 3.2. Information Models**

::: mediaobject
![Information Models](model2.png)
:::
::::

:::::::::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id861060}3.2.1. Representation Graph {#representation-graph .title}
:::
::::
:::::

YAML's []{#id861069 .indexterm}[]{#representation/}*representation* of native data is a rooted, connected, directed graph of []{#id861083 .indexterm}[tagged](#tag/information%20model) []{#id861099 .indexterm}[nodes](#node/information%20model). By "[directed graph]{.quote}" we mean a set of []{#id861118 .indexterm}[nodes](#node/information%20model) and directed edges ("[arrows]{.quote}"), where each edge connects one []{#id861138 .indexterm}[node](#node/information%20model) to another (see [a formal definition](http://www.nist.gov/dads/HTML/directedGraph.html){target="_top"}). All the []{#id861161 .indexterm}[nodes](#node/information%20model) must be reachable from the []{#id861178 .indexterm}[]{#root node/}*root node* via such edges. Note that the YAML graph may include cycles, and a []{#id861192 .indexterm}[node](#node/information%20model) may have more than one incoming edge.

[]{#id861213 .indexterm}[Nodes](#node/information%20model) that are defined in terms of other []{#id861229 .indexterm}[nodes](#node/information%20model) are []{#id861245 .indexterm}[collections](#collection/information%20model) and []{#id861262 .indexterm}[nodes](#node/information%20model) that are independent of any other []{#id861277 .indexterm}[nodes](#node/information%20model) are []{#id861294 .indexterm}[scalars](#scalar/information%20model). YAML supports two []{#id861308 .indexterm}[kinds](#kind/) of []{#id861320 .indexterm}[collection nodes](#collection/information%20model), []{#id861336 .indexterm}[sequences](#sequence/information%20model) and []{#id861354 .indexterm}[mappings](#mapping/information%20model). []{#id861368 .indexterm}[Mapping nodes](#mapping/information%20model) are somewhat tricky because their []{#id861384 .indexterm}[keys](#key/information%20model) are unordered and must be []{#id861400 .indexterm}[unique](#equality/).

:::: figure
[]{#id861413}

**Figure 3.3. Representation Model**

::: mediaobject
![Representation Model](represent2.png)
:::
::::

::::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id861435}3.2.1.1. Nodes {#nodes .title}
:::
::::
:::::

YAML []{#id861443 .indexterm}[]{#node/information model}*nodes* have []{#id861462 .indexterm}[]{#content/information model}*content* of one of three []{#id861478 .indexterm}[]{#kind/}*kinds*: scalar, sequence, or mapping. In addition, each node has a []{#id861492 .indexterm}[tag](#tag/information%20model) which serves to restrict the set of possible values which the node's content can have.

::: variablelist

[Scalar]{.term}
:   The content of a []{#id861523 .indexterm}[]{#scalar/information model}*scalar* node is an opaque datum that can be []{#id861541 .indexterm}[presented](#present/) as a series of zero or more Unicode characters.

[Sequence]{.term}
:   The content of a []{#id861565 .indexterm}[]{#sequence/information model}*sequence* node is an ordered series of zero or more nodes. In particular, a sequence may contain the same node more than once or it could even contain itself (directly or indirectly).

[Mapping]{.term}
:   The content of a []{#id861597 .indexterm}[]{#mapping/information model}*mapping* node is an unordered set of []{#id861614 .indexterm}[]{#key/information model}*key:* []{#id861633 .indexterm}[]{#value/information model}*value* node pairs, with the restriction that each of the keys is []{#id861650 .indexterm}[unique](#equality/). YAML places no further restrictions on the nodes. In particular, keys may be arbitrary nodes, the same node may be used as the value of several key: value pairs, and a mapping could even contain itself as a key or a value (directly or indirectly).
:::

When appropriate, it is convenient to consider sequences and mappings together, as []{#id861679 .indexterm}[]{#collection/information model}*collections*. In this view, sequences are treated as mappings with integer keys starting at zero. Having a unified collections view for sequences and mappings is helpful both for creating practical YAML tools and APIs and for theoretical analysis.
:::::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id861700}3.2.1.2. Tags {#tags-1 .title}
:::
::::
:::::

YAML []{#id861708 .indexterm}[represents](#represent/) type information of native data structures with a simple identifier, called a []{#id861722 .indexterm}[]{#tag/information model}*tag*. []{#id861741 .indexterm}[]{#global tag/}*Global tags* are [URIs](http://www.ietf.org/rfc/rfc2396.txt){target="_top"} and hence globally unique across all []{#id861760 .indexterm}[applications](#application/). The "[**`tag`**]{.quote}": [URI scheme](http://www.taguri.org){target="_top"} ([mirror](/spec/taguri.txt){target="_top"}) is recommended for all global YAML tags. In contrast, []{#id861792 .indexterm}[]{#local tag/}*local tags* are specific to a single []{#id861806 .indexterm}[application](#application/). Local tags start with []{#id861819 .indexterm}[]{#! local tag/}*"[**`!`**]{.quote}"*, are not URIs and are not expected to be globally unique. YAML provides a []{#id861841 .indexterm}["[**`TAG`**]{.quote}" directive](#TAG%20directive/) to make tag notation less verbose; it also offers easy migration from local to global tags. To ensure this, local tags are restricted to the URI character set and use URI character []{#id861861 .indexterm}[escaping](#escaping%20in%20URI/).

YAML does not mandate any special relationship between different tags that begin with the same substring. Tags ending with URI fragments (containing []{#id861882 .indexterm}["[**`#`**]{.quote}"](##%20comment/)) are no exception; tags that share the same base URI but differ in their fragment part are considered to be different, independent tags. By convention, fragments are used to identify different "[variants]{.quote}" of a tag, while "[**`/`**]{.quote}" is used to define nested tag "[namespace]{.quote}" hierarchies. However, this is merely a convention, and each tag may employ its own rules. For example, Perl tags may use "[**`::`**]{.quote}" to express namespace hierarchies, Java tags may use "[**`.`**]{.quote}", etc.

YAML tags are used to associate meta information with each []{#id861941 .indexterm}[node](#node/information%20model). In particular, each tag must specify the expected []{#id861957 .indexterm}[node kind](#kind/) ([]{#id861968 .indexterm}[scalar](#scalar/information%20model), []{#id861984 .indexterm}[sequence](#sequence/information%20model), or []{#id862001 .indexterm}[mapping](#mapping/information%20model)). []{#id862016 .indexterm}[Scalar](#scalar/information%20model) tags must also provide mechanism for converting []{#id862032 .indexterm}[formatted content](#format/) to a []{#id862044 .indexterm}[canonical form](#canonical%20form/) for supporting []{#id862057 .indexterm}[equality](#equality/) testing. Furthermore, a tag may provide additional information such as the set of allowed []{#id862071 .indexterm}[content values](#content/information%20model) for validation, a mechanism for []{#id862090 .indexterm}[tag resolution](#tag%20resolution/), or any other data that is applicable to all of the tag's []{#id862105 .indexterm}[nodes](#node/information%20model).
::::::

::::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id862121}3.2.1.3. Nodes Comparison {#nodes-comparison .title}
:::
::::
:::::

Since YAML []{#id862129 .indexterm}[mappings](#mapping/information%20model) require []{#id862147 .indexterm}[key](#key/information%20model) uniqueness, []{#id862161 .indexterm}[representations](#representation/) must include a mechanism for testing the equality of []{#id862174 .indexterm}[nodes](#node/information%20model). This is non-trivial since YAML allows various ways to []{#id862191 .indexterm}[format](#format/) a given []{#id862203 .indexterm}[scalar content](#scalar/information%20model). For example, the integer eleven can be written as "[**`013`**]{.quote}" (octal) or "[**`0xB`**]{.quote}" (hexadecimal). If both forms are used as []{#id862234 .indexterm}[keys](#key/information%20model) in the same []{#id862252 .indexterm}[mapping](#mapping/information%20model), only a YAML []{#id862266 .indexterm}[processor](#processor/) which recognizes integer []{#id862279 .indexterm}[formats](#format/) would correctly flag the duplicate []{#id862292 .indexterm}[key](#key/information%20model) as an error.

::: variablelist

[Canonical Form]{.term}
:   YAML supports the need for []{#id862321 .indexterm}[scalar](#scalar/information%20model) equality by requiring that every []{#id862337 .indexterm}[scalar](#scalar/information%20model)[]{#id862352 .indexterm}[tag](#tag/information%20model) must specify a mechanism to producing the []{#id862368 .indexterm}[]{#canonical form/}*canonical form* of any []{#id862381 .indexterm}[formatted content](#format/). This form is a Unicode character string which []{#id862396 .indexterm}[presents](#present/) the []{#id862408 .indexterm}[content](#content/information%20model) and can be used for equality testing. While this requirement is stronger than a well defined equality operator, it has other uses, such as the production of digital signatures.

[Equality]{.term}
:   Two []{#id862440 .indexterm}[nodes](#node/information%20model) must have the same []{#id862457 .indexterm}[tag](#tag/information%20model) and []{#id862471 .indexterm}[content](#content/information%20model) to be []{#id862488 .indexterm}[]{#equality/}*equal*. Since each []{#id862501 .indexterm}[tag](#tag/information%20model) applies to exactly one []{#id862516 .indexterm}[kind](#kind/), this implies that the two []{#id862529 .indexterm}[nodes](#node/information%20model) must have the same []{#id862546 .indexterm}[kind](#kind/) to be equal. Two []{#id862558 .indexterm}[scalars](#scalar/information%20model) are equal only when their []{#id862577 .indexterm}[tags](#tag/information%20model) and canonical forms are equal character-by-character. Equality of []{#id862593 .indexterm}[collections](#collection/information%20model) is defined recursively. Two []{#id862610 .indexterm}[sequences](#sequence/information%20model) are equal only when they have the same []{#id862627 .indexterm}[tag](#tag/information%20model) and length, and each []{#id862642 .indexterm}[node](#node/information%20model) in one []{#id862657 .indexterm}[sequence](#sequence/information%20model) is equal to the corresponding []{#id862676 .indexterm}[node](#node/information%20model) in the other []{#id862691 .indexterm}[sequence](#sequence/information%20model). Two []{#id862706 .indexterm}[mappings](#mapping/information%20model) are equal only when they have the same []{#id862724 .indexterm}[tag](#tag/information%20model) and an equal set of []{#id862740 .indexterm}[keys](#key/information%20model), and each []{#id862754 .indexterm}[key](#key/information%20model) in this set is associated with equal []{#id862772 .indexterm}[values](#value/information%20model) in both []{#id862788 .indexterm}[mappings](#mapping/information%20model).

[Identity]{.term}
:   Two []{#id862812 .indexterm}[nodes](#node/information%20model) are []{#id862830 .indexterm}[]{#identity/}*identical* only when they []{#id862842 .indexterm}[represent](#represent/) the same native data structure. Typically, this corresponds to a single memory address. Identity should not be confused with equality; two equal []{#id862858 .indexterm}[nodes](#node/information%20model) need not have the same identity. A YAML []{#id862877 .indexterm}[processor](#processor/) may treat equal []{#id862888 .indexterm}[scalars](#scalar/information%20model) as if they were identical. In contrast, the separate identity of two distinct but equal []{#id862908 .indexterm}[collections](#collection/information%20model) must be preserved.
:::
:::::::
::::::::::::::::::::::

:::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id862929}3.2.2. Serialization Tree {#serialization-tree .title}
:::
::::
:::::

To express a YAML []{#id862938 .indexterm}[representation](#representation/) using a serial API, it necessary to impose an []{#id862950 .indexterm}[order](#key%20order/) on []{#id862964 .indexterm}[mapping keys](#key/information%20model) and employ []{#id862978 .indexterm}[alias nodes](#alias/information%20model) to indicate a subsequent occurrence of a previously encountered []{#id862995 .indexterm}[node](#node/information%20model). The result of this process is a []{#id863013 .indexterm}[]{#serialization/}*serialization tree*, where each []{#id863026 .indexterm}[node](#node/information%20model) has an ordered set of children. This tree can be traversed for a serial event-based API. []{#id863043 .indexterm}[Construction](#construct/) of native structures from the serial interface should not use []{#id863056 .indexterm}[key order](#key%20order/) or []{#id863069 .indexterm}[anchors](#anchor/information%20model) for the preservation of important data.

:::: figure
[]{#id863089}

**Figure 3.4. Serialization Model**

::: mediaobject
![Serialization Model](serialize2.png)
:::
::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id863110}3.2.2.1. Keys Order {#keys-order .title}
:::
::::
:::::

In the []{#id863118 .indexterm}[representation](#representation/) model, []{#id863129 .indexterm}[mapping keys](#key/information%20model) do not have an order. To []{#id863145 .indexterm}[serialize](#serialize/) a []{#id863157 .indexterm}[mapping](#mapping/information%20model), it is necessary to impose an []{#id863174 .indexterm}[]{#key order/}*ordering* on its []{#id863189 .indexterm}[keys](#key/information%20model). This order is a []{#id863204 .indexterm}[serialization detail](#serialization%20detail/) and should not be used when []{#id863217 .indexterm}[composing](#compose/) the []{#id863231 .indexterm}[representation graph](#representation/) (and hence for the preservation of important data). In every case where []{#id863246 .indexterm}[node](#node/information%20model) order is significant, a []{#id863264 .indexterm}[sequence](#sequence/information%20model) must be used. For example, an ordered []{#id863279 .indexterm}[mapping](#mapping/information%20model) can be []{#id863294 .indexterm}[represented](#represent/) as a []{#id863306 .indexterm}[sequence](#sequence/information%20model) of []{#id863322 .indexterm}[mappings](#mapping/information%20model), where each []{#id863340 .indexterm}[mapping](#mapping/information%20model) is a single []{#id863354 .indexterm}[key:](#key/information%20model) []{#id863371 .indexterm}[value](#value/information%20model) pair. YAML provides convenient compact notation for this case.
::::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id863390}3.2.2.2. Anchors and Aliases {#anchors-and-aliases .title}
:::
::::
:::::

In the []{#id863397 .indexterm}[representation graph](#representation/), a []{#id863410 .indexterm}[node](#node/information%20model) may appear in more than one []{#id863428 .indexterm}[collection](#collection/information%20model). When []{#id863444 .indexterm}[serializing](#serialize/) such data, the first occurrence of the []{#id863456 .indexterm}[node](#node/information%20model) is []{#id863473 .indexterm}[]{#identified/}*identified* by an []{#id863486 .indexterm}[]{#anchor/information model}*anchor* and each subsequent occurrence is []{#id863503 .indexterm}[serialized](#serialize/) as an []{#id863515 .indexterm}[]{#alias/information model}*alias node* which refers back to this anchor. Otherwise, anchor names are a []{#id863534 .indexterm}[serialization detail](#serialization%20detail/) and are discarded once []{#id863548 .indexterm}[composing](#compose/) is completed. When []{#id863561 .indexterm}[composing](#compose/) a []{#id863573 .indexterm}[representation graph](#representation/) from []{#id863586 .indexterm}[serialized](#serialize/) events, an alias node refers to the most recent []{#id863599 .indexterm}[node](#node/information%20model) in the []{#id863616 .indexterm}[serialization](#serialization/) having the specified anchor. Therefore, anchors need not be unique within a []{#id863629 .indexterm}[serialization](#serialization/). In addition, an anchor need not have an alias node referring to it. It is therefore possible to provide an anchor for all []{#id863644 .indexterm}[nodes](#node/information%20model) in []{#id863660 .indexterm}[serialization](#serialization/).
::::::
::::::::::::::::

:::::::::::::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id863676}3.2.3. Presentation Stream {#presentation-stream .title}
:::
::::
:::::

A YAML []{#id863684 .indexterm}[]{#presentation/}*presentation* is a []{#id863698 .indexterm}[]{#stream/information model}*stream* of Unicode characters making use of of []{#id863718 .indexterm}[styles](#style/), []{#id863729 .indexterm}[formats](#format/), []{#id863741 .indexterm}[comments](#comment/information%20model), []{#id863757 .indexterm}[directives](#directive/information%20model) and other []{#id863774 .indexterm}[presentation details](#presentation%20detail/) to []{#id863787 .indexterm}[present](#present/) a YAML []{#id863798 .indexterm}[serialization](#serialization/) in a human readable way. Although a YAML []{#id863811 .indexterm}[processor](#processor/) may provide these []{#id863824 .indexterm}[details](#presentation%20detail/) when []{#id863839 .indexterm}[parsing](#parse/), they should not be reflected in the resulting []{#id863852 .indexterm}[serialization](#serialization/). YAML allows several []{#id863865 .indexterm}[serializations](#serialization/) to be contained in the same YAML character stream as a series of []{#id863878 .indexterm}[]{#document/information model}*documents* separated by []{#id863896 .indexterm}[document boundary markers](#document%20boundary%20marker/). Documents appearing in the same stream are independent; that is, a []{#id863912 .indexterm}[node](#node/information%20model) must not appear in more than one []{#id863928 .indexterm}[serialization tree](#serialization/) or []{#id863940 .indexterm}[representation graph](#representation/).

:::: figure
[]{#id863954}

**Figure 3.5. Presentation Model**

::: mediaobject
![Presentation Model](present2.png)
:::
::::

:::::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id863975}3.2.3.1. Node Styles {#node-styles .title}
:::
::::
:::::

Each []{#id863984 .indexterm}[node](#node/information%20model) is presented in some []{#id864002 .indexterm}[]{#style/}*style*, depending on its []{#id864014 .indexterm}[kind](#kind/). The node style is a []{#id864026 .indexterm}[presentation detail](#presentation%20detail/) and is not reflected in the []{#id864040 .indexterm}[serialization tree](#serialization/) or []{#id864053 .indexterm}[representation graph](#representation/). There are two groups of styles, []{#id864067 .indexterm}[]{#block style/information model}*block* and []{#id864084 .indexterm}[]{#flow style/information model}*flow*. Block styles use []{#id864104 .indexterm}[indentation](#indentation%20space/) to denote nesting and scope within the []{#id864117 .indexterm}[document](#document/information%20model). In contrast, flow styles rely on explicit []{#id864133 .indexterm}[indicators](#indicator/) to denote nesting and scope.

YAML provides a rich set of []{#id864149 .indexterm}[scalar styles](#scalar/information%20model). []{#id864165 .indexterm}[]{#block scalar style/information model}*Block scalar styles* include the []{#id864184 .indexterm}[]{#literal style/information model}*literal style* and the []{#id864204 .indexterm}[]{#folded style/information model}*folded style*; []{#id864220 .indexterm}[]{#flow scalar style/information model}*flow scalar styles* include the []{#id864238 .indexterm}[]{#plain style/information model}*plain style* and two []{#id864257 .indexterm}[]{#quoted style/information model}*quoted styles*, the []{#id864275 .indexterm}[]{#single-quoted style/information model}*single-quoted style* and the []{#id864293 .indexterm}[]{#double-quoted style/information model}*double-quoted style*. These styles offer a range of trade-offs between expressive power and readability.

Normally, the []{#id864315 .indexterm}[content](#content/information%20model) of []{#id864332 .indexterm}[]{#block collection style/information model}*block collections* begins on the next line. In most cases, YAML also allows block collections to start []{#id864351 .indexterm}[]{#in-line style/information model}*in-line* for more compact notation when nesting []{#id864369 .indexterm}[]{#block sequence style/information model}*block sequences* and []{#id864388 .indexterm}[]{#block mapping style/information model}*block mappings* inside each other. When nesting []{#id864408 .indexterm}[]{#flow collection style/information model}*flow collections*, a []{#id864427 .indexterm}[]{#flow mapping style/information model}*flow mapping* with a []{#id864445 .indexterm}[]{#single pair style/information model}*single key: value pair* may be specified directly inside a []{#id864464 .indexterm}[]{#flow sequence style/information model}*flow sequence*, allowing for a compact "[ordered mapping]{.quote}" notation.

:::: figure
[]{#id864487}

**Figure 3.6. Kind/Style Combinations**

::: mediaobject
![Kind/Style Combinations](styles2.png)
:::
::::
::::::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id864510}3.2.3.2. Scalar Formats {#scalar-formats .title}
:::
::::
:::::

YAML allows []{#id864518 .indexterm}[scalar content](#scalar/information%20model) to be []{#id864536 .indexterm}[presented](#present/) in several []{#id864547 .indexterm}[]{#format/}*formats*. For example, the boolean "[**`true`**]{.quote}" might also be written as "[**`yes`**]{.quote}". []{#id864574 .indexterm}[Tags](#tag/information%20model) must specify a mechanism for converting any formatted []{#id864593 .indexterm}[scalar content](#scalar/information%20model) to a []{#id864609 .indexterm}[canonical form](#canonical%20form/) for use in []{#id864621 .indexterm}[equality](#equality/) testing. Like []{#id864633 .indexterm}[node style](#style/), the format is a []{#id864645 .indexterm}[presentation detail](#presentation%20detail/) and is not reflected in the []{#id864659 .indexterm}[serialization tree](#serialization/) and []{#id864672 .indexterm}[representation graph](#representation/).
::::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id864687}3.2.3.3. Comments {#comments .title}
:::
::::
:::::

[]{#id864695 .indexterm}[]{#comment/information model}*Comments* are a []{#id864714 .indexterm}[presentation detail](#presentation%20detail/) and must not have any effect on the []{#id864728 .indexterm}[serialization tree](#serialization/) or []{#id864739 .indexterm}[representation graph](#representation/). In particular, comments are not associated with a particular []{#id864753 .indexterm}[node](#node/information%20model). The usual purpose of a comment is to communicate between the human maintainers of a file. A typical example is comments in a configuration file. Comments may not appear inside []{#id864774 .indexterm}[scalars](#scalar/information%20model), but may be interleaved with such []{#id864791 .indexterm}[scalars](#scalar/information%20model) inside []{#id864805 .indexterm}[collections](#collection/information%20model).
::::::

:::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id864824}3.2.3.4. Directives {#directives .title}
:::
::::
:::::

Each []{#id864833 .indexterm}[document](#document/information%20model) may be associated with a set of []{#id864849 .indexterm}[]{#directive/information model}*directives*. A directive has a name and an optional sequence of parameters. Directives are instructions to the YAML []{#id864869 .indexterm}[processor](#processor/), and like all other []{#id864880 .indexterm}[presentation details](#presentation%20detail/) are not reflected in the YAML []{#id864894 .indexterm}[serialization tree](#serialization/) or []{#id864907 .indexterm}[representation graph](#representation/). This version of YAML defines a two directives, []{#id864921 .indexterm}["[**`YAML`**]{.quote}"](#YAML%20directive/) and []{#id864940 .indexterm}["[**`TAG`**]{.quote}"](#TAG%20directive/). All other directives are []{#id864955 .indexterm}[reserved](#reserved%20directive/) for future versions of YAML.
::::::
::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id864977}3.3. Loading Failure Points {#loading-failure-points .title style="clear: both"}
:::
::::
:::::

The process of []{#id864985 .indexterm}[loading](#load/) native data structures from a YAML []{#id864997 .indexterm}[stream](#stream/information%20model) has several potential []{#id865016 .indexterm}[]{#load failure point/}*failure points*. The character []{#id865031 .indexterm}[stream](#stream/information%20model) may be []{#id865045 .indexterm}[ill-formed](#ill-formed%20stream/), []{#id865058 .indexterm}[aliases](#alias/information%20model) may be []{#id865074 .indexterm}[unidentified](#unidentified%20alias/), []{#id865089 .indexterm}[unspecified tags](#non-specific%20tag/) may be []{#id865102 .indexterm}[unresolvable](#unresolved%20tag/), []{#id865115 .indexterm}[tags](#tag/information%20model) may be []{#id865131 .indexterm}[unrecognized](#unrecognized%20tag/), the []{#id865146 .indexterm}[content](#content/information%20model) may be []{#id865161 .indexterm}[invalid](#invalid%20content/), and a native type may be []{#id865177 .indexterm}[unavailable](#unavailable%20tag/). Each of these failures results with an incomplete loading.

A []{#id865194 .indexterm}[]{#partial representation/}*partial representation* need not []{#id865209 .indexterm}[resolve](#tag%20resolution/) the []{#id865224 .indexterm}[tag](#tag/information%20model) of each []{#id865238 .indexterm}[node](#node/information%20model), and the []{#id865253 .indexterm}[canonical form](#canonical%20form/) of []{#id865266 .indexterm}[scalar content](#scalar/information%20model) need not be available. This weaker representation is useful for cases of incomplete knowledge of the types used in the []{#id865284 .indexterm}[document](#document/information%20model). In contrast, a []{#id865300 .indexterm}[]{#complete representation/}*complete representation* specifies the []{#id865315 .indexterm}[tag](#tag/information%20model) of each []{#id865330 .indexterm}[node](#node/information%20model), and provides the []{#id865346 .indexterm}[canonical form](#canonical%20form/) of []{#id865358 .indexterm}[scalar content](#scalar/information%20model), allowing for []{#id865374 .indexterm}[equality](#equality/) testing. A complete representation is required in order to []{#id865388 .indexterm}[construct](#construct/) native data structures.

:::: figure
[]{#id865402}

**Figure 3.7. Loading Failure Points**

::: mediaobject
![Loading Failure Points](validity2.png)
:::
::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id865423}3.3.1. Well-Formed and Identified {#well-formed-and-identified .title}
:::
::::
:::::

A []{#id865432 .indexterm}[]{#well-formed stream/}*well-formed* character []{#id865446 .indexterm}[stream](#stream/information%20model) must match the productions specified in the next chapter. Successful loading also requires that each []{#id865466 .indexterm}[alias](#alias/information%20model) shall refer to a previous []{#id865481 .indexterm}[node](#node/information%20model) []{#id865498 .indexterm}[identified](#identified/) by the []{#id865509 .indexterm}[anchor](#anchor/information%20model). A YAML []{#id865524 .indexterm}[processor](#processor/) should reject []{#id865537 .indexterm}[]{#ill-formed stream/}*ill-formed streams* and []{#id865552 .indexterm}[]{#unidentified alias/}*unidentified aliases*. A YAML []{#id865568 .indexterm}[processor](#processor/) may recover from syntax errors, possibly by ignoring certain parts of the input, but it must provide a mechanism for reporting such errors.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id865585}3.3.2. Resolved {#resolved .title}
:::
::::
:::::

It is not required that all the []{#id865594 .indexterm}[tags](#tag/information%20model) of the []{#id865610 .indexterm}[complete representation](#complete%20representation/) be explicitly specified in the character []{#id865625 .indexterm}[stream](#stream/information%20model). During []{#id865641 .indexterm}[parsing](#parse/), []{#id865653 .indexterm}[nodes](#node/information%20model) that omit the []{#id865669 .indexterm}[tag](#tag/information%20model) are given a []{#id865684 .indexterm}[]{#non-specific tag/}*non-specific tag*: []{#id865699 .indexterm}[]{#? non-specific tag/}*"[**`?`**]{.quote}"* for []{#id865718 .indexterm}[plain scalars](#plain%20style/information%20model) and []{#id865735 .indexterm}[]{#! non-specific tag/}*"[**`!`**]{.quote}"* for all other []{#id865754 .indexterm}[nodes](#node/information%20model). These non-specific tags must be []{#id865769 .indexterm}[]{#tag resolution/}*resolved* to a []{#id865784 .indexterm}[]{#specific tag/}*specific tag* (either a []{#id865798 .indexterm}[local tag](#local%20tag/) or a []{#id865810 .indexterm}[global tag](#global%20tag/)) for a []{#id865822 .indexterm}[complete representation](#complete%20representation/) to be []{#id865837 .indexterm}[composed](#compose/).

Resolving the []{#id865852 .indexterm}[tag](#tag/information%20model) of a []{#id865869 .indexterm}[node](#node/information%20model) must only depend on the following three parameters: the non-specific tag of the []{#id865885 .indexterm}[node](#node/information%20model), the path leading from the []{#id865901 .indexterm}[root node](#root%20node/) to the []{#id865914 .indexterm}[node](#node/information%20model), and the []{#id865931 .indexterm}[content](#content/information%20model) (and hence the []{#id865946 .indexterm}[kind](#kind/)) of the []{#id865958 .indexterm}[node](#node/information%20model). In particular, resolution must not consider []{#id865975 .indexterm}[presentation details](#presentation%20detail/) such as []{#id865988 .indexterm}[comments](#comment/information%20model), []{#id866006 .indexterm}[indentation](#indentation%20space/) and []{#id866019 .indexterm}[node style](#style/). Also, resolution must not consider the []{#id866031 .indexterm}[content](#content/information%20model) of any other []{#id866046 .indexterm}[node](#node/information%20model), except for the []{#id866065 .indexterm}[content](#content/information%20model) of the []{#id866079 .indexterm}[key nodes](#key/information%20model) directly along the path leading from the []{#id866095 .indexterm}[root node](#root%20node/) to the resolved []{#id866109 .indexterm}[node](#node/information%20model). In particular, resolution must not consider the []{#id866125 .indexterm}[content](#content/information%20model) of a sibling []{#id866140 .indexterm}[node](#node/information%20model) in a []{#id866156 .indexterm}[collection](#collection/information%20model) or the []{#id866173 .indexterm}[content](#content/information%20model) of the []{#id866188 .indexterm}[value node](#value/information%20model) associated with a resolved []{#id866204 .indexterm}[key node](#key/information%20model).

Tag resolution is specific to the []{#id866224 .indexterm}[application](#application/), hence a YAML []{#id866236 .indexterm}[processor](#processor/) should provide a mechanism allowing the []{#id866249 .indexterm}[application](#application/) to specify the tag resolution rules. It is recommended that []{#id866262 .indexterm}[nodes](#node/information%20model) having the "[**`!`**]{.quote}" non-specific tag should be resolved as "[**`tag:yaml.org,2002:seq`**]{.quote}", "[**`tag:yaml.org,2002:map`**]{.quote}" or "[**`tag:yaml.org,2002:str`**]{.quote}" depending on the []{#id866308 .indexterm}[node's kind](#node/information%20model). This convention allows the author of a YAML character []{#id866326 .indexterm}[stream](#stream/information%20model) to exert some measure of control over the tag resolution process. By explicitly specifying a []{#id866343 .indexterm}[plain scalar](#plain%20style/information%20model) has the "[**`!`**]{.quote}" non-specific tag, the []{#id866366 .indexterm}[node](#node/information%20model) is resolved as a string, as if it was []{#id866383 .indexterm}[quoted](#quoted%20style/information%20model) or written in a []{#id866398 .indexterm}[block style](#block%20style/information%20model). Note, however, that each []{#id866415 .indexterm}[application](#application/) may override this behavior. For example, an []{#id866428 .indexterm}[application](#application/) may automatically detect the type of programming language used in source code []{#id866442 .indexterm}[presented](#present/) as a non-[]{#id866454 .indexterm}[plain](#plain%20style/information%20model) []{#id866470 .indexterm}[scalar](#scalar/information%20model) and resolve it accordingly.

When a []{#id866492 .indexterm}[node](#node/information%20model) has more than one occurrence (using an []{#id866508 .indexterm}[anchor](#anchor/information%20model) and []{#id866523 .indexterm}[alias nodes](#alias/information%20model)), tag resolution must depend only on the path to the first occurrence of the []{#id866539 .indexterm}[node](#node/information%20model). Typically, the path leading to a []{#id866558 .indexterm}[node](#node/information%20model) is sufficient to determine its specific tag. In cases where the path does not imply a single specific tag, the resolution also needs to consider the []{#id866575 .indexterm}[node content](#content/information%20model) to select amongst the set of possible []{#id866593 .indexterm}[tags](#tag/information%20model). Thus, []{#id866607 .indexterm}[plain scalars](#plain%20style/information%20model) may be matched against a set of regular expressions to provide automatic resolution of integers, floats, timestamps, and similar types. Similarly, the []{#id866627 .indexterm}[content](#content/information%20model) of []{#id866641 .indexterm}[mapping nodes](#mapping/information%20model) may be matched against sets of expected []{#id866657 .indexterm}[keys](#key/information%20model) to automatically resolve points, complex numbers, and similar types.

The combined effect of these rules is to ensure that tag resolution can be performed as soon as a []{#id866680 .indexterm}[node](#node/information%20model) is first encountered in the []{#id866696 .indexterm}[stream](#stream/information%20model), typically before its []{#id866714 .indexterm}[content](#content/information%20model) is []{#id866728 .indexterm}[parsed](#parse/). Also, tag resolution only requires referring to a relatively small number of previously parsed []{#id866742 .indexterm}[nodes](#node/information%20model). Thus, tag resolution in one-pass []{#id866760 .indexterm}[processors](#processor/) is both possible and practical.

If a []{#id866775 .indexterm}[document](#document/information%20model) contains []{#id866793 .indexterm}[]{#unresolved tag/}*unresolved tags*, the YAML []{#id866806 .indexterm}[processor](#processor/) is unable to []{#id866818 .indexterm}[compose](#compose/) a []{#id866829 .indexterm}[complete representation](#complete%20representation/) graph. In such a case, the YAML []{#id866846 .indexterm}[processor](#processor/) may []{#id866857 .indexterm}[compose](#compose/) an []{#id866869 .indexterm}[partial representation](#partial%20representation/), based on each []{#id866884 .indexterm}[node's kind](#kind/) and allowing for non-specific tags.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id866900}3.3.3. Recognized and Valid {#recognized-and-valid .title}
:::
::::
:::::

To be []{#id866908 .indexterm}[]{#valid content/}*valid*, a []{#id866922 .indexterm}[node](#node/information%20model) must have a []{#id866938 .indexterm}[tag](#tag/information%20model) which is []{#id866953 .indexterm}[]{#recognized tag/}*recognized* by the YAML []{#id866967 .indexterm}[processor](#processor/) and its []{#id866980 .indexterm}[content](#content/information%20model) must satisfy the constraints imposed by this []{#id866999 .indexterm}[tag](#tag/information%20model). If a []{#id867013 .indexterm}[document](#document/information%20model) contains a []{#id867030 .indexterm}[scalar node](#scalar/information%20model) with an []{#id867044 .indexterm}[]{#unrecognized tag/}*unrecognized tag* or []{#id867058 .indexterm}[]{#invalid content/}*invalid content*, only a []{#id867074 .indexterm}[partial representation](#partial%20representation/) may be []{#id867087 .indexterm}[composed](#compose/). In contrast, a YAML []{#id867100 .indexterm}[processor](#processor/) can always []{#id867113 .indexterm}[compose](#compose/) a []{#id867125 .indexterm}[complete representation](#complete%20representation/) for an unrecognized or an invalid []{#id867141 .indexterm}[collection](#collection/information%20model), since []{#id867157 .indexterm}[collection](#collection/information%20model) []{#id867172 .indexterm}[equality](#equality/) does not depend upon knowledge of the []{#id867184 .indexterm}[collection's](#collection/information%20model) data type. However, such a []{#id867201 .indexterm}[complete representation](#complete%20representation/) can not be used to []{#id867214 .indexterm}[construct](#construct/) a native data structure.
::::::

:::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id867229}3.3.4. Available {#available .title}
:::
::::
:::::

In a given processing environment, there need not be an []{#id867238 .indexterm}[]{#available tag/}*available* native type corresponding to a given []{#id867253 .indexterm}[tag](#tag/information%20model). If a []{#id867271 .indexterm}[node's tag](#tag/information%20model) is []{#id867285 .indexterm}[]{#unavailable tag/}*unavailable*, a YAML []{#id867299 .indexterm}[processor](#processor/) will not be able to []{#id867311 .indexterm}[construct](#construct/) a native data structure for it. In this case, a []{#id867325 .indexterm}[complete representation](#complete%20representation/) may still be []{#id867341 .indexterm}[composed](#compose/), and an []{#id867352 .indexterm}[application](#application/) may wish to use this []{#id867364 .indexterm}[representation](#representation/) directly.
::::::
::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id867381}Chapter 4. Productions Conventions {#chapter-4.-productions-conventions .title}
:::
::::
:::::

The following chapters describe the syntax of YAML character []{#id867391 .indexterm}[streams](#stream/syntax) in detail using a series of BNF productions. In most cases, productions are introduced in a "[bottom-up]{.quote}" order; basic productions are specified before the more complex productions using them. Examples accompanying the productions display sample YAML text side-by-side with equivalent YAML text using only []{#id867415 .indexterm}[flow collections](#flow%20collection%20style/syntax) and []{#id867432 .indexterm}[double-quoted scalars](#double-quoted%20style/syntax). For improved readability, the equivalent YAML text uses the "[**`!!seq`**]{.quote}", "[**`!!map`**]{.quote}", and "[**`!!str`**]{.quote}" []{#id867471 .indexterm}[shorthands](#tag%20shorthand/) instead of the []{#id867486 .indexterm}[verbatim](#verbatim%20tag/) "[**`!<tag:yaml.org,2002:seq>`**]{.quote}", "[**`!<tag:yaml.org,2002:map>`**]{.quote}" and "[**`!<tag:yaml.org,2002:str>`**]{.quote}" forms. These types are used to []{#id867521 .indexterm}[resolve](#tag%20resolution/) all []{#id867533 .indexterm}[untagged nodes](#non-specific%20tag/), except for a few examples that use the "[**`!!int`**]{.quote}" and "[**`!!float`**]{.quote}" types.

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id867562}4.1. Production Prefixes {#production-prefixes .title style="clear: both"}
:::
::::
:::::

To make the syntax easier to follow, production names use Hungarian-style notation. Each production is given one of the following prefix based on the type of characters it matches.

::: variablelist

[ **`e-`** ]{.term}
:   A production matching no characters.

[ **`c-`** ]{.term}
:   A production matching one or more characters starting and ending with a special (non-space) character.

[ **`b-`** ]{.term}
:   A production matching a single []{#id867618 .indexterm}[line break](#line%20break%20character/).

[ **`nb-`** ]{.term}
:   A production matching one or more characters starting and ending with a non-[]{#id867646 .indexterm}[break](#line%20break%20character/) character.

[ **`s-`** ]{.term}
:   A production matching one or more characters starting and ending with a space character.

[ **`ns-`** ]{.term}
:   A production matching one or more characters starting and ending with a non-space character.

[ `X`{.varname}**`-`**`Y`{.varname}**`-`** ]{.term}
:   A production matching a sequence of one or more characters, starting with an `X`{.varname}**`-`** character and ending with a `Y`{.varname}**`-`** character.

[ **`l-`** ]{.term}
:   A production matching one or more lines (shorthand for **`s-b-`**).

[ `X`{.varname}**`+`**, `X`{.varname}**`-`**`Y`{.varname}**`+`** ]{.term}
:   A production as above, with the additional property that the []{#id867785 .indexterm}[indentation](#indentation%20space/) level used is greater than the specified `n`{.varname} parameter.
:::
:::::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id867808}4.2. Production Parameters {#production-parameters .title style="clear: both"}
:::
::::
:::::

As YAML's syntax is designed for maximal readability, it makes heavy use of the context that each syntactical entity appears in. For notational compactness, this is expressed using parameterized BNF productions. The set of parameters and the range of allowed values depend on the specific production. The full list of possible parameters and their values is:

::: variablelist

[ Indentation: `n`{.varname} or `m`{.varname} ]{.term}
:   Since the character []{#id867836 .indexterm}[stream](#stream/syntax) depends upon []{#id867851 .indexterm}[indentation](#indentation%20space/) level to delineate blocks, many productions are parameterized by it. In some cases, the notations "[**`production(<n)`**]{.quote}", "[**`production(≤n)`**]{.quote}" and "[**`production(>n)`**]{.quote}" are used; these are shorthands for "[**`production(m)`**]{.quote}" for some specific `m`{.varname} where 0 ≤ `m`{.varname} \< `n`{.varname}, 0 ≤ `m`{.varname} ≤ `n`{.varname} and `m`{.varname} \> `n`{.varname}, respectively.

[Context: `c`{.varname}]{.term}
:   YAML supports two groups of []{#id867935 .indexterm}[]{#context/}*contexts*, distinguishing between []{#id867949 .indexterm}[block styles](#block%20style/syntax) and []{#id867967 .indexterm}[flow styles](#flow%20style/syntax). In the []{#id867984 .indexterm}[block styles](#block%20style/syntax), []{#id868000 .indexterm}[indentation](#indentation%20space/) is used to delineate structure. Due to the fact that the []{#id868014 .indexterm}["[**`-`**]{.quote}"](#-%20block%20sequence%20entry/) character denoting a []{#id868032 .indexterm}[block sequence](#block%20sequence%20style/syntax) entry is perceived as an []{#id868049 .indexterm}[indentation](#indentation%20space/) character, some productions distinguish between the []{#id868064 .indexterm}[block-in](#block-in%20context/) context (inside a []{#id868078 .indexterm}[block sequence](#block%20sequence%20style/syntax)) and the []{#id868094 .indexterm}[block-out](#block-out%20context/) context (outside one). In the []{#id868111 .indexterm}[flow styles](#flow%20style/syntax), explicit []{#id868125 .indexterm}[indicators](#indicator/) are used to delineate structure. As []{#id868138 .indexterm}[plain scalars](#plain%20style/syntax) have no such []{#id868155 .indexterm}[indicators](#indicator/), they are the most context sensitive, distinguishing between being nested inside a []{#id868169 .indexterm}[flow collection](#flow%20collection%20style/syntax) ([]{#id868185 .indexterm}[flow-in](#flow-in%20context/) context) or being outside one ([]{#id868202 .indexterm}[flow-out](#flow-out%20context/) context). YAML also provides a terse and intuitive syntax for []{#id868217 .indexterm}[simple keys](#simple%20key/). []{#id868231 .indexterm}[Plain scalars](#plain%20style/syntax) in this ([]{#id868246 .indexterm}[flow-key](#flow-key%20context/)) context are the most restricted, for readability and implementation reasons.

[(Scalar) Style: `s`{.varname}]{.term}
:   []{#id868274 .indexterm}[Scalar content](#scalar/syntax) may be []{#id868290 .indexterm}[presented](#present/) in one of five []{#id868303 .indexterm}[styles](#scalar/syntax): the []{#id868318 .indexterm}[plain](#plain%20style/syntax), []{#id868334 .indexterm}[double-quoted](#double-quoted%20style/syntax) and []{#id868350 .indexterm}[single-quoted](#single-quoted%20style/syntax)[]{#id868367 .indexterm}[flow styles](#flow%20style/syntax), and the []{#id868384 .indexterm}[literal](#literal%20style/syntax) and []{#id868399 .indexterm}[folded](#folded%20style/syntax)[]{#id868416 .indexterm}[block styles](#block%20style/syntax).

[(Block) Chomping: `t`{.varname}]{.term}
:   Block scalars offer three possible mechanisms for []{#id868444 .indexterm}[chomping](#chomping/) any trailing []{#id868458 .indexterm}[line breaks](#line%20break%20character/): []{#id868471 .indexterm}[strip](#strip%20chomping/), []{#id868484 .indexterm}[clip](#clip%20chomping/) and []{#id868499 .indexterm}[keep](#keep%20chomping/).
:::
:::::::
::::::::::::::::

::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id868518}Chapter 5. Characters {#chapter-5.-characters .title}
:::
::::
:::::

:::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id868524}5.1. Character Set {#character-set .title style="clear: both"}
:::
::::
:::::

YAML []{#id868532 .indexterm}[streams](#stream/syntax) use the []{#id868546 .indexterm}[]{#printable character/}*printable* subset of the Unicode character set. On input, a YAML []{#id868562 .indexterm}[processor](#processor/) must accept all printable ASCII characters, the space, []{#id868575 .indexterm}[tab](#tab/), []{#id868587 .indexterm}[line break](#line%20break%20character/), and all Unicode characters beyond #x9F. On output, a YAML []{#id868602 .indexterm}[processor](#processor/) must only produce these acceptable characters, and should also []{#id868615 .indexterm}[escape](#escaping%20in%20double-quoted%20style/) all non-printable Unicode characters. The allowed character range explicitly excludes the surrogate block **`#xD800-#xDFFF`**, DEL **`#x7F`**, the C0 control block **`#x0-#x1F`** (except for **`#x9`**, **`#xA`**, and **`#xD`**), the C1 control block **`#x80-#x9F`**, **`#xFFFE`**, and **`#xFFFF`**. Any such characters must be []{#id868686 .indexterm}[presented](#present/) using []{#id868698 .indexterm}[escape](#escaping%20in%20double-quoted%20style/) sequences.

+------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------- -------------- ----------------------------------------------------------------- -------------- |
|   \[1\]            []{#c-printable}c-printable     `::=`        #x9 \| #xA \| #xD \| \[#x20-#x7E\]          /\* 8 bit \*/\                     |
|                                                               \| #x85 \| \[#xA0-#xD7FF\] \| \[#xE000-#xFFFD\] /\* 16 bit \*/\                  |
|                                                               \| \[#x10000-#x10FFFF\]                     /\* 32 bit \*/                       |
|                                                                                                                                                |
|   -------------- ----------------------------- -------------- ----------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------+
::::::

:::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id868742}5.2. Character Encoding {#character-encoding .title style="clear: both"}
:::
::::
:::::

All characters mentioned in this specification are Unicode code points. Each such code point is written as one or more octets depending on the []{#id868753 .indexterm}[]{#character encoding/}*character encoding* used. Note that in UTF-16, characters above **`#xFFFF`** are written as four octets, using a surrogate pair. A YAML []{#id868774 .indexterm}[processor](#processor/) must support the UTF-16 and UTF-8 character encodings. If a character []{#id868788 .indexterm}[stream](#stream/syntax) does not begin with a []{#id868803 .indexterm}[]{#byte order mark/}*byte order mark* (**`#FEFF`**), the character encoding shall be UTF-8. Otherwise it shall be either UTF-8, UTF-16 LE, or UTF-16 BE as indicated by the byte order mark. On output, it is recommended that a byte order mark should only be emitted for UTF-16 character encodings. Note that the UTF-32 encoding is explicitly not supported. For more information about the byte order mark and the Unicode character encoding schemes see the Unicode [FAQ](http://www.unicode.org/unicode/faq/utf_bom.html){target="_top"}.

+--------------------------------------------------------------------------+
|   ------- ----------------------------------------- ------- -------- --- |
|   \[2\]     []{#c-byte-order-mark}c-byte-order-mark  `::=`  #xFEFF       |
|   ------- ----------------------------------------- ------- -------- --- |
+--------------------------------------------------------------------------+

In the examples, byte order mark characters are displayed as "[**`⇔`**]{.quote}".

::: example
[]{#id868866}

**Example 5.1. Byte Order Mark**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ⇔# Comment only.                  | # This stream contains no         |
| ```                               | # documents, only comments.       |
|                                   | ```                               |
| ``` synopsis                      |                                   |
| Legend:                           |                                   |
|   c-byte-order-mark               |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: example
[]{#id868932}

**Example 5.2. Invalid Byte Order Mark**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| # Invalid use of BOM              | ERROR:                            |
| ⇔# inside a                       |  A BOM must not appear            |
| # document.                       |  inside a document.               |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
::::::::

::::::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id868988}5.3. Indicator Characters {#indicator-characters .title style="clear: both"}

y[char.c-tag+2]

:::
::::
:::::

[]{#id868996 .indexterm}[]{#indicator/}*Indicators* are characters that have special semantics used to describe the structure and []{#id869011 .indexterm}[content](#content/syntax) of a YAML []{#id869027 .indexterm}[document](#document/syntax).

::: itemizedlist
- A []{#id869048 .indexterm}["[**`-`**]{.quote}"](#-%20block%20sequence%20entry/) (**`#2D`**, hyphen) denotes a []{#id869073 .indexterm}[block sequence](#block%20sequence%20style/syntax) entry.
:::

+-------------------------------------------------------------------------------+
|   ------- --------------------------------------- ------- --------------- --- |
|   \[3\]     []{#c-sequence-entry}c-sequence-entry  `::=`  "[-]{.quote}"       |
|   ------- --------------------------------------- ------- --------------- --- |
+-------------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869113 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) (**`#3F`**, question mark) denotes a []{#id869136 .indexterm}[mapping key](#key/syntax).
:::

+-------------------------------------------------------------------------+
|   ------- --------------------------------- ------- --------------- --- |
|   \[4\]     []{#c-mapping-key}c-mapping-key  `::=`  "[?]{.quote}"       |
|   ------- --------------------------------- ------- --------------- --- |
+-------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869176 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) (**`#3A`**, colon) denotes a []{#id869199 .indexterm}[mapping value](#value/syntax).
:::

+-----------------------------------------------------------------------------+
|   ------- ------------------------------------- ------- --------------- --- |
|   \[5\]     []{#c-mapping-value}c-mapping-value  `::=`  "[:]{.quote}"       |
|   ------- ------------------------------------- ------- --------------- --- |
+-----------------------------------------------------------------------------+

::: example
[]{#id869235}

**Example 5.3. Block Structure Indicators**

+-----------------------------------+------------------------------------+
| ``` programlisting                | ``` programlisting                 |
| sequence:                         | %YAML 1.1                          |
| - one                             | ---                                |
| - two                             | !!map {                            |
| mapping:                          |   ? !!str "sequence"               |
|   ? sky                           |   : !!seq [                        |
|   : blue                          |     !!str "one", !!str "two"       |
|   ? sea : green                   |   ],                               |
| ```                               |   ? !!str "mapping"                |
|                                   |   : !!map {                        |
| ``` synopsis                      |     ? !!str "sky" : !!str "blue",  |
| Legend:                           |     ? !!str "sea" : !!str "green", |
|   c-sequence-entry                |   }                                |
|   c-mapping-key                   | }                                  |
|   c-mapping-value                 | ```                                |
| ```                               |                                    |
+-----------------------------------+------------------------------------+
:::

::: itemizedlist
- A []{#id869368 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) (**`#2C`**, comma) ends a []{#id869393 .indexterm}[flow collection](#flow%20collection%20style/syntax) entry.
:::

+-----------------------------------------------------------------------------+
|   ------- ------------------------------------- ------- --------------- --- |
|   \[6\]     []{#c-collect-entry}c-collect-entry  `::=`  "[,]{.quote}"       |
|   ------- ------------------------------------- ------- --------------- --- |
+-----------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869436 .indexterm}["[**`[`**]{.quote}"](#%5B%20start%20flow%20sequence/) (**`#5B`**, left bracket) starts a []{#id869460 .indexterm}[flow sequence](#flow%20sequence%20style/syntax).
:::

+--------------------------------------------------------------------------------+
|   ------- --------------------------------------- ------- ---------------- --- |
|   \[7\]     []{#c-sequence-start}c-sequence-start  `::=`  "[\[]{.quote}"       |
|   ------- --------------------------------------- ------- ---------------- --- |
+--------------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869501 .indexterm}["[**`]`**]{.quote}"](#%5D%20end%20flow%20sequence/) (**`#5D`**, right bracket) ends a []{#id869528 .indexterm}[flow sequence](#flow%20sequence%20style/syntax).
:::

+----------------------------------------------------------------------------+
|   ------- ----------------------------------- ------- ---------------- --- |
|   \[8\]     []{#c-sequence-end}c-sequence-end  `::=`  "[\]]{.quote}"       |
|   ------- ----------------------------------- ------- ---------------- --- |
+----------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869567 .indexterm}["[**`{`**]{.quote}"](#%7B%20start%20flow%20mapping/) (**`#7B`**, left brace) starts a []{#id869594 .indexterm}[flow mapping](#flow%20mapping%20style/syntax).
:::

+-----------------------------------------------------------------------------+
|   ------- ------------------------------------- ------- --------------- --- |
|   \[9\]     []{#c-mapping-start}c-mapping-start  `::=`  "[{]{.quote}"       |
|   ------- ------------------------------------- ------- --------------- --- |
+-----------------------------------------------------------------------------+

::: itemizedlist
- A []{#id869634 .indexterm}["[**`}`**]{.quote}"](#%7D%20end%20flow%20mapping/) (**`#7D`**, right brace) ends a []{#id869660 .indexterm}[flow mapping](#flow%20mapping%20style/syntax).
:::

+--------------------------------------------------------------------------+
|   -------- --------------------------------- ------- --------------- --- |
|   \[10\]     []{#c-mapping-end}c-mapping-end  `::=`  "[}]{.quote}"       |
|   -------- --------------------------------- ------- --------------- --- |
+--------------------------------------------------------------------------+

::: example
[]{#id869695}

**Example 5.4. Flow Collection Indicators**

+------------------------------------+------------------------------------+
| ``` programlisting                 | ``` programlisting                 |
| sequence: [ one, two, ]            | %YAML 1.1                          |
| mapping: { sky: blue, sea: green } | ---                                |
| ```                                | !!map {                            |
|                                    |   ? !!str "sequence"               |
| ``` synopsis                       |   : !!seq [                        |
| Legend:                            |     !!str "one", !!str "two"       |
|   c-sequence-start c-sequence-end  |   ],                               |
|   c-mapping-start  c-mapping-end   |   ? !!str "mapping"                |
|   c-collect-entry                  |   : !!map {                        |
| ```                                |     ? !!str "sky" : !!str "blue",  |
|                                    |     ? !!str "sea" : !!str "green", |
|                                    |   }                                |
|                                    | }                                  |
|                                    | ```                                |
+------------------------------------+------------------------------------+
:::

::: itemizedlist
- An []{#id869844 .indexterm}["[**`#`**]{.quote}"](##%20comment/) (**`#23`**, octothorpe, hash, sharp, number sign) denotes a []{#id869867 .indexterm}[comment](#comment/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- ------------------------- ------- ---------------- ---     |
|   \[11\]     []{#c-comment}c-comment  `::=`  "[\#]{.quote}"           |
|   -------- ------------------------- ------- ---------------- ---     |
+-----------------------------------------------------------------------+

::: example
[]{#id869903}

**Example 5.5. Comment Indicator**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| # Comment only.                   | # This stream contains no         |
| ```                               | # documents, only comments.       |
|                                   | ```                               |
| ``` synopsis                      |                                   |
| Legend:                           |                                   |
|   c-comment                       |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- An []{#id869972 .indexterm}["[**`&`**]{.quote}"](#&%20anchor/) (**`#26`**, ampersand) denotes a []{#id869997 .indexterm}[node's anchor property](#anchor/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- ----------------------- ------- --------------- ---        |
|   \[12\]     []{#c-anchor}c-anchor  `::=`  "[&]{.quote}"              |
|   -------- ----------------------- ------- --------------- ---        |
+-----------------------------------------------------------------------+

::: itemizedlist
- An []{#id870038 .indexterm}["[**`*`**]{.quote}"](#*%20alias/) (**`#2A`**, asterisk) denotes an []{#id870060 .indexterm}[alias node](#alias/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- --------------------- ------- ---------------- ---         |
|   \[13\]     []{#c-alias}c-alias  `::=`  "[\*]{.quote}"               |
|   -------- --------------------- ------- ---------------- ---         |
+-----------------------------------------------------------------------+

::: itemizedlist
- An []{#id870101 .indexterm}["[**`!`**]{.quote}"](#!%20tag%20indicator/) (**`#21`**, exclamation) denotes a []{#id870124 .indexterm}[node's tag](#tag/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- ----------------- ------- --------------- ---              |
|   \[14\]     []{#c-tag}c-tag  `::=`  "[!]{.quote}"                    |
|   -------- ----------------- ------- --------------- ---              |
+-----------------------------------------------------------------------+

::: example
[]{#id870160}

**Example 5.6. Node Property Indicators**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| anchored: !local &anchor value    | %YAML 1.1                         |
| alias: *anchor                    | ---                               |
| ```                               | !!map {                           |
|                                   |   ? !!str "anchored"              |
| ``` synopsis                      |   : !local &A1 "value",           |
| Legend:                           |   ? !!str "alias"                 |
|   c-anchor                        |   : *A1,                          |
|   c-alias                         | }                                 |
|   c-tag                           | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- A []{#id870265 .indexterm}["[**`|`**]{.quote}"](#%7C%20literal%20style/) (**`7C`**, vertical bar) denotes a []{#id870287 .indexterm}[literal block scalar](#literal%20style/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- ------------------------- ------- ---------------- ---     |
|   \[15\]     []{#c-literal}c-literal  `::=`  "[\|]{.quote}"           |
|   -------- ------------------------- ------- ---------------- ---     |
+-----------------------------------------------------------------------+

::: itemizedlist
- A []{#id870329 .indexterm}["[**`>`**]{.quote}"](#%3E%20folded%20style/) (**`#3E`**, greater than) denotes a []{#id870354 .indexterm}[folded block scalar](#folded%20style/syntax).
:::

+-----------------------------------------------------------------------+
|   -------- ----------------------- ------- ---------------- ---       |
|   \[16\]     []{#c-folded}c-folded  `::=`  "[\>]{.quote}"             |
|   -------- ----------------------- ------- ---------------- ---       |
+-----------------------------------------------------------------------+

::: example
[]{#id870388}

**Example 5.7. Block Scalar Indicators**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| literal: |                        | %YAML 1.1                         |
|   text                            | ---                               |
| folded: >                         | !!map {                           |
|   text                            |   ? !!str "literal"               |
| ```                               |   : !!str "text\n",               |
|                                   |   ? !!str "folded"                |
| ``` synopsis                      |   : !!str "text\n",               |
| Legend:                           | }                                 |
|   c-literal                       | ```                               |
|   c-folded                        |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- An []{#id870475 .indexterm}["[**`'`**]{.quote}"](#'%20single-quoted%20style/) (**`#27`**, apostrophe, single quote) surrounds a []{#id870502 .indexterm}[single-quoted flow scalar](#single-quoted%20style/syntax).
:::

+-----------------------------------------------------------------------------+
|   -------- ----------------------------------- ------- ---------------- --- |
|   \[17\]     []{#c-single-quote}c-single-quote  `::=`  "[\']{.quote}"       |
|   -------- ----------------------------------- ------- ---------------- --- |
+-----------------------------------------------------------------------------+

::: itemizedlist
- A []{#id870542 .indexterm}["[**`"`**]{.quote}"](#%22%20double-quoted%20style/) (**`#22`**, double quote) surrounds a []{#id870568 .indexterm}[double-quoted flow scalar](#double-quoted%20style/syntax).
:::

+-----------------------------------------------------------------------------+
|   -------- ----------------------------------- ------- ---------------- --- |
|   \[18\]     []{#c-double-quote}c-double-quote  `::=`  "[\"]{.quote}"       |
|   -------- ----------------------------------- ------- ---------------- --- |
+-----------------------------------------------------------------------------+

::: example
[]{#id870602}

**Example 5.8. Quoted Scalar Indicators**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| single: 'text'                    | %YAML 1.1                         |
| double: "text"                    | ---                               |
| ```                               | !!map {                           |
|                                   |   ? !!str "double"                |
| ``` synopsis                      |   : !!str "text",                 |
| Legend:                           |   ? !!str "single"                |
|   c-single-quote                  |   : !!str "text",                 |
|   c-double-quote                  | }                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- A []{#id870700 .indexterm}["[**`%`**]{.quote}"](#%%20directive/) (**`#25`**, percent) denotes a []{#id870723 .indexterm}[directive](#directive/syntax) line.
:::

+-----------------------------------------------------------------------+
|   -------- ----------------------------- ------- --------------- ---  |
|   \[19\]     []{#c-directive}c-directive  `::=`  "[%]{.quote}"        |
|   -------- ----------------------------- ------- --------------- ---  |
+-----------------------------------------------------------------------+

::: example
[]{#id870761}

**Example 5.9. Directive Indicator**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| %YAML 1.1                         | %YAML 1.1                         |
| --- text                          | ---                               |
| ```                               | !!str "text"                      |
|                                   | ```                               |
| ``` synopsis                      |                                   |
| Legend:                           |                                   |
|   c-directive                     |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- The []{#id870830 .indexterm}[]{#@ reserved indicator/}*"[**`@`**]{.quote}"* (**`#40`**, at) and []{#id870857 .indexterm}[]{#' reserved indicator/}*"[**`` ` ``**]{.quote}"* (**`#60`**, grave accent) are []{#id870882 .indexterm}[]{#reserved indicator/}*reserved* for future use.
:::

+--------------------------------------------------------------------------------------+
|   -------- --------------------------- ------- --------------------------------- --- |
|   \[20\]     []{#c-reserved}c-reserved  `::=`  "[@]{.quote}" \| "[\`]{.quote}"       |
|   -------- --------------------------- ------- --------------------------------- --- |
+--------------------------------------------------------------------------------------+

::: example
[]{#id870919}

**Example 5.10. Invalid use of Reserved Indicators**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| commercial-at: @text              | ERROR:                            |
| grave-accent: `text               |  Reserved indicators can't        |
| ```                               |  start a plain scalar.            |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- Any indicator character:
:::

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------- -------------- -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
|   \[21\]           []{#c-indicator}c-indicator     `::=`        ["[-]{.quote}"](#c-sequence-entry) \| ["[?]{.quote}"](#c-mapping-key) \| ["[:]{.quote}"](#c-mapping-value) \| ["[,]{.quote}"](#c-collect-entry) \| ["[\[]{.quote}"](#c-sequence-start) \| ["[\]]{.quote}"](#c-sequence-end) \| ["[{]{.quote}"](#c-mapping-start) \| ["[}]{.quote}"](#c-mapping-end)\                  |
|                                                               \| ["[\#]{.quote}"](#c-comment) \| ["[&]{.quote}"](#c-anchor) \| ["[\*]{.quote}"](#c-alias) \| ["[!]{.quote}"](#c-tag) \| ["[\|]{.quote}"](#c-literal) \| ["[\>]{.quote}"](#c-folded) \| ["[\']{.quote}"](#c-single-quote) \| ["[\"]{.quote}"](#c-double-quote)\                                                        |
|                                                               \| ["[%]{.quote}"](#c-directive) \| ["[@]{.quote}" \| "[\`]{.quote}"](#c-reserved)                                                                                                                                                                                                                                      |
|                                                                                                                                                                                                                                                                                                                                                                                       |
|   -------------- ----------------------------- -------------- -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
:::::::::::::::::::::::::::::::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id871136}5.4. Line Break Characters {#line-break-characters .title style="clear: both"}
:::
::::
:::::

The Unicode standard defines the following []{#id871145 .indexterm}[]{#line break character/}*line break* characters:

+--------------------------------------------------------------------------------------------+
|   -------- ------------------------------------------------- ------- ----------------- --- |
|   \[22\]                         []{#b-line-feed}b-line-feed  `::=`  #xA /\*LF\*/          |
|   \[23\]             []{#b-carriage-return}b-carriage-return  `::=`  #xD /\*CR\*/          |
|   \[24\]                         []{#b-next-line}b-next-line  `::=`  #x85 /\*NEL\*/        |
|   \[25\]               []{#b-line-separator}b-line-separator  `::=`  #x2028 /\*LS\*/       |
|   \[26\]     []{#b-paragraph-separator}b-paragraph-separator  `::=`  #x2029 /\*PS\*/       |
|   -------- ------------------------------------------------- ------- ----------------- --- |
+--------------------------------------------------------------------------------------------+

A YAML []{#id871235 .indexterm}[processor](#processor/) must accept all the possible Unicode line break characters.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------- -------------- ---------------------------------------------------------------------------------------------------------- -------------- |
|   \[27\]           []{#b-char}b-char     `::=`        [b-line-feed](#b-line-feed) \| [b-carriage-return](#b-carriage-return) \| [b-next-line](#b-next-line)\                  |
|                                                     \| [b-line-separator](#b-line-separator) \| [b-paragraph-separator](#b-paragraph-separator)                               |
|                                                                                                                                                                               |
|   -------------- ------------------- -------------- ---------------------------------------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Line breaks can be grouped into two categories. []{#id871293 .indexterm}[]{#specific line break/}*Specific line breaks* have well-defined semantics for breaking text into lines and paragraphs, and must be preserved by the YAML []{#id871310 .indexterm}[processor](#processor/) inside []{#id871322 .indexterm}[scalar content](#scalar/syntax).

+-----------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------- ------- ------------------------------------------------------------------------------------------ --- |
|   \[28\]     []{#b-specific}b-specific  `::=`  [b-line-separator](#b-line-separator) \| [b-paragraph-separator](#b-paragraph-separator)       |
|   -------- --------------------------- ------- ------------------------------------------------------------------------------------------ --- |
+-----------------------------------------------------------------------------------------------------------------------------------------------+

[]{#id871365 .indexterm}[]{#generic line break/}*Generic line breaks* do not carry a meaning beyond "[ending a line]{.quote}". Unlike specific line breaks, there are several widely used forms for generic line breaks.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
|   \[29\]           []{#b-generic}b-generic     `::=`        ( [b-carriage-return](#b-carriage-return) [b-line-feed](#b-line-feed) ) /\* DOS, Windows \*/\                  |
|                                                           \| [b-carriage-return](#b-carriage-return)                 /\* Macintosh \*/\                                    |
|                                                           \| [b-line-feed](#b-line-feed)                       /\* UNIX \*/\                                               |
|                                                           \| [b-next-line](#b-next-line)                       /\* Unicode \*/                                             |
|                                                                                                                                                                            |
|   -------------- ------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Generic line breaks inside []{#id871447 .indexterm}[scalar content](#scalar/syntax) must be []{#id871460 .indexterm}[]{#line break normalization/}*normalized* by the YAML []{#id871475 .indexterm}[processor](#processor/). Each such line break must be []{#id871488 .indexterm}[parsed](#parse/) into a single line feed character. The original line break form is a []{#id871502 .indexterm}[presentation detail](#presentation%20detail/) and must not be used to convey []{#id871516 .indexterm}[content information](#content/information%20model).

+-----------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------- ------- ---------------------------------------------------------------- --- |
|   \[30\]     []{#b-as-line-feed}b-as-line-feed  `::=`  [b-generic](#b-generic)                                              |
|   \[31\]         []{#b-normalized}b-normalized  `::=`  [b-as-line-feed](#b-as-line-feed) \| [b-specific](#b-specific)       |
|   -------- ----------------------------------- ------- ---------------------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------------+

Normalization does not apply to ignored ([]{#id871577 .indexterm}[escaped](#escaped%20(ignored)%20line%20break/) or []{#id871594 .indexterm}[chomped](#chomping/)) generic line breaks.

+--------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------- ------- ------------------------- --- |
|   \[32\]     []{#b-ignored-generic}b-ignored-generic  `::=`  [b-generic](#b-generic)       |
|   -------- ----------------------------------------- ------- ------------------------- --- |
+--------------------------------------------------------------------------------------------+

Outside []{#id871628 .indexterm}[scalar content](#scalar/syntax), YAML allows any line break to be used to terminate lines.

+-----------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------- ------- ------------------------------------------------------ --- |
|   \[33\]     []{#b-ignored-any}b-ignored-any  `::=`  [b-generic](#b-generic) \| [b-specific](#b-specific)       |
|   -------- --------------------------------- ------- ------------------------------------------------------ --- |
+-----------------------------------------------------------------------------------------------------------------+

On output, a YAML []{#id871673 .indexterm}[processor](#processor/) is free to []{#id871685 .indexterm}[present](#present/) line breaks using whatever convention is most appropriate, though specific line breaks must be preserved in []{#id871700 .indexterm}[scalar content](#scalar/syntax). These rules are compatible with [Unicode's newline guidelines](http://www.unicode.org/unicode/reports/tr13/){target="_top"}.

In the examples, line break characters are displayed as follows: "[**`↓`**]{.quote}" or no glyph for a generic line break, "[**`⇓`**]{.quote}" for a line separator and "[**`¶`**]{.quote}" for a paragraph separator.

::: example
[]{#id871752}

**Example 5.11. Line Break Characters**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| |                                 | %YAML 1.1                         |
|   Generic line break (no glyph)   | --- !!str                         |
|   Generic line break (glyphed)↓   | "Generic line break (no glyph)\n\ |
|   Line separator⇓                 |  Generic line break (glyphed)\n\  |
|   Paragraph separator¶            |  Line separator\u2028\            |
| ```                               |  Paragraph separator\u2029"       |
|                                   | ```                               |
| ``` synopsis                      |                                   |
| Legend:                           |                                   |
|   b-generic b-line-separator      |                                   |
|   b-paragraph-separator           |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::
:::::::

::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id871856}5.5. Miscellaneous Characters {#miscellaneous-characters .title style="clear: both"}

y[char.misc.tag-preserve+2]

:::
::::
:::::

The YAML syntax productions make use of the following character range definitions:

::: itemizedlist
- A non-[]{#id871872 .indexterm}[break](#line%20break%20character/) character:
:::

+------------------------------------------------------------------------------------------------+
|   -------- --------------------- ------- ------------------------------------------------- --- |
|   \[34\]     []{#nb-char}nb-char  `::=`  [c-printable](#c-printable) - [b-char](#b-char)       |
|   -------- --------------------- ------- ------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------+

::: itemizedlist
- An ignored space character outside []{#id871918 .indexterm}[scalar content](#scalar/syntax). Such spaces are used for []{#id871933 .indexterm}[indentation](#indentation%20space/) and []{#id871947 .indexterm}[separation](#separation%20space/) between tokens. To maintain portability, []{#id871963 .indexterm}[]{#tab/}*tab* characters must not be used in these cases, since different systems treat tabs differently. Note that most modern editors may be configured so that pressing the tab key results in the insertion of an appropriate number of spaces.
:::

+------------------------------------------------------------------------------+
|   -------- ------------------------------------- ------- --------------- --- |
|   \[35\]     []{#s-ignored-space}s-ignored-space  `::=`  #x20 /\*SP\*/       |
|   -------- ------------------------------------- ------- --------------- --- |
+------------------------------------------------------------------------------+

::: example
[]{#id871998}

**Example 5.12. Invalid Use of Tabs**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| # Tabs do's and don'ts:           | ERROR:                            |
| # comment: →                      |  Tabs may appear inside           |
| quoted: "Quoted →"                |  comments and quoted or           |
| block: |                          |  block scalar content.            |
|   void main() {                   |  Tabs must not appear             |
|   →printf("Hello, world!\n");     |  elsewhere, such as               |
|   }                               |  in indentation and               |
| elsewhere:→# separation           |  separation spaces.               |
| →indentation, in→plain scalar     | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- A []{#id872096 .indexterm}[]{#white space/}*white space* character in []{#id872110 .indexterm}[quoted](#quoted%20style/syntax) or []{#id872127 .indexterm}[block scalar content](#block%20scalar%20style/syntax):
:::

+-------------------------------------------------------------------------------+
|   -------- --------------------- ------- -------------------------------- --- |
|   \[36\]     []{#s-white}s-white  `::=`  #x9 /\*TAB\*/ \| #x20 /\*SP\*/       |
|   -------- --------------------- ------- -------------------------------- --- |
+-------------------------------------------------------------------------------+

In the examples, tab characters are displayed as the glyph "[**`→`**]{.quote}". Space characters are sometimes displayed as the glyph "[**`·`**]{.quote}" for clarity.

::: example
[]{#id872184}

**Example 5.13. Tabs and Spaces**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ··"Text·containing···             | %YAML 1.1                         |
| ··both·space·and→                 | --- !!str                         |
| ··→tab→characters"                | "Text·containing·\                |
| ```                               |  both·space·and·\                 |
|                                   |  tab→characters"                  |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   #x9 (TAB) #x20 (SP)             |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: itemizedlist
- An ignored white space character inside []{#id872327 .indexterm}[scalar content](#scalar/syntax):
:::

+------------------------------------------------------------------------------------+
|   -------- ------------------------------------- ------- --------------------- --- |
|   \[37\]     []{#s-ignored-white}s-ignored-white  `::=`  [s-white](#s-white)       |
|   -------- ------------------------------------- ------- --------------------- --- |
+------------------------------------------------------------------------------------+

::: itemizedlist
- A non space (and non-[]{#id872367 .indexterm}[break](#line%20break%20character/)) character:
:::

+------------------------------------------------------------------------------------------+
|   -------- --------------------- ------- ------------------------------------------- --- |
|   \[38\]     []{#ns-char}ns-char  `::=`  [nb-char](#nb-char) - [s-white](#s-white)       |
|   -------- --------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------+

::: itemizedlist
- A decimal digit for numbers:
:::

+----------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- ------------------------- --- |
|   \[39\]     []{#ns-dec-digit}ns-dec-digit  `::=`  \[#x30-#x39\] /\*0-9\*/       |
|   -------- ------------------------------- ------- ------------------------- --- |
+----------------------------------------------------------------------------------+

::: itemizedlist
- A hexadecimal digit for []{#id872437 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/):
:::

+----------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- ------------------------------------------------------------------------------------- --- |
|   \[40\]     []{#ns-hex-digit}ns-hex-digit  `::=`  [ns-dec-digit](#ns-dec-digit) \| \[#x41-#x46\] /\*A-F\*/ \| \[#x61-#x66\] /\*a-f\*/       |
|   -------- ------------------------------- ------- ------------------------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- An ASCII letter (alphabetic) character:
:::

+-------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------------- ------- ---------------------------------------------------- --- |
|   \[41\]     []{#ns-ascii-letter}ns-ascii-letter  `::=`  \[#x41-#x5A\] /\*A-Z\*/ \| \[#x61-#x7A\] /\*a-z\*/       |
|   -------- ------------------------------------- ------- ---------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- A word (alphanumeric) character for identifiers:
:::

+------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- --------------------------------------------------------------------------------------- --- |
|   \[42\]     []{#ns-word-char}ns-word-char  `::=`  [ns-dec-digit](#ns-dec-digit) \| [ns-ascii-letter](#ns-ascii-letter) \| "[-]{.quote}"       |
|   -------- ------------------------------- ------- --------------------------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- A URI character for []{#id872540 .indexterm}[tags](#tag/syntax), as specified in [RFC2396](http://www.ietf.org/rfc/rfc2396.txt){target="_top"} with the addition of the []{#id872560 .indexterm}["[**`[`**]{.quote}"](#%5B%20start%20flow%20sequence/) and []{#id872579 .indexterm}["[**`]`**]{.quote}"](#%5D%20end%20flow%20sequence/) for presenting IPv6 addresses as proposed in [RFC2732](http://www.ietf.org/rfc/rfc2732.txt){target="_top"}. A limited form of 8-bit []{#id872604 .indexterm}[]{#escaping in URI/}*escaping* is available using the []{#id872620 .indexterm}[]{#% escaping in URI/}*"[**`%`**]{.quote}"* character. By convention, URIs containing 16 and 32 bit Unicode characters are []{#id872638 .indexterm}[encoded](#character%20encoding/) in UTF-8, and then each octet is written as a separate character.
:::

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------- -------------- --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
|   \[43\]           []{#ns-uri-char}ns-uri-char     `::=`        [ns-word-char](#ns-word-char) \| "[%]{.quote}" [ns-hex-digit](#ns-hex-digit) [ns-hex-digit](#ns-hex-digit)\                                                                                    |
|                                                               \| "[;]{.quote}" \| "[/]{.quote}" \| "[?]{.quote}" \| "[:]{.quote}" \| "[@]{.quote}" \| "[&]{.quote}" \| "[=]{.quote}" \| "[+]{.quote}" \| "[\$]{.quote}" \| "[,]{.quote}"\                      |
|                                                               \| "[\_]{.quote}" \| "[.]{.quote}" \| "[!]{.quote}" \| "[\~]{.quote}" \| "[\*]{.quote}" \| "[\']{.quote}" \| "[(]{.quote}" \| "[)]{.quote}" \| "[\[]{.quote}" \| "[\]]{.quote}"                  |
|                                                                                                                                                                                                                                                                |
|   -------------- ----------------------------- -------------- --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- The []{#id872764 .indexterm}["[**`!`**]{.quote}"](#!%20named%20handle/) character is used to indicate the end of a []{#id872783 .indexterm}[named tag handle](#named%20tag%20handle/); hence its use in []{#id872800 .indexterm}[tag shorthands](#tag%20shorthand/) is restricted.
:::

+--------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- ------------------------------------------------------- --- |
|   \[44\]     []{#ns-tag-char}ns-tag-char  `::=`  [ns-uri-char](#ns-uri-char) - ["[!]{.quote}"](#c-tag)       |
|   -------- ----------------------------- ------- ------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------+
:::::::::::::::::::

:::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id872840}5.6. Escape Sequences {#escape-sequences .title style="clear: both"}
:::
::::
:::::

All non-[]{#id872849 .indexterm}[printable](#printable%20character/) characters must be []{#id872862 .indexterm}[presented](#present/) as []{#id872875 .indexterm}[]{#escaping in double-quoted style/}*escape sequences*. Each escape sequences must be []{#id872891 .indexterm}[parsed](#parse/) into the appropriate Unicode character. The original escape sequence form is a []{#id872905 .indexterm}[presentation detail](#presentation%20detail/) and must not be used to convey []{#id872919 .indexterm}[content information](#content/information%20model). YAML escape sequences use the []{#id872936 .indexterm}[]{#\\ escaping in double-quoted style/}*"[**`\`**]{.quote}"* notation common to most modern computer languages. Note that escape sequences are only interpreted in []{#id872960 .indexterm}[double-quoted scalars](#double-quoted%20style/syntax). In all other []{#id872974 .indexterm}[scalar styles](#scalar/syntax), the []{#id872989 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/) character has no special meaning and non-[]{#id873012 .indexterm}[printable](#printable%20character/) characters are not available.

+-----------------------------------------------------------------------+
|   -------- ----------------------- ------- ---------------- ---       |
|   \[45\]     []{#c-escape}c-escape  `::=`  "[\\]{.quote}"             |
|   -------- ----------------------- ------- ---------------- ---       |
+-----------------------------------------------------------------------+

YAML escape sequences are a superset of C's escape sequences:

::: itemizedlist
- Escaped ASCII null (**`#x0`**) character:
:::

+--------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- ------------------------------------------- --- |
|   \[46\]     []{#ns-esc-null}ns-esc-null  `::=`  ["[\\]{.quote}"](#c-escape) "[0]{.quote}"       |
|   -------- ----------------------------- ------- ------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII bell (**`#x7`**) character:
:::

+--------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- ------------------------------------------- --- |
|   \[47\]     []{#ns-esc-bell}ns-esc-bell  `::=`  ["[\\]{.quote}"](#c-escape) "[a]{.quote}"       |
|   -------- ----------------------------- ------- ------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII backspace (**`#x8`**) character:
:::

+------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
|   \[48\]     []{#ns-esc-backspace}ns-esc-backspace  `::=`  ["[\\]{.quote}"](#c-escape) "[b]{.quote}"       |
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII horizontal []{#id873168 .indexterm}[tab](#tab/) (**`#x9`**) character:
:::


y[char.ns-esc-horizontal-tab+2]

+---------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------------------------- ------- ------------------------------------------------------------------------------ --- |
|   \[49\]     []{#ns-esc-horizontal-tab}ns-esc-horizontal-tab  `::=`  ["[\\]{.quote}"](#c-escape) "[t]{.quote}" \| ["[\\]{.quote}"](#c-escape) #x9       |
|   -------- ------------------------------------------------- ------- ------------------------------------------------------------------------------ --- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII []{#id873225 .indexterm}[line feed](#generic%20line%20break/) (**`#xA`**) character:
:::

+------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
|   \[50\]     []{#ns-esc-line-feed}ns-esc-line-feed  `::=`  ["[\\]{.quote}"](#c-escape) "[n]{.quote}"       |
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII vertical tab (**`#xB`**) character:
:::

+------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------- ------- ------------------------------------------- --- |
|   \[51\]     []{#ns-esc-vertical-tab}ns-esc-vertical-tab  `::=`  ["[\\]{.quote}"](#c-escape) "[v]{.quote}"       |
|   -------- --------------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII form feed (**`#xC`**) character:
:::

+------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
|   \[52\]     []{#ns-esc-form-feed}ns-esc-form-feed  `::=`  ["[\\]{.quote}"](#c-escape) "[f]{.quote}"       |
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII []{#id873355 .indexterm}[carriage return](#generic%20line%20break/) (**`#xD`**) character:
:::

+------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------------- ------- ------------------------------------------- --- |
|   \[53\]     []{#ns-esc-carriage-return}ns-esc-carriage-return  `::=`  ["[\\]{.quote}"](#c-escape) "[r]{.quote}"       |
|   -------- --------------------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII escape (**`#x1B`**) character:
:::

+------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------- ------- ------------------------------------------- --- |
|   \[54\]     []{#ns-esc-escape}ns-esc-escape  `::=`  ["[\\]{.quote}"](#c-escape) "[e]{.quote}"       |
|   -------- --------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII space (**`#x20`**) character:
:::

+-------------------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- ---------------------------------- --- |
|   \[55\]     []{#ns-esc-space}ns-esc-space  `::=`  ["[\\]{.quote}"](#c-escape) #x20       |
|   -------- ------------------------------- ------- ---------------------------------- --- |
+-------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII double quote ([]{#id873481 .indexterm}["[**`"`**]{.quote}"](#%22%20double-quoted%20style/)):
:::

+--------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------- ------- --------------------------------------------------------------- --- |
|   \[56\]     []{#ns-esc-double-quote}ns-esc-double-quote  `::=`  ["[\\]{.quote}"](#c-escape) ["[\"]{.quote}"](#c-double-quote)       |
|   -------- --------------------------------------------- ------- --------------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped ASCII back slash ([]{#id873536 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/)):
:::

+--------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- --------------------------------------------------------- --- |
|   \[57\]     []{#ns-esc-backslash}ns-esc-backslash  `::=`  ["[\\]{.quote}"](#c-escape) ["[\\]{.quote}"](#c-escape)       |
|   -------- --------------------------------------- ------- --------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped Unicode []{#id873590 .indexterm}[next line](#generic%20line%20break/) (**`#x85`**) character:
:::

+------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
|   \[58\]     []{#ns-esc-next-line}ns-esc-next-line  `::=`  ["[\\]{.quote}"](#c-escape) "[N]{.quote}"       |
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped Unicode non-breaking space (**`#xA0`**) character:
:::

+-------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------------------- ------- -------------------------------------------- --- |
|   \[59\]     []{#ns-esc-non-breaking-space}ns-esc-non-breaking-space  `::=`  ["[\\]{.quote}"](#c-escape) "[\_]{.quote}"       |
|   -------- --------------------------------------------------------- ------- -------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped Unicode []{#id873679 .indexterm}[line separator](#specific%20line%20break/) (**`#x2028`**) character:
:::

+----------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------------------------- ------- ------------------------------------------- --- |
|   \[60\]     []{#ns-esc-line-separator}ns-esc-line-separator  `::=`  ["[\\]{.quote}"](#c-escape) "[L]{.quote}"       |
|   -------- ------------------------------------------------- ------- ------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped Unicode []{#id873732 .indexterm}[paragraph separator](#specific%20line%20break/) (**`#x2029`**) character:
:::

+--------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------------------------- ------- ------------------------------------------- --- |
|   \[61\]     []{#ns-esc-paragraph-separator}ns-esc-paragraph-separator  `::=`  ["[\\]{.quote}"](#c-escape) "[P]{.quote}"       |
|   -------- ----------------------------------------------------------- ------- ------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped 8-bit Unicode character:
:::

+------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- --------------------------------------------------------------------------------- --- |
|   \[62\]     []{#ns-esc-8-bit}ns-esc-8-bit  `::=`  ["[\\]{.quote}"](#c-escape) "[x]{.quote}" ( [ns-hex-digit](#ns-hex-digit) x 2 )       |
|   -------- ------------------------------- ------- --------------------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped 16-bit Unicode character:
:::

+--------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------- ------- --------------------------------------------------------------------------------- --- |
|   \[63\]     []{#ns-esc-16-bit}ns-esc-16-bit  `::=`  ["[\\]{.quote}"](#c-escape) "[u]{.quote}" ( [ns-hex-digit](#ns-hex-digit) x 4 )       |
|   -------- --------------------------------- ------- --------------------------------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Escaped 32-bit Unicode character:
:::

+--------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------- ------- --------------------------------------------------------------------------------- --- |
|   \[64\]     []{#ns-esc-32-bit}ns-esc-32-bit  `::=`  ["[\\]{.quote}"](#c-escape) "[U]{.quote}" ( [ns-hex-digit](#ns-hex-digit) x 8 )       |
|   -------- --------------------------------- ------- --------------------------------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Any escaped character:
:::

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------- -------------- --------------------------------------------------------------------------------------------------------------------------- -------------- |
|   \[65\]           []{#ns-esc-char}ns-esc-char     `::=`        [ns-esc-null](#ns-esc-null) \| [ns-esc-bell](#ns-esc-bell) \| [ns-esc-backspace](#ns-esc-backspace)\                                     |
|                                                               \| [ns-esc-horizontal-tab](#ns-esc-horizontal-tab) \| [ns-esc-line-feed](#ns-esc-line-feed)\                                               |
|                                                               \| [ns-esc-vertical-tab](#ns-esc-vertical-tab) \| [ns-esc-form-feed](#ns-esc-form-feed)\                                                   |
|                                                               \| [ns-esc-carriage-return](#ns-esc-carriage-return) \| [ns-esc-escape](#ns-esc-escape) \| [ns-esc-space](#ns-esc-space)\                  |
|                                                               \| [ns-esc-double-quote](#ns-esc-double-quote) \| [ns-esc-backslash](#ns-esc-backslash)\                                                   |
|                                                               \| [ns-esc-next-line](#ns-esc-next-line) \| [ns-esc-non-breaking-space](#ns-esc-non-breaking-space)\                                       |
|                                                               \| [ns-esc-line-separator](#ns-esc-line-separator) \| [ns-esc-paragraph-separator](#ns-esc-paragraph-separator)\                           |
|                                                               \| [ns-esc-8-bit](#ns-esc-8-bit) \| [ns-esc-16-bit](#ns-esc-16-bit) \| [ns-esc-32-bit](#ns-esc-32-bit)\                                    |
|                                                                                                                                                                                                          |
|   -------------- ----------------------------- -------------- --------------------------------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id891492}

**Example 5.14. Escaped Characters**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| "Fun with \\                      | %YAML 1.1                         |
|  \" \a \b \e \f \↓                | ---                               |
|  \n \r \t \v \0 \⇓                | "Fun with \x5C                    |
|  \  \_ \N \L \P \¶                |  \x22 \x07 \x08 \x1B \0C          |
|  \x41 \u0041 \U00000041"          |  \x0A \x0D \x09 \x0B \x00         |
| ```                               |  \x20 \xA0 \x85 \u2028 \u2029     |
|                                   |  A A A"                           |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   ns-esc-char                     |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: example
[]{#id891672}

**Example 5.15. Invalid Escaped Characters**

+-----------------------------------+--------------------------------------+
| ``` screen                        | ``` screen                           |
| Bad escapes:                      | ERROR:                               |
|   "\c                             | - c is an invalid escaped character. |
|   \xq-"                           | - q and - are invalid hex digits.    |
| ```                               | ```                                  |
+-----------------------------------+--------------------------------------+
:::
::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id891745}Chapter 6. Syntax Primitives {#chapter-6.-syntax-primitives .title}
:::
::::
:::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id891751}6.1. Indentation Spaces {#indentation-spaces .title style="clear: both"}
:::
::::
:::::

In a YAML character []{#id891760 .indexterm}[stream](#stream/syntax), structure is often determined from []{#id891775 .indexterm}[]{#indentation space/}*indentation*, where indentation is defined as a []{#id891791 .indexterm}[line break](#line%20break%20character/) character (or the start of the []{#id891806 .indexterm}[stream](#stream/syntax)) followed by zero or more space characters. Note that indentation must not contain any []{#id891822 .indexterm}[tab](#tab/) characters. The amount of indentation is a []{#id891834 .indexterm}[presentation detail](#presentation%20detail/) used exclusively to delineate structure and is otherwise ignored. In particular, indentation characters must never be considered part of a []{#id891852 .indexterm}[node's content information](#content/information%20model).

+------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- ----------------------------------------- --- |
|   \[66\]     []{#s-indent(n)}s-indent(n)  `::=`  [s-ignored-space](#s-ignored-space) x n       |
|   -------- ----------------------------- ------- ----------------------------------------- --- |
+------------------------------------------------------------------------------------------------+

::: example
[]{#id891888}

**Example 6.1. Indentation Spaces**

+---------------------------------------+--------------------------------------+
| ``` programlisting                    | ``` programlisting                   |
| ··# Leading comment line spaces are   | %YAML 1.1                            |
| ···# neither content nor indentation. | ---                                  |
| ····                                  | !!map {                              |
| Not indented:                         |   ? !!str "Not indented"             |
| ·By one space: |                      |   : !!map {                          |
| ····By four                           |       ? !!str "By one space"         |
| ······spaces                          |       : !!str "By four\n  spaces\n", |
| ·Flow style: [    # Leading spaces    |       ? !!str "Flow style"           |
| ···By two,        # in flow style     |       : !!seq [                      |
| ··Also by two,    # are neither       |           !!str "By two",            |
| ··→Still by two   # content nor       |           !!str "Still by two",      |
| ····]             # indentation.      |           !!str "Again by two",      |
| ```                                   |         ]                            |
|                                       |     }                                |
| ``` synopsis                          | }                                    |
| Legend:                               | ```                                  |
|   s-indent(n) Content                 |                                      |
|   Neither content nor indentation     |                                      |
| ```                                   |                                      |
+---------------------------------------+--------------------------------------+
:::

In general, a []{#id892050 .indexterm}[node](#node/syntax) must be indented further than its parent []{#id892067 .indexterm}[node](#node/syntax). All sibling []{#id892082 .indexterm}[nodes](#node/syntax) must use the exact same indentation level, however the []{#id892098 .indexterm}[content](#content/syntax) of each sibling []{#id892113 .indexterm}[node](#node/syntax) may be further indented independently. The []{#id892129 .indexterm}["[**`-`**]{.quote}"](#-%20block%20sequence%20entry/), []{#id892148 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) and []{#id892165 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) characters used to denote []{#id892182 .indexterm}[block collection](#block%20collection%20style/syntax) entries are perceived by people to be part of the indentation. Hence the indentation rules are slightly more flexible when dealing with these []{#id892201 .indexterm}[indicators](#indicator/). First, a []{#id892212 .indexterm}[block sequence](#block%20sequence%20style/syntax) need not be indented relative to its parent []{#id892229 .indexterm}[node](#node/syntax), unless that []{#id892244 .indexterm}[node](#node/syntax) is also a []{#id892259 .indexterm}[block sequence](#block%20sequence%20style/syntax). Second, compact []{#id892275 .indexterm}[in-line](#in-line%20style/syntax) notations allow a nested []{#id892292 .indexterm}[collection](#collection/syntax) to begin immediately following the []{#id892309 .indexterm}[indicator](#indicator/) (where the []{#id892321 .indexterm}[indicator](#indicator/) is counted as part of the indentation). This provides for an intuitive []{#id892335 .indexterm}[collection](#collection/syntax) nesting syntax.
:::::::

::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id892353}6.2. Comments {#comments-1 .title style="clear: both"}
:::
::::
:::::

An explicit []{#id892361 .indexterm}[]{#comment/syntax}*comment* is marked by a []{#id892378 .indexterm}[]{## comment/}*"[**`#`**]{.quote}" indicator*. Comments are a []{#id892397 .indexterm}[presentation detail](#presentation%20detail/) and must have no effect on the []{#id892411 .indexterm}[serialization tree](#serialization/) (and hence the []{#id892424 .indexterm}[representation graph](#representation/)).

+-----------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------- ------- ---------------------------------------------------- --- |
|   \[67\]     []{#c-nb-comment-text}c-nb-comment-text  `::=`  ["[\#]{.quote}"](#c-comment) [nb-char](#nb-char)\*       |
|   -------- ----------------------------------------- ------- ---------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------+

Comments always span to the end of the line.

+---------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- -------------------------------------------------------------------------- --- |
|   \[68\]     []{#c-b-comment}c-b-comment  `::=`  [c-nb-comment-text](#c-nb-comment-text)? [b-ignored-any](#b-ignored-any)       |
|   -------- ----------------------------- ------- -------------------------------------------------------------------------- --- |
+---------------------------------------------------------------------------------------------------------------------------------+

Outside []{#id892496 .indexterm}[scalar content](#scalar/syntax), comments may appear on a line of their own, independent of the []{#id892513 .indexterm}[indentation](#indentation%20space/) level. Note that []{#id892528 .indexterm}[tab](#tab/) characters must not be used and that []{#id892540 .indexterm}[empty lines](#empty%20line/) outside []{#id892552 .indexterm}[scalar content](#scalar/syntax) are taken to be (empty) comment lines.

+----------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------- ------- ------------------------------------------------------------------- --- |
|   \[69\]     []{#l-comment}l-comment  `::=`  [s-ignored-space](#s-ignored-space)\* [c-b-comment](#c-b-comment)       |
|   -------- ------------------------- ------- ------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id892593}

**Example 6.2. Comment Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ··# Comment↓                      | # This stream contains no         |
| ···↓                              | # documents, only comments.       |
| ↓                                 | ```                               |
| ```                               |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   c-b-comment l-comment           |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

When a comment follows another syntax element, it must be []{#id892698 .indexterm}[separated](#separation%20space/) from it by space characters. Like the comment itself, such characters are not considered part of the []{#id892714 .indexterm}[content information](#content/information%20model).

+--------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------- -------------- ------------------------------------------------------------------------------------- -------------- |
|   \[70\]           []{#s-b-comment}s-b-comment     `::=`      ( [s-ignored-space](#s-ignored-space)+ [c-nb-comment-text](#c-nb-comment-text)? )?\                  |
|                                                               [b-ignored-any](#b-ignored-any)                                                                      |
|                                                                                                                                                                    |
|   -------------- ----------------------------- -------------- ------------------------------------------------------------------------------------- -------------- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id892762}

**Example 6.3. Comments Ending a Line**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| key:····# Comment↓                | %YAML 1.1                         |
|   value↓                          | ---                               |
| ```                               | !!map {                           |
|                                   |   ? !!str "key"                   |
| ``` synopsis                      |   : !!str "value"                 |
| Legend:                           | }                                 |
|   c-nb-comment-text s-b-comment   | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

In most cases, when a line may end with a comment, YAML allows it to be followed by additional comment lines.

+----------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------- ------- ------------------------------------------------------- --- |
|   \[71\]     []{#c-l-comments}c-l-comments  `::=`  [c-b-comment](#c-b-comment) [l-comment](#l-comment)\*       |
|   \[72\]     []{#s-l-comments}s-l-comments  `::=`  [s-b-comment](#s-b-comment) [l-comment](#l-comment)\*       |
|   -------- ------------------------------- ------- ------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------+

::: example
[]{#id892902}

**Example 6.4. Multi-Line Comments**

+--------------------------------------+-----------------------------------+
| ``` programlisting                   | ``` programlisting                |
| key:····# Comment↓                   | %YAML 1.1                         |
| ········# lines↓                     | ---                               |
|   value↓                             | !!map {                           |
| ↓                                    |   ? !!str "key"                   |
| ```                                  |   : !!str "value"                 |
|                                      | }                                 |
| ``` synopsis                         | ```                               |
| Legend:                              |                                   |
|   s-b-comment l-comment s-l-comments |                                   |
| ```                                  |                                   |
+--------------------------------------+-----------------------------------+
:::
:::::::::

::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id893014}6.3. Separation Spaces {#separation-spaces .title style="clear: both"}
:::
::::
:::::

Outside []{#id893023 .indexterm}[scalar content](#scalar/syntax), YAML uses space characters for []{#id893039 .indexterm}[]{#separation space/}*separation* between tokens. Note that separation must not contain []{#id893056 .indexterm}[tab](#tab/) characters. Separation spaces are a []{#id893068 .indexterm}[presentation detail](#presentation%20detail/) used exclusively to delineate structure and are otherwise ignored; in particular, such characters must never be considered part of a []{#id893085 .indexterm}[node's content information](#content/information%20model).

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
|   \[73\]           []{#s-separate(n,c)}s-separate(n,c)     `::=`      `c`{.varname} = block-out ⇒ [s-separate-lines(n)](#s-separate-lines(n))\                  |
|                                                                       `c`{.varname} = block-in  ⇒ [s-separate-lines(n)](#s-separate-lines(n))\                  |
|                                                                       `c`{.varname} = flow-out  ⇒ [s-separate-lines(n)](#s-separate-lines(n))\                  |
|                                                                       `c`{.varname} = flow-in   ⇒ [s-separate-lines(n)](#s-separate-lines(n))\                  |
|                                                                       `c`{.varname} = flow-key  ⇒ [s-separate-spaces](#s-separate-spaces)                       |
|                                                                                                                                                                 |
|   -------------- ------------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- YAML usually allows separation spaces to include a []{#id893173 .indexterm}[comment](#comment/syntax) ending the line and additional []{#id893188 .indexterm}[comment](#comment/syntax) lines. Note that the token following the separation []{#id893204 .indexterm}[comment](#comment/syntax) lines must be properly []{#id893219 .indexterm}[indented](#indentation%20space/), even though there is no such restriction on the separation []{#id893234 .indexterm}[comment](#comment/syntax) lines themselves.
:::

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- -------------------------------------------------------------------------------------------------------- -------------- |
|   \[74\]           []{#s-separate-lines(n)}s-separate-lines(n)     `::=`        [s-ignored-space](#s-ignored-space)+\                                                                                 |
|                                                                               \| ( [s-l-comments](#s-l-comments) [s-indent(n)](#s-indent(n)) [s-ignored-space](#s-ignored-space)\* )                  |
|                                                                                                                                                                                                       |
|   -------------- --------------------------------------------- -------------- -------------------------------------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Inside []{#id893293 .indexterm}[simple keys](#simple%20key/), however, separation spaces are confined to the current line.
:::

+---------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------- ------- -------------------------------------- --- |
|   \[75\]     []{#s-separate-spaces}s-separate-spaces  `::=`  [s-ignored-space](#s-ignored-space)+       |
|   -------- ----------------------------------------- ------- -------------------------------------- --- |
+---------------------------------------------------------------------------------------------------------+

::: example
[]{#id893330}

**Example 6.5. Separation Spaces**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| {·first:·Sammy,·last:·Sosa·}:↓    | %YAML 1.1                         |
| # Statistics:                     | ---                               |
| ··hr:··# Home runs                | !!map {                           |
| ····65                            |   ? !!map {                       |
| ··avg:·# Average                  |     ? !!str "first"               |
| ····0.278                         |     : !!str "Sammy",              |
| ```                               |     ? !!str "last"                |
|                                   |     : !!str "Sosa"                |
| ``` synopsis                      |   }                               |
| Legend:                           |   : !!map {                       |
|   s-separate-spaces               |     ? !!str "hr"                  |
|   s-separate-lines(n)             |     : !!int "65",                 |
|   s-indent(n)                     |     ? !!str "avg"                 |
| ```                               |     : !!float "0.278"             |
|                                   |   }                               |
|                                   | }                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::

::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id893482}6.4. Ignored Line Prefix {#ignored-line-prefix .title style="clear: both"}
:::
::::
:::::

YAML discards the "[empty]{.quote}" []{#id893494 .indexterm}[]{#ignored line prefix/}*prefix* of each []{#id893511 .indexterm}[scalar content](#scalar/syntax) line. This prefix always includes the []{#id893526 .indexterm}[indentation](#indentation%20space/), and depending on the scalar style may also include all leading []{#id893541 .indexterm}[white space](#white%20space/). The ignored prefix is a []{#id893554 .indexterm}[presentation detail](#presentation%20detail/) and must never be considered part of a []{#id893568 .indexterm}[node's content information](#content/information%20model).

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
|   \[76\]           []{#s-ignored-prefix(n,s)}s-ignored-prefix(n,s)     `::=`      `s`{.varname} = plain   ⇒ [s-ignored-prefix-plain(n)](#s-ignored-prefix-plain(n))\                    |
|                                                                                   `s`{.varname} = double  ⇒ [s-ignored-prefix-quoted(n)](#s-ignored-prefix-quoted(n))\                  |
|                                                                                   `s`{.varname} = single  ⇒ [s-ignored-prefix-quoted(n)](#s-ignored-prefix-quoted(n))\                  |
|                                                                                   `s`{.varname} = literal ⇒ [s-ignored-prefix-block(n)](#s-ignored-prefix-block(n))\                    |
|                                                                                   `s`{.varname} = folded  ⇒ [s-ignored-prefix-block(n)](#s-ignored-prefix-block(n))                     |
|                                                                                                                                                                                         |
|   -------------- ------------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Plain scalars must not contain any []{#id893656 .indexterm}[tab](#tab/) characters, and all leading spaces are always discarded.
:::

+------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------------------- ------- ------------------------------------------------------------------- --- |
|   \[77\]     []{#s-ignored-prefix-plain(n)}s-ignored-prefix-plain(n)  `::=`  [s-indent(n)](#s-indent(n)) [s-ignored-space](#s-ignored-space)\*       |
|   -------- --------------------------------------------------------- ------- ------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Quoted scalars may contain []{#id893701 .indexterm}[tab](#tab/) characters. Again, all leading []{#id893714 .indexterm}[white space](#white%20space/) is always discarded.
:::

+--------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------------------------- ------- ------------------------------------------------------------------- --- |
|   \[78\]     []{#s-ignored-prefix-quoted(n)}s-ignored-prefix-quoted(n)  `::=`  [s-indent(n)](#s-indent(n)) [s-ignored-white](#s-ignored-white)\*       |
|   -------- ----------------------------------------------------------- ------- ------------------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Block scalars rely on []{#id893759 .indexterm}[indentation](#indentation%20space/); hence leading []{#id893775 .indexterm}[white space](#white%20space/), if any, is not discarded.
:::

+----------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------------------- ------- ----------------------------- --- |
|   \[79\]     []{#s-ignored-prefix-block(n)}s-ignored-prefix-block(n)  `::=`  [s-indent(n)](#s-indent(n))       |
|   -------- --------------------------------------------------------- ------- ----------------------------- --- |
+----------------------------------------------------------------------------------------------------------------+

::: example
[]{#id893810}

**Example 6.6. Ignored Prefix**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| plain: text                       | %YAML 1.1                         |
| ··lines                           | ---                               |
| quoted: "text                     | !!map {                           |
| ··→lines"                         |   ? !!str "plain"                 |
| block: |                          |   : !!str "text lines",           |
| ··text                            |   ? !!str "quoted"                |
| ···→lines                         |   : !!str "text lines",           |
| ```                               |   ? !!str "block"                 |
|                                   |   : !!str "text·→lines\n"         |
| ``` synopsis                      | }                                 |
| Legend:                           | ```                               |
|   s-ignored-prefix-plain(n)       |                                   |
|   s-ignored-prefix-quoted(n)      |                                   |
|   s-ignored-prefix-block(n)       |                                   |
|   s-indent(n)                     |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

An []{#id893944 .indexterm}[]{#empty line/}*empty line* line consists of the ignored prefix followed by a []{#id893956 .indexterm}[line break](#line%20break%20character/). When trailing []{#id893972 .indexterm}[block scalars](#block%20scalar%20style/syntax), such lines can also be interpreted as (empty) []{#id893987 .indexterm}[comment](#comment/syntax) lines. YAML provides a []{#id894002 .indexterm}[chomping](#chomping/) mechanism to resolve this ambiguity.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------- -------------- --------------------------------------------------------------------------------------- -------------- |
|   \[80\]           []{#l-empty(n,s)}l-empty(n,s)     `::=`      ( [s-indent(\<n)](#s-indent(n)) \| [s-ignored-prefix(n,s)](#s-ignored-prefix(n,s)) )\                  |
|                                                                 [b-normalized](#b-normalized)                                                                          |
|                                                                                                                                                                        |
|   -------------- ------------------------------- -------------- --------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id894049}

**Example 6.7. Empty Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - foo                             | %YAML 1.1                         |
| ·↓                                | ---                               |
|   bar                             | !!seq {                           |
| - |-                              |   !!str "foo\nbar",               |
|   foo                             |   !!str "foo\n\nbar"              |
| ·↓                                | }                                 |
|   bar                             | ```                               |
| ··↓                               |                                   |
| ```                               | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   l-empty(n,s)                    |
|                                   |   l-comment                       |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::::

:::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id894136}6.5. Line Folding {#line-folding .title style="clear: both"}
:::
::::
:::::

[]{#id894144 .indexterm}[]{#line folding/}*Line folding* allows long lines to be broken for readability, while retaining the original semantics of a single long line. When folding is done, any []{#id894160 .indexterm}[line break](#line%20break%20character/) ending an []{#id894174 .indexterm}[empty line](#empty%20line/) is preserved. In addition, any []{#id894187 .indexterm}[specific line breaks](#specific%20line%20break/) are also preserved, even when ending a non-[]{#id894201 .indexterm}[empty line](#empty%20line/).

+--------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------------------------------- ------- ----------------------------------------------------------- --- |
|   \[81\]     []{#b-l-folded-specific(n,s)}b-l-folded-specific(n,s)  `::=`  [b-specific](#b-specific) [l-empty(n,s)](#l-empty(n,s))\*       |
|   -------- ------------------------------------------------------- ------- ----------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------------+

Hence, folding only applies to []{#id894242 .indexterm}[generic line breaks](#generic%20line%20break/) that end non-[]{#id894257 .indexterm}[empty lines](#empty%20line/). If the following line is also not []{#id894269 .indexterm}[empty](#empty%20line/), the []{#id894281 .indexterm}[generic line break](#generic%20line%20break/) is converted to a single space (**`#x20`**).

+------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------- ------- ------------------------- --- |
|   \[82\]     []{#b-l-folded-as-space}b-l-folded-as-space  `::=`  [b-generic](#b-generic)       |
|   -------- --------------------------------------------- ------- ------------------------- --- |
+------------------------------------------------------------------------------------------------+

If the following line is []{#id894325 .indexterm}[empty line](#empty%20line/), the []{#id894338 .indexterm}[generic line break](#generic%20line%20break/) is ignored.

+-------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------------------- ------- ------------------------------------------------------------------------ --- |
|   \[83\]     []{#b-l-folded-trimmed(n,s)}b-l-folded-trimmed(n,s)  `::=`  [b-ignored-generic](#b-ignored-generic) [l-empty(n,s)](#l-empty(n,s))+       |
|   -------- ----------------------------------------------------- ------- ------------------------------------------------------------------------ --- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------+

Thus, a folded non-[]{#id894380 .indexterm}[empty line](#empty%20line/) may end with one of three possible folded line break forms. The original form of such a folded line break is a []{#id894395 .indexterm}[presentation detail](#presentation%20detail/) and must not be used to convey []{#id894409 .indexterm}[node's content information](#content/information%20model).

+---------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------- -------------- |
|   \[84\]           []{#b-l-folded-any(n,s)}b-l-folded-any(n,s)     `::=`        [b-l-folded-specific(n,s)](#b-l-folded-specific(n,s))\                  |
|                                                                               \| [b-l-folded-as-space](#b-l-folded-as-space)\                           |
|                                                                               \| [b-l-folded-trimmed(n,s)](#b-l-folded-trimmed(n,s))                    |
|                                                                                                                                                         |
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id894461}

**Example 6.8. Line Folding**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| >-                                | %YAML 1.1                         |
|   specific⇓                       | --- !!str                         |
|   trimmed↓                        | "specific\L\                      |
| ··↓                               |  trimmed\n\n\n\                   |
| ·↓                                |  as space"                        |
| ↓                                 | ```                               |
|   as↓                             |                                   |
|   space                           | ``` synopsis                      |
| ```                               | Legend:                           |
|                                   |   b-l-folded-specific(n,s)        |
|                                   |   b-l-folded-trimmed(n,s)         |
|                                   |   b-l-folded-as-space             |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

The above rules are common to both the []{#id894567 .indexterm}[folded block style](#folded%20style/syntax) and the []{#id894583 .indexterm}[scalar flow styles](#flow%20scalar%20style/syntax). Folding does distinguish between the []{#id894601 .indexterm}[folded block style](#folded%20style/syntax) and the []{#id894617 .indexterm}[scalar flow styles](#flow%20scalar%20style/syntax) in the following way:

::: variablelist

[Block Folding]{.term}
:   In the []{#id894645 .indexterm}[folded block style](#folded%20style/syntax), folding does not apply to []{#id894661 .indexterm}[line breaks](#line%20break%20character/) or []{#id894674 .indexterm}[empty lines](#empty%20line/) that precede or follow a text line containing leading []{#id894688 .indexterm}[white space](#white%20space/). Note that such a line may consist of only such leading []{#id894702 .indexterm}[white space](#white%20space/); an []{#id894714 .indexterm}[empty](#empty%20line/)[]{#id894726 .indexterm}[block](#block%20style/syntax) line is confined to (optional) []{#id894743 .indexterm}[indentation](#indentation%20space/) spaces only. Further, the final []{#id894758 .indexterm}[line break](#line%20break%20character/) and []{#id894770 .indexterm}[empty lines](#empty%20line/) are subject to []{#id894783 .indexterm}[chomping](#chomping/), and are never folded. The combined effect of these rules is that each "[paragraph]{.quote}" is interpreted as a line, []{#id894801 .indexterm}[empty lines](#empty%20line/) are used to []{#id894813 .indexterm}[present](#present/) a line feed, the formatting of []{#id894826 .indexterm}["[more indented]{.quote}" lines](#more%20indented%20line/) is preserved, and final []{#id894843 .indexterm}[line breaks](#line%20break%20character/) may be included or excluded from the []{#id894859 .indexterm}[node's content information](#content/information%20model) as appropriate.

[Flow Folding]{.term}
:   Folding in []{#id894885 .indexterm}[flow styles](#flow%20style/syntax) provides more relaxed, less powerful semantics. []{#id894901 .indexterm}[Flow styles](#flow%20style/syntax) typically depend on explicit []{#id894917 .indexterm}[indicators](#indicator/) to convey structure, rather than []{#id894930 .indexterm}[indentation](#indentation%20space/). Hence, in []{#id894945 .indexterm}[flow styles](#flow%20style/syntax), spaces preceding or following the text in a line are a []{#id894962 .indexterm}[presentation detail](#presentation%20detail/) and must not be considered a part of the []{#id894976 .indexterm}[node's content information](#content/information%20model). Once all such spaces have been discarded, folding proceeds as described above. In contrast with the []{#id894995 .indexterm}[block folded style](#folded%20style/syntax), all []{#id895010 .indexterm}[line breaks](#line%20break%20character/) are folded, without exception, and a line consisting only of spaces is considered to be an []{#id895025 .indexterm}[empty line](#empty%20line/), regardless of the number of spaces. The combined effect of these processing rules is that each "[paragraph]{.quote}" is interpreted as a line, []{#id895043 .indexterm}[empty lines](#empty%20line/) are used to []{#id895056 .indexterm}[present](#present/) a line feed, and text can be freely []{#id895069 .indexterm}["[more indented]{.quote}"](#more%20indented%20line/) without affecting the []{#id895084 .indexterm}[node's content information](#content/information%20model).
:::
::::::::
::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id895107}Chapter 7. YAML Character Stream {#chapter-7.-yaml-character-stream .title}
:::
::::
:::::

A YAML character []{#id895115 .indexterm}[stream](#stream/syntax) may contain several YAML []{#id895130 .indexterm}[documents](#document/syntax), denoted by []{#id895146 .indexterm}[document boundary markers](#document%20boundary%20marker/). Each []{#id895159 .indexterm}[document](#document/syntax) []{#id895174 .indexterm}[presents](#present/) a single independent []{#id895187 .indexterm}[root node](#root%20node/) and may be preceded by a series of []{#id895200 .indexterm}[directives](#directive/syntax).

::::::::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id895217}7.1. Directives {#directives-1 .title style="clear: both"}

y[struct.c-named-tag-handle+2]

y[struct.c-ns-local-tag-prefix+2]

y[struct.c-primary-tag-handle+2]

y[struct.c-secondary-tag-handle+2]

y[struct.c-tag-handle+2]

y[struct.directive.ignore-unknown+2]

y[struct.named-tag-handle.must-be-declared+2]

y[struct.named-tag-handle.not-content+2]

y[struct.ns-global-tag-prefix+2]

y[struct.ns-tag-directive+2]

y[struct.ns-tag-prefix+2]

y[struct.ns-yaml-directive+2]

y[struct.tag-directive.at-most-once-per-handle+2]

y[struct.yaml-directive.must-accept-prior+2]

y[struct.yaml-directive.should-process-prior-as-current+2]

:::
::::
:::::

[]{#id895225 .indexterm}[]{#directive/syntax}*Directives* are instructions to the YAML []{#id895242 .indexterm}[processor](#processor/). Like []{#id895254 .indexterm}[comments](#comment/syntax), directives are []{#id895270 .indexterm}[presentation details](#presentation%20detail/) and are not reflected in the []{#id895284 .indexterm}[serialization tree](#serialization/) (and hence the []{#id895297 .indexterm}[representation graph](#representation/)). This specification defines two directives, []{#id895310 .indexterm}["[**`YAML`**]{.quote}"](#YAML%20directive/) and []{#id895329 .indexterm}["[**`TAG`**]{.quote}"](#TAG%20directive/), and []{#id895346 .indexterm}[]{#reserved directive/}*reserves* all other directives for future use. There is no way to define private directives. This is intentional.

+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------- --- |
|   \[85\]     []{#l-directive}l-directive  `::=`  [l-yaml-directive](#l-yaml-directive) \| [l-tag-directive](#l-tag-directive) \| [l-reserved-directive](#l-reserved-directive)       |
|   -------- ----------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------- --- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Each directive is specified on a separate non-[]{#id895395 .indexterm}[indented](#indentation%20space/) line starting with the []{#id895409 .indexterm}[]{#% directive/}*"[**`%`**]{.quote}" indicator*, followed by the directive name and a space-separated list of parameters. The semantics of these tokens depend on the specific directive. A YAML []{#id895431 .indexterm}[processor](#processor/) should ignore unknown directives with an appropriate warning.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------- -------------- ----------------------------------------------------------------------------------------------- -------------- |
|   \[86\]               []{#l-reserved-directive}l-reserved-directive     `::=`      ["[%]{.quote}"](#c-directive) [ns-directive-name](#ns-directive-name)\                                         |
|                                                                                     ( [s-ignored-space](#s-ignored-space)+ [ns-directive-parameter](#ns-directive-parameter) )\*\                  |
|                                                                                     [s-l-comments](#s-l-comments)                                                                                  |
|                                                                                                                                                                                                    |
|   \[87\]                     []{#ns-directive-name}ns-directive-name     `::=`      [ns-char](#ns-char)+                                                                                           |
|                                                                                                                                                                                                    |
|   \[88\]           []{#ns-directive-parameter}ns-directive-parameter     `::=`      [ns-char](#ns-char)+                                                                                           |
|   -------------- --------------------------------------------------- -------------- ----------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id895525}

**Example 7.1. Reserved Directives**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| %FOO  bar baz # Should be ignored | %YAML 1.1                         |
|                # with a warning.  | --- !!str                         |
| --- "foo"                         | "foo"                             |
| ```                               | ```                               |
|                                   |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   l-reserved-directive            |
|                                   |   ns-directive-name               |
|                                   |   ns-directive-parameter          |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

:::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id895631}7.1.1. "[**`YAML`**]{.quote}" Directive {#yaml-directive .title}
:::
::::
:::::

The []{#id895645 .indexterm}[]{#YAML directive/}*"[**`YAML`**]{.quote}" directive* specifies the version of YAML the []{#id895665 .indexterm}[document](#document/syntax) adheres to. This specification defines version "[**`1.1`**]{.quote}". A version 1.1 YAML []{#id895688 .indexterm}[processor](#processor/) should accept []{#id895701 .indexterm}[documents](#document/syntax) with an explicit "[**`%YAML 1.1`**]{.quote}" directive, as well as []{#id895723 .indexterm}[documents](#document/syntax) lacking a "[**`YAML`**]{.quote}" directive. []{#id895745 .indexterm}[Documents](#document/syntax) with a "[**`YAML`**]{.quote}" directive specifying a higher minor version (e.g. "[**`%YAML 1.2`**]{.quote}") should be processed with an appropriate warning. []{#id895776 .indexterm}[Documents](#document/syntax) with a "[**`YAML`**]{.quote}" directive specifying a higher major version (e.g. "[**`%YAML 2.0`**]{.quote}") should be rejected with an appropriate error message.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------- -------------- ---------------------------------------------------------------------------------------- -------------- |
|   \[89\]           []{#l-yaml-directive}l-yaml-directive     `::=`      ["[%]{.quote}"](#c-directive) "[Y]{.quote}" "[A]{.quote}" "[M]{.quote}" "[L]{.quote}"\                  |
|                                                                         [s-ignored-space](#s-ignored-space)+ [ns-yaml-version](#ns-yaml-version)\                               |
|                                                                         [s-l-comments](#s-l-comments)                                                                           |
|                                                                                                                                                                                 |
|   \[90\]             []{#ns-yaml-version}ns-yaml-version     `::=`      [ns-dec-digit](#ns-dec-digit)+ "[.]{.quote}" [ns-dec-digit](#ns-dec-digit)+                             |
|   -------------- --------------------------------------- -------------- ---------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id895885}

**Example 7.2. "[**`YAML`**]{.quote}" directive**

+-----------------------------------+------------------------------------+
| ``` programlisting                | ``` programlisting                 |
| %YAML 1.2 # Attempt parsing       | %YAML 1.1                          |
|            # with a warning       | ---                                |
| ---                               | !!str "foo"                        |
| "foo"                             | ```                                |
| ```                               |                                    |
|                                   | ``` synopsis                       |
|                                   | Legend:                            |
|                                   |   l-yaml-directive ns-yaml-version |
|                                   | ```                                |
+-----------------------------------+------------------------------------+
:::

It is an error to specify more than one "[**`YAML`**]{.quote}" directive for the same document, even if both occurrences give the same version number.

::: example
[]{#id895987}

**Example 7.3. Invalid Repeated YAML directive**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| %YAML 1.1                         | ERROR:                            |
| %YAML 1.1                         | The YAML directive must only be   |
| foo                               | given at most once per document.  |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
::::::::

:::::::::::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id896044}7.1.2. "[**`TAG`**]{.quote}" Directive {#tag-directive .title}
:::
::::
:::::

The []{#id896057 .indexterm}[]{#TAG directive/}*"[**`TAG`**]{.quote}" directive* establishes a []{#id896076 .indexterm}[shorthand](#tag%20shorthand/) notation for specifying []{#id896091 .indexterm}[node tags](#tag/syntax). Each "[**`TAG`**]{.quote}" directive associates a []{#id896111 .indexterm}[handle](#tag%20handle/) with a []{#id896126 .indexterm}[prefix](#tag%20prefix/), allowing for compact and readable []{#id896141 .indexterm}[tag](#tag/syntax) notation.

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
|   \[91\]           []{#l-tag-directive}l-tag-directive     `::=`      ["[%]{.quote}"](#c-directive) "[T]{.quote}" "[A]{.quote}" "[G]{.quote}"\                  |
|                                                                       [s-ignored-space](#s-ignored-space)+ [c-tag-handle](#c-tag-handle)\                       |
|                                                                       [s-ignored-space](#s-ignored-space)+ [ns-tag-prefix](#ns-tag-prefix)\                     |
|                                                                       [s-l-comments](#s-l-comments)                                                             |
|                                                                                                                                                                 |
|   -------------- ------------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id896213}

**Example 7.4. "[**`TAG`**]{.quote}" directive**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| %TAG !yaml! tag:yaml.org,2002:↓   | %YAML 1.1                         |
| ---                               | ---                               |
| !yaml!str "foo"                   | !!str "foo"                       |
| ```                               | ```                               |
|                                   |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   l-tag-directive                 |
|                                   |   c-tag-handle ns-tag-prefix      |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

It is an error to specify more than one "[**`TAG`**]{.quote}" directive for the same []{#id896326 .indexterm}[handle](#tag%20handle/) in the same document, even if both occurrences give the same []{#id896342 .indexterm}[prefix](#tag%20prefix/).

::: example
[]{#id896356}

**Example 7.5. Invalid Repeated TAG directive**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| %TAG ! !foo                       | ERROR:                            |
| %TAG ! !foo                       | The TAG directive must only       |
| bar                               | be given at most once per         |
| ```                               | handle in the same document.      |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

::::::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id896411}7.1.2.1. Tag Prefixes {#tag-prefixes .title}
:::
::::
:::::

There are two []{#id896420 .indexterm}[]{#tag prefix/}*tag prefix* variants:

+---------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------- ------- ---------------------------------------------------------------------------------------------- --- |
|   \[92\]     []{#ns-tag-prefix}ns-tag-prefix  `::=`  [ns-local-tag-prefix](#ns-local-tag-prefix) \| [ns-global-tag-prefix](#ns-global-tag-prefix)       |
|   -------- --------------------------------- ------- ---------------------------------------------------------------------------------------------- --- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Local Tags]{.term}
:   If the prefix begins with a []{#id896469 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/) character, []{#id896489 .indexterm}[shorthands](#tag%20shorthand/) using the []{#id896501 .indexterm}[handle](#tag%20handle/) are expanded to a []{#id896513 .indexterm}[local tag](#local%20tag/) beginning with []{#id896526 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/). Note that such a []{#id896545 .indexterm}[tag](#tag/syntax) is intentionally not a valid URI, since its semantics are specific to the []{#id896561 .indexterm}[application](#application/). In particular, two []{#id896573 .indexterm}[documents](#document/syntax) in the same []{#id896588 .indexterm}[stream](#stream/syntax) may assign different semantics to the same []{#id896603 .indexterm}[local tag](#local%20tag/).
:::

+------------------------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------------- ------- ------------------------------------------------------- --- |
|   \[93\]     []{#ns-local-tag-prefix}ns-local-tag-prefix  `::=`  ["[!]{.quote}"](#c-tag) [ns-uri-char](#ns-uri-char)\*       |
|   -------- --------------------------------------------- ------- ------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Global Tags]{.term}
:   If the prefix begins with a character other than []{#id896657 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/), it must to be a valid URI prefix, and should contain at least the scheme and the authority. []{#id896676 .indexterm}[Shorthands](#tag%20shorthand/) using the associated []{#id896691 .indexterm}[handle](#tag%20handle/) are expanded to globally unique URI tags, and their semantics is consistent across []{#id896704 .indexterm}[applications](#application/). In particular, two []{#id896717 .indexterm}[documents](#document/syntax) in different []{#id896732 .indexterm}[streams](#stream/syntax) must assign the same semantics to the same []{#id896747 .indexterm}[global tag](#global%20tag/).
:::

+------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------------- ------- ----------------------------------------------------------- --- |
|   \[94\]     []{#ns-global-tag-prefix}ns-global-tag-prefix  `::=`  [ns-tag-char](#ns-tag-char) [ns-uri-char](#ns-uri-char)\*       |
|   -------- ----------------------------------------------- ------- ----------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id896788}

**Example 7.6. Tag Prefixes**

+--------------------------------------------+-------------------------------------+
| ``` programlisting                         | ``` programlisting                  |
| %TAG !      !foo                           | %YAML 1.1                           |
| %TAG !yaml! tag:yaml.org,2002:             | ---                                 |
| ---                                        | !!seq [                             |
| - !bar "baz"                               |   !<!foobar> "baz",                 |
| - !yaml!str "string"                       |   !<tag:yaml.org,2002:str> "string" |
| ```                                        | ]                                   |
|                                            | ```                                 |
| ``` synopsis                               |                                     |
| Legend:                                    |                                     |
|   ns-local-tag-prefix ns-global-tag-prefix |                                     |
| ```                                        |                                     |
+--------------------------------------------+-------------------------------------+
:::
:::::::::

::::::::::: {.sect3 lang="en"}
::::: titlepage
:::: {}
::: {}
#### []{#id896876}7.1.2.2. Tag Handles {#tag-handles .title}
:::
::::
:::::

The []{#id896884 .indexterm}[]{#tag handle/}*tag handle* exactly matches the prefix of the affected []{#id896897 .indexterm}[shorthand](#tag%20shorthand/). There are three tag handle variants:

+-----------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------- -------------- -------------------------------------------------------- -------------- |
|   \[95\]           []{#c-tag-handle}c-tag-handle     `::=`        [c-primary-tag-handle](#c-primary-tag-handle)\                        |
|                                                                 \| [ns-secondary-tag-handle](#c-secondary-tag-handle)\                  |
|                                                                 \| [c-named-tag-handle](#c-named-tag-handle)                            |
|                                                                                                                                         |
|   -------------- ------------------------------- -------------- -------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Primary Handle]{.term}
:   The []{#id896955 .indexterm}[]{#primary tag handle/}*primary tag handle* is a single []{#id896971 .indexterm}["[**`!`**]{.quote}"](#!%20tag%20indicator/) character. This allows using the most compact possible notation for a single "[primary]{.quote}" name space. By default, the prefix associated with this handle is []{#id896997 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/). Thus, by default, []{#id897014 .indexterm}[shorthands](#tag%20shorthand/) using this handle are interpreted as []{#id897026 .indexterm}[local tags](#local%20tag/). It is possible to override this behavior by providing an explicit "[**`TAG`**]{.quote}" directive associating a different prefix for this handle. This provides smooth migration from using []{#id897048 .indexterm}[local tags](#local%20tag/) to using []{#id897063 .indexterm}[global tags](#global%20tag/) by a simple addition of a single "[**`TAG`**]{.quote}" directive.
:::

+--------------------------------------------------------------------------------------------------+
|   -------- ----------------------------------------------- ------- ------------------------- --- |
|   \[96\]     []{#c-primary-tag-handle}c-primary-tag-handle  `::=`  ["[!]{.quote}"](#c-tag)       |
|   -------- ----------------------------------------------- ------- ------------------------- --- |
+--------------------------------------------------------------------------------------------------+

::: example
[]{#id897109}

**Example 7.7. Migrating from Local to Global Tags**

+-----------------------------------+----------------------------------------+
| ``` programlisting                | ``` programlisting                     |
| # Private application:            | %YAML 1.1                              |
| !foo "bar"                        | ---                                    |
| ```                               | !<!foo> "bar"                          |
|                                   | ```                                    |
| ``` programlisting                |                                        |
| # Migrated to global:             | ``` programlisting                     |
| %TAG ! tag:ben-kiki.org,2000:app/ | %YAML 1.1                              |
| ---                               | ---                                    |
| !foo "bar"                        | !<tag:ben-kiki.org,2000:app/foo> "bar" |
| ```                               | ```                                    |
+-----------------------------------+----------------------------------------+
:::

::: variablelist

[Secondary Handle]{.term}
:   The []{#id897186 .indexterm}[]{#secondary tag handle/}*secondary tag handle* is written as []{#id897202 .indexterm}["[**`!!`**]{.quote}"](#!%20tag%20indicator/). This allows using a compact notation for a single "[secondary]{.quote}" name space. By default, the prefix associated with this handle is "[**`tag:yaml.org,2002:`**]{.quote}" used by the [YAML tag repository](/type/index.html){target="_top"} providing recommended []{#id897240 .indexterm}[tags](#tag/information%20model) for increasing the portability of YAML []{#id897257 .indexterm}[documents](#document/syntax) between different []{#id897272 .indexterm}[applications](#application/). It is possible to override this behavior by providing an explicit "[**`TAG`**]{.quote}" directive associating a different prefix for this handle.
:::

+-------------------------------------------------------------------------------------------------------------------------------+
|   -------- ---------------------------------------------------- ------- ------------------------------------------------- --- |
|   \[97\]     []{#c-secondary-tag-handle}ns-secondary-tag-handle  `::=`  ["[!]{.quote}"](#c-tag) ["[!]{.quote}"](#c-tag)       |
|   -------- ---------------------------------------------------- ------- ------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Named Handles]{.term}
:   A []{#id897336 .indexterm}[]{#named tag handle/}*named tag handle* surrounds the non-empty name with []{#id897352 .indexterm}[]{#! named handle/}*"[**`!`**]{.quote}"* characters. A handle name must not be used in a []{#id897371 .indexterm}[shorthand](#tag%20shorthand/) unless an explicit "[**`TAG`**]{.quote}" directive has associated some prefix with it. The name of the handle is a []{#id897392 .indexterm}[presentation detail](#presentation%20detail/) and is not part of the []{#id897406 .indexterm}[node's content information](#content/information%20model). In particular, the YAML []{#id897424 .indexterm}[processor](#processor/) need not preserve the handle name once []{#id897437 .indexterm}[parsing](#parse/) is completed.
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------- ------------------------------------------- ------- -------------------------------------------------------------------------------- --- |
|   \[98\]     []{#c-named-tag-handle}c-named-tag-handle  `::=`  ["[!]{.quote}"](#c-tag) [ns-word-char](#ns-word-char)+ ["[!]{.quote}"](#c-tag)       |
|   -------- ------------------------------------------- ------- -------------------------------------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id897487}

**Example 7.8. Tag Handles**

+----------------------------------------+---------------------------------------+
| ``` programlisting                     | ``` programlisting                    |
| # Explicitly specify default settings: | %YAML 1.1                             |
| %TAG !     !                           | ---                                   |
| %TAG !!    tag:yaml.org,2002:          | !!seq [                               |
| # Named handles have no default:       |   !<!foo> "bar",                      |
| %TAG !o! tag:ben-kiki.org,2000:        |   !<tag:yaml.org,2002:str> "string"   |
| ---                                    |   !<tag:ben-kiki.org,2000:type> "baz" |
| - !foo "bar"                           | ]                                     |
| - !!str "string"                       | ```                                   |
| - !o!type "baz"                        |                                       |
| ```                                    | ``` synopsis                          |
|                                        | Legend:                               |
|                                        |   c-primary-tag-handle                |
|                                        |   c-secondary-tag-handle              |
|                                        |   c-named-tag-handle                  |
|                                        | ```                                   |
+----------------------------------------+---------------------------------------+
:::
:::::::::::
::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id897596}7.2. Document Boundary Markers {#document-boundary-markers .title style="clear: both"}
:::
::::
:::::

YAML []{#id897604 .indexterm}[streams](#stream/syntax) use []{#id897619 .indexterm}[]{#document boundary marker/}*document boundary markers* to allow more than one []{#id897634 .indexterm}[document](#document/syntax) to be contained in the same []{#id897650 .indexterm}[stream](#stream/syntax). Such markers are a []{#id897665 .indexterm}[presentation detail](#presentation%20detail/) and are used exclusively to convey structure. A line beginning with "[**`---`**]{.quote}" may be used to explicitly denote the beginning of a new YAML []{#id897688 .indexterm}[document](#document/syntax).

+------------------------------------------------------------------------------------------------------------+
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
|   \[99\]     []{#c-document-start}c-document-start  `::=`  "[-]{.quote}" "[-]{.quote}" "[-]{.quote}"       |
|   -------- --------------------------------------- ------- ------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------+

When YAML is used as the format of a communication channel, it is useful to be able to indicate the end of a []{#id897733 .indexterm}[document](#document/syntax) without closing the []{#id897749 .indexterm}[stream](#stream/syntax), independent of starting the next []{#id897764 .indexterm}[document](#document/syntax). Lacking such a marker, the YAML []{#id897780 .indexterm}[processor](#processor/) reading the []{#id897792 .indexterm}[stream](#stream/syntax) would be forced to wait for the header of the next []{#id897808 .indexterm}[document](#document/syntax) (that may be long time in coming) in order to detect the end of the previous one. To support this scenario, a YAML []{#id897825 .indexterm}[document](#document/syntax) may be terminated by an explicit end line denoted by "[**`...`**]{.quote}", followed by optional []{#id897848 .indexterm}[comments](#comment/syntax). To ease the task of concatenating YAML []{#id897864 .indexterm}[streams](#stream/syntax), the end marker may be repeated.

+------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------- ------- ---------------------------------------------------------------------- --- |
|   \[100\]           []{#c-document-end}c-document-end  `::=`  "[.]{.quote}" "[.]{.quote}" "[.]{.quote}"                                  |
|   \[101\]     []{#l-document-suffix}l-document-suffix  `::=`  ( [c-document-end](#c-document-end) [s-l-comments](#s-l-comments) )+       |
|   --------- ----------------------------------------- ------- ---------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id897927}

**Example 7.9. Document Boundary Markers**

+-----------------------------------+--------------------------------------+
| ``` programlisting                | ``` programlisting                   |
| ---↓                              | %YAML 1.1                            |
| foo                               | ---                                  |
| ...                               | !!str "foo"                          |
| # Repeated end marker.            | %YAML 1.1                            |
| ...↓                              | ---                                  |
| ---↓                              | !!str "bar"                          |
| bar                               | %YAML 1.1                            |
| # No end marker.                  | ---                                  |
| ---↓                              | !!str "baz"                          |
| baz                               | ```                                  |
| ...↓                              |                                      |
| ```                               | ``` synopsis                         |
|                                   | Legend:                              |
|                                   |   c-document-start l-document-suffix |
|                                   | ```                                  |
+-----------------------------------+--------------------------------------+
:::
:::::::

::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id898031}7.3. Documents {#documents .title style="clear: both"}
:::
::::
:::::

A YAML []{#id898040 .indexterm}[]{#document/syntax}*document* is a single native data structure []{#id898056 .indexterm}[presented](#present/) as a single []{#id898069 .indexterm}[root](#root%20node/) []{#id898081 .indexterm}[node](#node/syntax). []{#id898096 .indexterm}[Presentation details](#presentation%20detail/) such as []{#id898111 .indexterm}[directives](#directive/syntax), []{#id898125 .indexterm}[comments](#comment/syntax), []{#id898139 .indexterm}[indentation](#indentation%20space/) and []{#id898153 .indexterm}[styles](#style/) are not considered part of the []{#id898165 .indexterm}[content information](#content/information%20model) of the document.

::: variablelist

[Explicit Documents]{.term}
:   An []{#id898193 .indexterm}[]{#explicit document/}*explicit document* begins with a []{#id898209 .indexterm}[document start marker](#document%20boundary%20marker/) followed by the []{#id898225 .indexterm}[presentation](#presentation/) of the []{#id898236 .indexterm}[root node](#root%20node/). The []{#id898248 .indexterm}[node](#node/syntax) may begin in the same line as the []{#id898264 .indexterm}[document start marker](#document%20boundary%20marker/). If the explicit document's []{#id898270 .indexterm}[node](#node/syntax) is []{#id898294 .indexterm}[completely empty](#completely%20empty%20node/), it is assumed to be an empty []{#id898312 .indexterm}[plain scalar](#plain%20style/syntax) with no specified []{#id898326 .indexterm}[properties](#node%20property/). Optional []{#id898341 .indexterm}[document end marker(s)](#document%20boundary%20marker/) may follow the document.
:::

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
|   \[102\]          []{#l-explicit-document}l-explicit-document     `::=`      [c-document-start](#c-document-start)\                                                                           |
|                                                                               ( [s-l+block-node(-1,block-in)](#s-l+block-node(n,c)) \| [s-l-empty-block](#s-l-empty-block) )\                  |
|                                                                               [l-document-suffix](#l-document-suffix)?                                                                         |
|                                                                                                                                                                                                |
|   -------------- --------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Implicit Documents]{.term}
:   An []{#id898408 .indexterm}[]{#implicit document/}*implicit document* does not begin with a []{#id898423 .indexterm}[document start marker](#document%20boundary%20marker/). In this case, the []{#id898437 .indexterm}[root node](#root%20node/) must not be []{#id898452 .indexterm}[presented](#present/) as a []{#id898463 .indexterm}[completely empty node](#completely%20empty%20node/). Again, optional []{#id898476 .indexterm}[document end marker(s)](#document%20boundary%20marker/) may follow the document.
:::

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------------------------- -------------- |
|   \[103\]          []{#l-implicit-document}l-implicit-document     `::=`      [s-ignored-space](#s-ignored-space)\* [ns-l+block-node(-1,block-in)](#ns-l+block-node(n,c))\                  |
|                                                                               [l-document-suffix](#l-document-suffix)?                                                                      |
|                                                                                                                                                                                             |
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

In general, the document's []{#id898482 .indexterm}[node](#node/syntax) is []{#id898545 .indexterm}[indented](#indentation%20space/) as if it has a parent []{#id898561 .indexterm}[indented](#indentation%20space/) at -1 spaces. Since a []{#id898573 .indexterm}[node](#node/syntax) must be more []{#id898588 .indexterm}[indented](#indentation%20space/) that its parent []{#id898601 .indexterm}[node](#node/syntax), this allows the document's []{#id898618 .indexterm}[node](#node/syntax) to be []{#id898632 .indexterm}[indented](#indentation%20space/) at zero or more spaces. Note that []{#id898648 .indexterm}[flow scalar](#flow%20scalar%20style/syntax) continuation lines must be []{#id898663 .indexterm}[indented](#indentation%20space/) by at least one space, even if their first line is not []{#id898679 .indexterm}[indented](#indentation%20space/).

::: example
[]{#id898692}

**Example 7.10. Documents**

+-------------------------------------------+-----------------------------------+
| ``` programlisting                        | ``` programlisting                |
| "Root flow                                | %YAML 1.1                         |
|  scalar"                                  | ---                               |
| --- !!str >                               | !!str "Root flow scalar"          |
|  Root block                               | %YAML 1.1                         |
|  scalar                                   | ---                               |
| ---                                       | !!str "Root block scalar"         |
| # Root collection:                        | %YAML 1.1                         |
| foo : bar                                 | ---                               |
| ... # Is optional.                        | !!map {                           |
| ---                                       |   ? !!str "foo"                   |
| # Explicit document may be empty.         |   : !!str "bar"                   |
| ```                                       | }                                 |
|                                           | ---                               |
| ``` synopsis                              | !!str ""                          |
| Legend:                                   | ```                               |
|   l-implicit-document l-explicit-document |                                   |
| ```                                       |                                   |
+-------------------------------------------+-----------------------------------+
:::
:::::::::

::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id898785}7.4. Complete Stream {#complete-stream .title style="clear: both"}
:::
::::
:::::

A sequence of bytes is a YAML character []{#id898794 .indexterm}[]{#stream/syntax}*stream* if, taken as a whole, it complies with the [**`l-yaml-stream`**](#l-yaml-stream) production. The stream begins with a prefix containing an optional []{#id898823 .indexterm}[byte order mark](#byte%20order%20mark/) denoting its []{#id898838 .indexterm}[character encoding](#character%20encoding/), followed by optional []{#id898852 .indexterm}[comments](#comment/syntax). Note that the stream may contain no []{#id898867 .indexterm}[documents](#document/syntax), even if it contains a non-empty prefix. In particular, a stream containing no characters is valid and contains no []{#id898884 .indexterm}[documents](#document/syntax).

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------- -------------- ---------------------------------------------------------------------------------- -------------- |
|   \[104\]          []{#l-yaml-stream}l-yaml-stream     `::=`      [c-byte-order-mark](#c-byte-order-mark)? [l-comment](#l-comment)\*\                               |
|                                                                   ( [l-first-document](#l-first-document) [l-next-document](#l-next-document)\* )?                  |
|                                                                                                                                                                     |
|   -------------- --------------------------------- -------------- ---------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id898937}

**Example 7.11. Empty Stream**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ⇔# A stream may contain           | # This stream contains no         |
| # no documents.                   | # documents, only comments.       |
| ```                               | ```                               |
|                                   |                                   |
| ``` synopsis                      |                                   |
| Legend:                           |                                   |
|   l-yaml-stream                   |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

The first []{#id899004 .indexterm}[document](#document/syntax) may be []{#id899019 .indexterm}[implicit](#implicit%20document/) (omit the []{#id899034 .indexterm}[document start marker](#document%20boundary%20marker/)). In such a case it must not specify any []{#id899048 .indexterm}[directives](#directive/syntax) and will be []{#id899064 .indexterm}[parsed](#parse/) using the default settings. If the []{#id899076 .indexterm}[document](#document/syntax) is []{#id899091 .indexterm}[explicit](#explicit%20document/) (begins with an []{#id899106 .indexterm}[document start marker](#document%20boundary%20marker/)), it may specify []{#id899120 .indexterm}[directives](#directive/syntax) to control its []{#id899136 .indexterm}[parsing](#parse/).

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------- -------------- ------------------------------------------------------------------------------------ -------------- |
|   \[105\]          []{#l-first-document}l-first-document     `::=`      ( [l-implicit-document](#l-implicit-document)\                                                      |
|                                                                         \| ( [l-directive](#l-directive)\* [l-explicit-document](#l-explicit-document) ) )                  |
|                                                                                                                                                                             |
|   -------------- --------------------------------------- -------------- ------------------------------------------------------------------------------------ -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id899182}

**Example 7.12. First Document**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| # Implicit document. Root         | %YAML 1.1                         |
| # collection (mapping) node.      | ---                               |
| foo : bar                         | !!map {                           |
| ```                               |   ? !!str "foo"                   |
|                                   |   : !!str "bar"                   |
| ``` programlisting                | }                                 |
| # Explicit document. Root         | ```                               |
| # scalar (literal) node.          |                                   |
| --- |                             | ``` programlisting                |
|  Text content                     | %YAML 1.1                         |
| ```                               | ---                               |
|                                   | !!str "Text content\n"            |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   l-first-document                |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

To ease the task of concatenating character streams, following []{#id899282 .indexterm}[documents](#document/syntax) may begin with a []{#id899297 .indexterm}[byte order mark](#byte%20order%20mark/) and []{#id899310 .indexterm}[comments](#comment/syntax), though the same []{#id899325 .indexterm}[character encoding](#character%20encoding/) must be used through the stream. Each following []{#id899340 .indexterm}[document](#document/syntax) must be []{#id899355 .indexterm}[explicit](#explicit%20document/) (begin with a []{#id899369 .indexterm}[document start marker](#document%20boundary%20marker/)). If the []{#id899382 .indexterm}[document](#document/syntax) specifies no []{#id899397 .indexterm}[directives](#directive/syntax), it is []{#id899413 .indexterm}[parsed](#parse/) using the same settings as the previous []{#id899426 .indexterm}[document](#document/syntax). If the []{#id899441 .indexterm}[document](#document/syntax) does specify any []{#id899456 .indexterm}[directives](#directive/syntax), all []{#id899472 .indexterm}[directives](#directive/syntax) of previous []{#id899487 .indexterm}[documents](#document/syntax), if any, are ignored.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------- -------------- --------------------------------------------------------------------------- -------------- |
|   \[106\]          []{#l-next-document}l-next-document     `::=`      [c-byte-order-mark](#c-byte-order-mark)? [l-comment](#l-comment)\*\                        |
|                                                                       [l-directive](#l-directive)\* [l-explicit-document](#l-explicit-document)                  |
|                                                                                                                                                                  |
|   -------------- ------------------------------------- -------------- --------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id899540}

**Example 7.13. Next Documents**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ! "First document"                | %YAML 1.1                         |
| ---                               | ---                               |
| !foo "No directives"              | !!str "First document"            |
| %TAG ! !foo                       | ---                               |
| ---                               | !<!foo> "No directives"           |
| !bar "With directives"            | ---                               |
| %YAML 1.1                         | !<!foobar> "With directives"      |
| ---                               | ---                               |
| !baz "Reset settings"             | !<!baz> "Reset settings"          |
| ```                               | ```                               |
|                                   |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   l-next-document                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id899622}Chapter 8. Nodes {#chapter-8.-nodes .title}
:::
::::
:::::

Each []{#id899630 .indexterm}[]{#node/syntax}*presentation node* may have two optional []{#id899646 .indexterm}[]{#node property/}*properties*, []{#id899662 .indexterm}[anchor](#anchor/syntax) and []{#id899675 .indexterm}[tag](#tag/syntax), in addition to its []{#id899690 .indexterm}[content](#content/syntax). Node properties may be specified in any order before the []{#id899705 .indexterm}[node's content](#content/syntax), and either or both may be omitted from the character []{#id899722 .indexterm}[stream](#stream/syntax).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------- -------------- |
|   \[107\]          []{#c-ns-properties(n,c)}c-ns-properties(n,c)     `::=`        ( [c-ns-tag-property](#c-ns-tag-property)\                                                                   |
|                                                                                     ( [s-separate(n,c)](#s-separate(n,c)) [c-ns-anchor-property](#c-ns-anchor-property) )? )\                  |
|                                                                                 \| ( [c-ns-anchor-property](#c-ns-anchor-property)\                                                            |
|                                                                                     ( [s-separate(n,c)](#s-separate(n,c)) [c-ns-tag-property](#c-ns-tag-property) )? )                         |
|                                                                                                                                                                                                |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id899790}

**Example 8.1. Node Properties**

+------------------------------------------+-----------------------------------+
| ``` programlisting                       | ``` programlisting                |
| !!str                                    | %YAML 1.1                         |
|  &a1↓                                    | ---                               |
|   "foo" : !!str bar                      | !!map {                           |
| &a2 baz : *a1                            |   ? &A1 !!str "foo"               |
| ```                                      |   : !!str "bar",                  |
|                                          |   ? !!str &A2 "baz"               |
| ``` synopsis                             |   : *a1                           |
| Legend:                                  | }                                 |
|   c-ns-anchor-property c-ns-tag-property | ```                               |
|   c-ns-properties(n,c)                   |                                   |
| ```                                      |                                   |
+------------------------------------------+-----------------------------------+
:::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id899912}8.1. Node Anchors {#node-anchors .title style="clear: both"}
:::
::::
:::::

The []{#id899920 .indexterm}[]{#anchor/syntax}*anchor property* marks a []{#id899935 .indexterm}[node](#node/syntax) for future reference. An anchor is denoted by the []{#id899951 .indexterm}[]{#& anchor/}*"[**`&`**]{.quote}" indicator*. An []{#id899973 .indexterm}[alias node](#alias/syntax) can then be used to indicate additional inclusions of the anchored node by specifying its anchor. An anchored node need not be referenced by any []{#id899989 .indexterm}[alias node](#alias/syntax); in particular, it is valid for all []{#id900004 .indexterm}[nodes](#node/syntax) to be anchored.

+----------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------------- ------- -------------------------------------------------------------- --- |
|   \[108\]     []{#c-ns-anchor-property}c-ns-anchor-property  `::=`  ["[&]{.quote}"](#c-anchor) [ns-anchor-name](#ns-anchor-name)       |
|   --------- ----------------------------------------------- ------- -------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------+

Note that as a []{#id900049 .indexterm}[serialization detail](#serialization%20detail/), the anchor name is preserved in the []{#id900064 .indexterm}[serialization tree](#serialization/). However, it is not reflected in the []{#id900077 .indexterm}[representation](#representation/) graph and must not be used to convey []{#id900090 .indexterm}[content information](#content/information%20model). In particular, the YAML []{#id900109 .indexterm}[processor](#processor/) need not preserve the anchor name once the []{#id900121 .indexterm}[representation](#representation/) is []{#id900133 .indexterm}[composed](#compose/).

+------------------------------------------------------------------------------------+
|   --------- ----------------------------------- ------- ---------------------- --- |
|   \[109\]     []{#ns-anchor-name}ns-anchor-name  `::=`  [ns-char](#ns-char)+       |
|   --------- ----------------------------------- ------- ---------------------- --- |
+------------------------------------------------------------------------------------+

::: example
[]{#id900166}

**Example 8.2. Node Anchors**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| First occurrence: &anchor Value   | %YAML 1.1                         |
| Second occurrence: *anchor        | ---                               |
| ```                               | !!map {                           |
|                                   |   ? !!str "First occurrence"      |
| ``` synopsis                      |   : &A !!str "Value",             |
| Legend:                           |   ? !!str "Second occurrence"     |
|   c-ns-anchor-property            |   : *A                            |
|   ns-anchor-name                  | }                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::

:::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id900262}8.2. Node Tags {#node-tags .title style="clear: both"}

y[struct.c-non-specific-tag+2]

y[struct.c-ns-shorthand-tag+2]

y[struct.c-ns-tag-property+2]

y[struct.c-verbatim-tag+2]

y[struct.shorthand-tag.handle-must-have-prefix+2]

y[struct.shorthand-tag.handle-not-content+2]

y[struct.shorthand-tag.result-must-be-local-or-uri+2]

y[struct.shorthand-tag.suffix-escape+2]

y[struct.shorthand-tag.suffix-no-bang+2]

y[struct.verbatim-tag.deliver-as-is+2]

y[struct.verbatim-tag.must-be-local-or-uri+2]

:::
::::
:::::

The []{#id900269 .indexterm}[]{#tag/syntax}*tag property* identifies the type of the native data structure []{#id900285 .indexterm}[presented](#present/) by the []{#id900297 .indexterm}[node](#node/syntax). A tag is denoted by the []{#id900312 .indexterm}[]{#! tag indicator/}*"[**`!`**]{.quote}" indicator*. In contrast with []{#id900333 .indexterm}[anchors](#anchor/syntax), tags are an inherent part of the []{#id900348 .indexterm}[representation](#representation/) graph.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------- -------------- |
|   \[110\]          []{#c-ns-tag-property}c-ns-tag-property     `::=`        [c-verbatim-tag](#c-verbatim-tag) \| [c-ns-shorthand-tag](#c-ns-shorthand-tag)\                  |
|                                                                           \| [c-ns-non-specific-tag](#c-ns-non-specific-tag)                                                 |
|                                                                                                                                                                              |
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Verbatim Tags]{.term}
:   A tag may be written []{#id900405 .indexterm}[]{#verbatim tag/}*verbatim* by surrounding it with the []{#id900421 .indexterm}[]{#< … > verbatim tag/}*"[**`<`**]{.quote}" and "[**`>`**]{.quote}"* characters. In this case, the YAML []{#id900449 .indexterm}[processor](#processor/) must deliver the verbatim tag as-is to the []{#id900461 .indexterm}[application](#application/). In particular, verbatim tags are not subject to []{#id900474 .indexterm}[tag resolution](#tag%20resolution/). A verbatim tag must either begin with a []{#id900489 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/) (a []{#id900506 .indexterm}[local tag](#local%20tag/)) or be a valid URI (a []{#id900519 .indexterm}[global tag](#global%20tag/)).
:::

+--------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------- ------- ------------------------------------------------------------------------------------ --- |
|   \[111\]     []{#c-verbatim-tag}c-verbatim-tag  `::=`  ["[!]{.quote}"](#c-tag) "[\<]{.quote}" [ns-uri-char](#ns-uri-char)+ "[\>]{.quote}"       |
|   --------- ----------------------------------- ------- ------------------------------------------------------------------------------------ --- |
+--------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id900566}

**Example 8.3. Verbatim Tags**

+-----------------------------------+------------------------------------+
| ``` programlisting                | ``` programlisting                 |
| !<tag:yaml.org,2002:str> foo :    | %YAML 1.1                          |
|   !<!bar> baz                     | ---                                |
| ```                               | !!map {                            |
|                                   |   ? !<tag:yaml.org,2002:str> "foo" |
| ``` synopsis                      |   : !<!bar> "baz"                  |
| Legend:                           | }                                  |
|   c-verbatim-tag                  | ```                                |
| ```                               |                                    |
+-----------------------------------+------------------------------------+
:::

::: example
[]{#id900640}

**Example 8.4. Invalid Verbatim Tags**

+-----------------------------------+------------------------------------+
| ``` screen                        | ``` screen                         |
| - !<!> foo                        | ERROR:                             |
| - !<$:?> bar                      | - Verbatim tags aren't resolved,   |
| ```                               |   so ! is invalid.                 |
|                                   | - The $:? tag is neither a global  |
|                                   |   URI tag nor a local tag starting |
|                                   |   with “!”.                        |
|                                   | ```                                |
+-----------------------------------+------------------------------------+
:::

::: variablelist

[Tag Shorthands]{.term}
:   A []{#id900720 .indexterm}[]{#tag shorthand/}*tag shorthand* consists of a valid []{#id900734 .indexterm}[tag handle](#tag%20handle/) followed by a non-empty suffix. The []{#id900748 .indexterm}[tag handle](#tag%20handle/) must be associated with a []{#id900761 .indexterm}[prefix](#tag%20prefix/), either by default or by using a []{#id900774 .indexterm}["[**`TAG`**]{.quote}" directive](#TAG%20directive/). The resulting []{#id900794 .indexterm}[parsed](#parse/) tag is the concatenation of the prefix and the suffix, and must either begin with []{#id900807 .indexterm}["[**`!`**]{.quote}"](#!%20local%20tag/) (a []{#id900823 .indexterm}[local tag](#local%20tag/)) or be a valid URI (a []{#id900836 .indexterm}[global tag](#global%20tag/)). When the []{#id900849 .indexterm}[primary tag handle](#primary%20tag%20handle/) is used, the suffix must not contain any []{#id900866 .indexterm}["[**`!`**]{.quote}"](#!%20named%20handle/) character, as this would cause the tag shorthand to be interpreted as having a []{#id900884 .indexterm}[named tag handle](#named%20tag%20handle/). If the []{#id900897 .indexterm}["[**`!`**]{.quote}"](#!%20named%20handle/) character exists in the suffix of a tag using the []{#id900917 .indexterm}[primary tag handle](#primary%20tag%20handle/), it must be []{#id900931 .indexterm}[escaped](#escaping%20in%20URI/) as []{#id900943 .indexterm}["[**`%21`**]{.quote}"](#%%20escaping%20in%20URI/), and the parser should expand this particular escape sequence before passing the tag to the application. This behavior is consistent with the URI character quoting rules (specifically, section 1.3 of [RFC2396](http://www.ietf.org/rfc/rfc2396.txt){target="_top"}), and ensures the choice of []{#id900973 .indexterm}[tag handle](#tag%20handle/) remains a []{#id900985 .indexterm}[presentation detail](#presentation/) and is not reflected in the []{#id900998 .indexterm}[serialization tree](#serialization/) (and hence the []{#id901012 .indexterm}[representation](#representation/) graph). In particular, the []{#id901025 .indexterm}[tag handle](#tag%20handle/) may be discarded once []{#id901038 .indexterm}[parsing](#parse/) is completed.
:::

+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
|   \[112\]          []{#c-ns-shorthand-tag}c-ns-shorthand-tag     `::=`        ( [c-primary-tag-handle](#c-primary-tag-handle) [ns-tag-char](#ns-tag-char)+ )\                        |
|                                                                             \| ( [ns-secondary-tag-handle](#c-secondary-tag-handle) [ns-uri-char](#ns-uri-char)+ )\                  |
|                                                                             \| ( [c-named-tag-handle](#c-named-tag-handle) [ns-uri-char](#ns-uri-char)+ )                            |
|                                                                                                                                                                                      |
|   -------------- ------------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id901102}

**Example 8.5. Tag Shorthands**

+-----------------------------------+----------------------------------------+
| ``` programlisting                | ``` programlisting                     |
| %TAG !o! tag:ben-kiki.org,2000:   | %YAML 1.1                              |
| ---                               | ---                                    |
| - !local foo                      | !!seq [                                |
| - !!str bar                       |   !<!local> "foo",                     |
| - !o!type baz                     |   !<tag:yaml.org,2002:str> "bar",      |
| ```                               |   !<tag:ben-kiki.org,2000:type> "baz", |
|                                   | ]                                      |
| ``` synopsis                      | ```                                    |
| Legend:                           |                                        |
|   c-ns-shorthand-tag              |                                        |
| ```                               |                                        |
+-----------------------------------+----------------------------------------+
:::

::: example
[]{#id901185}

**Example 8.6. Invalid Shorthand Tags**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| %TAG !o! tag:ben-kiki.org,2000:   | ERROR:                            |
| ---                               | - The !$a! looks like a handle.   |
| - !$a!b foo                       | - The !o! handle has no suffix.   |
| - !o! bar                         | - The !h! handle wasn't declared. |
| - !h!type baz                     | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: variablelist

[Non-Specific Tags]{.term}
:   If a []{#id901274 .indexterm}[node](#node/syntax) has no tag property, it is assigned a []{#id901288 .indexterm}[non-specific tag](#non-specific%20tag/): []{#id901301 .indexterm}["[**`?`**]{.quote}"](#?%20non-specific%20tag/) for []{#id901321 .indexterm}[plain scalars](#plain%20style/syntax) and []{#id901336 .indexterm}["[**`!`**]{.quote}"](#!%20non-specific%20tag/) for all other []{#id901357 .indexterm}[nodes](#node/syntax). []{#id901371 .indexterm}[Non-specific tags](#non-specific%20tag/) must be []{#id901384 .indexterm}[resolved](#tag%20resolution/) to a []{#id901397 .indexterm}[specific tag](#specific%20tag/) for a []{#id901411 .indexterm}[complete representation](#complete%20representation/) graph to be []{#id901425 .indexterm}[composed](#compose/). It is also possible for the tag property to explicitly specify the []{#id901439 .indexterm}[node](#node/syntax) has the []{#id901454 .indexterm}["[**`!`**]{.quote}" non-specific tag](#!%20non-specific%20tag/). This is only useful for []{#id901476 .indexterm}[plain scalars](#plain%20style/syntax), causing them to be []{#id901490 .indexterm}[resolved](#tag%20resolution/) as if they were non-[]{#id901502 .indexterm}[plain](#plain%20style/syntax) (hence, by the common []{#id901518 .indexterm}[tag resolution](#tag%20resolution/) convention, as "[**`tag:yaml.org,2002:str`**]{.quote}"). There is no way to explicitly set the tag to the []{#id901539 .indexterm}["[**`?`**]{.quote}" non-specific](#?%20non-specific%20tag/) tag. This is intentional.
:::

+-----------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------------- ------- ------------------------- --- |
|   \[113\]     []{#c-ns-non-specific-tag}c-ns-non-specific-tag  `::=`  ["[!]{.quote}"](#c-tag)       |
|   --------- ------------------------------------------------- ------- ------------------------- --- |
+-----------------------------------------------------------------------------------------------------+

::: example
[]{#id901586}

**Example 8.7. Non-Specific Tags**

+-------------------------------------+-----------------------------------+
| ``` programlisting                  | ``` programlisting                |
| # Assuming conventional resolution: | %YAML 1.1                         |
| - "12"                              | ---                               |
| - 12                                | !!seq [                           |
| - ! 12                              |   !<tag:yaml.org,2002:str> "12",  |
| ```                                 |   !<tag:yaml.org,2002:int> "12",  |
|                                     |   !<tag:yaml.org,2002:str> "12",  |
| ``` synopsis                        | ]                                 |
| Legend:                             | ```                               |
|   c-ns-non-specific-tag             |                                   |
| ```                                 |                                   |
+-------------------------------------+-----------------------------------+
:::
::::::::::::::

::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id901659}8.3. Node Content {#node-content .title style="clear: both"}
:::
::::
:::::

[]{#id901666 .indexterm}[]{#content/syntax}*Node content* may be []{#id901680 .indexterm}[presented](#present/) in either a []{#id901693 .indexterm}[flow style](#flow%20style/syntax) or a []{#id901709 .indexterm}[block style](#block%20style/syntax). []{#id901724 .indexterm}[Block content](#block%20style/syntax) always extends to the end of a line and uses []{#id901741 .indexterm}[indentation](#indentation%20space/) to denote structure, while []{#id901756 .indexterm}[flow content](#flow%20style/syntax) starts and ends at some non-space character within a line and uses []{#id901772 .indexterm}[indicators](#indicator/) to denote structure. Each collection []{#id901784 .indexterm}[kind](#kind/) can be presented in a single []{#id901797 .indexterm}[flow collection style](#flow%20collection%20style/syntax) or a single []{#id901813 .indexterm}[block collection style](#block%20collection%20style/syntax). However, each collection []{#id901830 .indexterm}[kind](#kind/) also provides compact []{#id901843 .indexterm}[in-line](#in-line%20style/syntax) forms for common cases. []{#id901859 .indexterm}[Scalar content](#scalar/syntax) may be []{#id901874 .indexterm}[presented](#present/) in the []{#id901886 .indexterm}[plain style](#plain%20style/syntax) or one of the two []{#id901905 .indexterm}[quoted styles](#quoted%20style/syntax) (the []{#id901919 .indexterm}[single-quoted style](#single-quoted%20style/syntax) and the []{#id901934 .indexterm}[double-quoted style](#double-quoted%20style/syntax)). Regardless of style, []{#id901950 .indexterm}[scalar content](#scalar/syntax) must always be []{#id901966 .indexterm}[indented](#indentation%20space/) by at least one space. In contrast, []{#id901980 .indexterm}[collection content](#collection/syntax) need not be []{#id901995 .indexterm}[indented](#indentation%20space/) (note that the []{#id902009 .indexterm}[indentation](#indentation%20space/) of the first []{#id902022 .indexterm}[flow scalar](#flow%20scalar%20style/syntax) line is determined by the []{#id902039 .indexterm}[block collection](#block%20collection%20style/syntax) it is nested in, if any).

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------------ -------------- |
|   \[114\]                      []{#ns-flow-scalar(n,c)}ns-flow-scalar(n,c)     `::=`        [c-plain(max(n,1),c)](#ns-plain(n,c))\                                                                              |
|                                                                                           \| [c-single-quoted(max(n,1),c)](#c-single-quoted(n,c))\                                                              |
|                                                                                           \| [c-double-quoted(max(n,1),c)](#c-double-quoted(n,c))                                                               |
|                                                                                                                                                                                                                 |
|   \[115\]                []{#c-flow-collection(n,c)}c-flow-collection(n,c)     `::=`      [c-flow-sequence(n,c)](#c-flow-sequence(n,c)) \| [c-flow-mapping(n,c)](#c-flow-mapping(n,c))                          |
|                                                                                                                                                                                                                 |
|   \[116\]                    []{#ns-flow-content(n,c)}ns-flow-content(n,c)     `::=`      [ns-flow-scalar(n,c)](#ns-flow-scalar(n,c)) \| [c-flow-collection(n,c)](#c-flow-collection(n,c))                      |
|                                                                                                                                                                                                                 |
|   \[117\]                      []{#c-l+block-scalar(n)}c-l+block-scalar(n)     `::=`      [c-l+folded(max(n,0))](#c-l+folded(n)) \| [c-l+literal(max(n,0))](#c-l+literal(n))                                    |
|                                                                                                                                                                                                                 |
|   \[118\]          []{#c-l-block-collection(n,c)}c-l-block-collection(n,c)     `::=`      [c-l-block-sequence(n,c)](#c-l-block-sequence(n,c)) \| [c-l-block-mapping(n)](#c-l-block-mapping(n))                  |
|                                                                                                                                                                                                                 |
|   \[119\]                []{#c-l+block-content(n,c)}c-l+block-content(n,c)     `::=`        [c-l+block-scalar(n)](#c-l+block-scalar(n))\                                                                        |
|                                                                                           \| [c-l-block-collection(\>n,c)](#c-l-block-collection(n,c))                                                          |
|   -------------- --------------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------------ -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id902212}

**Example 8.8. Mandatory Scalar Indentation**

+---------------------------------------+-----------------------------------+
| ``` programlisting                    | ``` programlisting                |
| ---                                   | %YAML 1.1                         |
| foo:                                  | ---                               |
| ·"bar                                 | !!map {                           |
| ·baz"                                 |   ? !!str "foo"                   |
| ---                                   |   : !!str "bar baz"               |
| "foo                                  | }                                 |
| ·bar"                                 | %YAML 1.1                         |
| ---                                   | ---                               |
| foo                                   | !!str "foo bar"                   |
| ·bar                                  | %YAML 1.1                         |
| --- |                                 | ---                               |
| ·foo                                  | !!str "foo bar"                   |
| ...                                   | %YAML 1.1                         |
| ```                                   | ---                               |
|                                       | !!str "foo bar\n"                 |
| ``` synopsis                          | ```                               |
| Legend:                               |                                   |
|   Normal "more-indented" indentation  |                                   |
|   Mandatory for "non-indented" scalar |                                   |
| ```                                   |                                   |
+---------------------------------------+-----------------------------------+
:::

::: example
[]{#id902304}

**Example 8.9. Flow Content**

+-----------------------------------+---------------------------------------+
| ``` programlisting                | ``` programlisting                    |
| ---                               | %YAML 1.1                             |
| scalars:                          | --- !!map {                           |
|   plain: !!str some text↓         |   ? !!str "scalars" : !!map {         |
|   quoted:                         |       ? !!str "plain"                 |
|     single: 'some text'↓          |       : !!str "some text",            |
|     double: "some text"↓          |       ? !!str "quoted"                |
| collections:                      |       : !!map {                       |
|   sequence: !!seq [ !str entry,   |         ? !!str "single"              |
|     # Mapping entry:↓             |         : !!str "some text",          |
|       key: value ]↓               |         ? !!str "double"              |
|   mapping: { key: value }↓        |         : !!str "some text"           |
| ```                               |   } },                                |
|                                   |   ? !!str "collections": : !!map {    |
| ``` synopsis                      |     ? !!str "sequence" : !!seq [      |
| Legend:                           |       ? !!str "entry",                |
|   ns-flow-scalar                  |       : !!map {                       |
|   c-flow-collection               |         ? !!str "key" : !!str "value" |
|   not content                     |     } ],                              |
| ```                               |     ? !!str "mapping": : !!map {      |
|                                   |       ? !!str "key" : !!str "value"   |
|                                   | } } }                                 |
|                                   | ```                                   |
+-----------------------------------+---------------------------------------+
:::

::: example
[]{#id902431}

**Example 8.10. Block Content**

+-----------------------------------+-----------------------------------------+
| ``` programlisting                | ``` programlisting                      |
| block styles:                     | %YAML 1.1                               |
|   scalars:                        | ---                                     |
|     literal: !!str |              | !!map {                                 |
|       #!/usr/bin/perl             |   ? !!str "block styles" : !!map {      |
|       print "Hello, world!\n";↓   |     ? !!str "scalars" : !!map {         |
|     folded: >                     |       ? !!str "literal"                 |
|       This sentence               |       : !!str "#!!/usr/bin/perl\n\      |
|       is false.↓                  |           print \"Hello,                |
|   collections: !!seq              |           world!!\\n\";\n",             |
|     sequence: !!seq # Entry:↓     |       ? !!str "folded"                  |
|       - entry # Plain             |       : !!str "This sentence            |
|       # Mapping entry:↓           |           is false.\n"                  |
|       - key: value↓               |     },                                  |
|     mapping: ↓                    |     ? !!str "collections" : !!map {     |
|       key: value↓                 |       ? !!str "sequence" : !!seq [      |
| ```                               |         !!str "entry",                  |
|                                   |         !!map {                         |
| ``` synopsis                      |           ? !!str "key" : !!str "value" |
| Legend:                           |         }                               |
|   c-l+block-scalar                |       ],                                |
|   c-l-block-collection            |       ? !!str "mapping" : !!map {       |
|   not content                     |         ? !!str "key" : !!str "value"   |
| ```                               | } } } }                                 |
|                                   | ```                                     |
+-----------------------------------+-----------------------------------------+
:::
:::::::::

::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id902561}8.4. Alias Nodes {#alias-nodes .title style="clear: both"}
:::
::::
:::::

Subsequent occurrences of a previously []{#id902569 .indexterm}[serialized](#serialize/) node are []{#id902582 .indexterm}[presented](#present/) as []{#id902594 .indexterm}[]{#alias/syntax}*alias nodes*, denoted by the []{#id902611 .indexterm}[]{#* alias/}*"[**`*`**]{.quote}" indicator*. The first occurrence of the []{#id902630 .indexterm}[node](#node/syntax) must be marked by an []{#id902646 .indexterm}[anchor](#anchor/syntax) to allow subsequent occurrences to be []{#id902661 .indexterm}[presented](#present/) as alias nodes. An alias node refers to the most recent preceding []{#id902675 .indexterm}[node](#node/syntax) having the same []{#id902690 .indexterm}[anchor](#anchor/syntax). It is an error to have an alias node use an []{#id902706 .indexterm}[anchor](#anchor/syntax) that does not previously occur in the []{#id902722 .indexterm}[document](#document/syntax). It is not an error to specify an []{#id902737 .indexterm}[anchor](#anchor/syntax) that is not used by any alias node. Note that an alias node must not specify any []{#id902754 .indexterm}[properties](#node%20property/) or []{#id902766 .indexterm}[content](#content/syntax), as these were already specified at the first occurrence of the []{#id902782 .indexterm}[node](#node/syntax).

+------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------- ------- -------------------------------------------------------------- --- |
|   \[120\]     []{#c-ns-alias-node}c-ns-alias-node  `::=`  ["[\*]{.quote}"](#c-alias) [ns-anchor-name](#ns-anchor-name)       |
|   --------- ------------------------------------- ------- -------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id902824}

**Example 8.11. Alias Nodes**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| First occurrence: &anchor Value   | %YAML 1.1                         |
| Second occurrence: *anchor        | ---                               |
| ```                               | !!map {                           |
|                                   |   ? !!str "First occurrence"      |
| ``` synopsis                      |   : &A !!str "Value",             |
| Legend:                           |   ? !!str "Second occurrence"     |
|   c-ns-alias-node                 |   : *A                            |
|   ns-anchor-name                  | }                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::

:::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id902919}8.5. Complete Nodes {#complete-nodes .title style="clear: both"}
:::
::::
:::::

:::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id902924}8.5.1. Flow Nodes {#flow-nodes .title}
:::
::::
:::::

A complete []{#id902932 .indexterm}[]{#flow style/syntax}*flow node* is either an []{#id902948 .indexterm}[alias node](#alias/syntax) []{#id902962 .indexterm}[presenting](#present/) a second occurrence of a previous []{#id902976 .indexterm}[node](#node/syntax), or consists of the []{#id902991 .indexterm}[node properties](#node%20property/) followed by the []{#id903004 .indexterm}[node's content](#content/syntax). A []{#id903020 .indexterm}[node](#node/syntax) with empty []{#id903035 .indexterm}[content](#content/syntax) is considered to be an empty []{#id903050 .indexterm}[plain scalar](#plain%20style/syntax).

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- -------------------------------------------------------------------------------------------------- -------------- |
|   \[121\]          []{#ns-flow-node(n,c)}ns-flow-node(n,c)     `::=`        [c-ns-alias-node](#c-ns-alias-node) \| [ns-flow-content(n,c)](#ns-flow-content(n,c))\                           |
|                                                                           \| ( [c-ns-properties(n,c)](#c-ns-properties(n,c))\                                                               |
|                                                                               ( /\* empty plain scalar content \*/\                                                                         |
|                                                                               \| ( [s-separate(n,c)](#s-separate(n,c)) [ns-flow-content(n,c)](#ns-flow-content(n,c)) ) ) )                  |
|                                                                                                                                                                                             |
|   -------------- ----------------------------------------- -------------- -------------------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id903117}

**Example 8.12. Flow Nodes in Flow Context**

+------------------------------------------+-----------------------------------+
| ``` programlisting                       | ``` programlisting                |
| [                                        | %YAML 1.1                         |
|   Without properties,                    | ---                               |
|   &anchor "Anchored",                    | !!seq [                           |
|   !!str 'Tagged',                        |   !!str "Without properties",     |
|   *anchor, # Alias node                  |   &A !!str "Anchored",            |
|   !!str,   # Empty plain scalar          |   !!str "Tagged",                 |
| ]                                        |   *A,                             |
| ```                                      |   !!str "",                       |
|                                          | ]                                 |
| ``` synopsis                             | ```                               |
| Legend:                                  |                                   |
|   ns-flow-node(n,c) ns-flow-content(n,c) |                                   |
| ```                                      |                                   |
+------------------------------------------+-----------------------------------+
:::

Since both the []{#id903245 .indexterm}[node's properties](#node%20property/) and []{#id903257 .indexterm}[node content](#content/syntax) are optional, this allows for a []{#id903272 .indexterm}[]{#completely empty node/}*completely empty node*. Completely empty nodes are only valid when following some explicit []{#id903289 .indexterm}[indicator](#indicator/) for their existence.

+-------------------------------------------------------------------------------------------+
|   --------- ------------------------------- ------- --------------------------------- --- |
|   \[122\]     []{#e-empty-flow}e-empty-flow  `::=`  /\* empty plain scalar node \*/       |
|   --------- ------------------------------- ------- --------------------------------- --- |
+-------------------------------------------------------------------------------------------+

In the examples, completely empty nodes are displayed as the glyph "[**`°`**]{.quote}". Note that this glyph corresponds to a position in the characters []{#id903331 .indexterm}[stream](#stream/syntax) rather than to an actual character.

::: example
[]{#id903348}

**Example 8.13. Completely Empty Flow Nodes**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| {                                 | %YAML 1.1                         |
|   ? foo :°,                       | ---                               |
|   ?° : bar,                       | !!map {                           |
| }                                 |   ? !!str "foo"                   |
| ```                               |   : !!str "",                     |
|                                   |   ? !!str "",                     |
| ``` synopsis                      |   : !!str "bar",                  |
| Legend:                           | }                                 |
|   e-empty-flow                    | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::
::::::::

:::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id903421}8.5.2. Block Nodes {#block-nodes .title}
:::
::::
:::::

A complete []{#id903430 .indexterm}[]{#block style/syntax}*block node* consists of the []{#id903448 .indexterm}[node's properties](#node%20property/) followed by the []{#id903462 .indexterm}[node's content](#content/syntax). In addition, a block node may consist of a (possibly []{#id903478 .indexterm}[completely empty](#completely%20empty%20node/)) []{#id903492 .indexterm}[flow node](#flow%20style/syntax) followed by a []{#id903508 .indexterm}[line break](#line%20break%20character/) (with optional []{#id903524 .indexterm}[comments](#comment/syntax)).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------------- -------------- --------------------------------------------------------------------------------------------- -------------- |
|   \[123\]            []{#ns-l+flow-in-block(n,c)}ns-l+flow-in-block(n,c)     `::=`      [ns-flow-node(n+1,flow-out)](#ns-flow-node(n,c)) [s-l-comments](#s-l-comments)                               |
|                                                                                                                                                                                                      |
|   \[124\]          []{#ns-l+block-in-block(n,c)}ns-l+block-in-block(n,c)     `::=`      ( [c-ns-properties(n+1,c)](#c-ns-properties(n,c)) [s-separate(n+1,c)](#s-separate(n,c)) )?\                  |
|                                                                                         [c-l+block-content(n,c)](#c-l+block-content(n,c))                                                            |
|                                                                                                                                                                                                      |
|   \[125\]                  []{#ns-l+block-node(n,c)}ns-l+block-node(n,c)     `::=`        [ns-l+block-in-block(n,c)](#ns-l+block-in-block(n,c))\                                                     |
|                                                                                         \| [ns-l+flow-in-block(n,c)](#ns-l+flow-in-block(n,c))                                                       |
|                                                                                                                                                                                                      |
|   \[126\]                    []{#s-l+block-node(n,c)}s-l+block-node(n,c)     `::=`      [s-separate(n+1,c)](#s-separate(n,c)) [ns-l+block-node(n,c)](#ns-l+block-node(n,c))                          |
|   -------------- ------------------------------------------------------- -------------- --------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id903643}

**Example 8.14. Block Nodes**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| -·"flow in block"↓                | %YAML 1.1                         |
| -·>                               | ---                               |
|  Block scalar↓                    | !!seq [                           |
| -·!!map # Block collection        |   !!str "flow in block",          |
|   foo : bar↓                      |   !!str "Block scalar\n",         |
| ```                               |   !!map {                         |
|                                   |     ? !!str "foo"                 |
| ``` synopsis                      |     : !!str "bar"                 |
| Legend:                           |   }                               |
|   ns-l+flow-in-block(n,c)         | ]                                 |
|   ns-l+block-in-block(n,c)        | ```                               |
|   s-l+block-node(n,c)             |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

A block node always spans to the end of the line, even when []{#id903767 .indexterm}[completely empty](#completely%20empty%20node/). []{#id903780 .indexterm}[Completely empty](#completely%20empty%20node/) block nodes may only appear when there is some explicit []{#id903797 .indexterm}[indicator](#indicator/) for their existence.

+-----------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------- ------- ------------------------------------------------------------- --- |
|   \[127\]     []{#s-l-empty-block}s-l-empty-block  `::=`  [e-empty-flow](#e-empty-flow) [s-l-comments](#s-l-comments)       |
|   --------- ------------------------------------- ------- ------------------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id903833}

**Example 8.15. Completely Empty Block Nodes**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| seq:                              | %YAML 1.1                         |
| -° # Empty plain scalar↓          | ---                               |
| - ? foo                           | !!seq [                           |
|   :°↓                             |   !!str "",                       |
|   ?°↓                             |   !!map {                         |
|   : bar,                          |     ? !!str "foo"                 |
| ```                               |     : !!str "",                   |
|                                   |     ? !!str "",                   |
| ``` synopsis                      |     : !!str "bar",                |
| Legend:                           |   }                               |
|   s-l-empty-block                 | ]                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
::::::::
::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id903915}Chapter 9. Scalar Styles {#chapter-9.-scalar-styles .title}
:::
::::
:::::

YAML provides a rich set of []{#id903924 .indexterm}[]{#scalar/syntax}*scalar styles* to choose from, depending upon the readability requirements: three []{#id903941 .indexterm}[scalar flow styles](#flow%20scalar%20style/syntax) (the []{#id903957 .indexterm}[plain style](#plain%20style/syntax) and the two []{#id903973 .indexterm}[]{#quoted style/syntax}*quoted styles*: []{#id903990 .indexterm}[single-quoted](#single-quoted%20style/syntax) and []{#id904006 .indexterm}[double-quoted](#double-quoted%20style/syntax)), and two []{#id904022 .indexterm}[scalar block styles](#block%20scalar%20style/syntax) (the []{#id904037 .indexterm}[literal style](#literal%20style/syntax) and the []{#id904053 .indexterm}[folded style](#folded%20style/syntax)). []{#id904069 .indexterm}[Comments](#comment/syntax) may precede or follow scalar content, but must not appear inside it. Scalar node style is a []{#id904085 .indexterm}[presentation detail](#presentation%20detail/) and must not be used to convey []{#id904099 .indexterm}[content information](#content/information%20model), with the exception that []{#id904115 .indexterm}[untagged](#non-specific%20tag/) []{#id904129 .indexterm}[plain scalars](#plain%20style/syntax) are []{#id904146 .indexterm}[resolved](#tag%20resolution/) in a distinct way.

::::::::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id904158}9.1. Flow Scalar Styles {#flow-scalar-styles .title style="clear: both"}
:::
::::
:::::

All []{#id904166 .indexterm}[]{#flow scalar style/syntax}*flow scalar styles* may span multiple lines, except when used in []{#id904184 .indexterm}[simple keys](#simple%20key/). Flow scalars are subject to (flow) []{#id904197 .indexterm}[line folding](#line%20folding/). This allows flow scalar content to be broken anywhere a single space character (**`#x20`**) separates non-space characters, at the cost of requiring an []{#id904218 .indexterm}[empty line](#empty%20line/) to []{#id904231 .indexterm}[present](#present/) each line feed character.

::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id904245}9.1.1. Double Quoted {#double-quoted .title}
:::
::::
:::::

The []{#id904253 .indexterm}[]{#double-quoted style/syntax}*double-quoted style* is specified by surrounding []{#id904271 .indexterm}[]{#\" double-quoted style/}*"[**`"`**]{.quote}" indicators*. This is the only []{#id904294 .indexterm}[scalar style](#scalar/syntax) capable of expressing arbitrary strings, by using []{#id904308 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/) []{#id904327 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/). Therefore, the []{#id904342 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/) and "[**`"`**]{.quote}" characters must also be []{#id904367 .indexterm}[escaped](#\%20escaping%20in%20double-quoted%20style/) when present in double-quoted content. Note it is an error for double-quoted content to contain invalid []{#id904384 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------- ------- -------------------------------------------------------------------------------------------------------------------------- --- |
|   \[128\]     []{#nb-double-char}nb-double-char  `::=`  ( [nb-char](#nb-char) - ["[\\]{.quote}"](#c-escape) - ["[\"]{.quote}"](#c-double-quote) ) \| [ns-esc-char](#ns-esc-char)       |
|   \[129\]     []{#ns-double-char}ns-double-char  `::=`  [nb-double-char](#nb-double-char) - [s-white](#s-white)                                                                        |
|   --------- ----------------------------------- ------- -------------------------------------------------------------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Double-quoted scalars are restricted to a single line when contained inside a []{#id904461 .indexterm}[simple key](#simple%20key/).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
|   \[130\]          []{#c-double-quoted(n,c)}c-double-quoted(n,c)     `::=`      ["[\"]{.quote}"](#c-double-quote) [nb-double-text(n,c)](#nb-double-text(n,c)) ["[\"]{.quote}"](#c-double-quote)                  |
|                                                                                                                                                                                                                  |
|   \[131\]            []{#nb-double-text(n,c)}nb-double-text(n,c)     `::=`      `c`{.varname} = flow-out ⇒ [nb-double-any(n)](#nb-double-any(n))\                                                                |
|                                                                                 `c`{.varname} = flow-in  ⇒ [nb-double-any(n)](#nb-double-any(n))\                                                                |
|                                                                                 `c`{.varname} = flow-key ⇒ [nb-double-single](#nb-double-single)                                                                 |
|                                                                                                                                                                                                                  |
|   \[132\]                  []{#nb-double-any(n)}nb-double-any(n)     `::=`      [nb-double-single](#nb-double-single) \| [nb-double-multi(n)](#nb-double-multi(n))                                               |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id904573}

**Example 9.1. Double Quoted Scalars**

+---------------------------------------+-----------------------------------+
| ``` programlisting                    | ``` programlisting                |
| "simple key" : {                      | %YAML 1.1                         |
|   "also simple" : value,              | ---                               |
|   ? "not a                            | !!map {                           |
|   simple key" : "any                  |   ? !!str "simple key"            |
|   value"                              |   : !!map {                       |
| }                                     |     ? !!str "also simple"         |
| ```                                   |     : !!str "value",              |
|                                       |     ? !!str "not a simple key"    |
| ``` synopsis                          |     : !!str "any value"           |
| Legend:                               |   }                               |
|   nb-double-single nb-double-multi(n) | }                                 |
|   c-double-quoted(n,c)                | ```                               |
| ```                                   |                                   |
+---------------------------------------+-----------------------------------+
:::

A single line double-quoted scalar is a sequence of (possibly []{#id904712 .indexterm}[escaped](#escaping%20in%20double-quoted%20style/)) non-[]{#id904728 .indexterm}[break](#line%20break%20character/) Unicode characters. All characters are considered []{#id904742 .indexterm}[content](#content/syntax), including any leading or trailing []{#id904757 .indexterm}[white space](#white%20space/) characters.

+-------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------- ------- ------------------------------------- --- |
|   \[133\]     []{#nb-double-single}nb-double-single  `::=`  [nb-double-char](#nb-double-char)\*       |
|   --------- --------------------------------------- ------- ------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------+

In a multi-line double-quoted scalar, []{#id904794 .indexterm}[line breaks](#line%20break%20character/) are subject to flow line []{#id904810 .indexterm}[folding](#line%20folding/), and any trailing []{#id904821 .indexterm}[white space](#white%20space/) is excluded from the []{#id904834 .indexterm}[content](#content/syntax). However, an []{#id904849 .indexterm}[]{#escaped (ignored) line break/}*escaped line break* (using a []{#id904864 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/)) is excluded from the []{#id904885 .indexterm}[content](#content/syntax), while []{#id904898 .indexterm}[white space](#white%20space/) preceding it is preserved. This allows double-quoted content to be broken at arbitrary positions.

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------- -------------- -------------------------------------------------------------------------------------------------- -------------- |
|   \[134\]            []{#s-l-double-folded(n)}s-l-double-folded(n)     `::=`      [s-ignored-white](#s-ignored-white)\* [b-l-folded-any(n,double)](#b-l-folded-any(n,s))                            |
|                                                                                                                                                                                                     |
|   \[135\]          []{#s-l-double-escaped(n)}s-l-double-escaped(n)     `::=`      [s-white](#s-white)\* ["[\\]{.quote}"](#c-escape) [b-ignored-any](#b-ignored-any)\                                |
|                                                                                   [l-empty(n,double)](#l-empty(n,s))\*                                                                              |
|                                                                                                                                                                                                     |
|   \[136\]              []{#s-l-double-break(n)}s-l-double-break(n)     `::=`      [s-l-double-folded(n)](#s-l-double-folded(n)) \| [s-l-double-escaped(n)](#s-l-double-escaped(n))                  |
|   -------------- ------------------------------------------------- -------------- -------------------------------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id905000}

**Example 9.2. Double Quoted Line Breaks**

+-----------------------------------+----------------------------------------------+
| ``` programlisting                | ``` programlisting                           |
|  "as space→↓                      | %YAML 1.1                                    |
|  trimmed·↓                        | ---                                          |
| ↓                                 | !!str "as space \                            |
|  specific⇓                        |   trimmed\n\                                 |
| ↓                                 |   specific\L\n\                              |
|  escaped→\¶                       |   escaped\t\                                 |
| ·↓                                |   none"                                      |
|  none"                            | ```                                          |
| ```                               |                                              |
|                                   | ``` synopsis                                 |
|                                   | Legend:                                      |
|                                   |   s-l-double-folded(n) s-l-double-escaped(n) |
|                                   |   s-ignored-white      s-white (Content)     |
|                                   | ```                                          |
+-----------------------------------+----------------------------------------------+
:::

A multi-line double-quoted scalar consists of a (possibly empty) first line, any number of inner lines, and a final (possibly empty) last line.

+---------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------- -------------- ------------------------------------------------ -------------- |
|   \[137\]          []{#nb-double-multi(n)}nb-double-multi(n)     `::=`      [nb-l-double-first(n)](#nb-l-double-first(n))\                  |
|                                                                             [l-double-inner(n)](#l-double-inner(n))\*\                      |
|                                                                             [s-nb-double-last(n)](#s-nb-double-last(n))                     |
|                                                                                                                                             |
|   -------------- ------------------------------------------- -------------- ------------------------------------------------ -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------+

Leading []{#id905171 .indexterm}[white space](#white%20space/) in the first line is considered []{#id905184 .indexterm}[content](#content/syntax) only if followed by a non-space character or an escaped (ignored) line break.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------- -------------- |
|   \[138\]          []{#nb-l-double-first(n)}nb-l-double-first(n)     `::=`      ( [nb-double-char](#nb-double-char)\* [ns-double-char](#ns-double-char) )?\                  |
|                                                                                 [s-l-double-break(n)](#s-l-double-break(n))                                                  |
|                                                                                                                                                                              |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id905233}

**Example 9.3. First Double Quoted Line**

+----------------------------------------+-----------------------------------+
| ``` programlisting                     | ``` programlisting                |
| - "↓                                   | %YAML 1.1                         |
|   last"                                | ---                               |
| - "·→↓                                 | !!seq [                           |
|   last"                                |   !!str " last",                  |
| - "·→first↓                            |   !!str " last",                  |
|   last"                                |   !!str " \tfirst last",          |
| ```                                    | ]                                 |
|                                        | ```                               |
| ``` synopsis                           |                                   |
| Legend:                                |                                   |
|   nb-l-double-first(n) s-ignored-white |                                   |
| ```                                    |                                   |
+----------------------------------------+-----------------------------------+
:::

All leading and trailing []{#id905329 .indexterm}[white space](#white%20space/) of an inner lines are excluded from the []{#id905343 .indexterm}[content](#content/syntax). Note that while []{#id905358 .indexterm}[prefix white space](#ignored%20line%20prefix/) may contain []{#id905373 .indexterm}[tab](#tab/) characters, line []{#id905385 .indexterm}[indentation](#indentation%20space/) is restricted to space characters only. It is possible to force considering leading []{#id905400 .indexterm}[white space](#white%20space/) as []{#id905412 .indexterm}[content](#content/syntax) by []{#id905427 .indexterm}[escaping](#escaping%20in%20double-quoted%20style/) the first character ([]{#id905442 .indexterm}["[**`\·`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/), []{#id905462 .indexterm}["[**`\→`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/) or []{#id905480 .indexterm}["[**`\t`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/)).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
|   \[139\]          []{#l-double-inner(n)}l-double-inner(n)     `::=`      [s-ignored-prefix(n,double)](#s-ignored-prefix(n,s)) [ns-double-char](#ns-double-char)\                  |
|                                                                           ( [nb-double-char](#nb-double-char)\* [ns-double-char](#ns-double-char) )?\                              |
|                                                                           [s-l-double-break(n)](#s-l-double-break(n))                                                              |
|                                                                                                                                                                                    |
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id905543}

**Example 9.4. Inner Double Quoted Lines**

+---------------------------------------------+-----------------------------------+
| ``` programlisting                          | ``` programlisting                |
|  "first                                     | %YAML 1.1                         |
| ·→inner 1→↓                                 | ---                               |
| ·\·inner 2·\↓                               | !!str "first·\                    |
|  last"                                      |   inner 1··\                      |
| ```                                         |   inner 2·\                       |
|                                             |   last"                           |
| ``` synopsis                                | ```                               |
| Legend:                                     |                                   |
|   l-double-inner(n)                         |                                   |
|   s-ignored-prefix(n,s) s-l-double-break(n) |                                   |
| ```                                         |                                   |
+---------------------------------------------+-----------------------------------+
:::

The leading []{#id905662 .indexterm}[prefix](#ignored%20line%20prefix/) []{#id905675 .indexterm}[white space](#white%20space/) of the last line is stripped in the same way as for inner lines. Trailing []{#id905689 .indexterm}[white space](#white%20space/) is considered []{#id905701 .indexterm}[content](#content/syntax) only if preceded by a non-space character.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------- -------------- |
|   \[140\]          []{#s-nb-double-last(n)}s-nb-double-last(n)     `::=`      [s-ignored-prefix(n,double)](#s-ignored-prefix(n,s))\                                       |
|                                                                               ( [ns-double-char](#ns-double-char) [nb-double-char](#nb-double-char)\* )?                  |
|                                                                                                                                                                           |
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id905751}

**Example 9.5. Last Double Quoted Line**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - "first                          | %YAML 1.1                         |
| ··→"                              | ---                               |
| - "first                          | !!seq [                           |
|                                   |   !!str "first ",                 |
| ··→last"                          |   !!str "first\nlast",            |
| - "first                          |   !!str "first inner··\tlast",    |
|  inner                            | ]                                 |
| ·\·→last"                         | ```                               |
| ```                               |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   s-nb-double-last(n)             |
|                                   |   s-ignored-prefix(n,s)           |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::::

:::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id905860}9.1.2. Single Quoted {#single-quoted .title}
:::
::::
:::::

The []{#id905868 .indexterm}[]{#single-quoted style/syntax}*single-quoted style* is specified by surrounding []{#id905887 .indexterm}[]{#' single-quoted style/}*"[**`'`**]{.quote}" indicators*. Therefore, within a single-quoted scalar such characters need to be repeated. This is the only form of []{#id905912 .indexterm}[]{#escaping in single-quoted style/}*escaping* performed in single-quoted scalars. In particular, the []{#id905929 .indexterm}["[**`\`**]{.quote}"](#\%20escaping%20in%20double-quoted%20style/) and []{#id905946 .indexterm}["[**`"`**]{.quote}"](#%22%20double-quoted%20style/) characters may be freely used. This restricts single-quoted scalars to []{#id905966 .indexterm}[printable](#printable%20character/) characters.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------- ------- -------------------------------------------------------------------------------------------------- --- |
|   \[141\]     []{#c-quoted-quote}c-quoted-quote  `::=`  ["[\']{.quote}"](#c-single-quote) ["[\']{.quote}"](#c-single-quote)                                    |
|   \[142\]     []{#nb-single-char}nb-single-char  `::=`  ( [nb-char](#nb-char) - ["[\"]{.quote}"](#c-single-quote) ) \| [c-quoted-quote](#c-quoted-quote)       |
|   \[143\]     []{#ns-single-char}ns-single-char  `::=`  [nb-single-char](#nb-single-char) - [s-white](#s-white)                                                |
|   --------- ----------------------------------- ------- -------------------------------------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id906056}

**Example 9.6. Single Quoted Quotes**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
|  'here''s to "quotes"'            | %YAML 1.1                         |
| ```                               | ---                               |
|                                   | !!str "here's to \"quotes\""      |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   single-quoted-quote             |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Single-quoted scalars are restricted to a single line when contained inside a []{#id906126 .indexterm}[simple key](#simple%20key/).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
|   \[144\]          []{#c-single-quoted(n,c)}c-single-quoted(n,c)     `::=`      ["[\']{.quote}"](#c-single-quote) [nb-single-text(n,c)](#nb-single-text(n,c)) ["[\']{.quote}"](#c-single-quote)                  |
|                                                                                                                                                                                                                  |
|   \[145\]            []{#nb-single-text(n,c)}nb-single-text(n,c)     `::=`      `c`{.varname} = flow-out ⇒ [nb-single-any(n)](#nb-single-any(n))\                                                                |
|                                                                                 `c`{.varname} = flow-in  ⇒ [nb-single-any(n)](#nb-single-any(n))\                                                                |
|                                                                                 `c`{.varname} = flow-key ⇒ [nb-single-single(n)](#nb-single-single)                                                              |
|                                                                                                                                                                                                                  |
|   \[146\]                  []{#nb-single-any(n)}nb-single-any(n)     `::=`      [nb-single-single(n)](#nb-single-single) \| [nb-single-multi(n)](#nb-single-multi(n))                                            |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id906238}

**Example 9.7. Single Quoted Scalars**

+---------------------------------------+-----------------------------------+
| ``` programlisting                    | ``` programlisting                |
| 'simple key' : {                      | %YAML 1.1                         |
|   'also simple' : value,              | ---                               |
|   ? 'not a                            | !!map {                           |
|   simple key' : 'any                  |   ? !!str "simple key"            |
|   value'                              |   : !!map {                       |
| }                                     |     ? !!str "also simple"         |
| ```                                   |     : !!str "value",              |
|                                       |     ? !!str "not a simple key"    |
| ``` synopsis                          |     : !!str "any value"           |
| Legend:                               |   }                               |
|   nb-single-single nb-single-multi(n) | }                                 |
|   c-single-quoted(n,c)                | ```                               |
| ```                                   |                                   |
+---------------------------------------+-----------------------------------+
:::

A single line single-quoted scalar is a sequence of non-[]{#id906376 .indexterm}[break](#line%20break%20character/) []{#id906390 .indexterm}[printable](#printable%20character/) characters. All characters are considered []{#id906404 .indexterm}[content](#content/syntax), including any leading or trailing []{#id906419 .indexterm}[white space](#white%20space/) characters.

+----------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------ ------- ------------------------------------- --- |
|   \[147\]     []{#nb-single-single}nb-single-single(n)  `::=`  [nb-single-char](#nb-single-char)\*       |
|   --------- ------------------------------------------ ------- ------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------+

In a multi-line single-quoted scalar, []{#id906457 .indexterm}[line breaks](#line%20break%20character/) are subject to (flow) []{#id906473 .indexterm}[line folding](#line%20folding/), and any trailing []{#id906484 .indexterm}[white space](#white%20space/) is excluded from the []{#id906497 .indexterm}[content](#content/syntax).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------------- ------- ---------------------------------------------------------------------------------------- --- |
|   \[148\]     []{#s-l-single-break(n)}s-l-single-break(n)  `::=`  [s-ignored-white](#s-ignored-white)\* [b-l-folded-any(n,single)](#b-l-folded-any(n,s))       |
|   --------- --------------------------------------------- ------- ---------------------------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id906540}

**Example 9.8. Single Quoted Line Breaks**

+-----------------------------------+-------------------------------------+
| ``` programlisting                | ``` programlisting                  |
|  'as space→↓                      | %YAML 1.1                           |
|  trimmed·↓                        | ---                                 |
| ↓                                 | !!str "as space \                   |
|  specific⇓                        |   trimmed\n\                        |
| ↓                                 |   specific\L\n\                     |
|  none'                            |   none"                             |
| ```                               | ```                                 |
|                                   |                                     |
|                                   | ``` synopsis                        |
|                                   | Legend:                             |
|                                   |   s-l-single-break(n)               |
|                                   |   s-ignored-white s-white (Content) |
|                                   | ```                                 |
+-----------------------------------+-------------------------------------+
:::

A multi-line single-quoted scalar consists of a (possibly empty) first line, any number of inner lines, and a final (possibly empty) last line.

+---------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------- -------------- ------------------------------------------------ -------------- |
|   \[149\]          []{#nb-single-multi(n)}nb-single-multi(n)     `::=`      [nb-l-single-first(n)](#nb-l-single-first(n))\                  |
|                                                                             [l-single-inner(n)](#l-single-inner(n))\*\                      |
|                                                                             [s-nb-single-last(n)](#s-nb-single-last(n))                     |
|                                                                                                                                             |
|   -------------- ------------------------------------------- -------------- ------------------------------------------------ -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------+

Leading []{#id906689 .indexterm}[white space](#white%20space/) in the first line is considered []{#id906701 .indexterm}[content](#content/syntax) only if followed by a non-space character.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------- -------------- |
|   \[150\]          []{#nb-l-single-first(n)}nb-l-single-first(n)     `::=`      ( [nb-single-char](#nb-single-char)\* [ns-single-char](#ns-single-char) )?\                  |
|                                                                                 [s-l-single-break(n)](#s-l-single-break(n))                                                  |
|                                                                                                                                                                              |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id906750}

**Example 9.9. First Single Quoted Line**

+----------------------------------------+-----------------------------------+
| ``` programlisting                     | ``` programlisting                |
| - '↓                                   | %YAML 1.1                         |
|   last'                                | ---                               |
| - '·→↓                                 | !!seq [                           |
|   last'                                |   !!str " last",                  |
| - '·→first↓                            |   !!str " last",                  |
|   last'                                |   !!str " \tfirst last",          |
| ```                                    | ]                                 |
|                                        | ```                               |
| ``` synopsis                           |                                   |
| Legend:                                |                                   |
|   nb-l-single-first(n) s-ignored-white |                                   |
| ```                                    |                                   |
+----------------------------------------+-----------------------------------+
:::

All leading and trailing []{#id906846 .indexterm}[white space](#white%20space/) of inner lines is excluded from the []{#id906860 .indexterm}[content](#content/syntax). Note that while []{#id906875 .indexterm}[prefix white space](#ignored%20line%20prefix/) may contain []{#id906889 .indexterm}[tab](#tab/) characters, line []{#id906901 .indexterm}[indentation](#indentation%20space/) is restricted to space characters only. Unlike []{#id906917 .indexterm}[double-quoted scalars](#double-quoted%20style/syntax), it is impossible to force the inclusion of the leading or trailing spaces in the []{#id906933 .indexterm}[content](#content/syntax). Therefore, single-quoted scalars lines can only be broken where a single space character separates two non-space characters.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
|   \[151\]          []{#l-single-inner(n)}l-single-inner(n)     `::=`      [s-ignored-prefix(n,single)](#s-ignored-prefix(n,s)) [ns-single-char](#ns-single-char)\                  |
|                                                                           ( [nb-single-char](#nb-single-char)\* [ns-single-char](#ns-single-char) )?\                              |
|                                                                           [s-l-single-break(n)](#s-l-single-break(n))                                                              |
|                                                                                                                                                                                    |
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id906995}

**Example 9.10. Inner Single Quoted Lines**

+---------------------------------------------+-----------------------------------+
| ``` programlisting                          | ``` programlisting                |
|  'first                                     | %YAML 1.1                         |
| ·→inner→↓                                   | ---                               |
|  last'                                      | !!str "first \                    |
| ```                                         |   inner \                         |
|                                             |   last"                           |
| ``` synopsis                                | ```                               |
| Legend:                                     |                                   |
|   l-single-inner(n)                         |                                   |
|   s-ignored-prefix(n,s) s-l-single-break(n) |                                   |
| ```                                         |                                   |
+---------------------------------------------+-----------------------------------+
:::

The leading []{#id907096 .indexterm}[prefix](#ignored%20line%20prefix/) []{#id907109 .indexterm}[white space](#white%20space/) of the last line is stripped in the same way as for inner lines. Trailing []{#id907123 .indexterm}[white space](#white%20space/) is considered []{#id907136 .indexterm}[content](#content/syntax) only if preceded by a non-space character.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------- -------------- |
|   \[152\]          []{#s-nb-single-last(n)}s-nb-single-last(n)     `::=`      [s-ignored-prefix(n,single)](#s-ignored-prefix(n,s))\                                       |
|                                                                               ( [ns-single-char](#ns-single-char) [nb-single-char](#nb-single-char)\* )?                  |
|                                                                                                                                                                           |
|   -------------- --------------------------------------------- -------------- ---------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id907185}

**Example 9.11. Last Single Quoted Lines**

+---------------------------------------------+-----------------------------------+
| ``` programlisting                          | ``` programlisting                |
| - 'first                                    | %YAML 1.1                         |
| ··→'                                        | ---                               |
| - 'first                                    | !!seq [                           |
|                                             |   !!str "first ",                 |
| ··→last'                                    |   !!str "first\nlast",            |
| ```                                         | ]                                 |
|                                             | ```                               |
| ``` synopsis                                |                                   |
| Legend:                                     |                                   |
|   s-nb-double-last(n) s-ignored-prefix(n,s) |                                   |
| ```                                         |                                   |
+---------------------------------------------+-----------------------------------+
:::
::::::::::::

:::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id907281}9.1.3. Plain {#plain .title}
:::
::::
:::::

The []{#id907289 .indexterm}[]{#plain style/syntax}*plain style* uses no identifying []{#id907307 .indexterm}[indicators](#indicator/), and is therefore the most limited and most []{#id907320 .indexterm}[context](#context/) sensitive []{#id907334 .indexterm}[scalar style](#scalar/syntax). Plain scalars can never contain any []{#id907350 .indexterm}[tab](#tab/) characters. They also must not contain the []{#id907363 .indexterm}["[**`: `**]{.quote}"](#:%20mapping%20value/) and []{#id907382 .indexterm}["[**` #`**]{.quote}"](##%20comment/) character sequences as these combinations cause ambiguity with []{#id907400 .indexterm}[key:](#key/syntax) []{#id907413 .indexterm}[value](#value/syntax) pairs and []{#id907428 .indexterm}[comments](#comment/syntax). Inside []{#id907443 .indexterm}[flow collections](#flow%20collection%20style/syntax), plain scalars are further restricted to avoid containing the []{#id907460 .indexterm}["[**`[`**]{.quote}"](#%5B%20start%20flow%20sequence/), []{#id907480 .indexterm}["[**`]`**]{.quote}"](#%5D%20end%20flow%20sequence/), []{#id907497 .indexterm}["[**`{`**]{.quote}"](#%7B%20start%20flow%20mapping/), []{#id907514 .indexterm}["[**`}`**]{.quote}"](#%7D%20end%20flow%20mapping/) and []{#id907529 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) characters as these would cause ambiguity with the []{#id907548 .indexterm}[flow collection](#flow%20collection%20style/syntax) structure (hence the need for the []{#id907564 .indexterm}[]{#flow-in context/}*flow-in context* and the []{#id907580 .indexterm}[]{#flow-out context/}*flow-out context*).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
|   \[153\]            []{#nb-plain-char(c)}nb-plain-char(c)     `::=`      `c`{.varname} = flow-out ⇒ [nb-plain-char-out](#nb-plain-char-out)\                                                                                                                                                                          |
|                                                                           `c`{.varname} = flow-in  ⇒ [nb-plain-char-in](#nb-plain-char-in)\                                                                                                                                                                            |
|                                                                           `c`{.varname} = flow-key ⇒ [nb-plain-char-in](#nb-plain-char-in)                                                                                                                                                                             |
|                                                                                                                                                                                                                                                                                                                        |
|   \[154\]          []{#nb-plain-char-out}nb-plain-char-out     `::=`        ( [nb-char](#nb-char) - ["[:]{.quote}"](#c-mapping-value) - ["[\#]{.quote}"](#c-comment) - #x9 /\*TAB\*/ )\                                                                                                                                |
|                                                                           \| ( [ns-plain-char(flow-out)](#ns-plain-char(c)) ["[\#]{.quote}"](#c-comment) )\                                                                                                                                                            |
|                                                                           \| ( ["[:]{.quote}"](#c-mapping-value) [ns-plain-char(flow-out)](#ns-plain-char(c)) )                                                                                                                                                        |
|                                                                                                                                                                                                                                                                                                                        |
|   \[155\]            []{#nb-plain-char-in}nb-plain-char-in     `::=`      [nb-plain-char-out](#nb-plain-char-out) - ["[,]{.quote}"](#c-collect-entry) - ["[\[]{.quote}"](#c-sequence-start) - ["[\]]{.quote}"](#c-sequence-end) - ["[{]{.quote}"](#c-mapping-start) - ["[}]{.quote}"](#c-mapping-end)                  |
|                                                                                                                                                                                                                                                                                                                        |
|   \[156\]            []{#ns-plain-char(c)}ns-plain-char(c)     `::=`      [nb-plain-char(c)](#nb-plain-char(c)) - #x20 /\*SP\*/                                                                                                                                                                                        |
|   -------------- ----------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

The first plain character is further restricted to avoid most []{#id907776 .indexterm}[indicators](#indicator/) as these would cause ambiguity with various YAML structures. However, the first character may be []{#id907790 .indexterm}["[**`-`**]{.quote}"](#-%20block%20sequence%20entry/), []{#id907810 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) or []{#id907826 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) provided it is followed by a non-space character.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
|   \[157\]          []{#ns-plain-first-char(c)}ns-plain-first-char(c)     `::=`        ( [ns-plain-char(c)](#ns-plain-char(c)) - [c-indicator](#c-indicator) )\                                                                                                   |
|                                                                                     \| ( ( ["[-]{.quote}"](#c-sequence-entry) \| ["[?]{.quote}"](#c-mapping-key) \| ["[:]{.quote}"](#c-mapping-value) ) [ns-plain-char(c)](#ns-plain-char(c)) )                  |
|                                                                                                                                                                                                                                                                  |
|   -------------- --------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id907899}

**Example 9.12. Plain Characters**

+-----------------------------------------+-----------------------------------+
| ``` programlisting                      | ``` programlisting                |
| # Outside flow collection:              | %YAML 1.1                         |
| - ::std::vector                         | ---                               |
| - Up, up and away!                      | !!seq [                           |
| - -123                                  |   !!str "::std::vector",          |
| # Inside flow collection:               |   !!str "Up, up, and away!",      |
| - [ ::std::vector,                      |   !!int "-123",                   |
|   "Up, up and away!",                   |   !!seq [                         |
|   -123 ]                                |     !!str "::std::vector",        |
| ```                                     |     !!str "Up, up, and away!",    |
|                                         |     !!int "-123",                 |
| ``` synopsis                            |   ]                               |
| Legend:                                 | ]                                 |
|   ns-plain-first-char(c)                | ```                               |
|   ns-plain-char(c) Not ns-plain-char(c) |                                   |
| ```                                     |                                   |
+-----------------------------------------+-----------------------------------+
:::

Plain scalars are restricted to a single line when contained inside a []{#id908025 .indexterm}[simple key](#simple%20key/).

+-------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
|   \[158\]          []{#ns-plain(n,c)}ns-plain(n,c)     `::=`      `c`{.varname} = flow-out ⇒ [ns-plain-multi(n,c)](#ns-plain-multi(n,c))?\                  |
|                                                                   `c`{.varname} = flow-in  ⇒ [ns-plain-multi(n,c)](#ns-plain-multi(n,c))?\                  |
|                                                                   `c`{.varname} = flow-key ⇒ [ns-plain-single(c)](#ns-plain-single(c))                      |
|                                                                                                                                                             |
|   -------------- --------------------------------- -------------- -------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id908084}

**Example 9.13. Plain Scalars**

+------------------------------------------+-----------------------------------+
| ``` programlisting                       | ``` programlisting                |
| simple key : {                           | %YAML 1.1                         |
|   also simple : value,                   | ---                               |
|   ? not a                                | !!map {                           |
|   simple key : any                       |   ? !!str "simple key"            |
|   value                                  |   : !!map {                       |
| }                                        |     ? !!str "also simple"         |
| ```                                      |     : !!str "value",              |
|                                          |     ? !!str "not a simple key"    |
| ``` synopsis                             |     : !!str "any value"           |
| Legend:                                  |   }                               |
|   ns-plain-single(c) ns-plain-multi(n,c) | }                                 |
| ```                                      | ```                               |
+------------------------------------------+-----------------------------------+
:::

The first line of any []{#id908184 .indexterm}[flow scalar](#flow%20scalar%20style/syntax) is []{#id908200 .indexterm}[indented](#indentation%20space/) according to the []{#id908213 .indexterm}[collection](#collection/syntax) it is contained in. Therefore, there are two cases where a plain scalar begins on the first column of a line, without any preceding []{#id908232 .indexterm}[indentation](#indentation%20space/) spaces: a plain scalar used as a []{#id908245 .indexterm}[simple key](#simple%20key/) of a non-indented []{#id908258 .indexterm}[block mapping](#block%20mapping%20style/syntax), and any plain scalar nested in a non-indented []{#id908276 .indexterm}[flow collection](#flow%20collection%20style/syntax). In these cases, the first line of the plain scalar must not conflict with a []{#id908293 .indexterm}[document boundary marker](#document%20boundary%20marker/).

+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- --------------------------------------------------------------------------------- -------------- |
|   \[159\]          []{#l-forbidden-content}l-forbidden-content     `::=`      /\* start of line \*/\                                                                           |
|                                                                               ( [c-document-start](#c-document-start) \| [c-document-end](#c-document-end) )\                  |
|                                                                               /\* space or end of line \*/                                                                     |
|                                                                                                                                                                                |
|   -------------- --------------------------------------------- -------------- --------------------------------------------------------------------------------- -------------- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id908340}

**Example 9.14. Forbidden Non-Indented Plain Scalar Content**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` screen                        |
| ---                               | ERROR:                            |
| ---·||| : foo                     |  The --- and ... document         |
| ...·>>>: bar                      |  start and end markers must       |
| ---                               |  not be specified as the          |
| [                                 |  first content line of a          |
| ---↓                              |  non-indented plain scalar.       |
| ,                                 | ```                               |
| ...·,                             |                                   |
| {                                 |                                   |
| ---·:                             |                                   |
| ...·# Nested                      |                                   |
| }                                 |                                   |
| ]                                 |                                   |
| ...                               |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

YAML provides several easy ways to []{#id908434 .indexterm}[present](#present/) such []{#id908446 .indexterm}[content](#content/syntax) without conflicting with the []{#id908462 .indexterm}[document boundary markers](#document%20boundary%20marker/). For example:

::: example
[]{#id908477}

**Example 9.15. Document Marker Scalar Content**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ---                               | %YAML 1.1                         |
| "---" : foo                       | ---                               |
| ...: bar                          | !!map {                           |
| ---                               |   ? !!str "---"                   |
| [                                 |   : !!str "foo",                  |
| ---,                              |   ? !!str "...",                  |
| ...,                              |   : !!str "bar"                   |
| {                                 | }                                 |
| ? ---                             | %YAML 1.1                         |
| : ...                             | ---                               |
| }                                 | !!seq [                           |
| ]                                 |   !!str "---",                    |
| ...                               |   !!str "...",                    |
| ```                               |   !!map {                         |
|                                   |     ? !!str "---"                 |
| ``` synopsis                      |     : !!str "..."                 |
| Legend:                           |   }                               |
|   Content --- and ...             | ]                                 |
|   Document marker --- and ...     | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Thus, a single line plain scalar is a sequence of valid plain non-[]{#id908596 .indexterm}[break](#line%20break%20character/) []{#id908610 .indexterm}[printable](#printable%20character/) characters, beginning and ending with non-space character and not conflicting with a []{#id908625 .indexterm}[document boundary markers](#document%20boundary%20marker/). All characters are considered []{#id908640 .indexterm}[content](#content/syntax), including any inner space characters.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------- -------------- ------------------------------------------------------------------------------------------- -------------- |
|   \[160\]          []{#ns-plain-single(c)}ns-plain-single(c)     `::=`        ( [ns-plain-first-char(c)](#ns-plain-first-char(c))\                                                     |
|                                                                                 ( [nb-plain-char(c)](#nb-plain-char(c))\* [ns-plain-char(c)](#ns-plain-char(c)) )? )\                  |
|                                                                             - [l-forbidden-content](#l-forbidden-content)                                                              |
|                                                                                                                                                                                        |
|   -------------- ------------------------------------------- -------------- ------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

In a multi-line plain scalar, []{#id908698 .indexterm}[line breaks](#line%20break%20character/) are subject to (flow) []{#id908714 .indexterm}[line folding](#line%20folding/). Any []{#id908725 .indexterm}[prefix](#ignored%20line%20prefix/) and trailing spaces are excluded from the []{#id908739 .indexterm}[content](#content/syntax). Like []{#id908754 .indexterm}[single-quoted scalars](#single-quoted%20style/syntax), in plain scalars it is impossible to force the inclusion of the leading or trailing spaces in the []{#id926249 .indexterm}[content](#content/syntax). Therefore, plain scalars lines can only be broken where a single space character separates two non-space characters.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------- ------- --------------------------------------------------------------------------------------- --- |
|   \[161\]     []{#s-l-plain-break(n)}s-l-plain-break(n)  `::=`  [s-ignored-white](#s-ignored-white)\* [b-l-folded-any(n,plain)](#b-l-folded-any(n,s))       |
|   --------- ------------------------------------------- ------- --------------------------------------------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id926293}

**Example 9.16. Plain Line Breaks**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
|  as space→↓                       | %YAML 1.1                         |
|  trimmed·↓                        | ---                               |
| ↓                                 | !!str "as space \                 |
|  specific⇓                        |   trimmed\n\                      |
| ↓                                 |   specific\L\n\                   |
|  none                             |   none"                           |
| ```                               | ```                               |
|                                   |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   s-l-plain-break(n)              |
|                                   |   s-ignored-white                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

A multi-line plain scalar contains additional continuation lines following the first line.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------------- ------- ------------------------------------------------------------------------------------------- --- |
|   \[162\]     []{#ns-plain-multi(n,c)}ns-plain-multi(n,c)  `::=`  [ns-plain-single(c)](#ns-plain-single(c)) [s-ns-plain-more(n,c)](#s-ns-plain-more(n,c))\*       |
|   --------- --------------------------------------------- ------- ------------------------------------------------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Each continuation line must contain at least one non-space character. Note that it may be preceded by any number of []{#id926427 .indexterm}[empty lines](#empty%20line/).

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------------------------------------- -------------- |
|   \[163\]          []{#s-ns-plain-more(n,c)}s-ns-plain-more(n,c)     `::=`      [s-l-plain-break(n)](#s-l-plain-break(n))\                                                                  |
|                                                                                 [s-ignored-prefix(n,plain)](#s-ignored-prefix(n,s)) [ns-plain-char(c)](#ns-plain-char(c))\                  |
|                                                                                 ( [nb-plain-char(c)](#nb-plain-char(c))\* [ns-plain-char(c)](#ns-plain-char(c)) )?                          |
|                                                                                                                                                                                             |
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id926484}

**Example 9.17. Plain Scalars**

+----------------------------------------------+-----------------------------------+
| ``` programlisting                           | ``` programlisting                |
|  first line·↓                                | %YAML 1.1                         |
| ···↓                                         | ---                               |
| ··more line                                  | !!str "first line\n\              |
| ```                                          |       more line"                  |
|                                              | ```                               |
| ``` synopsis                                 |                                   |
| Legend:                                      |                                   |
|   ns-plain-single(c) s-l-plain-break(n)      |                                   |
|   s-ignored-prefix(n,s) s-ns-plain-more(n,c) |                                   |
| ```                                          |                                   |
+----------------------------------------------+-----------------------------------+
:::
::::::::::::
:::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id926597}9.2. Block Scalar Header {#block-scalar-header .title style="clear: both"}
:::
::::
:::::

[]{#id926605 .indexterm}[Block scalars](#block%20scalar%20style/syntax) are specified by several []{#id926621 .indexterm}[indicators](#indicator/) given in a []{#id926633 .indexterm}[]{#block scalar header/}*header* preceding the []{#id926649 .indexterm}[content](#content/syntax) itself. The header is followed by an ignored []{#id926664 .indexterm}[line break](#line%20break%20character/) (with an optional []{#id926680 .indexterm}[comment](#comment/syntax)).

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------- -------------- -------------------------------------------------------------------- -------------- |
|   \[164\]          []{#c-b-block-header(s,m,t)}c-b-block-header(s,m,t)     `::=`      [c-style-indicator(s)](#c-style-indicator(s))\                                      |
|                                                                                       ( ( [c-indentation-indicator(m)](#c-indentation-indicator(m))\                      |
|                                                                                           [c-chomping-indicator(t)](#c-chomping-indicator(t)) )\                          |
|                                                                                       \| ( [c-chomping-indicator(t)](#c-chomping-indicator(t))\                           |
|                                                                                           [c-indentation-indicator(m)](#c-indentation-indicator(m)) ) )\                  |
|                                                                                       [s-b-comment](#s-b-comment)                                                         |
|                                                                                                                                                                           |
|   -------------- ----------------------------------------------------- -------------- -------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id926746}

**Example 9.18. Block Scalar Header**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - | # Just the style↓             | %YAML 1.1                         |
|  literal                          | ---                               |
| - >1 # Indentation indicator↓     | !!seq [                           |
|  ·folded                          |   !!str "literal\n",              |
| - |+ # Chomping indicator↓        |   !!str "·folded\n",              |
|  keep                             |   !!str "keep\n\n",               |
|                                   |   !!str "·strip",                 |
| - >-1 # Both indicators↓          | ]                                 |
|  ·strip                           | ```                               |
| ```                               |                                   |
|                                   | ``` synopsis                      |
|                                   | Legend:                           |
|                                   |   c-b-block-header(s,m,t)         |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id926836}9.2.1. Block Style Indicator {#block-style-indicator .title}
:::
::::
:::::

The first character of the []{#id926845 .indexterm}[block scalar header](#block%20scalar%20header/) is either []{#id926861 .indexterm}[]{#| literal style/}*"[**`|`**]{.quote}"* for a []{#id926880 .indexterm}[literal scalar](#literal%20style/syntax) or []{#id926895 .indexterm}[]{#> folded style/}*"[**`>`**]{.quote}"* for a []{#id926915 .indexterm}[folded scalar](#folded%20style/syntax).

+----------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- --------------------------------------------- -------------- |
|   \[165\]          []{#c-style-indicator(s)}c-style-indicator(s)     `::=`      s = literal ⇒ ["[\|]{.quote}"](#c-literal)\                  |
|                                                                                 s = folded  ⇒ ["[\>]{.quote}"](#c-folded)                    |
|                                                                                                                                              |
|   -------------- ----------------------------------------------- -------------- --------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id926963}

**Example 9.19. Block Style Indicator**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - |                               | %YAML 1.1                         |
|  literal                          | ---                               |
| - >                               | !!seq [                           |
|  folded                           |   !!str "literal\n",              |
| ```                               |   !!str "folded\n",               |
|                                   | ]                                 |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   c-style-indicator(s)            |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::
:::::::

:::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id927035}9.2.2. Block Indentation Indicator {#block-indentation-indicator .title}
:::
::::
:::::

Typically, the []{#id927043 .indexterm}[indentation](#indentation%20space/) level of a []{#id927058 .indexterm}[block scalar](#block%20scalar%20style/syntax) is detected from its first non-[]{#id927075 .indexterm}[empty](#empty%20line/) line. This detection fails when this line contains leading space characters (note it may safely start with a []{#id927088 .indexterm}[tab](#tab/) or a []{#id927100 .indexterm}["[**`#`**]{.quote}"](##%20comment/) character). When detection fails, YAML requires that the []{#id927119 .indexterm}[indentation](#indentation%20space/) level for the []{#id927133 .indexterm}[content](#content/syntax) be given using an explicit []{#id927146 .indexterm}[]{#indentation indicator/}*indentation indicator*. This level is specified as the integer number of the additional []{#id927164 .indexterm}[indentation](#indentation%20space/) spaces used for the []{#id927177 .indexterm}[content](#content/syntax). If the []{#id927191 .indexterm}[block scalar](#block%20scalar%20style/syntax) begins with leading []{#id927208 .indexterm}[empty lines](#empty%20line/) followed by a non-[]{#id927222 .indexterm}[empty line](#empty%20line/), the []{#id927234 .indexterm}[indentation](#indentation%20space/) level is deduced from the non-[]{#id927248 .indexterm}[empty line](#empty%20line/). In this case, it is an error for any such leading []{#id927261 .indexterm}[empty line](#empty%20line/) to contain more spaces than the []{#id927274 .indexterm}[indentation](#indentation%20space/) level deduced from the non-[]{#id927288 .indexterm}[empty](#empty%20line/) line. It is always valid to specify an indentation indicator for a []{#id927304 .indexterm}[block scalar](#block%20scalar%20style/syntax) node, though a YAML []{#id927319 .indexterm}[processor](#processor/) should only do so in cases where detection will fail.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------------- -------------- -------------------------------------------------------------- -------------- |
|   \[166\]          []{#c-indentation-indicator(m)}c-indentation-indicator(m)     `::=`      explicit(m) ⇒ [ns-dec-digit](#ns-dec-digit) - "[0]{.quote}"\                  |
|                                                                                             detect(m)   ⇒ /\* empty \*/                                                   |
|                                                                                                                                                                           |
|   -------------- ----------------------------------------------------------- -------------- -------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id927362}

**Example 9.20. Block Indentation Indicator**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - |                               | %YAML 1.1                         |
| ·detected                         | ---                               |
| - >                               | !!seq [                           |
| ·                                 |   !!str "detected\n",             |
| ··                                |   !!str "\n\n# detected\n",       |
| ··# detected                      |   !!str "·explicit\n",            |
| - |1                              |   !!str "\t·detected\n",          |
| ··explicit                        | ]                                 |
| - >                               | ```                               |
| ·→                                |                                   |
| ·detected                         | ``` synopsis                      |
| ```                               | Legend:                           |
|                                   |   c-indentation-indicator(m)      |
|                                   |   s-indent(n)                     |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

::: example
[]{#id927476}

**Example 9.21. Invalid Block Scalar Indentation Indicators**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| - |                               | ERROR:                            |
| ··                                | - A leading all-space line must   |
| ·text                             |   not have too many spaces.       |
| - >                               | - A following text line must      |
| ··text                            |   not be less indented.           |
| ·text                             | - The text is less indented       |
| - |1                              |   than the indicated level.       |
| ·text                             | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::
::::::::

:::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id927557}9.2.3. Block Chomping Indicator {#block-chomping-indicator .title}
:::
::::
:::::

YAML supports three possible block []{#id927566 .indexterm}[]{#chomping/}*chomping* methods:

::: variablelist

[Strip]{.term}
:   []{#id927591 .indexterm}[]{#strip chomping/}*Stripping* is specified using the []{#id927605 .indexterm}[]{#- strip chomping/}*"[**`-`**]{.quote}" chomping indicator*. In this case, the []{#id927628 .indexterm}[line break](#line%20break%20character/) character of the last non-[]{#id927640 .indexterm}[empty line](#empty%20line/) (if any) is excluded from the []{#id927653 .indexterm}[scalar's content](#scalar/syntax). Any trailing []{#id927669 .indexterm}[empty lines](#empty%20line/) are considered to be (empty) []{#id927681 .indexterm}[comment](#comment/syntax) lines and are also discarded.

[Clip]{.term}
:   []{#id927707 .indexterm}[]{#clip chomping/}*Clipping* is the default behavior used if no explicit chomping indicator is specified. In this case, The []{#id927723 .indexterm}[line break](#line%20break%20character/) character of the last non-[]{#id927739 .indexterm}[empty line](#empty%20line/) (if any) is preserved in the []{#id927750 .indexterm}[scalar's content](#scalar/syntax). However, any trailing []{#id927766 .indexterm}[empty lines](#empty%20line/) are considered to be (empty) []{#id927779 .indexterm}[comment](#comment/syntax) lines and are discarded.

[Keep]{.term}
:   []{#id927804 .indexterm}[]{#keep chomping/}*Keeping* is specified using the []{#id927819 .indexterm}[]{#+ keep chomping/}*"[**`+`**]{.quote}" chomping indicator*. In this case, the []{#id927840 .indexterm}[line break](#line%20break%20character/) character of the last non-[]{#id927854 .indexterm}[empty line](#empty%20line/) (if any) is preserved in the []{#id927867 .indexterm}[scalar's content](#scalar/syntax). In addition, any trailing []{#id927884 .indexterm}[empty lines](#empty%20line/) are each considered to []{#id927896 .indexterm}[present](#present/) a single trailing []{#id927909 .indexterm}[content](#content/syntax)[]{#id927924 .indexterm}[line break](#line%20break%20character/). Note that these []{#id927940 .indexterm}[line breaks](#line%20break%20character/) are not subject to []{#id927953 .indexterm}[folding](#line%20folding/).
:::

The chomping method used is a []{#id927971 .indexterm}[presentation detail](#presentation%20detail/) and is not reflected in the []{#id927987 .indexterm}[serialization tree](#serialization/) (and hence the []{#id927998 .indexterm}[representation](#representation/) graph).

+-----------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------- -------------- ---------------------------------------- -------------- |
|   \[167\]          []{#c-chomping-indicator(t)}c-chomping-indicator(t)     `::=`      `t`{.varname} = strip ⇒ "[-]{.quote}"\                  |
|                                                                                       `t`{.varname} = clip  ⇒ /\* empty \*/\                  |
|                                                                                       `t`{.varname} = keep  ⇒ "[+]{.quote}"                   |
|                                                                                                                                               |
|   -------------- ----------------------------------------------------- -------------- ---------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------+

Thus, the final []{#id928054 .indexterm}[line break](#line%20break%20character/) of a []{#id928068 .indexterm}[block scalar](#block%20scalar%20style/syntax) may be included or excluded from the []{#id928085 .indexterm}[content](#content/syntax), depending on the specified chomping indicator.

+---------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------- -------------- -------------------------------------------------------- -------------- |
|   \[168\]          []{#b-chomped-last(t)}b-chomped-last(t)     `::=`      `t`{.varname} = strip ⇒ [b-strip-last](#b-strip-last)\                  |
|                                                                           `t`{.varname} = clip  ⇒ [b-keep-last](#b-keep-last)\                    |
|                                                                           `t`{.varname} = keep  ⇒ [b-keep-last](#b-keep-last)                     |
|                                                                                                                                                   |
|   \[169\]                    []{#b-strip-last}b-strip-last     `::=`      [b-ignored-any](#b-ignored-any)                                         |
|                                                                                                                                                   |
|   \[170\]                      []{#b-keep-last}b-keep-last     `::=`      [b-normalized](#b-normalized)                                           |
|   -------------- ----------------------------------------- -------------- -------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id928179}

**Example 9.22. Chomping Final Line Break**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| strip: |-                         | %YAML 1.1                         |
|   text¶                           | ---                               |
| clip: |                           | !!map {                           |
|   text↓                           |   ? !!str "strip"                 |
| keep: |+                          |   : !!str "text",                 |
|   text⇓                           |   ? !!str "clip"                  |
| ```                               |   : !!str "text\n",               |
|                                   |   ? !!str "keep"                  |
| ``` synopsis                      |   : !!str "text\L",               |
| Legend:                           | }                                 |
|   b-strip-last                    | ```                               |
|   b-keep-last                     |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Similarly, []{#id928272 .indexterm}[empty lines](#empty%20line/) immediately following the []{#id928285 .indexterm}[block scalar](#block%20scalar%20style/syntax) may be interpreted either as []{#id928301 .indexterm}[presenting](#present/) trailing []{#id928314 .indexterm}[line breaks](#line%20break%20character/) or as (empty) []{#id928327 .indexterm}[comment](#comment/syntax) lines, depending on the specified chomping indicator.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
|   \[171\]          []{#l-chomped-empty(n,t)}l-chomped-empty(n,t)     `::=`      `t`{.varname} = strip ⇒ [l-strip-empty(n)](#l-strip-empty(n))\                                                                   |
|                                                                                 `t`{.varname} = clip  ⇒ [l-strip-empty(n)](#l-strip-empty(n))\                                                                   |
|                                                                                 `t`{.varname} = keep  ⇒ [l-keep-empty(n)](#l-keep-empty(n))                                                                      |
|                                                                                                                                                                                                                  |
|   \[172\]                  []{#l-strip-empty(n)}l-strip-empty(n)     `::=`      ( [s-indent(≤n)](#s-indent(n)) [b-ignored-any](#b-ignored-any) )\* [l-trail-comments(n)](#l-trail-comments(n))?                  |
|                                                                                                                                                                                                                  |
|   \[173\]                    []{#l-keep-empty(n)}l-keep-empty(n)     `::=`      [l-empty(n,literal)](#l-empty(n,s))\* [l-trail-comments(n)](#l-trail-comments(n))?                                               |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Explicit []{#id928445 .indexterm}[comment](#comment/syntax) lines may then follow. To prevent ambiguity, the first such []{#id928461 .indexterm}[comment](#comment/syntax) line must be less []{#id928476 .indexterm}[indented](#indentation%20space/) than the []{#id928490 .indexterm}[block scalar content](#block%20scalar%20style/syntax). Additional []{#id928506 .indexterm}[comment](#comment/syntax) lines, if any, are not so restricted.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- -------------------------------------------------------------------------------------------------------- -------------- |
|   \[174\]          []{#l-trail-comments(n)}l-trail-comments(n)     `::=`      [s-indent(\<n)](#s-indent(n)) [c-nb-comment-text](#c-nb-comment-text) [b-ignored-any](#b-ignored-any)\                  |
|                                                                               [l-comment](#l-comment)\*                                                                                               |
|                                                                                                                                                                                                       |
|   -------------- --------------------------------------------- -------------- -------------------------------------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id928558}

**Example 9.23. Block Scalar Chomping**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
|  # Strip                          | %YAML 1.1                         |
|   # Comments:                     | ---                               |
| strip: |-                         | !!seq [                           |
|   # text¶                         |   ? !!str "strip"                 |
| ··⇓                               |   : !!str "# text",               |
| ·# Clip                           |   ? !!str "clip"                  |
| ··# comments:                     |   : !!str "# text\n",             |
| ↓                                 |   ? !!str "keep"                  |
| clip: |                           |   : !!str "# text\L\n",           |
|   # text↓                         | ]                                 |
| ·¶                                | ```                               |
| ·# Keep                           |                                   |
| ··# comments:                     | ``` synopsis                      |
| ↓                                 | Legend:                           |
| keep: |+                          |   l-strip-empty(n)                |
|   # text⇓                         |   l-keep-empty(n)                 |
| ↓                                 |   l-trail-comments(n)             |
| ·# Trail                          | ```                               |
| ··# comments.                     |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Note that if a []{#id928685 .indexterm}[block scalar](#block%20scalar%20style/syntax) consists of only []{#id928701 .indexterm}[empty lines](#empty%20line/), then these lines are considered trailing lines and hence are affected by chomping.

::: example
[]{#id928716}

**Example 9.24. Empty Scalar Chomping**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| strip: >-                         | %YAML 1.1                         |
| ↓                                 | ---                               |
| clip: >                           | !!seq [                           |
| ↓                                 |   ? !!str "strip"                 |
| keep: |+                          |   : !!str "",                     |
| ↓                                 |   ? !!str "clip"                  |
| ```                               |   : !!str "",                     |
|                                   |   ? !!str "keep"                  |
| ``` synopsis                      |   : !!str "\n",                   |
| Legend:                           | ]                                 |
|   l-strip-empty(n)                | ```                               |
|   l-keep-empty(n)                 |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::
::::::::::
::::::::::::::::::::::::::

::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id928806}9.3. Block Scalar Styles {#block-scalar-styles .title style="clear: both"}
:::
::::
:::::

YAML provides two []{#id928815 .indexterm}[]{#block scalar style/syntax}*Block scalar styles*, []{#id928833 .indexterm}[literal](#literal%20style/syntax) and []{#id928849 .indexterm}[folded](#folded%20style/syntax). The block scalar []{#id928864 .indexterm}[content](#content/syntax) is ended by a less-[]{#id928879 .indexterm}[indented](#indentation%20space/) line or the end of the characters []{#id928893 .indexterm}[stream](#stream/syntax).

:::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id928909}9.3.1. Literal {#literal .title}
:::
::::
:::::

The []{#id928917 .indexterm}[]{#literal style/syntax}*literal style* is the simplest, most restricted and most readable []{#id928935 .indexterm}[scalar style](#scalar/syntax). It is especially suitable for source code or other text containing significant use of []{#id928952 .indexterm}[indicators](#indicator/), []{#id928964 .indexterm}[escape sequences](#escaping%20in%20double-quoted%20style/) and []{#id928979 .indexterm}[line breaks](#line%20break%20character/). In particular, literal content lines may begin with a []{#id928995 .indexterm}[tab](#tab/) or a []{#id929006 .indexterm}["[**`#`**]{.quote}"](##%20comment/) character.

+-------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------- -------------- ------------------------------------------------------------ -------------- |
|   \[175\]          []{#c-l+literal(n)}c-l+literal(n)     `::=`      [c-b-block-header(literal,m,t)](#c-b-block-header(s,m,t))\                  |
|                                                                     [l-literal-content(n+m,t)](#l-literal-content(n,t))                         |
|                                                                                                                                                 |
|   -------------- ----------------------------------- -------------- ------------------------------------------------------------ -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id929052}

**Example 9.25. Literal Scalar**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| | # Simple block scalar↓          | %YAML 1.1                         |
|  literal↓                         | ---                               |
|  →text↓                           | !!seq [                           |
| ```                               |   !!str "literal\n\               |
|                                   |         \ttext\n"                 |
| ``` synopsis                      | ]                                 |
| Legend:                           | ```                               |
|   c-b-block-header(s,m,t)         |                                   |
|   l-literal-content(n,t)          |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Inside literal scalars, each non-[]{#id929137 .indexterm}[empty line](#empty%20line/) may be preceded by any number of []{#id929152 .indexterm}[empty lines](#empty%20line/). No processing is performed on these lines except for stripping the []{#id929165 .indexterm}[indentation](#indentation%20space/). In particular, such lines are never []{#id929178 .indexterm}[folded](#line%20folding/). Literal non-[]{#id929193 .indexterm}[empty lines](#empty%20line/) may include only spaces, []{#id929206 .indexterm}[tabs](#tab/), and other []{#id929217 .indexterm}[printable](#printable%20character/) characters.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------------- ------- -------------------------------------------------------------------------------------- --- |
|   \[176\]     []{#l-nb-literal-text(n)}l-nb-literal-text(n)  `::=`  [l-empty(n,block)](#l-empty(n,s))\* [s-indent(n)](#s-indent(n)) [nb-char](#nb-char)+       |
|   --------- ----------------------------------------------- ------- -------------------------------------------------------------------------------------- --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id929264}

**Example 9.26. Literal Text**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| |                                 | %YAML 1.1                         |
| ·                                 | ---                               |
| ··                                | !!str "\nliteral\n\ntext\n"       |
| ··literal↓                        | ```                               |
| ·                                 |                                   |
| ··text↓                           | ``` synopsis                      |
| ↓                                 | Legend:                           |
| ·# Comment                        |   l-nb-literal-text(n)            |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

The []{#id929338 .indexterm}[line break](#line%20break%20character/) following a non-[]{#id929352 .indexterm}[empty](#empty%20line/) inner literal line is []{#id929365 .indexterm}[normalized](#line%20break%20normalization/). Again, such []{#id929381 .indexterm}[line breaks](#line%20break%20character/) are never []{#id929394 .indexterm}[folded](#line%20folding/).

+------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ---------------------------------------------- ------- ----------------------------------------------------------------------------- --- |
|   \[177\]     []{#l-nb-literal-inner(n)}l-literal-inner(n)  `::=`  [l-nb-literal-text(n)](#l-nb-literal-text(n)) [b-normalized](#b-normalized)       |
|   --------- ---------------------------------------------- ------- ----------------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id929431}

**Example 9.27. Inner Literal Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| |                                 | %YAML 1.1                         |
| ·                                 | ---                               |
| ··                                | !!str "\nliteral\n\ntext\n"       |
| ··literal↓                        | ```                               |
| ·                                 |                                   |
| ··text↓                           | ``` synopsis                      |
| ↓                                 | Legend:                           |
| ·# Comment                        |   l-nb-literal-inner(n)           |
| ```                               |   b-normalized                    |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

The []{#id929519 .indexterm}[line break](#line%20break%20character/) following the final non-[]{#id929533 .indexterm}[empty](#empty%20line/) literal line is subject to []{#id929548 .indexterm}[chomping](#chomping/).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------------ ------- --------------------------------------------------------------------------------------- --- |
|   \[178\]     []{#l-nb-literal-last(n,t)}l-literal-last(n,t)  `::=`  [l-nb-literal-text(n)](#l-nb-literal-text(n)) [b-chomped-last(t)](#b-chomped-last(t))       |
|   --------- ------------------------------------------------ ------- --------------------------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Trailing []{#id929587 .indexterm}[empty lines](#empty%20line/) following the last literal non-[]{#id929600 .indexterm}[empty line](#empty%20line/), if any, are also subject to []{#id929613 .indexterm}[chomping](#chomping/).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------- -------------- |
|   \[179\]          []{#l-literal-content(n,t)}l-literal-content(n,t)     `::=`      ( [l-literal-inner(n)](#l-nb-literal-inner(n))\* [l-literal-last(n,t)](#l-nb-literal-last(n,t)) )?\                  |
|                                                                                     [l-chomped-empty(n,t)](#l-chomped-empty(n,t))?                                                                       |
|                                                                                                                                                                                                          |
|   -------------- --------------------------------------------------- -------------- ----------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id929660}

**Example 9.28. Last Literal Line**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| |                                 | %YAML 1.1                         |
| ·                                 | ---                               |
| ··                                | !!str "\nliteral\n\ntext\n"       |
| ··literal↓                        | ```                               |
| ·                                 |                                   |
| ··text↓                           | ``` synopsis                      |
| ↓                                 | Legend:                           |
| ·# Comment                        |   l-nb-literal-last(n,t)          |
| ```                               |   b-chomped-last(t)               |
|                                   |   l-chomped-empty(n,t)            |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
::::::::::

::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id929764}9.3.2. Folded {#folded .title}
:::
::::
:::::

The []{#id929773 .indexterm}[]{#folded style/syntax}*folded style* is similar to the []{#id929790 .indexterm}[literal style](#literal%20style/syntax). However, unlike []{#id929806 .indexterm}[literal content](#literal%20style/syntax), folded content is subject to (block) []{#id929822 .indexterm}[line folding](#line%20folding/).

+----------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------- -------------- ----------------------------------------------------------- -------------- |
|   \[180\]          []{#c-l+folded(n)}c-l+folded(n)     `::=`      [c-b-block-header(folded,m,t)](#c-b-block-header(s,m,t))\                  |
|                                                                   [l-folded-content(n+m,t)](#l-folded-content(n,t))                          |
|                                                                                                                                              |
|   -------------- --------------------------------- -------------- ----------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id929864}

**Example 9.29. Folded Scalar**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| > # Simple folded scalar↓         | %YAML 1.1                         |
|  folded↓                          | ---                               |
|  text↓                            | !!seq [                           |
|  →lines↓                          |   !!str "folded text\n\           |
| ```                               |         \tlines\n"                |
|                                   | ]                                 |
| ``` synopsis                      | ```                               |
| Legend:                           |                                   |
|   c-b-block-header(s,m,t)         |                                   |
|   l-folded-content(n,t)           |                                   |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

[]{#id929948 .indexterm}[Line folding](#line%20folding/) allows long []{#id929961 .indexterm}[content](#content/syntax) lines to be broken anywhere a single space character separates two non-space characters.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------- -------------- |
|   \[181\]            []{#s-nb-folded-text(n)}s-nb-folded-line(n)     `::=`      [s-indent(n)](#s-indent(n)) [ns-char](#ns-char) [nb-char](#nb-char)\*                  |
|                                                                                                                                                                        |
|   \[182\]          []{#l-nb-folded-lines(n)}l-nb-folded-lines(n)     `::=`      ( [s-nb-folded-line(n)](#s-nb-folded-text(n))\                                         |
|                                                                                   [b-l-folded-any(n,folded)](#b-l-folded-any(n,s)) )\*\                                |
|                                                                                 [s-nb-folded-line(n)](#s-nb-folded-text(n))                                            |
|   -------------- ----------------------------------------------- -------------- ----------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id930040}

**Example 9.30. Folded Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| >                                 | %YAML 1.1                         |
| ·folded↓                          | ---                               |
| ·line↓                            | !!seq [                           |
| ↓                                 |   !!str "folded line\n\           |
| ·next                             |         next line\n\              |
| ·line↓                            |         \  * bullet\n\            |
|                                   |         \  * list\n\              |
|    * bullet                       |         last line\n"              |
|    * list                         | ]                                 |
|                                   | ```                               |
| ·last↓                            |                                   |
| ·line↓                            | ``` synopsis                      |
|                                   | Legend:                           |
| # Comment                         |   l-nb-folded-lines(n)            |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

Lines starting with []{#id930118 .indexterm}[white space](#white%20space/) characters ([]{#id930131 .indexterm}[]{#more indented line/}*"[more indented]{.quote}" lines*) are not []{#id930150 .indexterm}[folded](#line%20folding/). Note that folded scalars, like []{#id930161 .indexterm}[literal scalars](#literal%20style/syntax), may contain []{#id930177 .indexterm}[tab](#tab/) characters. However, any such characters must be properly []{#id930190 .indexterm}[indented](#indentation%20space/) using only space characters.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- ------------------------------------------------------------------------------------ -------------- |
|   \[183\]                        []{#b-l-spaced(n)}b-l-spaced(n)     `::=`      [b-normalized](#b-normalized) [l-empty(n,folded)](#l-empty(n,s))\*                                  |
|                                                                                                                                                                                     |
|   \[184\]            []{#s-nb-spaced-text(n)}s-nb-spaced-text(n)     `::=`      [s-indent(n)](#s-indent(n)) [s-white](#s-white) [nb-char](#nb-char)\*                               |
|                                                                                                                                                                                     |
|   \[185\]          []{#l-nb-spaced-lines(n)}l-nb-spaced-lines(n)     `::=`      ( [s-nb-spaced-text(n)](#s-nb-spaced-text(n)) [b-l-spaced(n)](#b-l-spaced(n)) )\*\                  |
|                                                                                 [s-nb-spaced-text(n)](#s-nb-spaced-text(n))                                                         |
|   -------------- ----------------------------------------------- -------------- ------------------------------------------------------------------------------------ -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id930287}

**Example 9.31. Spaced Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| >                                 | %YAML 1.1                         |
|  folded                           | ---                               |
|  line                             | !!seq [                           |
|                                   |   !!str "folded line\n\           |
|  next                             |         next line\n\              |
|  line                             |         \  * bullet\n\            |
|                                   |         \  * list\n\              |
| ···* bullet↓                      |         last line\n"              |
| ···* list↓                        | ]                                 |
|                                   | ```                               |
|  last                             |                                   |
|  line                             | ``` synopsis                      |
|                                   | Legend:                           |
| # Comment                         |   l-nb-spaced-lines(n)            |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

Folded content may start with either line type. If the []{#id930358 .indexterm}[content](#content/syntax) begins with a "[more indented]{.quote}" line (starting with spaces), an []{#id930378 .indexterm}[indentation indicator](#indentation%20indicator/) must be specified in the block header. Note that leading []{#id930394 .indexterm}[empty lines](#empty%20line/) and []{#id930407 .indexterm}[empty lines](#empty%20line/) separating lines of a different type are never []{#id930420 .indexterm}[folded](#line%20folding/).

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------------- -------------- -------------------------------------------------------------------------------------------- -------------- |
|   \[186\]          []{#l-nb-start-with-folded(n)}l-nb-start-with-folded(n)     `::=`      [l-empty(n,block)](#l-empty(n,s))\* [l-nb-folded-lines(n)](#l-nb-folded-lines(n))\                          |
|                                                                                           ( [b-normalized](#b-normalized) [l-nb-start-with-spaced(n)](#l-nb-start-with-spaced(n)) )?                  |
|                                                                                                                                                                                                       |
|   \[187\]          []{#l-nb-start-with-spaced(n)}l-nb-start-with-spaced(n)     `::=`      [l-empty(n,block)](#l-empty(n,s))\* [l-nb-spaced-lines(n)](#l-nb-spaced-lines(n))\                          |
|                                                                                           ( [b-normalized](#b-normalized) [l-nb-start-with-folded(n)](#l-nb-start-with-folded(n)) )?                  |
|                                                                                                                                                                                                       |
|   \[188\]                []{#l-nb-start-with-any(n)}l-nb-start-with-any(n)     `::=`        [l-nb-start-with-folded(n)](#l-nb-start-with-folded(n))\                                                  |
|                                                                                           \| [l-nb-start-with-spaced(n)](#l-nb-start-with-spaced(n))                                                  |
|   -------------- --------------------------------------------------------- -------------- -------------------------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id930530}

**Example 9.32. Empty Separation Lines**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| >                                 | %YAML 1.1                         |
|  folded                           | ---                               |
|  line                             | !!seq [                           |
|                                   |   !!str "folded line\n\           |
|  next                             |         next line\n\              |
|  line↓                            |         \  * bullet\n\            |
| ↓                                 |         \  * list\n\              |
|    * bullet                       |         last line\n"              |
|    * list↓                        | ]                                 |
| ↓                                 | ```                               |
|  last                             |                                   |
|  line                             | ``` synopsis                      |
|                                   | Legend:                           |
| # Comment                         |   b-normalized l-empty(n,s)       |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

The final []{#id930626 .indexterm}[line break](#line%20break%20character/), and trailing []{#id930640 .indexterm}[empty lines](#empty%20line/), if any, are subject to []{#id930653 .indexterm}[chomping](#chomping/) and are never []{#id930666 .indexterm}[folded](#line%20folding/).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
|   \[189\]          []{#l-folded-content(n,t)}l-folded-content(n,t)     `::=`      ( [l-nb-start-with-any(n)](#l-nb-start-with-any(n)) [b-chomped-last(t)](#b-chomped-last(t)) )?\                  |
|                                                                                   [l-chomped-empty(n,t)](#l-chomped-empty(n,t))                                                                    |
|                                                                                                                                                                                                    |
|   -------------- ------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id930711}

**Example 9.33. Final Empty Lines**

+-----------------------------------+------------------------------------------+
| ``` programlisting                | ``` programlisting                       |
| >                                 | %YAML 1.1                                |
|  folded                           | ---                                      |
|  line                             | !!seq [                                  |
|                                   |   !!str "folded line\n\                  |
|  next                             |         next line\n\                     |
|  line                             |         \  * bullet\n\                   |
|                                   |         \  * list\n\                     |
|    * bullet                       |         last line\n"                     |
|    * list                         | ]                                        |
|                                   | ```                                      |
|  last                             |                                          |
|  line↓                            | ``` synopsis                             |
| ↓                                 | Legend:                                  |
| # Comment                         |   b-chomped-last(t) l-chomped-empty(n,t) |
| ```                               | ```                                      |
+-----------------------------------+------------------------------------------+
:::
:::::::::::
:::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::

::::::::::::::::::::::::::::::::::::::::::::::::::::::::: {.chapter lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id930798}Chapter 10. Collection Styles {#chapter-10.-collection-styles .title}
:::
::::
:::::

[]{#id930806 .indexterm}[]{#collection/syntax}*Collection content* can be presented in a single []{#id930824 .indexterm}[]{#flow collection style/syntax}*flow style* and a single []{#id930844 .indexterm}[]{#block collection style/syntax}*block style* for each of the two []{#id930860 .indexterm}[collection kinds](#kind/) ([]{#id930872 .indexterm}[sequence](#sequence/syntax) and []{#id930888 .indexterm}[mapping](#mapping/syntax)). In addition, YAML provides several []{#id930905 .indexterm}[in-line](#in-line%20style/syntax) compact syntax forms for improved readability of common special cases. In all cases, the collection style is a []{#id930923 .indexterm}[presentation detail](#presentation%20detail/) and must not be used to convey []{#id930938 .indexterm}[content information](#content/information%20model).

A flow collection may be nested within a block collection ([]{#id930957 .indexterm}[flow-out context](#flow-out%20context/)), nested within another flow collection ([]{#id930971 .indexterm}[flow-in context](#flow-in%20context/)), or be a part of a []{#id930985 .indexterm}[simple key](#simple%20key/) ([]{#id930999 .indexterm}[flow-key context](#flow-key%20context/)). Flow collection entries are separated by the []{#id931013 .indexterm}[]{#, end flow entry/}*"[**`,`**]{.quote}" indicator*. The final "[**`,`**]{.quote}" may be omitted. This does not cause ambiguity because flow collection entries can never be []{#id931040 .indexterm}[completely empty](#completely%20empty%20node/).

+------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------- -------------- ------------------------------------- -------------- |
|   \[190\]          []{#in-flow(c)}in-flow(c)     `::=`      `c`{.varname} = flow-out ⇒ flow-in\                  |
|                                                             `c`{.varname} = flow-in  ⇒ flow-in\                  |
|                                                             `c`{.varname} = flow-key ⇒ flow-key                  |
|                                                                                                                  |
|   -------------- --------------------------- -------------- ------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------+

::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id931088}10.1. Sequence Styles {#sequence-styles .title style="clear: both"}
:::
::::
:::::

[]{#id931095 .indexterm}[]{#sequence/syntax}*Sequence content* is an ordered collection of sub-[]{#id931112 .indexterm}[nodes](#node/syntax). []{#id931127 .indexterm}[Comments](#comment/syntax) may be interleaved between the sub-[]{#id931142 .indexterm}[nodes](#node/syntax). Sequences may be []{#id931157 .indexterm}[presented](#present/) in a []{#id931170 .indexterm}[flow style](#flow%20sequence%20style/syntax) or a []{#id931188 .indexterm}[block style](#block%20sequence%20style/syntax). YAML provides compact notations for []{#id931202 .indexterm}[in-line](#in-line%20style/syntax) nesting of a []{#id931218 .indexterm}[collection](#collection/syntax) in a []{#id931234 .indexterm}[block sequence](#block%20sequence%20style/syntax) and for nesting a []{#id931250 .indexterm}[single pair mapping](#single%20pair%20style/syntax) in a []{#id931269 .indexterm}[flow sequence](#flow%20sequence%20style/syntax).

:::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id931285}10.1.1. Flow Sequences {#flow-sequences .title}
:::
::::
:::::

[]{#id931292 .indexterm}[]{#flow sequence style/syntax}*Flow sequence content* is denoted by surrounding []{#id931310 .indexterm}[]{#[ start flow sequence/}*"[**`[`**]{.quote}"* and []{#id931329 .indexterm}[]{#] end flow sequence/}*"[**`]`**]{.quote}"* characters.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- --------------------------------------------------------------------------- -------------- |
|   \[191\]          []{#c-flow-sequence(n,c)}c-flow-sequence(n,c)     `::=`      ["[\[]{.quote}"](#c-sequence-start) [s-separate(n,c)](#s-separate(n,c))?\                  |
|                                                                                 [ns-s-flow-seq-inner(n,c)](#ns-s-flow-seq-inner(n,c))\*\                                   |
|                                                                                 [ns-s-flow-seq-last(n,c)](#ns-s-flow-seq-last(n,c))?\                                      |
|                                                                                 ["[\]]{.quote}"](#c-sequence-end)                                                          |
|                                                                                                                                                                            |
|   -------------- ----------------------------------------------- -------------- --------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Sequence entries are separated by a []{#id931402 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) character.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------ --- |
|   \[192\]     []{#ns-s-flow-seq-inner(n,c)}ns-s-flow-seq-inner(n,c)  `::=`  [ns-s-flow-seq-entry(n,c)](#ns-s-flow-seq-entry(n,c)) ["[,]{.quote}"](#c-collect-entry) [s-separate(n,c)](#s-separate(n,c))?       |
|   --------- ------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------ --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

The final entry may omit the []{#id931458 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) character. This does not cause ambiguity since sequence entries must not be []{#id931477 .indexterm}[completely empty](#completely%20empty%20node/).

+---------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------------------- ------- ------------------------------------------------------- --- |
|   \[193\]     []{#ns-s-flow-seq-last(n,c)}ns-s-flow-seq-last(n,c)  `::=`  [ns-s-flow-seq-entry(n,c)](#ns-s-flow-seq-entry(n,c))       |
|   --------- ----------------------------------------------------- ------- ------------------------------------------------------- --- |
+---------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id931509}

**Example 10.1. Flow Sequence**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - [ inner, inner, ]               | %YAML 1.1                         |
| - [inner,last]                    | ---                               |
| ```                               | !!seq [                           |
|                                   |   !!seq [                         |
| ``` synopsis                      |     !!str "inner",                |
| Legend:                           |     !!str "inner",                |
|   c-sequence-start c-sequence-end |   ],                              |
|   ns-s-flow-seq-inner(n,c)        |   !!seq [                         |
|   ns-s-flow-seq-last(n,c)         |     !!str "inner",                |
| ```                               |     !!str "last",                 |
|                                   |   ],                              |
|                                   | ]                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

Any []{#id931644 .indexterm}[flow node](#flow%20style/syntax) may be used as a flow sequence entry. In addition, YAML provides a compact form for the case where a flow sequence entry is a []{#id931662 .indexterm}[mapping](#mapping/syntax) with a []{#id931677 .indexterm}[single key: value pair](#single%20pair%20style/syntax), and neither the []{#id931694 .indexterm}[mapping node](#mapping/syntax) nor its single []{#id931709 .indexterm}[key node](#key/syntax) have any []{#id931724 .indexterm}[properties](#node%20property/) specified.

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
|   \[194\]          []{#ns-s-flow-seq-entry(n,c)}ns-s-flow-seq-entry(n,c)     `::=`        ( [ns-flow-node(n,](#ns-flow-node(n,c))[in-flow(c)](#in-flow(c)))\                                  |
|                                                                                             [s-separate(n,](#s-separate(n,c))[in-flow(c)](#in-flow(c)))? )\                                   |
|                                                                                         \| [ns-s-flow-single-pair(n,](#ns-s-flow-single-pair(n,c))[in-flow(c)](#in-flow(c)))                  |
|                                                                                                                                                                                               |
|   -------------- ------------------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id931790}

**Example 10.2. Flow Sequence Entries**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| [                                 | %YAML 1.1                         |
| "double                           | ---                               |
|  quoted", 'single                 | !!seq [                           |
|            quoted',               |   !!str "double quoted",          |
| plain                             |   !!str "single quoted",          |
|  text, [ nested ],                |   !!str "plain text",             |
| single: pair ,                    |   !!seq [                         |
| ]                                 |     !!str "nested",               |
| ```                               |   ],                              |
|                                   |   !!map {                         |
| ``` synopsis                      |     ? !!str "single"              |
| Legend:                           |     : !!str "pair"                |
|   ns-flow-node(n,c)               |   }                               |
|   ns-s-flow-single-pair(n,c)      | ]                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
::::::::

::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id931893}10.1.2. Block Sequences {#block-sequences .title}
:::
::::
:::::

A []{#id931901 .indexterm}[]{#block sequence style/syntax}*block sequence* is simply a series of entries, each []{#id931920 .indexterm}[presenting](#present/) a single []{#id931932 .indexterm}[node](#node/syntax).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------------------- ------- ---------------------------------------------------------------------------------- --- |
|   \[195\]     []{#c-l-block-sequence(n,c)}c-l-block-sequence(n,c)  `::=`  [c-l-comments](#c-l-comments) [l-block-seq-entry(n,c)](#l-block-seq-entry(n,c))+       |
|   --------- ----------------------------------------------------- ------- ---------------------------------------------------------------------------------- --- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id931973}

**Example 10.3. Block Sequence**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| block: # Block                    | %YAML 1.1                         |
|        # sequence↓                | ---                               |
| - one↓                            | !!map {                           |
| - two : three↓                    |   ? !!str "block"                 |
| ```                               |   : !!seq [                       |
|                                   |     !!str "one",                  |
| ``` synopsis                      |     !!map {                       |
| Legend:                           |       ? !!str "two"               |
|   c-l-comments                    |       : !!str "three"             |
|   l-block-seq-entry(n,c)          |     }                             |
| ```                               |   ]                               |
|                                   | }                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

Each block sequence entry is denoted by a leading []{#id932068 .indexterm}[]{#- block sequence entry/}*"[**`-`**]{.quote}" indicator*, []{#id932088 .indexterm}[separated](#separation%20space/) by spaces from the entry []{#id932104 .indexterm}[node](#node/syntax).

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------- -------------- ------------------------------------------------------------------------------- -------------- |
|   \[196\]          []{#l-block-seq-entry(n,c)}l-block-seq-entry(n,c)     `::=`      [s-indent(seq-spaces(n,c))](#s-indent(n)) ["[-]{.quote}"](#c-sequence-entry)\                  |
|                                                                                     [s-l+block-indented(seq-spaces(n,c),c)](#s-l+block-indented(n,c))                              |
|                                                                                                                                                                                    |
|   -------------- --------------------------------------------------- -------------- ------------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

People read the "[**`-`**]{.quote}" character as part of the []{#id932163 .indexterm}[indentation](#indentation%20space/). Hence, block sequence entries require one less space of []{#id932178 .indexterm}[indentation](#indentation%20space/), unless the block sequence is nested within another block sequence (hence the need for the []{#id932193 .indexterm}[]{#block-in context/}*block-in context* and []{#id932208 .indexterm}[]{#block-out context/}*block-out context*).

+-------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------- -------------- ---------------------------------- -------------- |
|   \[197\]          []{#seq-spaces(n,c)}seq-spaces(n,c)     `::=`      `c`{.varname} = block-out ⇒ n-1\                  |
|                                                                       `c`{.varname} = block-in  ⇒ n                     |
|                                                                                                                         |
|   -------------- ------------------------------------- -------------- ---------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id932251}

**Example 10.4. Block Sequence Entry Indentation**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| block:                            | %YAML 1.1                         |
| - one                             | ---                               |
| -                                 | !!map {                           |
| ·- two                            |   ? !!str "block"                 |
| ```                               |   : !!seq [                       |
|                                   |     !!str "one",                  |
| ``` synopsis                      |     !!seq [                       |
| Legend:                           |       !!str "two"                 |
|   s-indent(n)                     |     ]                             |
|   s-l+block-indented(n,c)         |   ]                               |
| ```                               | }                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

The entry []{#id932346 .indexterm}[node](#node/syntax) may be either []{#id932361 .indexterm}[completely empty](#completely%20empty%20node/), a normal []{#id932374 .indexterm}[block node](#block%20style/syntax), or use a compact in-line form.

+---------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------- -------------- -------------------------------------------------- -------------- |
|   \[198\]          []{#s-l+block-indented(n,c)}s-l+block-indented(n,c)     `::=`        [s-l-empty-block](#s-l-empty-block)\                            |
|                                                                                       \| [s-l+block-node(n,c)](#s-l+block-node(n,c))\                   |
|                                                                                       \| [s-l+block-in-line(n)](#s-l+block-in-line(n))                  |
|                                                                                                                                                         |
|   -------------- ----------------------------------------------------- -------------- -------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------+

The compact []{#id932428 .indexterm}[]{#in-line style/syntax}*in-line* form may be used in the common case when the block sequence entry is itself a []{#id932446 .indexterm}[block collection](#block%20collection%20style/syntax), and neither the []{#id932462 .indexterm}[collection](#collection/syntax) entry nor its first nested []{#id932479 .indexterm}[node](#node/syntax) have any []{#id932493 .indexterm}[properties](#node%20property/) specified. In this case, the nested []{#id932507 .indexterm}[collection](#collection/syntax) may be specified in the same line as the "[**`-`**]{.quote}" character, and any following spaces are considered part of the in-line nested []{#id932532 .indexterm}[collection's](#collection/syntax) []{#id932547 .indexterm}[indentation](#indentation%20space/).

+---------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------- -------------- |
|   \[199\]          []{#s-l+block-in-line(n)}s-l+block-in-line(n)     `::=`      [s-indent(m\>0)](#s-indent(n))\                                               |
|                                                                                 ( [ns-l-in-line-sequence(n+1+m)](#ns-l-in-line-sequence(n))\                  |
|                                                                                 \| [ns-l-in-line-mapping(n+1+m)](#ns-l-in-line-mapping(n)) )                  |
|                                                                                                                                                               |
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------+

An []{#id932601 .indexterm}[]{#in-line sequence style/}*in-line block sequence* begins with an []{#id932617 .indexterm}[indented](#indentation%20space/) same-line sequence entry, followed by optional additional normal block sequence entries, properly []{#id932634 .indexterm}[indented](#indentation%20space/).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
|   \[200\]          []{#ns-l-in-line-sequence(n)}ns-l-in-line-sequence(n)     `::=`      ["[-]{.quote}"](#c-sequence-entry) [s-l+block-indented(n,block-out)](#s-l+block-indented(n,c))\                  |
|                                                                                         [l-block-seq-entry(n,block-out)](#l-block-seq-entry(n,c))\*                                                      |
|                                                                                                                                                                                                          |
|   -------------- ------------------------------------------------------- -------------- ------------------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id932683}

**Example 10.5. Block Sequence Entry Types**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - # Empty                         | %YAML 1.1                         |
| - |                               | ---                               |
|  block node                       | !!seq [                           |
| -·- one # in-line                 |   !!str "",                       |
| ··- two # sequence                |   !!str "block node\n",           |
| - one: two # in-line              |   !!seq [                         |
|            # mapping              |     !!str "one",                  |
| ```                               |     !!str "two",                  |
|                                   |   ]                               |
| ``` synopsis                      |   !!map {                         |
| Legend:                           |     ? !!str "one"                 |
|   s-l-empty-block                 |     : !!str "two",                |
|   s-l+block-node(n,c)             |   }                               |
|   s-l+block-in-line(n)            | ]                                 |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::
:::::::::::::::::::

:::::::::::::::::::::::::::::::::::: {.sect1 lang="en"}
::::: titlepage
:::: {}
::: {}
## []{#id932806}10.2. Mapping Styles {#mapping-styles .title style="clear: both"}
:::
::::
:::::

A []{#id932814 .indexterm}[]{#mapping/syntax}*mapping node* is an unordered collection of []{#id932831 .indexterm}[]{#key/syntax}*key:* []{#id932847 .indexterm}[]{#value/syntax}*value* pairs. Of necessity, these pairs are []{#id932863 .indexterm}[presented](#present/) in some []{#id932876 .indexterm}[order](#key%20order/) in the characters []{#id932889 .indexterm}[stream](#stream/syntax). As a []{#id932904 .indexterm}[serialization detail](#serialization%20detail/), this []{#id932917 .indexterm}[key order](#key%20order/) is preserved in the []{#id932930 .indexterm}[serialization tree](#serialization/). However it is not reflected in the []{#id932944 .indexterm}[representation graph](#representation/) and hence must not be used when []{#id932957 .indexterm}[constructing](#construct/) native data structures. It is an error for two []{#id932970 .indexterm}[equal](#equality/) keys to appear in the same mapping node. In such a case the YAML []{#id932984 .indexterm}[processor](#processor/) may continue, ignoring the second key: value pair and issuing an appropriate warning. This strategy preserves a consistent information model for one-pass and random access []{#id932998 .indexterm}[applications](#application/).

::::::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id933010}10.2.1. Flow Mappings {#flow-mappings .title}
:::
::::
:::::

[]{#id933017 .indexterm}[]{#flow mapping style/syntax}*Flow mapping content* is denoted by surrounding []{#id933036 .indexterm}[]{#{ start flow mapping/}*"[**`{`**]{.quote}"* and []{#id933054 .indexterm}[]{#} end flow mapping/}*"[**`}`**]{.quote}"* characters.

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------- -------------- ------------------------------------------------------------------------- -------------- |
|   \[201\]          []{#c-flow-mapping(n,c)}c-flow-mapping(n,c)     `::=`      ["[{]{.quote}"](#c-mapping-start) [s-separate(n,c)](#s-separate(n,c))?\                  |
|                                                                               [ns-s-flow-map-inner(n,c)](#ns-s-flow-map-inner(n,c))\*\                                 |
|                                                                               [ns-s-flow-map-last(n,c)](#ns-s-flow-map-last(n,c))?\                                    |
|                                                                               ["[}]{.quote}"](#c-mapping-end)                                                          |
|                                                                                                                                                                        |
|   -------------- --------------------------------------------- -------------- ------------------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Mapping entries are separated by a []{#id933127 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) character.

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------ --- |
|   \[202\]     []{#ns-s-flow-map-inner(n,c)}ns-s-flow-map-inner(n,c)  `::=`  [ns-s-flow-map-entry(n,c)](#ns-s-flow-map-entry(n,c)) ["[,]{.quote}"](#c-collect-entry) [s-separate(n,c)](#s-separate(n,c))?       |
|   --------- ------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------------------------------------ --- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

The final entry may omit the []{#id933183 .indexterm}["[**`,`**]{.quote}"](#,%20end%20flow%20entry/) character. This does not cause ambiguity since mapping entries must not be []{#id933202 .indexterm}[completely empty](#completely%20empty%20node/).

+---------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ----------------------------------------------------- ------- ------------------------------------------------------- --- |
|   \[203\]     []{#ns-s-flow-map-last(n,c)}ns-s-flow-map-last(n,c)  `::=`  [ns-s-flow-map-entry(n,c)](#ns-s-flow-map-entry(n,c))       |
|   --------- ----------------------------------------------------- ------- ------------------------------------------------------- --- |
+---------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id933234}

**Example 10.6. Flow Mappings**

+-------------------------------------+-----------------------------------+
| ``` programlisting                  | ``` programlisting                |
| - { inner : entry , also: inner , } | %YAML 1.1                         |
| - {inner: entry,last : entry}       | ---                               |
| ```                                 | !!seq [                           |
|                                     |   !!map {                         |
| ``` synopsis                        |     ? !!str "inner"               |
| Legend:                             |     : !!str "entry",              |
|   c-mapping-start c-mapping-end     |     ? !!str "also"                |
|   ns-s-flow-map-inner(n,c)          |     : !!str "inner"               |
|   ns-s-flow-map-last(n,c)           |   },                              |
| ```                                 |   !!map {                         |
|                                     |     ? !!str "inner"               |
|                                     |     : !!str "entry",              |
|                                     |     ? !!str "last"                |
|                                     |     : !!str "entry"               |
|                                     |   }                               |
|                                     | ]                                 |
|                                     | ```                               |
+-------------------------------------+-----------------------------------+
:::

Flow mappings allow two forms of keys: explicit and simple.

::: variablelist

[Explicit Keys]{.term}
:   An []{#id933382 .indexterm}[]{#explicit key/}*explicit key* is denoted by the []{#id933396 .indexterm}[]{#? mapping key/}*"[**`?`**]{.quote}" indicator*, followed by []{#id933417 .indexterm}[separation](#separation%20space/) spaces.
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------------- -------------- ---------------------------------------------------------------------------------------------------------- -------------- |
|   \[204\]                    []{#s-flow-separated(n,c)}s-flow-separated(n,c)     `::=`        ( [s-separate(n,c)](#s-separate(n,c)) [ns-flow-node(n,](#ns-flow-node(n,c))[in-flow(c)](#in-flow(c)))\                  |
|                                                                                                 [s-separate(n,c)](#s-separate(n,c))? )\                                                                               |
|                                                                                             \| ( [e-empty-flow](#e-empty-flow) [s-separate(n,c)](#s-separate(n,c)) )                                                  |
|                                                                                                                                                                                                                       |
|   \[205\]          []{#c-s-flow-explicit-key(n,c)}c-s-flow-explicit-key(n,c)     `::=`      ["[?]{.quote}"](#c-mapping-key) [s-flow-separated(n,c)](#s-flow-separated(n,c))                                           |
|   -------------- ----------------------------------------------------------- -------------- ---------------------------------------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Simple Keys]{.term}
:   A []{#id933516 .indexterm}[]{#simple key/}*simple key* has no identifying mark. It is recognized as being a key either due to being inside a flow mapping, or by being followed by an explicit value. Hence, to avoid unbound lookahead in YAML []{#id933534 .indexterm}[processors](#processor/), simple keys are restricted to a single line and must not span more than 1024 []{#id933548 .indexterm}[stream](#stream/syntax) characters (hence the need for the []{#id933564 .indexterm}[]{#flow-key context/}*flow-key context*). Note the 1024 character limit is in terms of Unicode characters rather than stream octets, and that it includes the []{#id933583 .indexterm}[separation](#separation%20space/) following the key itself.
:::

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------- --- |
|   \[206\]     []{#ns-s-flow-simple-key(n,c)}ns-s-flow-simple-key(n,c)  `::=`  [ns-flow-node(n,flow-key)](#ns-flow-node(n,c)) [s-flow-separated(n,c)](#s-flow-separated(n,c))?       |
|   --------- --------------------------------------------------------- ------- ------------------------------------------------------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id933629}

**Example 10.7. Flow Mapping Keys**

+------------------------------------+-----------------------------------+
| ``` programlisting                 | ``` programlisting                |
| {                                  | %YAML 1.1                         |
| ?° : value # Empty key             | ---                               |
| ? explicit                         | !!map {                           |
|  key: value,                       |   ? !!str ""                      |
| simple key : value                 |   : !!str "value",                |
| [ collection, simple, key ]: value |   ? !!str "explicit key"          |
| }                                  |   : !!str "value",                |
| ```                                |   ? !!str "simple key"            |
|                                    |   : !!str "value",                |
| ``` synopsis                       |   ? !!seq [                       |
| Legend:                            |     !!str "collection",           |
|   c-s-flow-explicit-key(n,c)       |     !!str "simple",               |
|   ns-s-flow-simple-key(n,c)        |     !!str "key"                   |
| ```                                |   ]                               |
|                                    |   : !!str "value"                 |
|                                    | }                                 |
|                                    | ```                               |
+------------------------------------+-----------------------------------+
:::

::: example
[]{#id933726}

**Example 10.8. Invalid Flow Mapping Keys**

+-----------------------------------+-----------------------------------+
| ``` screen                        | ``` screen                        |
| {                                 | ERROR:                            |
| multi-line                        | - A simple key is restricted      |
|  simple key : value,              |   to only one line.               |
| very long ...(>1KB)... key: value | - A simple key must not be        |
| }                                 |   longer than 1024 characters.    |
| ```                               | ```                               |
+-----------------------------------+-----------------------------------+
:::

Flow mappings also allow two forms of values, explicit and []{#id933797 .indexterm}[completely empty](#completely%20empty%20node/).

::: variablelist

[Explicit Values]{.term}
:   An []{#id933822 .indexterm}[]{#explicit value/}*explicit value* is denoted by the []{#id933837 .indexterm}[]{#: mapping value/}*"[**`:`**]{.quote}" indicator*, followed by []{#id933859 .indexterm}[separation](#separation%20space/) spaces.
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------------------------------- ------- ----------------------------------------------------------------------------------- --- |
|   \[207\]     []{#c-s-flow-explicit-value(n,c)}c-s-flow-explicit-value(n,c)  `::=`  ["[:]{.quote}"](#c-mapping-value) [s-flow-separated(n,c)](#s-flow-separated(n,c))       |
|   --------- --------------------------------------------------------------- ------- ----------------------------------------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id933901}

**Example 10.9. Flow Mapping Values**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| {                                 | %YAML 1.1                         |
| key : value,                      | ---                               |
| empty:° # empty value↓            | !!map {                           |
| }                                 |   ? !!str "key"                   |
| ```                               |   : !!str "value",                |
|                                   |   ? !!str "empty"                 |
| ``` synopsis                      |   : !!str "",                     |
| Legend:                           | }                                 |
|   c-s-flow-explicit-value(n,c)    | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

Thus, there are four possible combinations for a flow mapping entry:

::: itemizedlist
- Explicit key and explicit value:
:::

+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------------------------- -------------- --------------------------------------------------------------- -------------- |
|   \[208\]          []{#c-s-flow-explicit-explicit(n,c)}c-s-flow-explicit-explicit(n,c)     `::=`      [c-s-flow-explicit-key(n,c)](#c-s-flow-explicit-key(n,c))\                     |
|                                                                                                       [c-s-flow-explicit-value(n,c)](#c-s-flow-explicit-value(n,c))                  |
|                                                                                                                                                                                      |
|   -------------- --------------------------------------------------------------------- -------------- --------------------------------------------------------------- -------------- |
+--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Explicit key and []{#id934018 .indexterm}[completely empty](#completely%20empty%20node/) value:
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- --------------------------------------------------------------- ------- ----------------------------------------------------------------------------------------- --- |
|   \[209\]     []{#c-s-flow-explicit-empty(n,c)}c-s-flow-explicit-empty(n,c)  `::=`  [c-s-flow-explicit-key(n,c)](#c-s-flow-explicit-key(n,c)) [e-empty-flow](#e-empty-flow)       |
|   --------- --------------------------------------------------------------- ------- ----------------------------------------------------------------------------------------- --- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Simple key and explicit value:
:::

+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------------------------- -------------- --------------------------------------------------------------- -------------- |
|   \[210\]          []{#ns-s-flow-simple-explicit(n,c)}ns-s-flow-simple-explicit(n,c)     `::=`      [ns-s-flow-simple-key(n,c)](#ns-s-flow-simple-key(n,c))\                       |
|                                                                                                     [c-s-flow-explicit-value(n,c)](#c-s-flow-explicit-value(n,c))                  |
|                                                                                                                                                                                    |
|   -------------- ------------------------------------------------------------------- -------------- --------------------------------------------------------------- -------------- |
+------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- Simple key and []{#id934099 .indexterm}[completely empty](#completely%20empty%20node/) value:
:::

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   --------- ------------------------------------------------------------- ------- --------------------------------------------------------------------------------------- --- |
|   \[211\]     []{#ns-s-flow-simple-empty(n,c)}ns-s-flow-simple-empty(n,c)  `::=`  [ns-s-flow-simple-key(n,c)](#ns-s-flow-simple-key(n,c)) [e-empty-flow](#e-empty-flow)       |
|   --------- ------------------------------------------------------------- ------- --------------------------------------------------------------------------------------- --- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

Inside flow mappings, all four combinations may be used.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ------------------------------------------------------- -------------- ------------------------------------------------------------------------ -------------- |
|   \[212\]          []{#ns-s-flow-map-entry(n,c)}ns-s-flow-map-entry(n,c)     `::=`        [c-s-flow-explicit-explicit(n,c)](#c-s-flow-explicit-explicit(n,c))\                  |
|                                                                                         \| [c-s-flow-explicit-empty(n,c)](#c-s-flow-explicit-empty(n,c))\                       |
|                                                                                         \| [ns-s-flow-simple-explicit(n,c)](#ns-s-flow-simple-explicit(n,c))\                   |
|                                                                                         \| [ns-s-flow-simple-empty(n,c)](#ns-s-flow-simple-empty(n,c))                          |
|                                                                                                                                                                                 |
|   -------------- ------------------------------------------------------- -------------- ------------------------------------------------------------------------ -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id934182}

**Example 10.10. Flow Mapping Key: Value Pairs**

+---------------------------------------+-----------------------------------+
| ``` programlisting                    | ``` programlisting                |
| {                                     | %YAML 1.1                         |
| ? explicit key1 : Explicit value,     | ---                               |
| ? explicit key2 :° , # Explicit empty | !!map {                           |
| ? explicit key3,     # Empty value    |   ? !!str "explicit key1"         |
| simple key1 : explicit value,         |   : !!str "explicit value",       |
| simple key2 :° ,     # Explicit empty |   ? !!str "explicit key2"         |
| simple key3,         # Empty value    |   : !!str "",                     |
| }                                     |   ? !!str "explicit key3"         |
| ```                                   |   : !!str "",                     |
|                                       |   ? !!str "simple key1"           |
| ``` synopsis                          |   : !!str "explicit value",       |
| Legend:                               |   ? !!str "simple key2"           |
|   c-s-flow-explicit-explicit(n,c)     |   : !!str "",                     |
|   c-s-flow-explicit-empty(n,c)        |   ? !!str "simple key3"           |
|   ns-s-flow-simple-explicit(n,c)      |   : !!str "",                     |
|   ns-s-flow-simple-empty(n,c)         | }                                 |
| ```                                   | ```                               |
+---------------------------------------+-----------------------------------+
:::

YAML also allows omitting the surrounding "[**`{`**]{.quote}" and "[**`}`**]{.quote}" characters when nesting a flow mapping in a []{#id934334 .indexterm}[flow sequence](#flow%20sequence%20style/syntax) if the mapping consists of a []{#id934350 .indexterm}[]{#single pair style/syntax}*single key: value pair* and neither the mapping nor the key have any []{#id934370 .indexterm}[properties](#node%20property/) specified. In this case, only three of the combinations may be used, to prevent ambiguity.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------------- -------------- ------------------------------------------------------------------------ -------------- |
|   \[213\]          []{#ns-s-flow-single-pair(n,c)}ns-s-flow-single-pair(n,c)     `::=`        [c-s-flow-explicit-explicit(n,c)](#c-s-flow-explicit-explicit(n,c))\                  |
|                                                                                             \| [c-s-flow-explicit-empty(n,c)](#c-s-flow-explicit-empty(n,c))\                       |
|                                                                                             \| [ns-s-flow-simple-explicit(n,c)](#ns-s-flow-simple-explicit(n,c))                    |
|                                                                                                                                                                                     |
|   -------------- ----------------------------------------------------------- -------------- ------------------------------------------------------------------------ -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id934419}

**Example 10.11. Single Pair Mappings**

+---------------------------------------+-----------------------------------+
| ``` programlisting                    | ``` programlisting                |
| [                                     | %YAML 1.1                         |
| ? explicit key1 : explicit value,     | ---                               |
| ? explicit key2 :° , # Explicit empty | !!seq [                           |
| ? explicit key3,     # Implicit empty |   !!map {                         |
| simple key1 : explicit value,         |     ? !!str "explicit key1"       |
| simple key2 :° ,     # Explicit empty |     : !!str "explicit value",     |
| ]                                     |   },                              |
| ```                                   |   !!map {                         |
|                                       |     ? !!str "explicit key2"       |
| ``` synopsis                          |     : !!str "",                   |
| Legend:                               |   },                              |
|   c-s-flow-explicit-explicit(n,c)     |   !!map {                         |
|   c-s-flow-explicit-empty(n,c)        |     ? !!str "explicit key3"       |
|   ns-s-flow-simple-explicit(n,c)      |     : !!str "",                   |
| ```                                   |   },                              |
|                                       |   !!map {                         |
|                                       |     ? !!str "simple key1"         |
|                                       |     : !!str "explicit value",     |
|                                       |   },                              |
|                                       |   !!map {                         |
|                                       |     ? !!str "simple key2"         |
|                                       |     : !!str "",                   |
|                                       |   },                              |
|                                       | ]                                 |
|                                       | ```                               |
+---------------------------------------+-----------------------------------+
:::
:::::::::::::::::::

::::::::::::::: {.sect2 lang="en"}
::::: titlepage
:::: {}
::: {}
### []{#id934537}10.2.2. Block Mappings {#block-mappings .title}
:::
::::
:::::

A []{#id934545 .indexterm}[]{#block mapping style/syntax}*Block mapping* is simply a series of entries, each []{#id934563 .indexterm}[presenting](#present/) a key: value pair.

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
|   \[214\]          []{#c-l-block-mapping(n)}c-l-block-mapping(n)     `::=`      [c-l-comments](#c-l-comments)\                                                                        |
|                                                                                 ( [s-indent(n)](#s-indent(n)) [ns-l-block-map-entry(n)](#ns-l-block-map-entry(n)) )+                  |
|                                                                                                                                                                                       |
|   -------------- ----------------------------------------------- -------------- -------------------------------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id934609}

**Example 10.12. Block Mappings**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| block: # Block                    | %YAML 1.1                         |
|     # mapping↓                    | ---                               |
| ·key: value↓                      | !!map {                           |
| ```                               |   ? !!str "block"                 |
|                                   |   : !!map {                       |
| ``` synopsis                      |     ? !!str "key",                |
| Legend:                           |     : !!str "value"               |
|   c-l-comments                    |   }                               |
|   s-indent(n)                     | }                                 |
|   ns-l-block-map-entry(n)         | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

A block mapping entry may be []{#id934709 .indexterm}[presented](#present/) using either an explicit or a simple key.

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------- -------------- ------------------------------------------------------------------ -------------- |
|   \[215\]          []{#ns-l-block-map-entry(n)}ns-l-block-map-entry(n)     `::=`        [ns-l-block-explicit-entry(n)](#ns-l-block-explicit-entry(n))\                  |
|                                                                                       \| [ns-l-block-simple-entry(n)](#ns-l-block-simple-entry(n))                      |
|                                                                                                                                                                         |
|   -------------- ----------------------------------------------------- -------------- ------------------------------------------------------------------ -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: variablelist

[Explicit Key Entries]{.term}
:   []{#id934761 .indexterm}[Explicit key nodes](#explicit%20key/) are denoted by the []{#id934776 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) character. YAML allows here the same []{#id934795 .indexterm}[inline](#in-line%20style/syntax) compact notation described above for []{#id934810 .indexterm}[block sequence](#block%20sequence%20style/syntax) entries, in which case the []{#id934826 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) character is considered part of the key's []{#id934844 .indexterm}[indentation](#indentation%20space/).
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------------- -------------- ---------------------------------------------------------------------------------------------- -------------- |
|   \[216\]          []{#ns-l-block-explicit-key(n)}ns-l-block-explicit-key(n)     `::=`      ["[?]{.quote}"](#c-mapping-key) [s-l+block-indented(n,block-out)](#s-l+block-indented(n,c))\                  |
|                                                                                                                                                                                                           |
|   -------------- ----------------------------------------------------------- -------------- ---------------------------------------------------------------------------------------------- -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- In an explicit key entry, value nodes begin on a separate line and are denoted by by the []{#id934901 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) character. Here again YAML allows the use of the []{#id934920 .indexterm}[inline](#in-line%20style/syntax) compact notation which case the []{#id934935 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) character is considered part of the values's []{#id934953 .indexterm}[indentation](#indentation%20space/).
:::

+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------------- -------------- ---------------------------------------------------------------- -------------- |
|   \[217\]          []{#l-block-explicit-value(n)}l-block-explicit-value(n)     `::=`      [s-indent(n)](#s-indent(n)) ["[:]{.quote}"](#c-mapping-value)\                  |
|                                                                                           [s-l+block-indented(n,block-out)](#s-l+block-indented(n,c))                     |
|                                                                                                                                                                           |
|   -------------- --------------------------------------------------------- -------------- ---------------------------------------------------------------- -------------- |
+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- An explicit key entry may also use a []{#id935012 .indexterm}[completely empty](#completely%20empty%20node/) value.
:::

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------------------- -------------- ------------------------------------------------------------ -------------- |
|   \[218\]          []{#ns-l-block-explicit-entry(n)}ns-l-block-explicit-entry(n)     `::=`      [ns-l-block-explicit-key(n)](#ns-l-block-explicit-key(n))\                  |
|                                                                                                 ( [l-block-explicit-value(n)](#l-block-explicit-value(n))\                  |
|                                                                                                 \| [e-empty-flow](#e-empty-flow) )                                          |
|                                                                                                                                                                             |
|   -------------- --------------------------------------------------------------- -------------- ------------------------------------------------------------ -------------- |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id935061}

**Example 10.13. Explicit Block Mapping Entries**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| ? explicit key # implicit value↓° | %YAML 1.1                         |
| ? |                               | ---                               |
|   block key↓                      | !!map {                           |
| :·- one # explicit in-line        |   ? !!str "explicit key"          |
| ··- two # block value↓            |   : !!str "",                     |
| ```                               |   ? !!str "block key\n"           |
|                                   |   : !!seq [                       |
| ``` synopsis                      |     !!str "one",                  |
| Legend:                           |     !!str "two",                  |
|   ns-l-block-explicit-key(n)      |   ]                               |
|   l-block-explicit-value(n)       | }                                 |
|   e-empty-flow                    | ```                               |
| ```                               |                                   |
+-----------------------------------+-----------------------------------+
:::

::: variablelist

[Simple Key Entries]{.term}
:   YAML allows the []{#id935176 .indexterm}["[**`?`**]{.quote}"](#?%20mapping%20key/) character to be omitted for []{#id935195 .indexterm}[simple keys](#simple%20key/). Similarly to flow mapping, such a key is recognized by a following []{#id935208 .indexterm}["[**`:`**]{.quote}"](#:%20mapping%20value/) character. Again, to avoid unbound lookahead in YAML []{#id935226 .indexterm}[processors](#processor/), simple keys are restricted to a single line and must not span more than 1024 []{#id935239 .indexterm}[stream](#stream/syntax) characters. Again, this limit is in terms of Unicode characters rather than stream octets, and includes the []{#id935256 .indexterm}[separation](#separation%20space/) following the key, if any.
:::

+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- --------------------------------------------------- -------------- -------------------------------------------------------------------------------- -------------- |
|   \[219\]          []{#ns-block-simple-key(n)}ns-block-simple-key(n)     `::=`      [ns-flow-node(n,flow-key)](#ns-flow-node(n,c))\                                                 |
|                                                                                     [s-separate(n,block-out)](#s-separate(n,c))? ["[:]{.quote}"](#c-mapping-value)                  |
|                                                                                                                                                                                     |
|   -------------- --------------------------------------------------- -------------- -------------------------------------------------------------------------------- -------------- |
+-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: itemizedlist
- In a simple key entry, an []{#id935317 .indexterm}[explicit value](#explicit%20value/) node may be []{#id935333 .indexterm}[presented](#present/) in the same line. Note however that in this case, the key is not considered to be a form of []{#id935346 .indexterm}[indentation](#indentation%20space/), hence the compact []{#id935360 .indexterm}[in-line](#in-line%20style/syntax) notation must not be used. The value following the simple key may also be []{#id935377 .indexterm}[completely empty](#completely%20empty%20node/).
:::

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------------- -------------- --------------------------------------------------------- -------------- |
|   \[220\]            []{#s-l+block-simple-value(n)}s-l+block-simple-value(n)     `::=`        [s-l+block-node(n,block-out)](#s-l+block-node(n,c))\                   |
|                                                                                             \| [s-l-empty-block](#s-l-empty-block)                                   |
|                                                                                                                                                                      |
|   \[221\]          []{#ns-l-block-simple-entry(n)}ns-l-block-simple-entry(n)     `::=`      [ns-block-simple-key(n)](#ns-block-simple-key(n))\                       |
|                                                                                             [s-l+block-simple-value(n)](#s-l+block-simple-value(n))                  |
|   -------------- ----------------------------------------------------------- -------------- --------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id935444}

**Example 10.14. Simple Block Mapping Entries**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| plain key:° # empty value↓        | %YAML 1.1                         |
| "quoted key":↓                    | ---                               |
| - one # explicit next-line        | !!map {                           |
| - two # block value↓              |   ? !!str "plain key"             |
| ```                               |   : !!str "",                     |
|                                   |   ? !!str "quoted key\n"          |
| ``` synopsis                      |   : !!seq [                       |
| Legend:                           |     !!str "one",                  |
|   ns-block-simple-key(n)          |     !!str "two",                  |
|   s-l+block-simple-value(n)       |   ]                               |
| ```                               | }                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::

An []{#id935541 .indexterm}[]{#in-line mapping style/}*in-line block mapping* begins with a same-line mapping entry, followed by optional additional normal block mapping entries, properly []{#id935559 .indexterm}[indented](#indentation%20space/).

+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
|   -------------- ----------------------------------------------------- -------------- --------------------------------------------------------------------------------------- -------------- |
|   \[222\]          []{#ns-l-in-line-mapping(n)}ns-l-in-line-mapping(n)     `::=`      [ns-l-block-map-entry(n)](#ns-l-block-map-entry(n))\                                                   |
|                                                                                       ( [s-indent(n)](#s-indent(n)) [ns-l-block-map-entry(n)](#ns-l-block-map-entry(n)) )\*                  |
|                                                                                                                                                                                              |
|   -------------- ----------------------------------------------------- -------------- --------------------------------------------------------------------------------------- -------------- |
+----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+

::: example
[]{#id935604}

**Example 10.15. In-Line Block Mappings**

+-----------------------------------+-----------------------------------+
| ``` programlisting                | ``` programlisting                |
| - sun: yellow↓                    | %YAML 1.1                         |
| - ? earth: blue↓                  | ---                               |
|   : moon: white↓                  | !!seq {                           |
| ```                               |   !!map {                         |
|                                   |     ? !!str "sun"                 |
| ``` synopsis                      |     : !!str "yellow",             |
| Legend:                           |   },                              |
|   ns-l-in-line-mapping(n)         |   !!map {                         |
| ```                               |     ? !!map {                     |
|                                   |       ? !!str "earth"             |
|                                   |       : !!str "blue"              |
|                                   |     }                             |
|                                   |     : !!map {                     |
|                                   |       ? !!str "moon"              |
|                                   |       : !!str "white"             |
|                                   |     }                             |
|                                   |   }                               |
|                                   | }                                 |
|                                   | ```                               |
+-----------------------------------+-----------------------------------+
:::
:::::::::::::::
::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::

:::::::::::::::::::::::::::::: index
::::: titlepage
:::: {}
::: {}
## []{#id935693}Index {#index .title}
:::
::::
:::::

:::::::::::::::::::::::::: index
::: indexdiv
### Indicators

[]{#index-entry-! local  tag}! local tag, [Tags](#id861700){.preferred}, [Tag Prefixes](#id896411), [Tag Handles](#id896876), [Node Tags](#id900262)

[]{#index-entry-! named handle}! named handle, [Miscellaneous Characters](#id871856), [Tag Handles](#id896876){.preferred}, [Node Tags](#id900262)

[]{#index-entry-! non-specific  tag}! non-specific tag, [Resolved](#id865585){.preferred}, [Node Tags](#id900262)

[]{#index-entry-! tag  indicator}! tag indicator, [Tags](#id858600), [Indicator Characters](#id868988), [Tag Handles](#id896876), [Node Tags](#id900262){.preferred}

[]{#index-entry-\" double-quoted  style}\" double-quoted style, [Indicator Characters](#id868988), [Escape Sequences](#id872840), [Double Quoted](#id904245){.preferred}, [Single Quoted](#id905860)

[]{#index-entry-#  comment}\# comment, [Structures](#id857577), [Tags](#id861700), [Indicator Characters](#id868988), [Comments](#id892353){.preferred}, [Plain](#id907281), [Block Indentation Indicator](#id927035), [Literal](#id928909)

[]{#index-entry-% directive}% directive, [Indicator Characters](#id868988), [Directives](#id895217){.preferred}

[]{#index-entry-%  escaping in URI}% escaping in URI, [Miscellaneous Characters](#id871856){.preferred}, [Node Tags](#id900262)

[]{#index-entry-&  anchor}& anchor, [Structures](#id857577), [Indicator Characters](#id868988), [Node Anchors](#id899912){.preferred}

[]{#index-entry-' reserved  indicator}\' reserved indicator, [Indicator Characters](#id868988){.preferred}

[]{#index-entry-' single-quoted  style}\' single-quoted style, [Indicator Characters](#id868988), [Single Quoted](#id905860){.preferred}

[]{#index-entry-*  alias}\* alias, [Structures](#id857577), [Indicator Characters](#id868988), [Alias Nodes](#id902561){.preferred}

[]{#index-entry-+ keep chomping}+ keep chomping, [Block Chomping Indicator](#id927557){.preferred}

[]{#index-entry-, end flow entry}, end flow entry, [Indicator Characters](#id868988), [Plain](#id907281), [Collection Styles](#id930798){.preferred}, [Flow Sequences](#id931285), [Flow Mappings](#id933010)

[]{#index-entry-- block sequence  entry}- block sequence entry, [Collections](#id857181), [Production Parameters](#id867808), [Indicator Characters](#id868988), [Indentation Spaces](#id891751), [Plain](#id907281), [Block Sequences](#id931893){.preferred}

[]{#index-entry-- strip  chomping}- strip chomping, [Block Chomping Indicator](#id927557){.preferred}

[]{#index-entry-: mapping  value}: mapping value, [Collections](#id857181), [Indicator Characters](#id868988), [Indentation Spaces](#id891751), [Plain](#id907281), [Flow Mappings](#id933010){.preferred}, [Block Mappings](#id934537)

[]{#index-entry-< … > verbatim tag}\< ... \> verbatim tag, [Node Tags](#id900262){.preferred}

[]{#index-entry-> folded  style}\> folded style, [Scalars](#id858081), [Indicator Characters](#id868988), [Block Style Indicator](#id926836){.preferred}

[]{#index-entry-? mapping  key}? mapping key, [Structures](#id857577), [Indicator Characters](#id868988), [Indentation Spaces](#id891751), [Plain](#id907281), [Flow Mappings](#id933010){.preferred}, [Block Mappings](#id934537)

[]{#index-entry-? non-specific tag}? non-specific tag, [Resolved](#id865585){.preferred}, [Node Tags](#id900262)

[]{#index-entry-@ reserved  indicator}@ reserved indicator, [Indicator Characters](#id868988){.preferred}

[]{#index-entry-[ start flow  sequence}\[ start flow sequence, [Indicator Characters](#id868988), [Miscellaneous Characters](#id871856), [Plain](#id907281), [Flow Sequences](#id931285){.preferred}

[]{#index-entry-\\ escaping in  double-quoted style}\\ escaping in double-quoted style, [Escape Sequences](#id872840){.preferred}, [Double Quoted](#id904245), [Single Quoted](#id905860)

[]{#index-entry-] end flow  sequence}\] end flow sequence, [Indicator Characters](#id868988), [Miscellaneous Characters](#id871856), [Plain](#id907281), [Flow Sequences](#id931285){.preferred}

[]{#index-entry-{ start flow  mapping}{ start flow mapping, [Indicator Characters](#id868988), [Plain](#id907281), [Flow Mappings](#id933010){.preferred}

[]{#index-entry-| literal  style}\| literal style, [Scalars](#id858081), [Indicator Characters](#id868988), [Block Style Indicator](#id926836){.preferred}

[]{#index-entry-} end flow  mapping}} end flow mapping, [Indicator Characters](#id868988), [Plain](#id907281), [Flow Mappings](#id933010){.preferred}
:::

::: indexdiv
### A

[]{#index-entry-alias}alias

information model, [Introduction](#id838426){xmlns=""}, [Prior Art](#id838686){xmlns=""}, [Structures](#id857577){xmlns=""}, [Serialize](#id859873){xmlns=""}, [Serialization Tree](#id862929){xmlns=""}, [Anchors and Aliases](#id863390){.preferred xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Well-Formed and Identified](#id865423){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Node Anchors](#id899912){xmlns=""}, [Alias Nodes](#id902561){.preferred xmlns=""}, [Flow Nodes](#id902924){xmlns=""}

[]{#index-entry-anchor}anchor

information model, [Structures](#id857577){xmlns=""}, [Serialize](#id859873){xmlns=""}, [Serialization Tree](#id862929){xmlns=""}, [Anchors and Aliases](#id863390){.preferred xmlns=""}, [Well-Formed and Identified](#id865423){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Nodes](#id899622){xmlns=""}, [Node Anchors](#id899912){.preferred xmlns=""}, [Node Tags](#id900262){xmlns=""}, [Alias Nodes](#id902561){xmlns=""}

[]{#index-entry-application}application, [Introduction](#id838426), [Prior Art](#id838686), [Tags](#id858600), [Processing YAML Information](#id859109){.preferred}, [Processes](#id859458), [Represent](#id859497), [Serialize](#id859873), [Present](#id860109), [Information Models](#id860735), [Tags](#id861700), [Resolved](#id865585), [Available](#id867229), [Tag Prefixes](#id896411), [Tag Handles](#id896876), [Node Tags](#id900262), [Mapping Styles](#id932806)

[]{#index-entry-available tag}available tag, [Available](#id867229){.preferred}
:::

::: indexdiv
### B

[]{#index-entry-block collection style}block collection style

information model, [Collections](#id857181){xmlns=""}, [Structures](#id857577){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Indentation Spaces](#id891751){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Collection Styles](#id930798){.preferred xmlns=""}, [Block Sequences](#id931893){xmlns=""}

[]{#index-entry-block mapping style}block mapping style

information model, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Plain](#id907281){xmlns=""}, [Block Mappings](#id934537){.preferred xmlns=""}

[]{#index-entry-block scalar header}block scalar header, [Block Scalar Header](#id926597){.preferred}, [Block Style Indicator](#id926836)

[]{#index-entry-block scalar style}block scalar style

information model, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Miscellaneous Characters](#id871856){xmlns=""}, [Ignored Line Prefix](#id893482){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Block Scalar Header](#id926597){xmlns=""}, [Block Indentation Indicator](#id927035){xmlns=""}, [Block Chomping Indicator](#id927557){xmlns=""}, [Block Scalar Styles](#id928806){.preferred xmlns=""}

[]{#index-entry-block  sequence style}block sequence style

information model, [Collections](#id857181){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Indentation Spaces](#id891751){xmlns=""}, [Sequence Styles](#id931088){xmlns=""}, [Block Sequences](#id931893){.preferred xmlns=""}, [Block Mappings](#id934537){xmlns=""}

[]{#index-entry-block style}block style

information model, [Prior Art](#id838686){xmlns=""}, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Line Folding](#id894136){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Block Nodes](#id903421){.preferred xmlns=""}, [Block Sequences](#id931893){xmlns=""}

[]{#index-entry-block-in context}block-in context, [Production Parameters](#id867808), [Block Sequences](#id931893){.preferred}

[]{#index-entry-block-out  context}block-out context, [Production Parameters](#id867808), [Block Sequences](#id931893){.preferred}

[]{#index-entry-byte order mark}byte order mark, [Character Encoding](#id868742){.preferred}, [Complete Stream](#id898785)
:::

::: indexdiv
### C

[]{#index-entry-canonical form}canonical form, [Prior Art](#id838686), [Tags](#id861700), [Nodes Comparison](#id862121){.preferred}, [Scalar Formats](#id864510), [Loading Failure Points](#id864977)

[]{#index-entry-character encoding}character encoding, [Character Encoding](#id868742){.preferred}, [Miscellaneous Characters](#id871856), [Complete Stream](#id898785)

[]{#index-entry-chomping}chomping, [Production Parameters](#id867808), [Line Break Characters](#id871136), [Ignored Line Prefix](#id893482), [Line Folding](#id894136), [Block Chomping Indicator](#id927557){.preferred}, [Literal](#id928909), [Folded](#id929764)

[]{#index-entry-clip  chomping}clip chomping, [Production Parameters](#id867808), [Block Chomping Indicator](#id927557){.preferred}

[]{#index-entry-collection}collection

information model, [Prior Art](#id838686){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Anchors and Aliases](#id863390){xmlns=""}, [Comments](#id864687){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}

syntax, [Indentation Spaces](#id891751){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Plain](#id907281){xmlns=""}, [Collection Styles](#id930798){.preferred xmlns=""}, [Sequence Styles](#id931088){xmlns=""}, [Block Sequences](#id931893){xmlns=""}

[]{#index-entry-comment}comment

information model, [Structures](#id857577){xmlns=""}, [Present](#id860109){xmlns=""}, [Construct](#id860557){xmlns=""}, [Presentation Stream](#id863676){xmlns=""}, [Comments](#id864687){.preferred xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Comments](#id892353){.preferred xmlns=""}, [Separation Spaces](#id893014){xmlns=""}, [Ignored Line Prefix](#id893482){xmlns=""}, [Directives](#id895217){xmlns=""}, [Document Boundary Markers](#id897596){xmlns=""}, [Documents](#id898031){xmlns=""}, [Complete Stream](#id898785){xmlns=""}, [Block Nodes](#id903421){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Plain](#id907281){xmlns=""}, [Block Scalar Header](#id926597){xmlns=""}, [Block Chomping Indicator](#id927557){xmlns=""}, [Sequence Styles](#id931088){xmlns=""}

[]{#index-entry-complete representation}complete representation, [Loading Failure Points](#id864977){.preferred}, [Resolved](#id865585), [Recognized and Valid](#id866900), [Available](#id867229), [Node Tags](#id900262)

[]{#index-entry-completely  empty node}completely empty node, [Documents](#id898031), [Flow Nodes](#id902924){.preferred}, [Block Nodes](#id903421), [Collection Styles](#id930798), [Flow Sequences](#id931285), [Block Sequences](#id931893), [Flow Mappings](#id933010), [Block Mappings](#id934537)

[]{#index-entry-compose}compose, [Compose](#id860452){.preferred}, [Keys Order](#id863110), [Anchors and Aliases](#id863390), [Resolved](#id865585), [Recognized and Valid](#id866900), [Available](#id867229), [Node Anchors](#id899912), [Node Tags](#id900262)

[]{#index-entry-construct}construct, [Processing YAML Information](#id859109), [Construct](#id860557){.preferred}, [Serialization Tree](#id862929), [Loading Failure Points](#id864977), [Recognized and Valid](#id866900), [Available](#id867229), [Mapping Styles](#id932806)

[]{#index-entry-content}content

information model, [Prior Art](#id838686){xmlns=""}, [Represent](#id859497){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Tags](#id861700){xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Node Styles](#id863975){xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}, [Line Break Characters](#id871136){xmlns=""}, [Escape Sequences](#id872840){xmlns=""}, [Indentation Spaces](#id891751){xmlns=""}, [Comments](#id892353){xmlns=""}, [Separation Spaces](#id893014){xmlns=""}, [Ignored Line Prefix](#id893482){xmlns=""}, [Line Folding](#id894136){xmlns=""}, [Tag Handles](#id896876){xmlns=""}, [Documents](#id898031){xmlns=""}, [Node Anchors](#id899912){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Collection Styles](#id930798){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Indentation Spaces](#id891751){xmlns=""}, [Nodes](#id899622){xmlns=""}, [Node Content](#id901659){.preferred xmlns=""}, [Alias Nodes](#id902561){xmlns=""}, [Flow Nodes](#id902924){xmlns=""}, [Block Nodes](#id903421){xmlns=""}, [Double Quoted](#id904245){xmlns=""}, [Single Quoted](#id905860){xmlns=""}, [Plain](#id907281){xmlns=""}, [Block Scalar Header](#id926597){xmlns=""}, [Block Indentation Indicator](#id927035){xmlns=""}, [Block Chomping Indicator](#id927557){xmlns=""}, [Block Scalar Styles](#id928806){xmlns=""}, [Folded](#id929764){xmlns=""}

[]{#index-entry-context}context, [Production Parameters](#id867808){.preferred}, [Plain](#id907281)
:::

::: indexdiv
### D

[]{#index-entry-directive}directive

information model, [Present](#id860109){xmlns=""}, [Construct](#id860557){xmlns=""}, [Presentation Stream](#id863676){xmlns=""}, [Directives](#id864824){.preferred xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [YAML Character Stream](#id895107){xmlns=""}, [Directives](#id895217){.preferred xmlns=""}, [Documents](#id898031){xmlns=""}, [Complete Stream](#id898785){xmlns=""}

[]{#index-entry-document}document

information model, [Prior Art](#id838686){xmlns=""}, [Structures](#id857577){xmlns=""}, [Presentation Stream](#id863676){.preferred xmlns=""}, [Node Styles](#id863975){xmlns=""}, [Directives](#id864824){xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [YAML Character Stream](#id895107){xmlns=""}, ["YAML" Directive](#id895631){xmlns=""}, [Tag Prefixes](#id896411){xmlns=""}, [Tag Handles](#id896876){xmlns=""}, [Document Boundary Markers](#id897596){xmlns=""}, [Documents](#id898031){.preferred xmlns=""}, [Complete Stream](#id898785){xmlns=""}, [Alias Nodes](#id902561){xmlns=""}

[]{#index-entry-document boundary  marker}document boundary marker, [Structures](#id857577), [Presentation Stream](#id863676), [YAML Character Stream](#id895107), [Document Boundary Markers](#id897596){.preferred}, [Documents](#id898031), [Complete Stream](#id898785), [Plain](#id907281)

[]{#index-entry-double-quoted style}double-quoted style

information model, [Prior Art](#id838686){xmlns=""}, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Productions Conventions](#id867381){xmlns=""}, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Escape Sequences](#id872840){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Double Quoted](#id904245){.preferred xmlns=""}, [Single Quoted](#id905860){xmlns=""}

[]{#index-entry-dump}dump, [Processing YAML Information](#id859109){.preferred}
:::

::: indexdiv
### E

[]{#index-entry-empty line}empty line, [Prior Art](#id838686), [Scalars](#id858081), [Comments](#id892353), [Ignored Line Prefix](#id893482){.preferred}, [Line Folding](#id894136), [Flow Scalar Styles](#id904158), [Plain](#id907281), [Block Indentation Indicator](#id927035), [Block Chomping Indicator](#id927557), [Literal](#id928909), [Folded](#id929764)

[]{#index-entry-equality}equality, [Represent](#id859497), [Representation Graph](#id861060), [Nodes](#id861435), [Tags](#id861700), [Nodes Comparison](#id862121){.preferred}, [Scalar Formats](#id864510), [Loading Failure Points](#id864977), [Recognized and Valid](#id866900), [Mapping Styles](#id932806)

[]{#index-entry-escaped  (ignored) line break}escaped (ignored) line break, [Line Break Characters](#id871136), [Double Quoted](#id904245){.preferred}

[]{#index-entry-escaping in  double-quoted style}escaping in double-quoted style, [Prior Art](#id838686), [Scalars](#id858081), [Character Set](#id868524), [Miscellaneous Characters](#id871856), [Escape Sequences](#id872840){.preferred}, [Double Quoted](#id904245), [Literal](#id928909)

[]{#index-entry-escaping in single-quoted  style}escaping in single-quoted style, [Single Quoted](#id905860){.preferred}

[]{#index-entry-escaping in URI}escaping in URI, [Tags](#id861700), [Miscellaneous Characters](#id871856){.preferred}, [Node Tags](#id900262)

[]{#index-entry-explicit document}explicit document, [Documents](#id898031){.preferred}, [Complete Stream](#id898785)

[]{#index-entry-explicit key}explicit key, [Flow Mappings](#id933010){.preferred}, [Block Mappings](#id934537)

[]{#index-entry-explicit value}explicit value, [Flow Mappings](#id933010){.preferred}, [Block Mappings](#id934537)
:::

::: indexdiv
### F

[]{#index-entry-flow collection style}flow collection style

information model, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Productions Conventions](#id867381){xmlns=""}, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Plain](#id907281){xmlns=""}, [Collection Styles](#id930798){.preferred xmlns=""}

[]{#index-entry-flow mapping style}flow mapping style

information model, [Collections](#id857181){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Flow Mappings](#id933010){.preferred xmlns=""}

[]{#index-entry-flow scalar style}flow scalar style

information model, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Line Folding](#id894136){xmlns=""}, [Documents](#id898031){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Flow Scalar Styles](#id904158){.preferred xmlns=""}, [Plain](#id907281){xmlns=""}

[]{#index-entry-flow sequence style}flow sequence style

information model, [Collections](#id857181){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Sequence Styles](#id931088){xmlns=""}, [Flow Sequences](#id931285){.preferred xmlns=""}, [Flow Mappings](#id933010){xmlns=""}

[]{#index-entry-flow style}flow style

information model, [Prior Art](#id838686){xmlns=""}, [Collections](#id857181){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Line Folding](#id894136){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Flow Nodes](#id902924){.preferred xmlns=""}, [Block Nodes](#id903421){xmlns=""}, [Flow Sequences](#id931285){xmlns=""}

[]{#index-entry-flow-in  context}flow-in context, [Production Parameters](#id867808), [Plain](#id907281){.preferred}, [Collection Styles](#id930798)

[]{#index-entry-flow-key context}flow-key context, [Production Parameters](#id867808), [Collection Styles](#id930798), [Flow Mappings](#id933010){.preferred}

[]{#index-entry-flow-out context}flow-out context, [Production Parameters](#id867808), [Plain](#id907281){.preferred}, [Collection Styles](#id930798)

[]{#index-entry-folded  style}folded style

information model, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Line Folding](#id894136){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Block Style Indicator](#id926836){xmlns=""}, [Block Scalar Styles](#id928806){xmlns=""}, [Folded](#id929764){.preferred xmlns=""}

[]{#index-entry-format}format, [Present](#id860109), [Construct](#id860557), [Tags](#id861700), [Nodes Comparison](#id862121), [Presentation Stream](#id863676), [Scalar Formats](#id864510){.preferred}
:::

::: indexdiv
### G

[]{#index-entry-generic line break}generic line break, [Line Break Characters](#id871136){.preferred}, [Escape Sequences](#id872840), [Line Folding](#id894136)

[]{#index-entry-global tag}global tag, [Prior Art](#id838686), [Tags](#id858600), [Represent](#id859497), [Tags](#id861700){.preferred}, [Resolved](#id865585), [Tag Prefixes](#id896411), [Tag Handles](#id896876), [Node Tags](#id900262)
:::

::: indexdiv
### I

[]{#index-entry-identified}identified, [Structures](#id857577), [Anchors and Aliases](#id863390){.preferred}, [Well-Formed and Identified](#id865423)

[]{#index-entry-identity}identity, [Nodes Comparison](#id862121){.preferred}

[]{#index-entry-ignored line  prefix}ignored line prefix, [Ignored Line Prefix](#id893482){.preferred}, [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281)

[]{#index-entry-ill-formed  stream}ill-formed stream, [Parse](#id860341), [Loading Failure Points](#id864977), [Well-Formed and Identified](#id865423){.preferred}

[]{#index-entry-implicit document}implicit document, [Documents](#id898031){.preferred}, [Complete Stream](#id898785)

[]{#index-entry-in-line mapping style}in-line mapping style, [Block Mappings](#id934537){.preferred}

[]{#index-entry-in-line sequence style}in-line sequence style, [Block Sequences](#id931893){.preferred}

[]{#index-entry-in-line style}in-line style

information model, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Indentation Spaces](#id891751){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Collection Styles](#id930798){xmlns=""}, [Sequence Styles](#id931088){xmlns=""}, [Block Sequences](#id931893){.preferred xmlns=""}, [Block Mappings](#id934537){xmlns=""}

[]{#index-entry-indentation  indicator}indentation indicator, [Block Indentation Indicator](#id927035){.preferred}, [Folded](#id929764)

[]{#index-entry-indentation  space}indentation space, [Introduction](#id838426), [Prior Art](#id838686), [Collections](#id857181), [Present](#id860109), [Construct](#id860557), [Information Models](#id860735), [Node Styles](#id863975), [Resolved](#id865585), [Production Prefixes](#id867562), [Production Parameters](#id867808), [Miscellaneous Characters](#id871856), [Indentation Spaces](#id891751){.preferred}, [Comments](#id892353), [Separation Spaces](#id893014), [Ignored Line Prefix](#id893482), [Line Folding](#id894136), [Directives](#id895217), [Documents](#id898031), [Node Content](#id901659), [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281), [Block Indentation Indicator](#id927035), [Block Chomping Indicator](#id927557), [Block Scalar Styles](#id928806), [Literal](#id928909), [Folded](#id929764), [Block Sequences](#id931893), [Block Mappings](#id934537)

[]{#index-entry-indicator}indicator, [Prior Art](#id838686), [Collections](#id857181), [Node Styles](#id863975), [Production Parameters](#id867808), [Indicator Characters](#id868988){.preferred}, [Indentation Spaces](#id891751), [Line Folding](#id894136), [Node Content](#id901659), [Flow Nodes](#id902924), [Block Nodes](#id903421), [Plain](#id907281), [Block Scalar Header](#id926597), [Literal](#id928909)

[]{#index-entry-invalid content}invalid content, [Loading Failure Points](#id864977), [Recognized and Valid](#id866900){.preferred}
:::

::: indexdiv
### K

[]{#index-entry-keep  chomping}keep chomping, [Production Parameters](#id867808), [Block Chomping Indicator](#id927557){.preferred}

[]{#index-entry-key}key

information model, [Introduction](#id838426){xmlns=""}, [Collections](#id857181){xmlns=""}, [Structures](#id857577){xmlns=""}, [Represent](#id859497){xmlns=""}, [Serialize](#id859873){xmlns=""}, [Information Models](#id860735){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Serialization Tree](#id862929){xmlns=""}, [Keys Order](#id863110){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Plain](#id907281){xmlns=""}, [Flow Sequences](#id931285){xmlns=""}, [Mapping Styles](#id932806){.preferred xmlns=""}

[]{#index-entry-key order}key order, [Serialize](#id859873), [Construct](#id860557), [Information Models](#id860735), [Serialization Tree](#id862929), [Keys Order](#id863110){.preferred}, [Mapping Styles](#id932806)

[]{#index-entry-kind}kind, [Represent](#id859497), [Representation Graph](#id861060), [Nodes](#id861435){.preferred}, [Tags](#id861700), [Nodes Comparison](#id862121), [Node Styles](#id863975), [Resolved](#id865585), [Node Content](#id901659), [Collection Styles](#id930798)
:::

::: indexdiv
### L

[]{#index-entry-line break character}line break character, [Prior Art](#id838686), [Scalars](#id858081), [Production Prefixes](#id867562), [Production Parameters](#id867808), [Character Set](#id868524), [Line Break Characters](#id871136){.preferred}, [Miscellaneous Characters](#id871856), [Indentation Spaces](#id891751), [Ignored Line Prefix](#id893482), [Line Folding](#id894136), [Block Nodes](#id903421), [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281), [Block Scalar Header](#id926597), [Block Chomping Indicator](#id927557), [Literal](#id928909), [Folded](#id929764)

[]{#index-entry-line break normalization}line break normalization, [Line Break Characters](#id871136){.preferred}, [Literal](#id928909)

[]{#index-entry-line folding}line folding, [Prior Art](#id838686), [Scalars](#id858081), [Line Folding](#id894136){.preferred}, [Flow Scalar Styles](#id904158), [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281), [Block Chomping Indicator](#id927557), [Literal](#id928909), [Folded](#id929764)

[]{#index-entry-literal style}literal style

information model, [Prior Art](#id838686){xmlns=""}, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Block Style Indicator](#id926836){xmlns=""}, [Block Scalar Styles](#id928806){xmlns=""}, [Literal](#id928909){.preferred xmlns=""}, [Folded](#id929764){xmlns=""}

[]{#index-entry-load}load, [Processing YAML Information](#id859109){.preferred}, [Loading Failure Points](#id864977)

[]{#index-entry-load failure point}load failure point, [Compose](#id860452), [Loading Failure Points](#id864977){.preferred}

[]{#index-entry-local tag}local tag, [Tags](#id858600), [Represent](#id859497), [Tags](#id861700){.preferred}, [Resolved](#id865585), [Tag Prefixes](#id896411), [Tag Handles](#id896876), [Node Tags](#id900262)
:::

::: indexdiv
### M

[]{#index-entry-mapping}mapping

information model, [Introduction](#id838426){xmlns=""}, [Prior Art](#id838686){xmlns=""}, [Collections](#id857181){xmlns=""}, [Represent](#id859497){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Tags](#id861700){xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Keys Order](#id863110){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Collection Styles](#id930798){xmlns=""}, [Flow Sequences](#id931285){xmlns=""}, [Mapping Styles](#id932806){.preferred xmlns=""}

[]{#index-entry-may}may, [Terminology](#id856957){.preferred}

[]{#index-entry-“more indented” line}"more indented" line, [Scalars](#id858081), [Line Folding](#id894136), [Folded](#id929764){.preferred}

[]{#index-entry-must}must, [Terminology](#id856957){.preferred}
:::

::: indexdiv
### N

[]{#index-entry-named  tag handle}named tag handle, [Miscellaneous Characters](#id871856), [Tag Handles](#id896876){.preferred}, [Node Tags](#id900262)

[]{#index-entry-need not}need not, [Terminology](#id856957){.preferred}

[]{#index-entry-node}node

information model, [Structures](#id857577){xmlns=""}, [Represent](#id859497){xmlns=""}, [Serialize](#id859873){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Tags](#id861700){xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Serialization Tree](#id862929){xmlns=""}, [Keys Order](#id863110){xmlns=""}, [Anchors and Aliases](#id863390){xmlns=""}, [Presentation Stream](#id863676){xmlns=""}, [Node Styles](#id863975){xmlns=""}, [Comments](#id864687){xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Well-Formed and Identified](#id865423){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}

syntax, [Indentation Spaces](#id891751){xmlns=""}, [Documents](#id898031){xmlns=""}, [Nodes](#id899622){.preferred xmlns=""}, [Node Anchors](#id899912){xmlns=""}, [Node Tags](#id900262){xmlns=""}, [Alias Nodes](#id902561){xmlns=""}, [Flow Nodes](#id902924){xmlns=""}, [Sequence Styles](#id931088){xmlns=""}, [Block Sequences](#id931893){xmlns=""}

[]{#index-entry-node  property}node property, [Documents](#id898031), [Nodes](#id899622){.preferred}, [Alias Nodes](#id902561), [Flow Nodes](#id902924), [Block Nodes](#id903421), [Flow Sequences](#id931285), [Block Sequences](#id931893), [Flow Mappings](#id933010)

[]{#index-entry-non-specific tag}non-specific tag, [Tags](#id858600), [Present](#id860109), [Loading Failure Points](#id864977), [Resolved](#id865585){.preferred}, [Productions Conventions](#id867381), [Node Tags](#id900262), [Scalar Styles](#id903915)
:::

::: indexdiv
### O

[]{#index-entry-optional}optional, [Terminology](#id856957){.preferred}
:::

::: indexdiv
### P

[]{#index-entry-parse}parse, [Parse](#id860341){.preferred}, [Presentation Stream](#id863676), [Resolved](#id865585), [Line Break Characters](#id871136), [Escape Sequences](#id872840), [Tag Handles](#id896876), [Complete Stream](#id898785), [Node Tags](#id900262)

[]{#index-entry-partial representation}partial representation, [Loading Failure Points](#id864977){.preferred}, [Resolved](#id865585), [Recognized and Valid](#id866900)

[]{#index-entry-plain  style}plain style

information model, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Documents](#id898031){xmlns=""}, [Node Tags](#id900262){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Flow Nodes](#id902924){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Plain](#id907281){.preferred xmlns=""}

[]{#index-entry-present}present, [Processing YAML Information](#id859109), [Represent](#id859497), [Present](#id860109){.preferred}, [Parse](#id860341), [Nodes](#id861435), [Nodes Comparison](#id862121), [Presentation Stream](#id863676), [Scalar Formats](#id864510), [Resolved](#id865585), [Production Parameters](#id867808), [Character Set](#id868524), [Line Break Characters](#id871136), [Escape Sequences](#id872840), [Line Folding](#id894136), [YAML Character Stream](#id895107), [Documents](#id898031), [Node Tags](#id900262), [Node Content](#id901659), [Alias Nodes](#id902561), [Flow Nodes](#id902924), [Flow Scalar Styles](#id904158), [Plain](#id907281), [Block Chomping Indicator](#id927557), [Sequence Styles](#id931088), [Block Sequences](#id931893), [Mapping Styles](#id932806), [Block Mappings](#id934537)

[]{#index-entry-presentation}presentation, [Processing YAML Information](#id859109), [Information Models](#id860735), [Presentation Stream](#id863676){.preferred}, [Documents](#id898031), [Node Tags](#id900262)

[]{#index-entry-presentation  detail}presentation detail, [Present](#id860109){.preferred}, [Parse](#id860341), [Construct](#id860557), [Information Models](#id860735), [Presentation Stream](#id863676), [Node Styles](#id863975), [Scalar Formats](#id864510), [Comments](#id864687), [Directives](#id864824), [Resolved](#id865585), [Line Break Characters](#id871136), [Escape Sequences](#id872840), [Indentation Spaces](#id891751), [Comments](#id892353), [Separation Spaces](#id893014), [Ignored Line Prefix](#id893482), [Line Folding](#id894136), [Directives](#id895217), [Tag Handles](#id896876), [Document Boundary Markers](#id897596), [Documents](#id898031), [Scalar Styles](#id903915), [Block Chomping Indicator](#id927557), [Collection Styles](#id930798)

[]{#index-entry-primary tag handle}primary tag handle, [Tag Handles](#id896876){.preferred}, [Node Tags](#id900262)

[]{#index-entry-printable character}printable character, [Introduction](#id838426), [Prior Art](#id838686), [Character Set](#id868524){.preferred}, [Escape Sequences](#id872840), [Single Quoted](#id905860), [Plain](#id907281), [Literal](#id928909)

[]{#index-entry-processor}processor, [Terminology](#id856957), [Processing YAML Information](#id859109){.preferred}, [Processes](#id859458), [Serialize](#id859873), [Present](#id860109), [Nodes Comparison](#id862121), [Presentation Stream](#id863676), [Directives](#id864824), [Well-Formed and Identified](#id865423), [Resolved](#id865585), [Recognized and Valid](#id866900), [Available](#id867229), [Character Set](#id868524), [Character Encoding](#id868742), [Line Break Characters](#id871136), [Directives](#id895217), ["YAML" Directive](#id895631), [Tag Handles](#id896876), [Document Boundary Markers](#id897596), [Node Anchors](#id899912), [Node Tags](#id900262), [Block Indentation Indicator](#id927035), [Mapping Styles](#id932806), [Flow Mappings](#id933010), [Block Mappings](#id934537)
:::

::: indexdiv
### Q

[]{#index-entry-quoted style}quoted style

information model, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Miscellaneous Characters](#id871856){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Scalar Styles](#id903915){.preferred xmlns=""}
:::

::: indexdiv
### R

[]{#index-entry-recognized tag}recognized tag, [Recognized and Valid](#id866900){.preferred}

[]{#index-entry-recommended}recommended, [Terminology](#id856957){.preferred}

[]{#index-entry-represent}represent, [Introduction](#id838426), [Prior Art](#id838686), [Represent](#id859497){.preferred}, [Tags](#id861700), [Nodes Comparison](#id862121), [Keys Order](#id863110)

[]{#index-entry-representation}representation, [Processing YAML Information](#id859109), [Serialize](#id859873), [Compose](#id860452), [Construct](#id860557), [Information Models](#id860735), [Representation Graph](#id861060){.preferred}, [Nodes Comparison](#id862121), [Serialization Tree](#id862929), [Keys Order](#id863110), [Anchors and Aliases](#id863390), [Presentation Stream](#id863676), [Node Styles](#id863975), [Scalar Formats](#id864510), [Comments](#id864687), [Directives](#id864824), [Available](#id867229), [Comments](#id892353), [Directives](#id895217), [Node Anchors](#id899912), [Node Tags](#id900262), [Block Chomping Indicator](#id927557), [Mapping Styles](#id932806)

[]{#index-entry-required}required, [Terminology](#id856957){.preferred}

[]{#index-entry-reserved  directive}reserved directive, [Directives](#id864824), [Directives](#id895217){.preferred}

[]{#index-entry-reserved indicator}reserved indicator, [Indicator Characters](#id868988){.preferred}

[]{#index-entry-root node}root node, [Representation Graph](#id861060){.preferred}, [Resolved](#id865585), [YAML Character Stream](#id895107), [Documents](#id898031)
:::

::: indexdiv
### S

[]{#index-entry-scalar}scalar

information model, [Introduction](#id838426){xmlns=""}, [Prior Art](#id838686){xmlns=""}, [Scalars](#id858081){xmlns=""}, [Represent](#id859497){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Tags](#id861700){xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Node Styles](#id863975){xmlns=""}, [Scalar Formats](#id864510){xmlns=""}, [Comments](#id864687){xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Line Break Characters](#id871136){xmlns=""}, [Miscellaneous Characters](#id871856){xmlns=""}, [Escape Sequences](#id872840){xmlns=""}, [Comments](#id892353){xmlns=""}, [Separation Spaces](#id893014){xmlns=""}, [Ignored Line Prefix](#id893482){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Scalar Styles](#id903915){.preferred xmlns=""}, [Double Quoted](#id904245){xmlns=""}, [Plain](#id907281){xmlns=""}, [Block Chomping Indicator](#id927557){xmlns=""}, [Literal](#id928909){xmlns=""}

[]{#index-entry-secondary tag handle}secondary tag handle, [Tag Handles](#id896876){.preferred}

[]{#index-entry-separation space}separation space, [Miscellaneous Characters](#id871856), [Comments](#id892353), [Separation Spaces](#id893014){.preferred}, [Block Sequences](#id931893), [Flow Mappings](#id933010), [Block Mappings](#id934537)

[]{#index-entry-sequence}sequence

information model, [Introduction](#id838426){xmlns=""}, [Prior Art](#id838686){xmlns=""}, [Represent](#id859497){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Tags](#id861700){xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Keys Order](#id863110){xmlns=""}

syntax, [Collection Styles](#id930798){xmlns=""}, [Sequence Styles](#id931088){.preferred xmlns=""}

[]{#index-entry-serialization}serialization, [Processing YAML Information](#id859109), [Serialize](#id859873), [Present](#id860109), [Parse](#id860341), [Compose](#id860452), [Construct](#id860557), [Information Models](#id860735), [Serialization Tree](#id862929){.preferred}, [Anchors and Aliases](#id863390), [Presentation Stream](#id863676), [Node Styles](#id863975), [Scalar Formats](#id864510), [Comments](#id864687), [Directives](#id864824), [Comments](#id892353), [Directives](#id895217), [Node Anchors](#id899912), [Node Tags](#id900262), [Block Chomping Indicator](#id927557), [Mapping Styles](#id932806)

[]{#index-entry-serialization  detail}serialization detail, [Serialize](#id859873){.preferred}, [Compose](#id860452), [Information Models](#id860735), [Keys Order](#id863110), [Anchors and Aliases](#id863390), [Node Anchors](#id899912), [Mapping Styles](#id932806)

[]{#index-entry-serialize}serialize, [Prior Art](#id838686), [Serialize](#id859873){.preferred}, [Compose](#id860452), [Keys Order](#id863110), [Anchors and Aliases](#id863390), [Alias Nodes](#id902561)

[]{#index-entry-shall}shall, [Terminology](#id856957){.preferred}

[]{#index-entry-should}should, [Terminology](#id856957){.preferred}

[]{#index-entry-simple  key}simple key, [Production Parameters](#id867808), [Separation Spaces](#id893014), [Flow Scalar Styles](#id904158), [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281), [Collection Styles](#id930798), [Flow Mappings](#id933010){.preferred}, [Block Mappings](#id934537)

[]{#index-entry-single pair style}single pair style

information model, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Sequence Styles](#id931088){xmlns=""}, [Flow Sequences](#id931285){xmlns=""}, [Flow Mappings](#id933010){.preferred xmlns=""}

[]{#index-entry-single-quoted style}single-quoted style

information model, [Scalars](#id858081){xmlns=""}, [Node Styles](#id863975){.preferred xmlns=""}

syntax, [Production Parameters](#id867808){xmlns=""}, [Indicator Characters](#id868988){xmlns=""}, [Node Content](#id901659){xmlns=""}, [Scalar Styles](#id903915){xmlns=""}, [Single Quoted](#id905860){.preferred xmlns=""}, [Plain](#id907281){xmlns=""}

[]{#index-entry-specific line break}specific line break, [Line Break Characters](#id871136){.preferred}, [Escape Sequences](#id872840), [Line Folding](#id894136)

[]{#index-entry-specific  tag}specific tag, [Resolved](#id865585){.preferred}, [Node Tags](#id900262)

[]{#index-entry-stream}stream

information model, [Prior Art](#id838686){xmlns=""}, [Structures](#id857577){xmlns=""}, [Processing YAML Information](#id859109){xmlns=""}, [Present](#id860109){xmlns=""}, [Parse](#id860341){xmlns=""}, [Presentation Stream](#id863676){.preferred xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Well-Formed and Identified](#id865423){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Productions Conventions](#id867381){xmlns=""}, [Production Parameters](#id867808){xmlns=""}, [Character Set](#id868524){xmlns=""}, [Character Encoding](#id868742){xmlns=""}, [Indentation Spaces](#id891751){xmlns=""}, [YAML Character Stream](#id895107){xmlns=""}, [Tag Prefixes](#id896411){xmlns=""}, [Document Boundary Markers](#id897596){xmlns=""}, [Complete Stream](#id898785){.preferred xmlns=""}, [Nodes](#id899622){xmlns=""}, [Flow Nodes](#id902924){xmlns=""}, [Block Scalar Styles](#id928806){xmlns=""}, [Mapping Styles](#id932806){xmlns=""}, [Flow Mappings](#id933010){xmlns=""}, [Block Mappings](#id934537){xmlns=""}

[]{#index-entry-strip chomping}strip chomping, [Production Parameters](#id867808), [Block Chomping Indicator](#id927557){.preferred}

[]{#index-entry-style}style, [Present](#id860109), [Construct](#id860557), [Information Models](#id860735), [Presentation Stream](#id863676), [Node Styles](#id863975){.preferred}, [Scalar Formats](#id864510), [Resolved](#id865585), [Documents](#id898031)
:::

::: indexdiv
### T

[]{#index-entry-tab}tab, [Prior Art](#id838686), [Character Set](#id868524), [Miscellaneous Characters](#id871856){.preferred}, [Escape Sequences](#id872840), [Indentation Spaces](#id891751), [Comments](#id892353), [Separation Spaces](#id893014), [Ignored Line Prefix](#id893482), [Double Quoted](#id904245), [Single Quoted](#id905860), [Plain](#id907281), [Block Indentation Indicator](#id927035), [Literal](#id928909), [Folded](#id929764)

[]{#index-entry-tag}tag

information model, [Prior Art](#id838686){xmlns=""}, [Tags](#id858600){xmlns=""}, [Represent](#id859497){xmlns=""}, [Present](#id860109){xmlns=""}, [Representation Graph](#id861060){xmlns=""}, [Nodes](#id861435){xmlns=""}, [Tags](#id861700){.preferred xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Scalar Formats](#id864510){xmlns=""}, [Loading Failure Points](#id864977){xmlns=""}, [Resolved](#id865585){xmlns=""}, [Recognized and Valid](#id866900){xmlns=""}, [Available](#id867229){xmlns=""}, [Tag Handles](#id896876){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Miscellaneous Characters](#id871856){xmlns=""}, ["TAG" Directive](#id896044){xmlns=""}, [Tag Prefixes](#id896411){xmlns=""}, [Nodes](#id899622){xmlns=""}, [Node Tags](#id900262){.preferred xmlns=""}

[]{#index-entry-TAG directive}TAG directive, [Tags](#id861700), [Directives](#id864824), [Directives](#id895217), ["TAG" Directive](#id896044){.preferred}, [Node Tags](#id900262)

[]{#index-entry-tag handle}tag handle, [Tags](#id858600), [Present](#id860109), ["TAG" Directive](#id896044), [Tag Prefixes](#id896411), [Tag Handles](#id896876){.preferred}, [Node Tags](#id900262)

[]{#index-entry-tag  prefix}tag prefix, ["TAG" Directive](#id896044), [Tag Prefixes](#id896411){.preferred}, [Node Tags](#id900262)

[]{#index-entry-tag resolution}tag resolution, [Tags](#id861700), [Loading Failure Points](#id864977), [Resolved](#id865585){.preferred}, [Productions Conventions](#id867381), [Node Tags](#id900262), [Scalar Styles](#id903915)

[]{#index-entry-tag shorthand}tag shorthand, [Tags](#id858600), [Productions Conventions](#id867381), [Miscellaneous Characters](#id871856), ["TAG" Directive](#id896044), [Tag Prefixes](#id896411), [Tag Handles](#id896876), [Node Tags](#id900262){.preferred}
:::

::: indexdiv
### U

[]{#index-entry-unavailable  tag}unavailable tag, [Construct](#id860557), [Loading Failure Points](#id864977), [Available](#id867229){.preferred}

[]{#index-entry-unidentified alias}unidentified alias, [Loading Failure Points](#id864977), [Well-Formed and Identified](#id865423){.preferred}

[]{#index-entry-unrecognized tag}unrecognized tag, [Loading Failure Points](#id864977), [Recognized and Valid](#id866900){.preferred}

[]{#index-entry-unresolved tag}unresolved tag, [Loading Failure Points](#id864977), [Resolved](#id865585){.preferred}
:::

::: indexdiv
### V

[]{#index-entry-valid content}valid content, [Recognized and Valid](#id866900){.preferred}

[]{#index-entry-value}value

information model, [Introduction](#id838426){xmlns=""}, [Collections](#id857181){xmlns=""}, [Structures](#id857577){xmlns=""}, [Represent](#id859497){xmlns=""}, [Nodes](#id861435){.preferred xmlns=""}, [Nodes Comparison](#id862121){xmlns=""}, [Keys Order](#id863110){xmlns=""}, [Resolved](#id865585){xmlns=""}

syntax, [Indicator Characters](#id868988){xmlns=""}, [Plain](#id907281){xmlns=""}, [Mapping Styles](#id932806){.preferred xmlns=""}

[]{#index-entry-verbatim  tag}verbatim tag, [Productions Conventions](#id867381), [Node Tags](#id900262){.preferred}
:::

::: indexdiv
### W

[]{#index-entry-well-formed stream}well-formed stream, [Well-Formed and Identified](#id865423){.preferred}

[]{#index-entry-white space}white space, [Miscellaneous Characters](#id871856){.preferred}, [Ignored Line Prefix](#id893482), [Line Folding](#id894136), [Double Quoted](#id904245), [Single Quoted](#id905860), [Folded](#id929764)
:::

::: indexdiv
### Y

[]{#index-entry-YAML  directive}YAML directive, [Directives](#id864824), [Directives](#id895217), ["YAML" Directive](#id895631){.preferred}
:::
::::::::::::::::::::::::::
::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
