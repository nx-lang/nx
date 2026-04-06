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
  isAdmin: bool = false
}
```

- Fields use `name: Type` and can optionally declare defaults with `=`.
- `abstract type` records can appear in annotations but cannot be instantiated directly.
- `extends Base` is valid only when `Base` resolves to an abstract record declaration.

## Enum Types
Enums declare a fixed set of named values.

```nx
enum DealStage = draft | pending_review | approved_for_launch

let stage = DealStage.pending_review
```

- Enum members conventionally use `snake_case`.
- NX serializes enum values exactly as they appear in source, so `pending_review` stays
  `"pending_review"` over the wire or in storage.

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

## Function Types
Function signatures describe argument and return types, enabling callbacks and higher-order functions.

```nx
type ItemRenderer = (User) => Element

let <SimpleList items:User[] renderer:ItemRenderer/> =
  <ul>
    for item in items {
      <li>{renderer(item)}</li>
    }
  </ul>
```

## See also
- Language Tour: [Types](/language-tour/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
