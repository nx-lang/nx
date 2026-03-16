---
title: 'Functions & Components'
description: 'Use `let` for functions and bindings, and `component` for declarations with emits or persistent state.'
---

NX separates `let` bindings from `component` declarations. Use `let` for values and reusable functions. Use `component` when a declaration needs an `emits` contract or persistent `state`.

Runtime note: `action` declarations parse, lower, and behave like records. Component action handler bindings now lower as lazy callbacks, while full component init/render/dispatch behavior remains deferred.

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

## Advanced Parameters

```nx
let <DataGrid
  data:object[]
  columns:object[]
  className:string? /> =
  <table className={if className { className } else { "data-grid" }}>
    <thead>
      <tr>
        for column in columns {
          <th>{column.Header}</th>
        }
      </tr>
    </thead>
    <tbody>
      for item in data {
        <tr>
          for column in columns {
            <td>{column.Render(item)}</td>
          }
        </tr>
      }
    </tbody>
  </table>
```

- Nullable types (`string?`) make optional props explicit.
- Complex defaults can reference other parameters or inline expressions.
- Iteration and conditionals in the body behave like any other expression.

## `component` Declarations

```nx
action SearchSubmitted = {
  searchString:string
}
```

- `action` uses the same record-style field syntax as `type Name = { ... }`.
- Actions remain record-compatible, so existing record construction paths keep working.
- The distinction matters only for contexts that explicitly require actions.

```nx
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

<SearchBox placeholder="Find docs" />
```

- The signature keeps the element-style prop syntax.
- `emits` can mix inline action definitions (`ValueChanged { ... }`) with references to existing actions (`SearchSubmitted`).
- Inline emitted actions become public action names such as `SearchBox.ValueChanged`.
- Call sites can bind handlers with `on<ActionName>` and read the emitted payload through the implicit `action` value.
- `state` declares persistent local fields before the rendered body expression.

```nx
action DoSearch = {
  search:string
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
