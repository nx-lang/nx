---
title: 'Types'
description: 'Declaring and using types in NX.'
---

NX embraces a strong, expressive type system to keep components, data models, and function signatures aligned.

## Type Aliases
Use aliases to name primitive or composite types.

```nx
type UserId = string
type EventHandler = (string) => void
```

## Object Types
Object types reuse element syntax so the shape of your data looks identical to the elements that consume it.

```nx
type <User id:UserId name:string email:string avatarUrl:string?/>
type <Point x:int y:int/>
type <ComponentProps data:object className:string? children:Element?/>
```

- Attributes represent required fields.
- `?` marks optional members.
- Default values can appear where it improves readability.

## Nested Objects

```nx
type <Address street:string city:string state:string zip:string/>
type <Person name:string email:string address:Address/>
```

Combining object types allows you to describe complex domain models without leaving the markup syntax.

## Object Creation
Create instances with the same syntax and pass them as expressions.

```nx
let user =
  <User
    id="123"
    name="John Doe"
    email="john@example.com"
    avatarUrl="/avatars/john.jpg"
  />

let origin = <Point x=0 y=0/>
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

## Type Checking
- The type checker validates modules at compile time.
- Nullable types (`T?`) express optional data explicitly.
- Future runtime integrations will expose diagnostics that highlight mismatches directly in editors.
