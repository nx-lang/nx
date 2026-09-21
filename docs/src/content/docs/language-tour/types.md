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

// Accepted: `n` is an int, and an int is exact in a float64.
let scaled(n: int) = { <Box width={n} height={n} /> }

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

- `abstract type` declares a shared record shape that can be referenced in type positions but not instantiated directly.
- `extends` reuses fields and defaults from an abstract base record.
- Concrete derived records remain constructible, so `<User id={1} name={"Ada"} />` is valid while `<UserBase ... />` is not.

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
`type User = { name:string email:string? }` that is `User.Property`, with the cases `name` and
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

## Type aliases for collections

```nx
type UserList = User[]
type MaybeUser = User?
type MaybeUsers = User[]?
```

The sequence suffix (`[]`) and the nullable suffix (`?`) apply to any type and compose in source
order, so `User?[]` is a sequence whose items may be null and `User[]?` is a nullable sequence.
Neither stacks on its own layer: `User??` is rejected, and so is `User[][]`, because a sequence
never contains a sequence. Where data has to nest, a record is what nests it — declare the row and
take a sequence of rows. The same rule reaches an alias, so `UserList[]` is not a type either.

## Using types in code

```nx
let displayName(user: UserBase) = user.name

let user: User = <User id={1} name={"Ada"} email={"ada@example.com"} />
let value = displayName(user)

let stage: DealStage = { DealStage.pending_review }

let badgeTone = if stage is {
  DealStage.draft => "neutral"
  DealStage.pending_review => "warning"
  else => "success"
}

let state: LoadState =
  <LoadState.failed message={"Network unavailable"} />

let loadLabel = if state is {
  LoadState.idle => "Idle"
  LoadState.loading => "Loading"
  LoadState.failed => state.message
  LoadState.loaded => "Loaded"
}
```

Union matches narrow the matched identifier inside each case arm, so `state.message` is available in
the `LoadState.failed` arm. Type annotations are optional when inference is obvious; add them for
clarity or to surface diagnostics early.

## See also (Reference/Grammar)
- Reference: [Types](/reference/syntax/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
