---
title: 'Functions & Components'
description: 'Use `let` for functions and bindings, and `component` for declarations with emits or persistent state.'
---

NX separates `let` bindings from `component` declarations. Use `let` for values and reusable functions. Use `component` when a declaration needs an `emits` contract or persistent `state`.

Parser note: this syntax currently parses and highlights, but lowering and interpreter support for emitted action types and component state are deferred.

## `let` Definition Mirrors Invocation

```nx
let <UserCard user:User className:string = "card"/> =
  <div className={className}>
    <img src={user.avatarUrl} alt="User avatar"/>
    <h3>{user.name}</h3>
    <p>{user.email}</p>
  </div>

// Later in the module
<UserCard user={currentUser} className="featured"/>
```

- Attributes in the definition carry type annotations.
- Default values use `=` just like standard attributes.
- Invocation reuses the same structure but supplies values instead of types.

## `component` Declarations

```nx
component <SearchBox
  placeholder:string
  emits {
    ValueChanged {
      value:string
    }
    SearchRequested {
      searchString:string
    }
  }
/> = {
  state {
    query:string
  }

  <TextInput value={query} placeholder={placeholder} />
}
```

- The signature keeps the element-style prop syntax.
- `emits` declares named action payload shapes attached to the component.
- `state` declares persistent local fields before the rendered body expression.

## Paren-style Functions

```nx
let formatName(name:string, title:string?) : string =
  if title { `${title} ${name}` } else { name }
```

- Use paren-style `let` functions for utility helpers when markup syntax would add noise.
- Element-style `let` definitions remain valid for reusable markup without component-specific features.

## See also
- Language Tour: [Functions & Bindings](/language-tour/functions)
- Reference: [Modules](/reference/syntax/modules)
- Grammar: [nx-grammar.md – Functions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#functions)
