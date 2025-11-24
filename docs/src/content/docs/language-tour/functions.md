---
title: 'Functions & Bindings'
description: 'Define reusable components and constants with `let`, using element or paren syntax.'
---

Components and constants share the same `let` keyword. Signatures mirror how you invoke them, keeping APIs self-documenting.

## Element-style functions

```nx
let <UserCard user:User tone:string = "neutral"/> =
  <div className={`card ${tone}`}>
    <img src={user.avatarUrl}/>
    <h3>{user.name}</h3>
  </div>

<UserCard user={currentUser} tone="info"/>
```

- Attributes in the signature declare names, types, and defaults.
- Invocation mirrors the signature: supply values instead of types.

## Paren-style functions

```nx
let formatName(name:string, title:string?) : string =
  if title { `${title} ${name}` } else { name }

let displayName = formatName("Ada", "Dr.")
```

Use this form for utility helpers when angle brackets would add noise.

## Constants

```nx
let primaryTone = "info"
let spacing = 12
```

`let` binds any expression to a name; use explicit types when inference isn’t obvious.

## Next up

## See also (Reference/Grammar)
- Reference: [Functions & Components](/reference/syntax/functions)
- Reference: [Modules](/reference/syntax/modules)
- Grammar: [nx-grammar.md – Functions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#functions)
