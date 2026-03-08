# Chapter 5. Character Productions

> Marked-up copy of YAML 1.2.2 specification Chapter 5, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

## 5.1. Character Set

To ensure readability, YAML [streams] use only the _printable_ subset of the
Unicode character set.
The allowed character range explicitly excludes the C0 control block[^c0-block]
`x00-x1F` (except for TAB `x09`, LF `x0A` and CR `x0D` which are allowed), DEL
`x7F`, the C1 control block `x80-x9F` (except for NEL `x85` which is allowed),
the surrogate block[^surrogates] `xD800-xDFFF`, `xFFFE` and `xFFFF`.

y[char.set.input-accept]
On input, a YAML [processor] must accept all characters in this printable
subset.

y[char.set.output-produce]
On output, a YAML [processor] must only produce only characters in this
printable subset.

y[char.set.escape-outside]
Characters outside this set must be [presented] using [escape] sequences.

In addition, any allowed characters known to be non-printable should also be
[escaped].

> Note: This isn't mandatory since a full implementation would require
extensive character property tables.

y[char.c-printable+3]
```
[#] c-printable ::=
                         # 8 bit
    x09                  # Tab (\t)
  | x0A                  # Line feed (LF \n)
  | x0D                  # Carriage Return (CR \r)
  | [x20-x7E]            # Printable ASCII
                         # 16 bit
  | x85                  # Next Line (NEL)
  | [xA0-xD7FF]          # Basic Multilingual Plane (BMP)
  | [xE000-xFFFD]        # Additional Unicode Areas
  | [x010000-x10FFFF]    # 32 bit
```


y[char.set.json-compat+3]
To ensure [JSON compatibility], YAML [processors] must allow all non-C0
characters inside [quoted scalars].
To ensure readability, non-printable characters should be [escaped] on output,
even inside such [scalars].

> Note: JSON [quoted scalars] cannot span multiple lines or contain [tabs], but
YAML [quoted scalars] can.

y[char.nb-json+3]
```
[#] nb-json ::=
    x09              # Tab character
  | [x20-x10FFFF]    # Non-C0-control characters
```

> Note: The production name `nb-json` means "non-break JSON compatible" here.


## 5.2. Character Encodings

All characters mentioned in this specification are Unicode code points.
Each such code point is written as one or more bytes depending on the
_character encoding_ used.
Note that in UTF-16, characters above `xFFFF` are written as four bytes, using
a surrogate pair.

y[char.encoding.not-content]
The character encoding is a [presentation detail] and must not be used to
convey [content] information.

y[char.encoding.input-utf8-utf16+3]
On input, a YAML [processor] must support the UTF-8 and UTF-16 character
encodings.

y[char.encoding.json-utf32+3]
For [JSON compatibility], the UTF-32 encodings must also be supported.

y[char.encoding.bom-detection]
If a character [stream] begins with a _byte order mark_, the character encoding
will be taken to be as indicated by the byte order mark.

y[char.encoding.ascii-first]
Otherwise, the [stream] must begin with an ASCII character.
This allows the encoding to be deduced by the pattern of null (`x00`)
characters.

y[char.encoding.same-encoding]
Byte order marks may appear at the start of any [document], however all
[documents] in the same [stream] must use the same character encoding.

y[char.encoding.bom-in-quoted+3]
To allow for [JSON compatibility], byte order marks are also allowed inside
[quoted scalars].
For readability, such [content] byte order marks should be [escaped] on output.

The encoding can therefore be deduced by matching the first few bytes of the
[stream] with the following table rows (in order):


|                       | Byte0 | Byte1 | Byte2 | Byte3 | Encoding
| --                    | --    | --    | --    | --    | --
| Explicit BOM          | x00   | x00   | xFE   | xFF   | UTF-32BE
| ASCII first character | x00   | x00   | x00   | any   | UTF-32BE
| Explicit BOM          | xFF   | xFE   | x00   | x00   | UTF-32LE
| ASCII first character | any   | x00   | x00   | x00   | UTF-32LE
| Explicit BOM          | xFE   | xFF   |       |       | UTF-16BE
| ASCII first character | x00   | any   |       |       | UTF-16BE
| Explicit BOM          | xFF   | xFE   |       |       | UTF-16LE
| ASCII first character | any   | x00   |       |       | UTF-16LE
| Explicit BOM          | xEF   | xBB   | xBF   |       | UTF-8
| Default               |       |       |       |       | UTF-8


The recommended output encoding is UTF-8.
If another encoding is used, it is recommended that an explicit byte order mark
be used, even if the first [stream] character is ASCII.

For more information about the byte order mark and the Unicode character
encoding schemes see the Unicode FAQ[^uni-faq].

y[char.c-byte-order-mark]
```
[#] c-byte-order-mark ::= xFEFF
```


In the examples, byte order mark characters are displayed as "`⇔`".


**Example #. Byte Order Mark**

```
⇔# Comment only.

```

```
# This stream contains no
# documents, only comments.
```

**Legend:**
* [c-byte-order-mark] <!-- 1:1 -->


**Example #. Invalid Byte Order Mark**

```
- Invalid use of BOM
⇔
- Inside a document.
```
<!-- ⇔ -->

```
ERROR:
 A BOM must not appear
 inside a document.
```
<!-- BOM -->


## 5.3. Indicator Characters

_Indicators_ are characters that have special semantics.

"`-`" (`x2D`, hyphen) denotes a [block sequence] entry.

y[char.c-sequence-entry]
```
[#] c-sequence-entry ::= '-'
```


"`?`" (`x3F`, question mark) denotes a [mapping key].

y[char.c-mapping-key]
```
[#] c-mapping-key ::= '?'
```


"`:`" (`x3A`, colon) denotes a [mapping value].

y[char.c-mapping-value]
```
[#] c-mapping-value ::= ':'
```


**Example #. Block Structure Indicators**

```
sequence:
- one
- two
mapping:
  ? sky
  : blue
  sea : green
```

```
{ "sequence": [
    "one",
    "two" ],
  "mapping": {
    "sky": "blue",
    "sea": "green" } }
```

**Legend:**
* [c-sequence-entry] <!-- - -->
* [c-mapping-key] <!-- ? -->
* [c-mapping-value] <!-- : -->


"`,`" (`x2C`, comma) ends a [flow collection] entry.

y[char.c-collect-entry]
```
[#] c-collect-entry ::= ','
```


"`[`" (`x5B`, left bracket) starts a [flow sequence].

y[char.c-sequence-start]
```
[#] c-sequence-start ::= '['
```


"`]`" (`x5D`, right bracket) ends a [flow sequence].

y[char.c-sequence-end]
```
[#] c-sequence-end ::= ']'
```


"`{`" (`x7B`, left brace) starts a [flow mapping].

y[char.c-mapping-start]
```
[#] c-mapping-start ::= '{'
```


"`}`" (`x7D`, right brace) ends a [flow mapping].

y[char.c-mapping-end]
```
[#] c-mapping-end ::= '}'
```


**Example #. Flow Collection Indicators**

```
sequence: [ one, two, ]
mapping: { sky: blue, sea: green }
```

```
{ "sequence": [ "one", "two" ],
  "mapping":
    { "sky": "blue", "sea": "green" } }
```

**Legend:**
* [c-sequence-start] [c-sequence-end] <!-- [ ] -->
* [c-mapping-start] [c-mapping-end] <!-- { } -->
* [c-collect-entry] <!-- , -->


"`#`" (`x23`, octothorpe, hash, sharp, pound, number sign) denotes a [comment].

y[char.c-comment]
```
[#] c-comment ::= '#'
```


**Example #. Comment Indicator**

```
# Comment only.

```

```
# This stream contains no
# documents, only comments.
```

**Legend:**
* [c-comment] <!-- # -->


"`&`" (`x26`, ampersand) denotes a [node's anchor property].

y[char.c-anchor]
```
[#] c-anchor ::= '&'
```

"`*`" (`x2A`, asterisk) denotes an [alias node].

y[char.c-alias]
```
[#] c-alias ::= '*'
```


The "`!`" (`x21`, exclamation) is used for specifying [node tags].
It is used to denote [tag handles] used in [tag directives] and [tag
properties]; to denote [local tags]; and as the [non-specific tag] for
non-[plain scalars].

y[char.c-tag+2]
```
[#] c-tag ::= '!'
```


**Example #. Node Property Indicators**

```
anchored: !local &anchor value
alias: *anchor
```

```
{ "anchored": !local &A1 "value",
  "alias": *A1 }
```

**Legend:**
* [c-tag] <!-- ! -->
* [c-anchor] <!-- & -->
* [c-alias] <!-- * -->


"`|`" (`7C`, vertical bar) denotes a [literal block scalar].

y[char.c-literal]
```
[#] c-literal ::= '|'
```


"`>`" (`x3E`, greater than) denotes a [folded block scalar].

y[char.c-folded]
```
[#] c-folded ::= '>'
```


**Example #. Block Scalar Indicators**

```
literal: |
  some
  text
folded: >
  some
  text
```

```
{ "literal": "some\ntext\n",
  "folded": "some text\n" }
```

**Legend:**
* [c-literal] <!-- | -->
* [c-folded] <!-- > -->

"`'`" (`x27`, apostrophe, single quote) surrounds a [single-quoted flow
scalar].

y[char.c-single-quote]
```
[#] c-single-quote ::= "'"
```


"`"`" (`x22`, double quote) surrounds a [double-quoted flow scalar].

y[char.c-double-quote]
```
[#] c-double-quote ::= '"'
```


**Example #. Quoted Scalar Indicators**

```
single: 'text'
double: "text"
```

```
{ "single": "text",
  "double": "text" }
```

**Legend:**
* [c-single-quote] <!-- ' -->
* [c-double-quote] <!-- 2:9 2:14 -->


"`%`" (`x25`, percent) denotes a [directive] line.

y[char.c-directive]
```
[#] c-directive ::= '%'
```


**Example #. Directive Indicator**

```
%YAML 1.2
--- text
```

```
"text"
```

**Legend:**
* [c-directive] <!-- % -->


The "`@`" (`x40`, at) and "<code>&grave;</code>" (`x60`, grave accent) are
_reserved_ for future use.

y[char.c-reserved]
```
[#] c-reserved ::=
    '@' | '`'
```


**Example #. Invalid use of Reserved Indicators**

```
commercial-at: @text
grave-accent: `text
```
<!-- @ ` -->

```
ERROR:
 Reserved indicators can't
 start a plain scalar.
```
<!-- Reserved_indicators -->


Any indicator character:

y[char.c-indicator]
```
[#] c-indicator ::=
    c-sequence-entry    # '-'
  | c-mapping-key       # '?'
  | c-mapping-value     # ':'
  | c-collect-entry     # ','
  | c-sequence-start    # '['
  | c-sequence-end      # ']'
  | c-mapping-start     # '{'
  | c-mapping-end       # '}'
  | c-comment           # '#'
  | c-anchor            # '&'
  | c-alias             # '*'
  | c-tag               # '!'
  | c-literal           # '|'
  | c-folded            # '>'
  | c-single-quote      # "'"
  | c-double-quote      # '"'
  | c-directive         # '%'
  | c-reserved          # '@' '`'
```


The "`[`", "`]`", "`{`", "`}`" and "`,`" indicators denote structure in [flow
collections].
They are therefore forbidden in some cases, to avoid ambiguity in several
constructs.
This is handled on a case-by-case basis by the relevant productions.

y[char.c-flow-indicator]
```
[#] c-flow-indicator ::=
    c-collect-entry     # ','
  | c-sequence-start    # '['
  | c-sequence-end      # ']'
  | c-mapping-start     # '{'
  | c-mapping-end       # '}'
```


## 5.4. Line Break Characters

YAML recognizes the following ASCII _line break_ characters.

y[char.b-line-feed+3]
```
[#] b-line-feed ::= x0A
```


y[char.b-carriage-return]
```
[#] b-carriage-return ::= x0D
```


y[char.b-char+3]
```
[#] b-char ::=
    b-line-feed          # x0A
  | b-carriage-return    # X0D
```


All other characters, including the form feed (`x0C`), are considered to be
non-break characters.
Note that these include the _non-ASCII line breaks_: next line (`x85`), line
separator (`x2028`) and paragraph separator (`x2029`).

y[char.line-break.non-ascii-compat+3]
[YAML version 1.1] did support the above non-ASCII line break characters;
however, JSON does not.
Hence, to ensure [JSON compatibility], YAML treats them as non-break characters
as of version 1.2.

y[char.line-break.v11-warning+3]
YAML 1.2 [processors] [parsing] a [version 1.1] [document] should therefore
treat these line breaks as non-break characters, with an appropriate warning.

y[char.nb-char]
```
[#] nb-char ::=
  c-printable - b-char - c-byte-order-mark
```


Line breaks are interpreted differently by different systems and have multiple
widely used formats.

y[char.b-break]
```
[#] b-break ::=
    (
      b-carriage-return  # x0A
      b-line-feed
    )                    # x0D
  | b-carriage-return
  | b-line-feed
```


y[char.line-break.normalize]
Line breaks inside [scalar content] must be _normalized_ by the YAML
[processor].

y[char.line-break.parse-as-lf]
Each such line break must be [parsed] into a single line feed character.

y[char.line-break.format-not-content]
The original line break format is a [presentation detail] and must not be used
to convey [content] information.

y[char.b-as-line-feed]
```
[#] b-as-line-feed ::=
  b-break
```


Outside [scalar content], YAML allows any line break to be used to terminate
lines.

y[char.b-non-content]
```
[#] b-non-content ::=
  b-break
```


On output, a YAML [processor] is free to emit line breaks using whatever
convention is most appropriate.

In the examples, line breaks are sometimes displayed using the "`↓`" glyph for
clarity.


**Example #. Line Break Characters**

```
|
  Line break (no glyph)
  Line break (glyphed)↓
```

```
"Line break (no glyph)\nLine break (glyphed)\n"
```

**Legend:**
* [b-break] <!-- ↓ -->


## 5.5. White Space Characters

YAML recognizes two _white space_ characters: _space_ and _tab_.

y[char.s-space]
```
[#] s-space ::= x20
```

y[char.s-tab]
```
[#] s-tab ::= x09
```

y[char.s-white]
```
[#] s-white ::=
  s-space | s-tab
```


The rest of the ([printable]) non-[break] characters are considered to be
non-space characters.

y[char.ns-char]
```
[#] ns-char ::=
  nb-char - s-white
```


In the examples, tab characters are displayed as the glyph "`→`".
Space characters are sometimes displayed as the glyph "`·`" for clarity.


**Example #. Tabs and Spaces**

```
# Tabs and spaces
quoted:·"Quoted →"
block:→|
··void main() {
··→printf("Hello, world!\n");
··}
```

```
{ "quoted": "Quoted \t",
  "block": "void main()
    {\n\tprintf(\"Hello, world!\\n\");\n}\n" }
```

**Legend:**
* [s-space] <!-- ·· · -->
* [s-tab] <!-- → -->


## 5.6. Miscellaneous Characters

The YAML syntax productions make use of the following additional character
classes:

A decimal digit for numbers:

y[char.ns-dec-digit]
```
[#] ns-dec-digit ::=
  [x30-x39]             # 0-9
```


A hexadecimal digit for [escape sequences]:

y[char.ns-hex-digit]
```
[#] ns-hex-digit ::=
    ns-dec-digit        # 0-9
  | [x41-x46]           # A-F
  | [x61-x66]           # a-f
```


ASCII letter (alphabetic) characters:

y[char.ns-ascii-letter]
```
[#] ns-ascii-letter ::=
    [x41-x5A]           # A-Z
  | [x61-x7A]           # a-z
```


Word (alphanumeric) characters for identifiers:

y[char.ns-word-char]
```
[#] ns-word-char ::=
    ns-dec-digit        # 0-9
  | ns-ascii-letter     # A-Z a-z
  | '-'                 # '-'
```

URI characters for [tags], as defined in the URI specification[^uri].

By convention, any URI characters other than the allowed printable ASCII
characters are first _encoded_ in UTF-8 and then each byte is _escaped_ using
the "`%`" character.

y[char.misc.uri-no-expand]
The YAML [processor] must not expand such escaped characters.

y[char.misc.tag-preserve+2]
[Tag] characters must be preserved and compared exactly as [presented] in the
YAML [stream], without any processing.

y[char.ns-uri-char]
```
[#] ns-uri-char ::=
    (
      '%'
      ns-hex-digit{2}
    )
  | ns-word-char
  | '#'
  | ';'
  | '/'
  | '?'
  | ':'
  | '@'
  | '&'
  | '='
  | '+'
  | '$'
  | ','
  | '_'
  | '.'
  | '!'
  | '~'
  | '*'
  | "'"
  | '('
  | ')'
  | '['
  | ']'
```


The "`!`" character is used to indicate the end of a [named tag handle]; hence
its use in [tag shorthands] is restricted.

y[char.misc.tag-shorthand-restrict+3]
In addition, such [shorthands] must not contain the "`[`", "`]`", "`{`", "`}`"
and "`,`" characters.
These characters would cause ambiguity with [flow collection] structures.

y[char.ns-tag-char+3]
```
[#] ns-tag-char ::=
    ns-uri-char
  - c-tag               # '!'
  - c-flow-indicator
```


## 5.7. Escaped Characters

y[char.escape.must-escape]
All non-[printable] characters must be _escaped_.
YAML escape sequences use the "`\`" notation common to most modern computer
languages.

y[char.escape.parse-to-unicode]
Each escape sequence must be [parsed] into the appropriate Unicode character.

y[char.escape.not-content]
The original escape sequence is a [presentation detail] and must not be used to
convey [content] information.

Note that escape sequences are only interpreted in [double-quoted scalars].
In all other [scalar styles], the "`\`" character has no special meaning and
non-[printable] characters are not available.

y[char.c-escape]
```
[#] c-escape ::= '\'
```


YAML escape sequences are a superset of C's escape sequences:

Escaped ASCII null (`x00`) character.

y[char.ns-esc-null]
```
[#] ns-esc-null ::= '0'
```


Escaped ASCII bell (`x07`) character.

y[char.ns-esc-bell]
```
[#] ns-esc-bell ::= 'a'
```


Escaped ASCII backspace (`x08`) character.

y[char.ns-esc-backspace]
```
[#] ns-esc-backspace ::= 'b'
```


Escaped ASCII horizontal tab (`x09`) character.
This is useful at the start or the end of a line to force a leading or trailing
tab to become part of the [content].

y[char.ns-esc-horizontal-tab+2]
```
[#] ns-esc-horizontal-tab ::=
  't' | x09
```


Escaped ASCII line feed (`x0A`) character.

y[char.ns-esc-line-feed]
```
[#] ns-esc-line-feed ::= 'n'
```


Escaped ASCII vertical tab (`x0B`) character.

y[char.ns-esc-vertical-tab]
```
[#] ns-esc-vertical-tab ::= 'v'
```


Escaped ASCII form feed (`x0C`) character.

y[char.ns-esc-form-feed]
```
[#] ns-esc-form-feed ::= 'f'
```


Escaped ASCII carriage return (`x0D`) character.

y[char.ns-esc-carriage-return]
```
[#] ns-esc-carriage-return ::= 'r'
```


Escaped ASCII escape (`x1B`) character.

y[char.ns-esc-escape]
```
[#] ns-esc-escape ::= 'e'
```


Escaped ASCII space (`x20`) character.
This is useful at the start or the end of a line to force a leading or trailing
space to become part of the [content].

y[char.ns-esc-space]
```
[#] ns-esc-space ::= x20
```


Escaped ASCII double quote (`x22`).

y[char.ns-esc-double-quote]
```
[#] ns-esc-double-quote ::= '"'
```


Escaped ASCII slash (`x2F`), for [JSON compatibility].

y[char.ns-esc-slash+3]
```
[#] ns-esc-slash ::= '/'
```


Escaped ASCII back slash (`x5C`).

y[char.ns-esc-backslash]
```
[#] ns-esc-backslash ::= '\'
```


Escaped Unicode next line (`x85`) character.

y[char.ns-esc-next-line]
```
[#] ns-esc-next-line ::= 'N'
```


Escaped Unicode non-breaking space (`xA0`) character.

y[char.ns-esc-non-breaking-space]
```
[#] ns-esc-non-breaking-space ::= '_'
```


Escaped Unicode line separator (`x2028`) character.

y[char.ns-esc-line-separator]
```
[#] ns-esc-line-separator ::= 'L'
```


Escaped Unicode paragraph separator (`x2029`) character.

y[char.ns-esc-paragraph-separator]
```
[#] ns-esc-paragraph-separator ::= 'P'
```


Escaped 8-bit Unicode character.

y[char.ns-esc-8-bit]
```
[#] ns-esc-8-bit ::=
  'x'
  ns-hex-digit{2}
```


Escaped 16-bit Unicode character.

y[char.ns-esc-16-bit]
```
[#] ns-esc-16-bit ::=
  'u'
  ns-hex-digit{4}
```


Escaped 32-bit Unicode character.

y[char.ns-esc-32-bit]
```
[#] ns-esc-32-bit ::=
  'U'
  ns-hex-digit{8}
```


Any escaped character:

y[char.c-ns-esc-char+3]
```
[#] c-ns-esc-char ::=
  c-escape         # '\'
  (
      ns-esc-null
    | ns-esc-bell
    | ns-esc-backspace
    | ns-esc-horizontal-tab
    | ns-esc-line-feed
    | ns-esc-vertical-tab
    | ns-esc-form-feed
    | ns-esc-carriage-return
    | ns-esc-escape
    | ns-esc-space
    | ns-esc-double-quote
    | ns-esc-slash
    | ns-esc-backslash
    | ns-esc-next-line
    | ns-esc-non-breaking-space
    | ns-esc-line-separator
    | ns-esc-paragraph-separator
    | ns-esc-8-bit
    | ns-esc-16-bit
    | ns-esc-32-bit
  )
```


**Example #. Escaped Characters**

```
- "Fun with \\"
- "\" \a \b \e \f"
- "\n \r \t \v \0"
- "\  \_ \N \L \P \
  \x41 \u0041 \U00000041"
```

```
[ "Fun with \\",
  "\" \u0007 \b \u001b \f",
  "\n \r \t \u000b \u0000",
  "\u0020 \u00a0 \u0085 \u2028 \u2029 A A A" ]
```

**Legend:**
* [c-ns-esc-char] <!-- \\ \" \a \b \e \f \↓ \n \r \t \v \0 4:4,2 4:7,2 \N \L \P \x41 \u0041 \U00000041 -->


**Example #. Invalid Escaped Characters**

```
Bad escapes:
  "\c
  \xq-"
```
<!-- 2:5 -->
<!-- 3:5,2 -->

```
ERROR:
- c is an invalid escaped character.
- q and - are invalid hex digits.
```
<!-- 2:3 -->
<!-- 3:3 3:9 -->
