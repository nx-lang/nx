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

## Records (object types)

```nx
type <User id:UserId name:string email:string avatarUrl:string?/>
type <Point x:int y:int/>
```

- `?` marks optional members.
- The angle-bracket syntax mirrors the elements that consume these shapes.

## Enums

```nx
enum Plan = "free" | "pro" | "enterprise"
```

Use enums when values must come from a fixed set.

## Type aliases for collections

```nx
type UserList = User[]
type NameLookup = (string, User)[]
```

Sequence modifiers (`[]`) and nullable modifiers (`?`) apply to any type.

## Using types in code

```nx
let user: User = <User id="1" name="Ada" email="ada@example.com"/>

let badgeTone = if plan is {
  "free" => "neutral"
  "pro" => "success"
  else => "info"
}
```

Type annotations are optional when inference is obvious; add them for clarity or to surface diagnostics early.

## Next up

## See also (Reference/Grammar)
- Reference: [Types](/reference/syntax/types)
- Reference: [Sequences & Object Duality](/reference/concepts/sequences-and-objects)
- Grammar: [nx-grammar.md – Types](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
