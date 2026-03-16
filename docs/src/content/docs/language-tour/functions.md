---
title: 'Functions & Bindings'
description: 'Use `let` for functions and bindings, and `component` for declarations with emits or state.'
---

`let` still handles values and reusable functions. The `action` keyword introduces reusable action-shaped records, and `component` is for element-style declarations that need emitted actions or persistent local state.

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

## Component declarations

```nx
action SearchSubmitted = {
  searchString:string
}

action DoSearch = {
  search:string
}

component <SearchBox
  placeholder:string
  emits {
    ValueChanged {
      value:string
    }
    SearchSubmitted
  }
/> = {
  state {
    query:string
  }

  <TextInput value={query} placeholder={placeholder} />
}

action TrackSearch = {
  value:string
}

<SearchBox
  placeholder="Find docs"
  onSearchSubmitted=<DoSearch search={action.searchString}/>
  onValueChanged=<TrackSearch value={action.value}/> />

let makeValueChanged(value:string): SearchBox.ValueChanged =
  <SearchBox.ValueChanged value={value} />
```

- Use `action` for shared action contracts that multiple components can emit.
- Use `component` when the declaration needs `emits` or `state`.
- `emits` can declare a new action inline or reference an existing `action` by name.
- Inline emitted actions become public action names such as `SearchBox.ValueChanged`.
- Component invocation sites can bind emitted actions through `on<ActionName>` properties with an implicit `action` value inside the handler body.
- Full component init/render/dispatch runtime behavior still lands in a later change.

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

## See also (Reference/Grammar)
- Reference: [Functions & Components](/reference/syntax/functions)
- Reference: [Modules](/reference/syntax/modules)
- Grammar: [nx-grammar.md – Functions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#functions)
