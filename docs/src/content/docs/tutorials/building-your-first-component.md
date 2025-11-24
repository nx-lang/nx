---
title: 'Building Your First Component'
description: 'Walk through composing an interactive NX component from scratch.'
---

This tutorial builds a small, realistic component with typed props, events, layout, and conditional rendering. It assumes you’ve completed [Getting Started](/tutorials/getting-started) and skimmed the [Language Tour](/language-tour/elements).

## 1) Define types and props
Create `examples/nx/profile-card.nx`:

```nx
type <User id:string name:string title:string? avatarUrl:string?/>
type <ProfileAction label:string onClick:() => void/>
```

- Optional props use `?`; callbacks are typed like ordinary functions.
- Keep types beside the component so they stay in sync.

## 2) Lay out the component
Add a basic skeleton with sensible defaults:

```nx
let <ProfileCard
  user:User
  actions:ProfileAction[] = []
  tone:string = "neutral"/> =
  <article className={`card tone-${tone}`}>
    <header>
      <img src={if user.avatarUrl { user.avatarUrl } else { "/placeholder.png" }} alt={user.name}/>
      <div>
        <h3>{user.name}</h3>
        if user.title { <p>{user.title}</p> }
      </div>
    </header>
    <footer>
      for action in actions {
        <button onClick={action.onClick}>{action.label}</button>
      }
    </footer>
  </article>
```

- Attributes accept expressions (including `if`) without leaving markup mode.
- Defaults keep the call site concise (`actions` defaults to an empty list).

## 3) Add interaction and stateful inputs
Model simple state by threading values through props and callbacks:

```nx
let <ProfileScreen user:User/> =
  let actions: ProfileAction[] = [
    <ProfileAction label="Message" onClick={() => sendMessage(user.id)}/>,
    <ProfileAction label="Follow" onClick={() => follow(user.id)}/>
  ]

  <ProfileCard user={user} actions={actions} tone="info"/>
```

- Event handlers are just functions; they can capture props.
- Because `ProfileAction` is typed, the compiler enforces consistent event shapes.

## 4) Render it
End the file with a root element to see everything together:

```nx
let currentUser = <User id="42" name="Kai" title="Designer"/>

<ProfileScreen user={currentUser}/>
```

## 5) Validate and iterate
- Run `cargo test --workspace` to confirm the NX toolchain still builds.
- If you have the `tree-sitter` CLI installed, run `cd crates/nx-syntax && tree-sitter parse ../../examples/nx/profile-card.nx` to sanity-check syntax.
- Use the patterns from the Reference (especially [Functions & Components](/reference/syntax/functions) and [if](/reference/syntax/if)) to refactor as the component grows.

## 6) Extend the pattern
- Add a `status:"online"|"away"|"offline"` field and render a badge using `if … is`.
- Introduce a `children` slot to let callers inject extra actions.
- Thread design tokens (see the next tutorial) into `tone` and button styling so the component respects theming.
