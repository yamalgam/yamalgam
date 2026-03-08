# Chapter 10. Recommended Schemas

> Marked-up copy of YAML 1.2.2 specification Chapter 10, with tracey requirement markers.
> Source: [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/)
> Prefix: `yaml12`

A YAML _schema_ is a combination of a set of [tags] and a mechanism for
[resolving] [non-specific tags].


## 10.1. Failsafe Schema

The _failsafe schema_ is guaranteed to work with any YAML [document].
It is therefore the recommended [schema] for generic YAML tools.

A YAML [processor] should therefore support this [schema], at least as an
option.


### 10.1.1. Tags

#### 10.1.1.1. Generic Mapping

URI
:
`tag:yaml.org,2002:map`


Kind
:
[Mapping].


Definition
:
[Represents] an associative container, where each [key] is unique in the
association and mapped to exactly one [value].
YAML places no restrictions on the type of [keys]; in particular, they are not
restricted to being [scalars].
Example [bindings] to [native] types include Perl's hash, Python's dictionary
and Java's Hashtable.


**Example #. `!!map` Examples**

```
Block style: !!map
  Clark : Evans
  Ingy  : döt Net
  Oren  : Ben-Kiki

Flow style: !!map { Clark: Evans, Ingy: döt Net, Oren: Ben-Kiki }
```


#### 10.1.1.2. Generic Sequence

URI
:
`tag:yaml.org,2002:seq`


Kind
:
[Sequence].


Definition
:
[Represents] a collection indexed by sequential integers starting with zero.
Example [bindings] to [native] types include Perl's array, Python's list or
tuple and Java's array or Vector.


**Example #. `!!seq` Examples**

```
Block style: !!seq
- Clark Evans
- Ingy döt Net
- Oren Ben-Kiki

Flow style: !!seq [ Clark Evans, Ingy döt Net, Oren Ben-Kiki ]
```


#### 10.1.1.3. Generic String

URI
:
`tag:yaml.org,2002:str`


Kind
:
[Scalar].


Definition
:
[Represents] a Unicode string, a sequence of zero or more Unicode characters.
This type is usually [bound] to the [native] language's string type or, for
languages lacking one (such as C), to a character array.


Canonical Form:
:
The obvious.


**Example #. `!!str` Examples**

```
Block style: !!str |-
  String: just a theory.

Flow style: !!str "String: just a theory."
```


### 10.1.2. Tag Resolution

All [nodes] with the "`!`" non-specific tag are [resolved], by the standard
[convention], to "`tag:yaml.org,2002:seq`", "`tag:yaml.org,2002:map`" or
"`tag:yaml.org,2002:str`", according to their [kind].

All [nodes] with the "`?`" non-specific tag are left [unresolved].
This constrains the [application] to deal with a [partial representation].


## 10.2. JSON Schema

The _JSON schema_ is the lowest common denominator of most modern computer
languages and allows [parsing] JSON files.

A YAML [processor] should therefore support this [schema], at least as an
option.
It is also strongly recommended that other [schemas] should be based on it.


### 10.2.1. Tags

The JSON [schema] uses the following [tags] in addition to those defined by the
[failsafe] schema:


#### 10.2.1.1. Null

URI
:
`tag:yaml.org,2002:null`


Kind
:
[Scalar].


Definition
:
[Represents] the lack of a value.
This is typically [bound] to a [native] null-like value (e.g., `undef` in Perl,
`None` in Python).
Note that a null is different from an empty string.
Also, a [mapping] entry with some [key] and a null [value] is valid and
different from not having that [key] in the [mapping].


Canonical Form
:
`null`.

**Example #. `!!null` Examples**

```
!!null null: value for null key
key with null value: !!null null
```


#### 10.2.1.2. Boolean

URI
:
`tag:yaml.org,2002:bool`


Kind
:
[Scalar].


Definition
:
[Represents] a true/false value.
In languages without a [native] Boolean type (such as C), they are usually
[bound] to a native integer type, using one for true and zero for false.


Canonical Form
:
Either `true` or `false`.


**Example #. `!!bool` Examples**

```
YAML is a superset of JSON: !!bool true
Pluto is a planet: !!bool false
```


#### 10.2.1.3. Integer

URI
:
`tag:yaml.org,2002:int`


Kind
:
[Scalar].


Definition
:
[Represents] arbitrary sized finite mathematical integers.
Scalars of this type should be [bound] to a [native] integer data type, if
possible.
:
Some languages (such as Perl) provide only a "number" type that allows for both
integer and floating-point values.
A YAML [processor] may use such a type for integers as long as they round-trip
properly.
:
In some languages (such as C), an integer may overflow the [native] type's
storage capability.

A YAML [processor] may reject such a value as an error, truncate it with a
warning or find some other manner to round-trip it.
In general, integers representable using 32 binary digits should safely
round-trip through most systems.


Canonical Form
:
Decimal integer notation, with a leading "`-`" character for negative values,
matching the regular expression `0 | -? [1-9] [0-9]*`


**Example #. `!!int` Examples**

```
negative: !!int -12
zero: !!int 0
positive: !!int 34
```


#### 10.2.1.4. Floating Point

URI
:
`tag:yaml.org,2002:float`


Kind
:
[Scalar].


Definition
:
[Represents] an approximation to real numbers, including three special values
(positive and negative infinity and "not a number").
:
Some languages (such as Perl) provide only a "number" type that allows for both
integer and floating-point values.
A YAML [processor] may use such a type for floating-point numbers, as long as
they round-trip properly.
:
Not all floating-point values can be stored exactly in any given [native] type.
Hence a float value may change by "a small amount" when round-tripped.
The supported range and accuracy depends on the implementation, though 32 bit
IEEE floats should be safe.

Since YAML does not specify a particular accuracy, using floating-point
[mapping keys] requires great care and is not recommended.


Canonical Form
:
Either `0`, `.inf`, `-.inf`, `.nan` or scientific notation matching the regular
expression
`-? [1-9] ( \. [0-9]* [1-9] )? ( e [-+] [1-9] [0-9]* )?`.


**Example #. `!!float` Examples**

```
negative: !!float -1
zero: !!float 0
positive: !!float 2.3e4
infinity: !!float .inf
not a number: !!float .nan
```


### 10.2.2. Tag Resolution

The [JSON schema] [tag resolution] is an extension of the [failsafe schema]
[tag resolution].

All [nodes] with the "`!`" non-specific tag are [resolved], by the standard
[convention], to "`tag:yaml.org,2002:seq`", "`tag:yaml.org,2002:map`" or
"`tag:yaml.org,2002:str`", according to their [kind].

[Collections] with the "`?`" non-specific tag (that is, [untagged]
[collections]) are [resolved] to "`tag:yaml.org,2002:seq`" or
"`tag:yaml.org,2002:map`" according to their [kind].

[Scalars] with the "`?`" non-specific tag (that is, [plain scalars]) are
matched with a list of regular expressions (first match wins, e.g. `0` is
resolved as `!!int`).

In principle, JSON files should not contain any [scalars] that do not match at
least one of these.
Hence the YAML [processor] should consider them to be an error.


| Regular expression        | Resolved to tag
| --                        | --
| `null`                    | tag:yaml.org,2002:null
| `true | false`            | tag:yaml.org,2002:bool
| `-? ( 0 | [1-9] [0-9]* )` | tag:yaml.org,2002:int
| `-? ( 0 | [1-9] [0-9]* ) ( \. [0-9]* )? ( [eE] [-+]? [0-9]+ )?` | tag:yaml.org,2002:float
| `*`                       | Error

> Note: The regular expression for `float` does not exactly match the one in
the JSON specification, where at least one digit is required after the dot: `(
\.  [0-9]+ )`.  The YAML 1.2 specification intended to match JSON behavior, but
this cannot be addressed in the 1.2.2 specification.

**Example #. JSON Tag Resolution**

```
A null: null
Booleans: [ true, false ]
Integers: [ 0, -0, 3, -19 ]
Floats: [ 0., -0.0, 12e03, -2E+05 ]
Invalid: [ True, Null,
  0o7, 0x3A, +12.3 ]
```

```
{ "A null": null,
  "Booleans": [ true, false ],
  "Integers": [ 0, 0, 3, -19 ],
  "Floats": [ 0.0, -0.0, 12000, -200000 ],
  "Invalid": [ "True", "Null",
    "0o7", "0x3A", "+12.3" ] }
```


## 10.3. Core Schema

The _Core schema_ is an extension of the [JSON schema], allowing for more
human-readable [presentation] of the same types.

This is the recommended default [schema] that YAML [processor] should use
unless instructed otherwise.
It is also strongly recommended that other [schemas] should be based on it.


### 10.3.1. Tags

The core [schema] uses the same [tags] as the [JSON schema].


### 10.3.2. Tag Resolution

The [core schema] [tag resolution] is an extension of the [JSON schema] [tag
resolution].

All [nodes] with the "`!`" non-specific tag are [resolved], by the standard
[convention], to "`tag:yaml.org,2002:seq`", "`tag:yaml.org,2002:map`" or
"`tag:yaml.org,2002:str`", according to their [kind].

[Collections] with the "`?`" non-specific tag (that is, [untagged]
[collections]) are [resolved] to "`tag:yaml.org,2002:seq`" or
"`tag:yaml.org,2002:map`" according to their [kind].

[Scalars] with the "`?`" non-specific tag (that is, [plain scalars]) are
matched with an extended list of regular expressions.

However, in this case, if none of the regular expressions matches, the [scalar]
is [resolved] to `tag:yaml.org,2002:str` (that is, considered to be a string).


| Regular expression                | Resolved to tag
| --                                | --
| `null | Null | NULL | ~`          | tag:yaml.org,2002:null
| `/* Empty */`                     | tag:yaml.org,2002:null
| `true | True | TRUE | false | False | FALSE` | tag:yaml.org,2002:bool
| `[-+]? [0-9]+`                    | tag:yaml.org,2002:int (Base 10)
| `0o [0-7]+`                       | tag:yaml.org,2002:int (Base 8)
| `0x [0-9a-fA-F]+`                 | tag:yaml.org,2002:int (Base 16)
| `[-+]? ( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?` | tag:yaml.org,2002:float (Number)
| `[-+]? ( \.inf | \.Inf | \.INF )` | tag:yaml.org,2002:float (Infinity)
| `\.nan | \.NaN | \.NAN`           | tag:yaml.org,2002:float (Not a number)
| `*`                               | tag:yaml.org,2002:str (Default)


**Example #. Core Tag Resolution**

```
A null: null
Also a null: # Empty
Not a null: ""
Booleans: [ true, True, false, FALSE ]
Integers: [ 0, 0o7, 0x3A, -19 ]
Floats: [
  0., -0.0, .5, +12e03, -2E+05 ]
Also floats: [
  .inf, -.Inf, +.INF, .NAN ]
```
```
{ "A null": null,
  "Also a null": null,
  "Not a null": "",
  "Booleans": [ true, true, false, false ],
  "Integers": [ 0, 7, 58, -19 ],
  "Floats": [
    0.0, -0.0, 0.5, 12000, -200000 ],
  "Also floats": [
    Infinity, -Infinity, Infinity, NaN ] }
```


## 10.4. Other Schemas

None of the above recommended [schemas] preclude the use of arbitrary explicit
[tags].
Hence YAML [processors] for a particular programming language typically provide
some form of [local tags] that map directly to the language's [native data
structures] (e.g., `!ruby/object:Set`).

While such [local tags] are useful for ad hoc [applications], they do not
suffice for stable, interoperable cross-[application] or cross-platform data
exchange.

Interoperable [schemas] make use of [global tags] (URIs) that [represent] the
same data across different programming languages.
In addition, an interoperable [schema] may provide additional [tag resolution]
rules.
Such rules may provide additional regular expressions, as well as consider the
path to the [node].
This allows interoperable [schemas] to use [untagged] [nodes].

It is strongly recommended that such [schemas] be based on the [core schema]
defined above.
