# occurrence-types Specification

## Purpose
Defines how many values a type admits. One occurrence suffix with four cardinalities — `T`, `T?`,
`T+`, `T*` — replaces the `[]` and nullable suffixes; `{}` is the one absent value; and every site
that joins, collects or iterates computes an occurrence by one small lattice.
## Requirements
### Requirement: A type reference carries at most one occurrence suffix
Anywhere NX accepts a type reference, the system SHALL allow a primitive, qualified, user-defined,
function, applied or parenthesized base type followed by at most one occurrence suffix. The suffixes
SHALL be `?` (zero or one), `+` (one or more) and `*` (zero or more). A base type with no suffix
SHALL denote exactly one value. A parenthesized type SHALL be `(`, a type reference, `)`, and SHALL
denote the type it encloses. A suffix written after a function type's result SHALL apply to the
result, so a suffix on the function type itself requires parentheses.

The system SHALL reject a second suffix wherever it appears along a type reference chain — in the
same chain, across a parenthesis, on a function type's result, or through an alias whose target
already carries a suffix — explaining that the type already carries an occurrence. There is no
distinction between an optional sequence and a sequence of optionals: neither is expressible.

The `[]` suffix SHALL be rejected with a diagnostic that names `*` and `+` as the spellings for a
sequence. The system SHALL NOT accept `null` as a literal, a pattern, a value or a type.

#### Scenario: Each suffix is accepted once
- **WHEN** a file contains `type A = string?`, `type B = string+`, `type C = string*` and `type D = (string)+`
- **THEN** parsing and lowering SHALL accept all four
- **AND** `D` SHALL be the same type as `B`

#### Scenario: A second suffix is rejected wherever it is written
- **WHEN** a file contains `type E = string??`, `type F = string?+`, `type G = (string+)*`, `type H = <function />: int*?` and `type I = A?` with `type A = string?`
- **THEN** the system SHALL reject each on its second suffix, or on the suffix applied to `A`
- **AND** each diagnostic SHALL explain that the type already carries an occurrence

#### Scenario: The list suffix is rejected with the replacement spellings
- **WHEN** a file contains `type Names = string[]`
- **THEN** parsing SHALL produce a validation error on `[]`
- **AND** the diagnostic SHALL name `string*` and `string+`

#### Scenario: A suffix after a function result binds to the result
- **WHEN** a file contains `type Loader = <function />: string+` and `type Loaders = (<function />: string)+`
- **THEN** `Loader` SHALL be exactly one function whose result is one or more strings
- **AND** `Loaders` SHALL be one or more functions, each returning exactly one string

#### Scenario: Null is not a literal
- **WHEN** a file contains `let x = null`
- **THEN** parsing SHALL reject `null` as an unknown name, or with a diagnostic that names `{}` as the absent value

### Requirement: Occurrences form a lattice that decides satisfaction
The four occurrences SHALL be ordered with exactly-one below `?` and `+`, and `?` and `+` below `*`.
A value of type `S` with occurrence `o1` SHALL satisfy an expected type `T` with occurrence `o2`
exactly when `S` satisfies `T` and `o1` is at or below `o2` in that order. An exactly-one value at a
suffixed site SHALL be accepted as a sequence of one, which is the scalar-to-sequence lift the
language already has; that lift SHALL apply once, since no suffixed type is an item type.

Where a value does not satisfy the expected occurrence, the diagnostic SHALL name both types by
their NX spelling.

#### Scenario: Exactly one satisfies every occurrence
- **WHEN** a file contains `let a:int? = 1`, `let b:int+ = 1` and `let c:int* = 1`
- **THEN** type checking SHALL accept all three
- **AND** evaluation SHALL bind `a` to `1` and `b` and `c` each to a one-item sequence

#### Scenario: Optional and one-or-more each satisfy zero-or-more and nothing narrower
- **WHEN** a file contains `let o:int? = {}`, `let p:int+ = {1 2}`, `let s:int* = o`, `let t:int* = p`, `let u:int+ = o` and `let v:int? = p`
- **THEN** type checking SHALL accept `s` and `t`
- **AND** SHALL reject `u` naming `int?` and `int+`
- **AND** SHALL reject `v` naming `int+` and `int?`

#### Scenario: An optional does not satisfy exactly one
- **WHEN** a file contains `let o:string? = {}` and `let s:string = o`
- **THEN** type checking SHALL reject the binding naming `string?` and `string`

### Requirement: The empty sequence is the absent value
The empty braced value `{}` SHALL be the one representation of "no value". Its type SHALL be the
empty type: the bottom item type with occurrence zero-or-one, rendered `{}` in every message. The
empty type SHALL satisfy every `?` and `*` type, and SHALL NOT satisfy any exactly-one or `+` type.
An `if` or `if … is` with no `else` that takes no branch, an optional property that was not written,
and a `for` whose body yields nothing SHALL each produce this value, and the system SHALL NOT
distinguish one such empty from another.

#### Scenario: The empty value satisfies optional and zero-or-more sites
- **WHEN** a file contains `let a:string? = {}`, `let b:string* = {}` and `type Box = { items?:string+ } let c = <Box items={} />`
- **THEN** type checking SHALL accept all three
- **AND** evaluation SHALL bind each to the empty sequence

#### Scenario: The empty value is rejected where at least one is required
- **WHEN** a file contains `let a:string = {}` and `let b:string+ = {}`
- **THEN** type checking SHALL reject both
- **AND** each diagnostic SHALL name the type found as `{}`

#### Scenario: Every source of emptiness is the same value
- **WHEN** a file contains `let f(c:boolean): int? = { if c { 1 } }`, `let g(): int? = {}` and `let h(ns?:int+): int* = { for n in ns {} }`
- **THEN** type checking SHALL accept all three
- **AND** `f(false)`, `g()` and `h({1})` SHALL evaluate to the same empty value
- **AND** called as entry calls, `f(false)` and `g()` SHALL return `null` to the host and `h({1})`
  SHALL return `[]`, since the canonical encoding of an entry call's result follows its type

### Requirement: Only an exactly-one type is an item type
A suffixed type SHALL NOT be an item type: no type reference, alias chain, type-argument
substitution or inference result SHALL produce a type whose item type itself carries an occurrence.
A type argument SHALL be an exactly-one type. Where a spelled type would put an occurrence inside
another, the system SHALL reject it; where an inferred type would, the system SHALL flatten by the
rules below. At runtime a sequence SHALL NOT hold a sequence as an element.

#### Scenario: A type argument that carries an occurrence is rejected
- **WHEN** a file contains `type Box = { T:type value:T }`, `type Maybe = int?` and `let a = <Box T=int? value=1 />`, `let b:<Box T=Maybe/> = <Box T=Maybe value=1 />`
- **THEN** the system SHALL reject both, explaining that a type argument must be exactly one value
- **AND** SHALL name `Maybe` as carrying an occurrence in the second

#### Scenario: Optionality moves to the slot
- **WHEN** a file contains `type Box = { T:type value?:T }` and `let b = <Box T=int />`
- **THEN** type checking SHALL accept the construction
- **AND** `b.value` SHALL have type `int?`

### Requirement: The join takes the least upper bound of the occurrences
Where the join of two types is taken — the arms of a conditional, the operands of `??`, the items
contributing to a collected sequence — the result's item type SHALL be the join of the item types by
the language's existing join rule, and the result's occurrence SHALL be the least upper bound of
the two occurrences in the lattice. An arm that is the empty type SHALL contribute its zero-or-one
occurrence and no item type, so `{}` joined with `T` is `T?`.

#### Scenario: A conditional with no else is optional
- **WHEN** a file contains `let v = { if c { 1 } }`
- **THEN** the inferred type of `v` SHALL be `int?`

#### Scenario: A conditional joins occurrences
- **WHEN** a file contains `let xs:int+ = {1 2}`, `let a = { if c { 1 } else { xs } }`, `let b = { if c { xs } }` and `let d = { if c { 1 } else { 2 } }`
- **THEN** `a` SHALL infer as `int+`, `b` as `int*` and `d` as `int`

#### Scenario: An empty arm takes its item type from the other arm
- **WHEN** a file binds `{<A/>{if c { <A n=2 /> } else { }}}` at a content property declared `A+`
- **THEN** type checking SHALL accept the binding
- **AND** SHALL NOT report the type found as `object*`

### Requirement: A collecting position adds occurrences and a for multiplies them
In a collecting position — a braced value, a call argument written as a braced value, element body
content, and the yields of a `for` — each item SHALL contribute its item type and its occurrence, and
the collected sequence's occurrence SHALL be their sum: at least one when any item is exactly-one or
`+`, otherwise possibly zero; more than one when there are several items or any item is `+` or `*`.
A `for` SHALL multiply: its result's occurrence SHALL be the product of the iterated occurrence and
the body's, where a product is at least one only when both factors are, and is bounded by one only
when both factors are. A single-item braced value SHALL have its item's type unchanged, so `{x}` is
`x`.

An item whose value is a sequence SHALL contribute its items in order, in every runtime, so no
runtime SHALL produce a sequence holding a sequence. This restates the splice rule of
`sequence-model` in occurrence terms; it adds no rule of its own.

#### Scenario: Two exactly-one items make one-or-more
- **WHEN** a file contains `let xs = {1 2}`
- **THEN** `xs` SHALL infer as `int+`

#### Scenario: An optional beside an item makes one-or-more
- **WHEN** a file contains `let o:int? = {}` and `let xs = {o 2}`
- **THEN** `xs` SHALL infer as `int+`
- **AND** evaluation SHALL produce the one-item sequence `2`

#### Scenario: Two optionals make zero-or-more
- **WHEN** a file contains `let o:int? = {}`, `let p:int? = 3` and `let xs = {o p}`
- **THEN** `xs` SHALL infer as `int*`
- **AND** evaluation SHALL produce the one-item sequence `3`

#### Scenario: A single item is itself
- **WHEN** a file contains `let o:int? = 1` and `let v = {o}`
- **THEN** `v` SHALL infer as `int?`, not `int*`

#### Scenario: A for over one-or-more with an exactly-one body yields one-or-more
- **WHEN** a file contains `let squares(ns:int+): int+ = { for n in ns { n * n } }`
- **THEN** type checking SHALL accept the return type

#### Scenario: A for whose body may yield nothing yields zero-or-more
- **WHEN** a file contains `let evens(ns:int+): int* = { for n in ns { if (n % 2 == 0) { n } } }` and `let bad(ns:int+): int+ = { for n in ns { if (n % 2 == 0) { n } } }`
- **THEN** type checking SHALL accept `evens`
- **AND** SHALL reject `bad`, naming `int*` as the type found

#### Scenario: A for over an optional yields an optional
- **WHEN** a file contains `let name(p?:Person): string? = { for x in p { x.name } }`
- **THEN** type checking SHALL accept the return type

### Requirement: Types render by their NX spelling
Every diagnostic, hover and generated-code comment that renders an NX type SHALL spell an occurrence
as its suffix — `Person?`, `Person+`, `Person*` — SHALL render the empty type as `{}`, and SHALL NOT
use the words "nullable" or "null" or the `[]` suffix. A function type under a suffix SHALL be
parenthesized, as today.

#### Scenario: A mismatch names both occurrences
- **WHEN** a file contains `type Box = { items:string+ }`, `let o:string? = {}` and `let b = <Box items={o} />`
- **THEN** the diagnostic SHALL contain `string+` and `string?`
- **AND** SHALL NOT contain `nullable`, `null` or `[]`

#### Scenario: The empty type renders as its spelling
- **WHEN** a file contains `let s:string = {}`
- **THEN** the diagnostic SHALL name the type found as `{}`

### Requirement: Canonical encoding of an occurrence-typed value
The canonical value encoding SHALL encode a `+` or `*` value as an array, including the empty array
for an empty `*`. It SHALL encode a `?` value that holds an item as that item. It SHALL encode an
empty `?` value by omitting the key when the value is a record field, and as `null` when it is the
result of an entry call — a host evaluating a function whose result type, declared or inferred, is
a standalone `T?` — since `null` is how hosts spell an absent single value, and typegen maps a
standalone `T?` to `T | null` in TypeScript and a nullable `T` in C#. Elsewhere an empty value
encodes as the empty array; inside the program the empty value is always the empty array, and how
a runtime represents it is its own choice. Decoding host input at a `?` or `*` site SHALL accept a missing key, `null` and an empty
array as the empty value, SHALL accept a one-element array at a `?` site as its element, and SHALL
reject a longer array at a `?` site. Decoding at a `+` site SHALL reject a missing key, `null` and an
empty array. Decoding at an exactly-one site SHALL reject `null`. First-party NX-syntax output SHALL
follow `unbraced-literal-forms`: an empty optional attribute is omitted, an empty top-level value is
`{}`.

#### Scenario: An entry call's empty optional result is null
- **WHEN** a host evaluates `let root(): string? = { if false { "a" } }`, `let root() = { if false { 1 } }` and `let root(): int* = { if false { 1 } }`
- **THEN** the first two SHALL return `null`
- **AND** the third SHALL return `[]`
- **AND** the .NET SDK's `NxRuntime.Evaluate<string?>` SHALL read the first as `null`

#### Scenario: A present optional encodes as its item and an absent one as a missing key
- **WHEN** a file contains `type Book = { title:string author?:string }` and `let a = <Book title="A" author="X" />`, `let b = <Book title="B" />`
- **THEN** the canonical JSON of `a` SHALL contain `"author": "X"`
- **AND** the canonical JSON of `b` SHALL NOT contain an `author` key

#### Scenario: Host null and an empty array both decode to the empty value
- **WHEN** a host constructs `Book` from JSON `{ "title": "A", "author": null }` and from `{ "title": "A", "author": [] }`
- **THEN** both SHALL construct with `author` empty
- **AND** both SHALL be equal to `<Book title="A" />`

#### Scenario: An empty array is rejected where at least one is required
- **WHEN** a host constructs `type Box = { items:string+ }` from JSON `{ "items": [] }`
- **THEN** construction SHALL fail with a runtime error naming `items`
