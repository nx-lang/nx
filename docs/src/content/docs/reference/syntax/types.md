---
title: 'Types'
description: 'Declaring and using types in NX.'
---

This page describes type declarations and usage. For formal grammar, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types).

## Type Aliases
Use aliases to name primitive or composite types.

```nx
type UserId = string
type EventHandler = (string) => void
```

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
annotated `let` bindings, declared return types, arguments, each element of a list, a content
property, and any of those declared nullable.

```nx
type Box = { width: float64  height: float64  opacity: float64 = 1 }

// Accepted: an int is exact in a float64, so `n` widens at the float64 property.
let scaled(n: int) = { <Box width={n} height={n} /> }

// Accepted: each element widens to the element type.
let widths(a: int, b: int32): float64[] = { a b }
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
The branches of an `if` or a match, and the elements of a list, are joined the same way: the result
has the narrowest type every branch widens to, and a narrower branch widens to it when the program
runs, as it would at a declared site. This holds inside lists and nullable types too.

```nx
let pick(b: boolean, n: int, x: float64) = { if b { n } else { x } }  // float64
let ratio(b: boolean, n: int, x: float64, d: int) = { pick(b, n, x) / d }
```

`pick(true, 3, 1.5)` is the `float64` `3.0`, so `ratio(true, 3, 1.5, 2)` divides as floats and is
`1.5`, not the integer division `1`.

A `null` branch makes the result nullable and changes nothing else, and a nullable branch keeps
the result nullable. If the branches have no common type other than `object`, the result is
`object`, which already admits `null`.

```nx
let maybe(b: boolean, n: int) = { if b { n } else { null } }              // int?
let mixed(b: boolean, n: int?, x: float64) = { if b { n } else { x } }    // float64?
let either(b: boolean, s: string?, x: float64) = { if b { s } else { x } } // object
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
abstract type Entity = {
  id: UserId
}

abstract type UserBase extends Entity = {
  name: string
  email: string?
}

type User extends UserBase = {
  isAdmin: boolean = false
}
```

- Fields use `name: Type` and can optionally declare defaults with `=`.
- Prefix one field with `content` when element body content should bind to that field during markup-style construction.
- `abstract type` records can appear in annotations but cannot be instantiated directly.
- `extends Base` is valid only when `Base` resolves to an abstract record declaration.
- Record construction is closed: supplied fields must be declared on the effective record shape,
  including inherited fields. Unknown fields are rejected instead of being ignored.

## Constant Unions
A union whose cases all carry no payload declares a fixed set of named values. This is the form
enums take in NX: there is no `enum` keyword, and no separate declaration for a closed set of
constants.

```nx
type DealStage = draft | pending_review | approved_for_launch

let stage = DealStage.pending_review
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
  | loaded { items:string[] }
```

- Every case is referenced through the owning union name, such as `LoadState.idle`.
- Fieldless cases can be used directly with member syntax.
- Payload cases are constructed with element-style syntax:
  `<LoadState.failed message={"Network unavailable"} />`.
- Case payload fields use the same `name: Type`, nullable, default, and `content` field rules as
  record fields.
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
    id={123}
    name={"John Doe"}
    email={"john@example.com"}
  />

let entityName(user: UserBase) = user.name
let result = entityName(user)
```

If a record has a stale field name, type checking reports it before defaults are applied.

```nx
type ChatLinkConfig = { standaloneAppearance:string }

let config = <ChatLinkConfig
  accentColor={"#3b82f6"}        // rejected: not declared on ChatLinkConfig
  standaloneAppearance={"split"}
/>
```

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
  ItemsSource:Contact[]?
  ItemTemplate:(<function Item:Contact Index:int />: DrawnNode)?
/>
component <Section extends DrawnNode Items:Contact[] Row:RowTemplate /> = {
  <List ItemsSource={Items} ItemTemplate={Row} />
}
```

A suffix written after the result binds to the result: `<function Count:int />: string?` is a
function returning a nullable string. To make the function itself nullable, or a list element,
parenthesize it: `(<function Item:Contact />: DrawnNode)?`, `(<function />: DrawnNode)[]`.
Parentheses group and nothing more — `(string)[]` is `string[]` — and they add no layer, so
`(string?)?` is rejected like `string??`.

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
