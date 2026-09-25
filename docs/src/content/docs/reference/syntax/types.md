---
title: 'Types'
description: 'Declaring and using types in NX.'
---

This page describes type declarations and usage. For formal grammar, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types).

## Type Aliases
Use aliases to name primitive or composite types.

```nx
type UserId = string
type Tags = string+
type RowTemplate = <function Item:string Index:int />: string
```

## Occurrences
A type reference is a base type followed by at most one *occurrence suffix*, which says how many
values the type admits. The base is a primitive, a declared type, an [applied generic](#generic-records),
a [function type](#function-types) or a parenthesized type.

| Type | Admits |
| --- | --- |
| `T` | exactly one value |
| `T?` | zero or one |
| `T+` | one or more |
| `T*` | zero or more |

```nx
type Name = string                        // exactly one
type Nickname = string?                   // zero or one
type Names = string+                      // one or more
type Tags = string*                       // zero or more
type Loaders = (<function />: string)+    // one or more functions, each returning one string
```

There is no `[]` suffix and no `null`. A sequence is `T+` or `T*`, and the absent value is `{}`, the
empty sequence. `T[]` is rejected with a diagnostic naming `T*` and `T+`.

### One suffix
A type carries at most one suffix, and the rule follows an alias. There is no optional sequence and
no sequence of optionals: a sequence with no items *is* the absent value, so `T*` is both, and a
suffix on a type that already has one is rejected:

```nx
// Rejected: each type already carries an occurrence.
type Names = string+
type Grid = Names*
type Twice = string??
type Paren = (string+)?
```

Where data has to nest, a record nests it: declare the row and take a sequence of rows. See
[Sequences & Object Duality](/reference/concepts/sequences-and-objects#nesting-data).

### The empty value
`{}` is the one absent value. It satisfies every `?` and `*` type and no exactly-one or `+` type,
and every source of emptiness produces it: an optional property that was not written, an `if` with
no `else` that takes no branch, a `for` whose body yields nothing. Nothing distinguishes one such
empty from another, and a diagnostic renders its type as `{}`.

```nx
let a:string? = {}
let b:string* = {}
```

```nx
// Rejected: {} satisfies neither an exactly-one type nor a + type.
let c:string = {}
let d:string+ = {}
```

### What satisfies what
Exactly one sits below `?` and `+`, and both sit below `*`. A value satisfies a site when its
occurrence is at or below the site's: a single value is accepted wherever a suffixed type is
expected, as a sequence of one; `?` and `+` each satisfy `*` and nothing narrower; `*` satisfies
only `*`. A mismatch names both types by their spelling.

```nx
let one:int = 1
let a:int? = {one}     // one value satisfies every occurrence
let b:int+ = {one}
let c:int* = {one}

let o:int? = {}
let p:int+ = {1 2}
let s:int* = {o}       // ? satisfies *
let t:int* = {p}       // + satisfies *
```

```nx
// Rejected
let o:int? = {}
let p:int+ = {1 2}
let u:int+ = {o}       // expects int+, found int?
let v:int? = {p}       // expects int?, found int+
let w:int = {o}        // expects int, found int?
```

Because `?` does not satisfy exactly one, a value that may be empty has to be tested, stepped
through or given a fallback before it is used as one value. The operators for that are `x?`,
`x?.m` and `x ?? y`; see [Expressions](/reference/syntax/expressions#values-that-may-be-empty).

### Occurrences add and multiply
Every site that collects, joins or iterates computes an occurrence from the same lattice, so it is
rarely written by hand:

- A **collecting position** — a braced value, element body content, a braced call argument, the
  yields of a `for` — adds its items' occurrences. Two items make `+`; an item that may be empty
  beside one that cannot still make `+`; two that may both be empty make `*`. A braced value of one
  item is that item.
- A **`for`** multiplies the iterated occurrence by its body's: over a `+` a body yielding one
  gives `+`, a body that may yield nothing gives `*`; over a `?`, a body yielding one gives `?`.
- A **join** — the branches of an `if`, the arms of a match, the operands of `??` — takes the least
  upper bound. An `if` with no `else` joins its branch with `{}`, so over `T` it is `T?` and over
  `T+` it is `T*`.

```nx
type Person = { name:string }

let o:int? = {}
let p:int? = 3
let pair = { 1 2 }         // int+
let some = { o 2 }         // int+: the 2 is always there
let few = { o p }          // int*: both may be empty
let lone = { o }           // int?: a braced value of one item is that item

let squares(ns:int+): int+ = { for n in ns { n * n } }
let evens(ns:int+): int* = { for n in ns { if (n % 2 == 0) { n } } }
let nameOf(p?:Person): string? = { for x in p { x.name } }
```

See [if](/reference/syntax/if#when-there-is-no-else) for the conditional and
[Sequences & Object Duality](/reference/concepts/sequences-and-objects) for the splice rule.

## Numeric Types and Conversions
The numeric primitives are `int`, `int32`, `int64`, `float32` and `float64`. `int` is the integer
type to use unless a declaration has a reason to name a width: it is exact over ±(2^53−1), the
widest range every NX backend represents exactly.

NX converts a value on its own only when the conversion is *total* (defined for every value of the
source type), *exact* (nothing is lost), and has *one obvious result*. Every implicit conversion in
the language passes all three tests, and a conversion that fails one is a type error naming both
types.

### Widening
For numbers the rule gives widening in one direction only:

| From | Widens to |
| --- | --- |
| `int32` | `int`, `int64`, `float64` |
| `int` | `int64`, `float64` |
| `float32` | `float64` |

The two crossings into floating point are there because they are exact: a `float64` holds every
integer in ±(2^53−1) without loss, which is precisely the range of `int`, and `int32` lies inside
it. Widening applies to an expression of any form, not only to a literal, and at every site that
declares a type: property bindings, property and record field defaults, record field values,
annotated `let` bindings, declared return types, arguments, each item of a sequence, a content
property, and any of those declared with an occurrence.

```nx
type Box = { width: float64  height: float64  opacity: float64 = 1 }

// Accepted: an int is exact in a float64, so `n` widens at the float64 property.
let scaled(n: int) = { <Box width={n} height={n} /> }

// Accepted: each item widens to the item type.
let widths(a: int, b: int32): float64+ = { a b }
```

The value takes the site's type when the program runs: `width` above is the `float64` `3.0` for
`scaled(3)`, not an integer a consumer is left to widen.

Nothing narrows, and a crossing that can lose information is not implicit:

```nx
// Rejected: an int64 may not fit an int32, and an int may not either.
let narrow(n: int64): int32 = { n }

// Rejected: a float64 has digits a float32 cannot hold.
let half(x: float64): float32 = { x }

// Rejected: int64 exceeds the integers a float64 holds exactly, and a float32 is exact only
// to ±2^24, so no integer type widens to it.
let lossy(n: int64): float64 = { n }
let small(n: int): float32 = { n }
```

NX has no explicit conversion functions yet, so for now a narrowing is written by changing the
declared type.

### Mixed operands
When the operands of an arithmetic operator (`+`, `-`, `*`, `/`, `%`) or a comparison operator
(`==`, `!=`, `<`, `<=`, `>`, `>=`) have different numeric types, the operation is typed at the
narrowest type both operands widen to. Arithmetic yields that type and a comparison yields
`boolean`.

| Operands | Common type |
| --- | --- |
| `int32` with `int` | `int` |
| `int32` or `int` with `int64` | `int64` |
| `float32` with `float64` | `float64` |
| `int32` or `int` with `float64` | `float64` |
| `int32` or `int` with `float32` | `float64` |
| `int64` with `float32` or `float64` | none: rejected |

```nx
let total(n: int, x: float64) = { n + x }     // float64: total(1, 1.5) is 2.5
let same(n: int, x: float64) = { n == x }     // boolean: same(2, 2.0) is true
let mean(a: int32, b: float32) = { a + b }    // float64: neither widens to the other
```

`int32` with `float32` is `float64` rather than `float32` because no integer widens exactly to a
`float32`, while both operands widen exactly to a `float64`. Division of two integers stays an
integer division, so `7 / 2` is `3`; when either operand is floating point it is `3.5`.

### Joined branches
The branches of an `if` or a match, and the items of a sequence, are joined the same way: the result
has the narrowest type every branch widens to, and a narrower branch widens to it when the program
runs, as it would at a declared site. This holds through an occurrence too: the item types are
joined and the occurrences take their least upper bound, as [Occurrences](#occurrences) describes.

```nx
let pick(b: boolean, n: int, x: float64) = { if b { n } else { x } }  // float64
let ratio(b: boolean, n: int, x: float64, d: int) = { pick(b, n, x) / d }
```

`pick(true, 3, 1.5)` is the `float64` `3.0`, so `ratio(true, 3, 1.5, 2)` divides as floats and is
`1.5`, not the integer division `1`.

A missing `else` makes the result optional and changes nothing else, and a branch that may be
empty keeps the result optional. If the branches have no common item type other than `object`, the
result is `object`, or `object?` when a branch may be empty.

```nx
let maybe(b: boolean, n: int) = { if b { n } }                             // int?
let mixed(b: boolean, x: float64, n?: int) = { if b { n } else { x } }     // float64?
let either(b: boolean, x: float64, s?: string) = { if b { s } else { x } } // object?
```

### Literals
An integer literal infers `int` and a literal with a decimal point infers `float64`, but a literal
takes the numeric type of the site it is written at instead — so a whole number needs no `.0`, and
`1` can be written where an `int32` is declared even though an `int` expression cannot.

```nx
let square = <Box width=24 height=24 opacity=0.5 />

let count = 24             // int, because nothing here declares otherwise
let ratio: float64 = 24    // float64, because the annotation says so
let small: int32 = 24      // int32
let scale: float32 = 1.5   // float32, rounded to the nearest float32
```

`width=24` and `width=24.0` are the same value: the literal is converted, not merely accepted, so
both produce identical output. A literal operand of a binary operator takes the other operand's
numeric type on the same terms, so a width chosen for one operand is not lost to the literal's
default: with `w: float32`, `w * 1.5` is a `float32`, and with `n: int32`, `n + 1` is an `int32`.

A literal that its target type cannot hold is rejected rather than rounded or wrapped, and a real
literal takes no integer type:

```nx
// Rejected: the value is not exactly representable as a float64.
let exact: float64 = 9007199254740993

// Rejected: the value is out of range for int32.
let big: int32 = 2147483648

// Rejected: a real literal is not an integer, whole-valued or not.
let whole: int = 1.0
```

### Constant expressions
Arithmetic whose operands are all numeric literals (`+`, `-`, `*`, `/`, `%` and negation, nested
freely) is a *constant expression*, and it takes its site's numeric type the way a single literal
does. The expression is first computed at the types its literals have on their own, exactly as it
would be when the program runs: two integers as an `int`, so `/` truncates, and anything with a real
operand as a `float64`. The result is then treated as a literal written at the site: range-checked
for `int32`, checked for exactness at a floating-point type, and rounded once for `float32`.

```nx
type Sizes = { gap: int32  ratio: float32  half: float32  wide: float64 }

let sizes = <Sizes gap={2 + 3} ratio={1.5 * 2} half={7 / 2} wide={7 / 2} />
// gap is the int32 5, ratio the float32 3.0, and half and wide are both 3.0:
// `7 / 2` is integer division wherever it is written.

let scaled(w: float32) = { w * (1.5 * 2) }   // float32: the constant takes w's type
```

Because the value is computed before the site is consulted, a constant expression means the same
thing at every site, and only whether it fits depends on the site. This is the rule Go, Ada, Zig
and Odin use for constants. Write `7.0 / 2` for the real quotient.

A constant expression that cannot take its site's type is rejected, as are a division by zero and
an integer result outside what an `int` can hold:

```nx
// Rejected: 3000000000 is out of range for int32.
let big: int32 = { 1000000 * 3000 }

// Rejected: the constant expression divides by zero.
let broken: float32 = { 1 / 0 }

// Rejected, for the same reason as a lone literal: 3000000000 is out of range for int32.
let compare(n: int32) = { n > 3000000000 }
```

The last case is rejected rather than compared at `int`, because a literal operand takes the other
operand's type before anything is promoted. An `int32` is never greater than 3000000000, so the
comparison is almost certainly a mistake; declare `n` as an `int` if the comparison is intended.

A constant expression that no site gives another type is left as written and computed when the
program runs, like any other expression: `let half = { 7 / 2 }` is the `int` `3`.

## Record Types
Record types use `type Name = { ... }` declarations. Record inheritance is limited to single-base
inheritance from abstract records.

```nx
type UserId = string

abstract type Entity = {
  id: UserId
}

abstract type UserBase extends Entity = {
  name: string
  email?: string
}

type User extends UserBase = {
  isAdmin: boolean = false
}
```

- Fields use `name: Type` and can optionally declare defaults with `=`. A field that may be left
  out is marked on its name, `name?: Type`; see [Optional properties](#optional-properties).
- Prefix one field with `content` when element body content should bind to that field during markup-style construction.
- `abstract type` records can appear in annotations but cannot be instantiated directly.
- `extends Base` is valid only when `Base` resolves to an abstract record declaration.
- An abstract record is an open set: any module may declare another record that extends it, and a
  site typed `Entity` accepts all of them. A closed set of records is a
  [discriminated union](#discriminated-union-types). Generated host types follow the same split. An
  abstract record is an open base type: an abstract class in C#, and in TypeScript an interface
  whose `$type` is any string, which each record extending it narrows to its own name. A union is
  a closed union type.
- Record construction is closed: supplied fields must be declared on the effective record shape,
  including inherited fields. Unknown fields are rejected instead of being ignored.

## Optional properties
A property — a record or action field, a component prop, a `state` field, an emitted action's
payload field, a function parameter, or a function type's parameter — admits no value only through
a `?` mark on its name. What follows the colon is an exactly-one type or a `+` type:

```nx
type Person = { name:string }

type Book = {
  title:string          // required: exactly one
  subtitle?:string      // zero or one
  authors:Person+       // one or more
  tags?:string+         // zero or more
}
```

A `?` or `*` in the type slot is rejected, whether spelled there or reached through an alias, and
the diagnostic shows the `name?:` form to write instead:

```nx
// Rejected: write `subtitle?:string` and `tags?:string+`.
type Book = { subtitle:string? tags:string* }
```

An optional property has no default. A default already makes a property omissible and gives it a
value, so the two forms are exclusive, and `name?:T = x` is rejected with both alternatives shown:

```nx
// Rejected: write `subtitle:string = "none"` or `subtitle?:string`.
type Book = { subtitle?:string = "none" }
```

### Reading and writing one
Reading a property declared `p?:T` gives `T?`, and reading one declared `p?:T+` gives `T*`. A
required or defaulted property reads as its declared type. This holds for a field read through a
record, a prop or state field read inside a component body, and a parameter read inside a function.

```nx
type Book = { title:string subtitle?:string tags?:string+ }

let sub(b:Book) = { b.subtitle }   // string?
let tags(b:Book) = { b.tags }      // string*
```

At a construction, component use or call, a property that is not written binds its default when it
has one, the empty value when it is optional, and is reported as missing otherwise. A written value
is checked against the read type, so an optional property accepts `{}`, a `?` value, an exactly-one
value, and a `*` value where its type is `+`. A required or defaulted property is checked against its
declared type, so `{}` or a value that may be empty is rejected there — the same mismatch as an
`int?` at an `int`.

```nx
type Book = { title:string = "Untitled" subtitle?:string tags?:string+ }

let o:string? = {}
let a = <Book title="A" subtitle="S" tags={"x" "y"} />
let b = <Book title="B" />                          // subtitle and tags are empty
let c = <Book subtitle={o} tags={} />               // the same as <Book />, with the default title
```

```nx
// Rejected: `title` has a default, so it cannot be written empty.
type Book = { title:string = "Untitled" }
let d = <Book title={} />
```

Because an optional property reads as `T?`, using it as one value takes a presence test, a step or
a fallback — `b.subtitle ?? "none"`, `if b.subtitle? { b.subtitle }` — described under
[Values that may be empty](/reference/syntax/expressions#values-that-may-be-empty). A `state`
field marked `?` starts empty.

## Constant Unions
A union whose cases all carry no payload declares a fixed set of named values. This is the form
enums take in NX: there is no `enum` keyword, and no separate declaration for a closed set of
constants.

```nx
type DealStage = draft | pending_review | approved_for_launch

let stage = { DealStage.pending_review }
```

- Case names conventionally use `snake_case`.
- NX serializes a constant case exactly as it appears in source, so `pending_review` stays
  `"pending_review"` over the wire or in storage.
- A union whose cases all carry no payload is a *constant union*: it is the plain scalar-choice
  form, and it generates a C# `enum` and a TypeScript string-literal union. Give a case a payload
  when that state needs to carry data.
- Every record, action, and component with `state` has a derived constant union of its field
  names, `T.Property`, so `User.Property.email` names the `email` field of `User` as a value. See
  [Property references](/reference/syntax/functions#property-references).

## Discriminated Union Types
Discriminated unions declare a closed set of scoped cases. A union uses `type Name =` followed by a
case list, with cases separated by `|`. A leading `|` before the first case is optional when the
list holds two or more cases, and required when it holds exactly one, because `type A = B` without
it is a type alias.

```nx
type LoadState =
  | idle
  | loading
  | failed { message:string retryable:boolean = true }
  | loaded { items:string+ }
```

- Every case is referenced through the owning union name, such as `LoadState.idle`.
- Fieldless cases can be used directly with member syntax.
- Payload cases are constructed with element-style syntax:
  `<LoadState.failed message={"Network unavailable"} />`.
- Case payload fields use the same `name: Type`, `name?: Type`, default, and `content` field rules
  as record fields.
- A union may extend an abstract record to share inherited fields across every case:

```nx
abstract type EventBase = {
  source:string
}

type UiEvent extends EventBase =
  | clicked { x:int y:int }
  | closed
```

Unions are closed: cases can only be declared in the union's own case list, and other declarations
cannot extend a union to add cases. `type Result = Success | Failure` is intentionally not a
discriminated union declaration in this feature; that spelling remains reserved for a possible
future union-alias proposal.

## Nested records

```nx
type Address = {
  street: string
  city: string
  state: string
  zip: string
}

type Person = {
  name: string
  email: string
  address: Address
}
```

Combining record types allows you to describe complex domain models while keeping named shapes explicit.

## Content-marked record fields

```nx
type Panel = {
  title: string
  content body: Element
}
```

When you instantiate a record with element syntax, any body content binds to the field marked with
`content`.

## Record creation
Concrete records are instantiated with the same element-style syntax used elsewhere in NX.

```nx
let user =
  <User
    id="123"
    name="John Doe"
    email="john@example.com"
  />

let entityName(user: UserBase) = { user.name }
let result = { entityName(user) }
```

If a record has a stale field name, type checking reports it before defaults are applied.

```nx
type ChatLinkConfig = { standaloneAppearance:string }

let config = <ChatLinkConfig
  accentColor={"#3b82f6"}        // rejected: not declared on ChatLinkConfig
  standaloneAppearance={"split"}
/>
```

## Generic records
A record whose fields hold values of a type the declaration does not fix declares a **type
parameter**, with the same `Name:type` syntax a component signature uses and the same placement
rules — before every field, with no default, no modifier, no `?` mark and no suffix:

```nx
type Bounds = {
  T:type
  low:T
  high:T
  inclusive:boolean = false
}
```

Inside the declaration `T` is a type like any other, and it is **rigid**: only `T` itself satisfies
it, which is what rejects `value:T = "text"`. It takes an occurrence suffix, the `?` mark on a
field, and a place in a function type like any other type, and a record may declare more than one:

```nx
type Pair = { TKey:type TValue:type key:TKey value:TValue }
type Page = { T:type items:T+ next?:T render?:<function item:T />: string }
```

A type parameter is not a field: a constructed `Bounds` has `low`, `high` and `inclusive` and
nothing else, and `Bounds.Property` has one case per field and none for `T`.

The examples below use [`Range`](#ranges), the built-in generic record, which is an ordinary
declaration of exactly this kind.

### Naming one instantiation
A generic record's name alone is not a type. Every type position takes an **applied type**, written
as the element that constructs the record with only its type arguments:

```nx
type Schedule = {
  week:<Range T=int/>
  spans?:<Range T=float64/>+
}

let week:<Range T=int/> = <Range T=int start={1} end={7} endInclusive={true} />
```

An applied type takes one occurrence suffix like any other base type, nests
(`<Page T=<Range T=int/>/>`), takes any exactly-one type as an argument — a suffixed type is not
one, since a record declaring `items:T+` would then have a field whose items carry an occurrence —
and may name a qualified tag, which is how the update companion is written (`<Range.Update T=int/>`). Arguments are matched to parameters **by name**, so
`<Pair TValue=int TKey=string/>` and `<Pair TKey=string TValue=int/>` are one type.

### Distinct per argument, and invariant
Two applied types are the same type exactly when they apply the same record to the same arguments,
after aliases are resolved. An argument never widens or converts, even where the argument types
themselves convert:

```nx
let ints:<Range T=int/> = <Range T=int start={1} end={5} endInclusive={false} />
let wider:<Range T=float64/> = {ints}   // rejected: int does not convert inside the argument
```

Reading a field gives its declared type with each parameter replaced by its argument, so
`week.start` is `int`. Every applied type satisfies `object`.

Invariance also decides what a sequence of them is. Two of one instantiation keep their arguments,
and two that differ have nothing in common below `object`:

```nx
let same = { (0..5) (5..=9) }       // <Range T=int/>+
let mixed = { (0..5) (0.0..1.0) }   // object+
```

### Constructing one
A construction binds each type parameter as a property, as a component use site does, and the
argument is a bare type name:

```nx
let r = <Range T=int start={1} end={5} endInclusive={false} />
```

Every parameter must be bound — there is no inference and no fallback, because the value's own type
is what the argument decides — and a braced, quoted or conditional argument is rejected, as it is
for a component. A type argument is not a field: it is neither missing, unknown nor duplicate, and
once type checking has consumed it nothing below the checker sees it.

An argument that is not a bare name is written through an alias, which is what a construction site
takes where a type position would take the spelled-out type:

```nx
type Box = { T:type value:T }
type IntRange = <Range T=int/>

let nested:<Box T=<Range T=int/>/> = <Box T=IntRange value={<Range T=int start={1} end={5} endInclusive={false} />} />
```

A type argument is an exactly-one type. A primitive, a record, a union, a function type and another
applied type all qualify; a type with an occurrence does not, so `<Box T=int?/>`, `<Box T=int+/>`
and an alias that names either are rejected as not being exactly one value. Where a field has to
hold many values or none, the occurrence goes on the field and the argument stays an item type:

```nx
type Box = { T:type value?:T }
type Boxes = { T:type items:T+ }

let full = <Box T=int value=1 />     // value reads as int?
let empty = <Box T=int />            // value is empty
let many = <Boxes T=int items={1 2} />
```

### Companions
The derived update record carries the same parameters and is named the same way, and the update
intrinsics carry the argument across: for a value of `<Range T=int/>`, `apply` expects and `diff`
produces `<Range.Update T=int/>`, and `merge` requires two updates of one instantiation. The
derived property union is parameter-independent: `Range.Property` takes no arguments and has the
cases `start`, `end` and `endInclusive`.

### Not part of inheritance
A record that declares type parameters cannot be `abstract` and cannot have an `extends` clause.

### Below the checker
Type arguments are a checking-time notion only. The interpreter, NX IR and the TypeScript IR
runtime see an applied type as its record and a parameter-typed field as `object`, so two values
built with different arguments and equal fields are equal values. `typegen` goes the other way and
emits a real generic — `Bounds<T>` in both C# and TypeScript, with an applied type rendered as the
instantiation (`Bounds<long>`, `Bounds<number>`) — because a host names the concrete instantiation at
its own deserialization site. The update companion follows the record: `Bounds_update<T>` in both
languages, so a patch of a `Bounds<long>` is typed as one and applies to it. The wire is unaffected —
a patch is still `Bounds.Update` with its field names and no type argument. Only a *component's*
state companion erases its parameters, because the type it patches is already concrete.

## Ranges
`Range` is a **built-in type**: a generic record NX declares for every program, so it needs no import
and no declaration of your own.

```nx
type Range = {
  T:type
  start:T
  end:T
  endInclusive:boolean
}
```

It is an ordinary [generic record](#generic-records) in every respect — an applied type names one
instantiation, it has the companions `Range.Update` and `Range.Property`, and its parameter is
unconstrained, so `<Range T=string/>` is a type like any other:

```nx
type Slider = { range:<Range T=float64/> }
let letters = <Range T=string start="a" end="f" endInclusive={true} />
```

### The operators are sugar for it
`a..b` and `a..=b` construct it, and nothing more: `1..5` *is*
`<Range T=int start={1} end={5} endInclusive={false} />`, and the two compare equal. `endInclusive`
is `false` for `..` and `true` for `..=`.

`T` is chosen by the site. Where the site expects a `<Range T=X/>`, each bound is checked against
`X`, so a literal or a value that widens to `X` is accepted:

```nx
type Slider = { range:<Range T=float64/> }
let s = <Slider range={0..1} />                  // a <Range T=float64/>
```

Otherwise `T` is the bounds' common numeric type under the rules in
[Numeric Types and Conversions](#numeric-types-and-conversions):

```nx
let small:int32 = 3
let a = {small..10}                              // <Range T=int/>
let b = {0..2.5}                                 // <Range T=float64/>
let bad:int64 = 3
let rejected = {bad..2.5}                        // rejected: no common type
```

Both bounds must be numeric. For a range of any other type, write the element form. Building a range
never fails: `5..2` is a valid, empty range.

Because the operators mean the built-in `Range`, a module that declares its own type under that name
cannot use them there — the declaration wins, as it does over any import, and the operator says so.
The element form still works, and every other module is unaffected.

### Only integer ranges iterate
[`for`](/reference/syntax/for#counting-with-a-range) counts over a `Range` whose argument is `int`,
`int32` or `int64`. A range of any other type is a value you can pass along, read fields from and
compare, but not iterate: `for x in 0..2.5` is a type error rather than a guessed step. There is no
step, no descending form and no open-ended range.

## Function Types
A function type is an element function's signature with `function` in the name slot: take
`let <ContactRow Item:Contact Index:int />: DrawnNode = ...`, remove `let`, the name and the body,
and what is left is the type of `ContactRow`. Parameters are property definitions, matched by name
wherever the type is used, and the result type after `/>` is required.

```nx
abstract external component <DrawnNode />
type Contact = { name:string }

// The type, aliased once and used like any other type.
type RowTemplate = <function Item:Contact Index:int />: DrawnNode

// A property declared at the type, inline or through the alias.
external component <List extends DrawnNode
  ItemsSource?:Contact+
  ItemTemplate?:<function Item:Contact Index:int />: DrawnNode
/>
component <Section extends DrawnNode Items:Contact+ Row:RowTemplate /> = {
  <List ItemsSource={Items} ItemTemplate={Row} />
}
```

A suffix written after the result binds to the result: `<function Count:int />: string?` is a
function returning an optional string. To give the function type itself an occurrence, parenthesize
it: `(<function />: DrawnNode)+` is one or more functions. Parentheses group and nothing more —
`(string)+` is `string+` — and they add no layer, so `(string?)?` is rejected like `string??`. An
optional function-typed property is marked on its name like any other, as `ItemTemplate?` above.

A function type's parameters carry no defaults (the caller supplies every one), at most one is
marked `content`, and none is a `type` parameter. `function` is a keyword only in this position; an
identifier named `function` keeps its meaning everywhere else.

A function satisfies a function type **by parameter name**: every parameter the function declares
must be one the type supplies, under the same name and `content` marking, at a type the function
accepts; the result must be acceptable where the type's result is expected. The function may
declare *fewer* parameters than the type — a caller always supplies every parameter of the type,
and the function ignores the rest — so `let <Compact Item:Contact />: DrawnNode` is a `RowTemplate`
that never reads `Index`, while a function needing a parameter the type lacks is not. Order does
not matter. See [Functions](/reference/syntax/functions#functions-as-values) for passing and
calling function values.

## See also
- Language Tour: [Types](/language-tour/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
