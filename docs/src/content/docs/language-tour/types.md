---
title: 'Types'
description: 'Declare primitives, records, enums, and aliases to keep data and components aligned.'
---

Types live beside your components so props and data stay consistent.

## Primitives and aliases

```nx
type UserId = string
type Score = int
```

Use aliases to name primitives or composite types for clarity.

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
  isAdmin: bool = false
}
```

- `abstract type` declares a shared record shape that can be referenced in type positions but not instantiated directly.
- `extends` reuses fields and defaults from an abstract base record.
- Concrete derived records remain constructible, so `<User id={1} name={"Ada"} />` is valid while `<UserBase ... />` is not.

## Enums

```nx
enum DealStage = draft | pending_review | closed_won
```

Use enums when values must come from a fixed set.

Enum members conventionally use `snake_case`. NX serializes enum values using the member name as
written, so `snake_case` keeps the source aligned with JSON or database values such as
`"pending_review"`.

## Type aliases for collections

```nx
type UserList = User[]
type NameLookup = (string, User)[]
```

Sequence modifiers (`[]`) and nullable modifiers (`?`) apply to any type.

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
```

Type annotations are optional when inference is obvious; add them for clarity or to surface diagnostics early.

## See also (Reference/Grammar)
- Reference: [Types](/reference/syntax/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
