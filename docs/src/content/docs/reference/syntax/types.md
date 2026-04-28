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
- Prefix one field with `content` when element body content should bind to that field during markup-style construction.
- `abstract type` records can appear in annotations but cannot be instantiated directly.
- `extends Base` is valid only when `Base` resolves to an abstract record declaration.
- Record construction is closed: supplied fields must be declared on the effective record shape,
  including inherited fields. Unknown fields are rejected instead of being ignored.

## Enum Types
Enums declare a fixed set of named values.

```nx
enum DealStage = draft | pending_review | approved_for_launch

let stage = DealStage.pending_review
```

- Enum members conventionally use `snake_case`.
- NX serializes enum values exactly as they appear in source, so `pending_review` stays
  `"pending_review"` over the wire or in storage.
- Use enums for simple scalar choices. Use discriminated unions when cases need per-state payloads.

## Discriminated Union Types
Discriminated unions declare a closed set of scoped cases. A union uses `type Name =` followed by a
required leading-pipe case list.

```nx
type LoadState =
  | idle
  | loading
  | failed { message:string retryable:bool = true }
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
