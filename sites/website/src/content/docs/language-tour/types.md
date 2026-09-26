---
title: 'Types'
description: 'Declare primitives, records, unions, and aliases to keep data and components aligned.'
---

Types live beside your components so props and data stay consistent.

## Primitives and aliases

```nx
type UserId = string
type Score = int
```

Use aliases to name primitives or composite types for clarity.

## Numbers

The numeric primitives are `int`, `int32`, `int64`, `float32` and `float64`. Use `int` unless a
declaration has a reason to name a width. A number converts on its own only when nothing can be
lost, so numeric types widen in one direction: `int32` to `int` to `int64`, `float32` to `float64`,
and `int32` or `int` to `float64`, which holds every `int` exactly.

```nx
type Box = { width: float64  height: float64 }

// Accepted: `size` is an int, and an int is exact in a float64.
let <Square size:int /> = <Box width={size} height={size} />

let total(n: int, x: float64) = { n + x }   // float64
```

Nothing narrows: an `int64` at an `int32` site, a `float64` at a `float32` site, and an `int64` at
any floating-point site are type errors. A literal is the exception that makes this comfortable. It
takes the numeric type of the site it is written at, so `<Box width=24 />` and `let small: int32 =
1` both type check. Arithmetic over literals alone, such as `1.5 * 2`, is computed first and then
does the same, so `let scale: float32 = { 1.5 * 2 }` type checks too. See
[Numeric Types and Conversions](/reference/syntax/types#numeric-types-and-conversions) for the full
rules.

## Records and inheritance

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

- `abstract type` declares a shared record shape that can be referenced in type positions but not instantiated directly.
- `extends` reuses fields and defaults from an abstract base record.
- A field marked `?` on its name, like `email?: string`, may be left out; it then holds the empty
  value `{}`. A field with a default may be left out too, and takes the default. Every other field
  is required.
- Concrete derived records remain constructible, so `<User id="1" name="Ada" />` is valid while `<UserBase ... />` is not.

## How many: `?`, `+` and `*`

A type names one value unless a suffix says otherwise. There are three, and a type takes at most
one of them:

```nx
type User = { id:string name:string }

type OneUser = User       // exactly one
type MaybeUser = User?    // zero or one
type Users = User+        // one or more
type AnyUsers = User*     // zero or more
```

There is no `[]` and no `null`. A sequence is `User+` or `User*`, and the absent value is `{}`, the
empty sequence — so an optional value and an empty sequence are the same thing, and `User*` already
covers "a sequence that may be missing". `User??` and `Users*` are rejected, because each type
already says how many values it admits. Where data has to nest, a record is what nests it — declare
the row and take a sequence of rows.

A single value stands wherever a sequence is expected, as a sequence of one. Going the other way
takes an operator: a `User?` is not a `User` until it has been tested, stepped through with `?.` or
given a fallback with `??`. See [Expressions](/language-tour/expressions#values-that-may-be-empty).

In a record field, a prop or a parameter, "may be left out" is written on the *name*, and the type
after the colon is exactly-one or `+`:

```nx
type Person = { name:string }

type Book = {
  title:string          // required
  subtitle?:string      // may be left out; reads as string?
  authors:Person+       // one or more
  tags?:string+         // may be left out; reads as string*
}

let a = <Book title="A" authors={<Person name="Ada" />} />
let b = <Book title="B" subtitle="S" authors={<Person name="Ada" /> <Person name="Kai" />} tags={"x" "y"} />
```

Writing `subtitle:string?` there is an error whose message shows `subtitle?:string`, and an
optional field cannot also have a default: a default already makes a field omissible. See
[Optional properties](/reference/syntax/types#optional-properties).

## Constant unions

NX has no `enum` keyword. A union whose cases all carry no payload is how NX represents what other
languages call an enum, the same convention F#, OCaml, and other functional languages follow:

```nx
type DealStage = draft | pending_review | closed_won
```

Use this form when values must come from a fixed set. Such a union is a *constant union*, and each
of its cases is a *constant case*.

Case names conventionally use `snake_case`. NX serializes a constant case using the case name as
written, so `snake_case` keeps the source aligned with JSON or database values such as
`"pending_review"`.

Every record also has a derived constant union of its field names. For
`type User = { name:string email?:string }` that is `User.Property`, with the cases `name` and
`email`, and it serializes a case as the bare field name.

## Discriminated unions

Use discriminated unions when each state belongs to a fixed set, but some states need their own
payload fields.

```nx
type LoadState =
  | idle
  | loading
  | failed { message:string retryable:boolean = true }
  | loaded { count:int }

let state: LoadState =
  <LoadState.failed message={"Network unavailable"} />
```

- A union declaration uses `type Name =` followed by cases separated by `|`. A leading `|` before
  the first case is optional; NX code omits it on one line and writes a `|` on every case when the
  list spans several lines.
- Fieldless cases can be referenced with `LoadState.idle`.
- Payload cases are constructed with element syntax such as
  `<LoadState.failed message={"Network unavailable"} />`.
- A case that carries no payload serializes as its bare name, exactly like a scalar choice; only a
  case with fields serializes as a `$type` record. One declaration form covers both.

## Using types in code

```nx
type UserId = string
abstract type UserBase = { id: UserId name: string email?: string }
type User extends UserBase = { isAdmin: boolean = false }
type DealStage = draft | pending_review | closed_won
type LoadState =
  | idle
  | loading
  | failed { message:string }
  | loaded { count:int }

let displayName(user: UserBase) = { user.name }

let user: User = <User id="1" name="Ada" email="ada@example.com" />
let value = { displayName(user) }

let stage: DealStage = { DealStage.pending_review }

let badgeTone = {
  if stage is {
    DealStage.draft => "neutral"
    DealStage.pending_review => "warning"
    else => "success"
  }
}

let state: LoadState =
  <LoadState.failed message={"Network unavailable"} />

let loadLabel = {
  if state is {
    LoadState.idle => "Idle"
    LoadState.loading => "Loading"
    LoadState.failed => state.message
    LoadState.loaded => "Loaded"
  }
}
```

Union matches narrow the matched identifier inside each case arm, so `state.message` is available in
the `LoadState.failed` arm. Type annotations are optional when inference is obvious; add them for
clarity or to surface diagnostics early.

## See also (Reference/Grammar)
- Reference: [Types](/reference/syntax/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
