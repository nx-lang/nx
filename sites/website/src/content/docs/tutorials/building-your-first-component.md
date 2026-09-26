---
title: 'Building Your First Component'
description: 'Walk through composing an interactive NX component from scratch.'
---

This tutorial builds a small, realistic component with typed props, actions, layout, and conditional rendering. It assumes you’ve completed [Getting Started](/tutorials/getting-started) and skimmed the [Language Tour](/language-tour/elements).

## 1) Define types and props
Start a file named `profile-card.nx`, or work in the [playground](/playground):

```nx
type User = { id:string name:string title?:string avatarUrl?:string }
type ProfileAction = { id:string label:string }

external component <Button label:string emits { Pressed { } } />
```

- Optional fields carry `?` on their name and may be left out.
- `Button` is an `external` component: the host application supplies it, and NX checks every use
  against its props and the `Pressed` action it emits.
- Keep types beside the component so they stay in sync.

## 2) Lay out the component
Add the component, with sensible defaults. Each block from here on continues the same file:

```nx fragment
component <ProfileCard
  user:User
  actions?:ProfileAction+
  tone:string = "neutral"
  emits { ActionChosen { actionId:string } }
/> = {
  <article className={"card tone-" + tone}>
    <header>
      <img src={user.avatarUrl ?? "/placeholder.png"} alt={user.name}/>
      <div>
        <h3>{user.name}</h3>
        if user.title? { <p>{user.title}</p> }
      </div>
    </header>
    <footer>
      for item in actions {
        <Button label={item.label} onPressed=<ProfileCard.ActionChosen actionId={item.id}/> />
      }
    </footer>
  </article>
}
```

- Attributes and body content accept expressions (including `if` and `for`) without leaving markup
  mode.
- Defaults keep the call site concise, and an optional prop such as `actions` can be left out
  entirely: it is then empty, and the `for` over it yields nothing.
- `emits` declares the actions the card produces. An action declared inline gets the public name
  `ProfileCard.ActionChosen`, and a button's `onPressed` handler builds one.

## 3) Handle actions
A handler is not a callback that runs code; it is an expression that builds an action record. The
screen that renders the card turns the card's action into one for the host:

```nx fragment
action ProfileActionRequested = { userId:string actionId:string }

let <ProfileScreen user:User/> =
  <ProfileCard
    user={user}
    tone="info"
    actions={
      <ProfileAction id="message" label="Message"/>
      <ProfileAction id="follow" label="Follow"/>
    }
    onActionChosen=<ProfileActionRequested userId={user.id} actionId={action.actionId}/> />
```

- `on<ActionName>` binds a handler to an action the component emits. Inside it, `action` is the
  emitted record, and the handler can also read props such as `user`.
- Because the card declares `ActionChosen { actionId:string }`, the compiler checks that
  `action.actionId` exists and that the handler returns an action.
- The host receives `ProfileActionRequested` and performs the side effect, such as sending a
  message.

## 4) Render it
End the file with a root element to see everything together:

```nx fragment
let currentUser = <User id="42" name="Kai" title="Designer"/>

<ProfileScreen user={currentUser}/>
```

Because `ProfileCard` is a `component`, the result is a `ProfileCard` record holding its props and
its bound handler. A host initializes that record to render the card and dispatches actions back to
it.

## 5) Validate and iterate
- Paste the whole file into the [playground](/playground): it compiles as you type and reports
  errors against your source.
- Use the patterns from the Reference (especially [Functions & Components](/reference/syntax/functions) and [if](/reference/syntax/if)) to refactor as the component grows.

## 6) Extend the pattern
- Add a `status:Status` field, with `type Status = online | away | offline`, and render a badge
  using `if user.status is { ... }`.
- Introduce a `content` property, such as `content extra?:Element`, to let callers inject extra
  markup into the footer.
- Give the card local state, `state { following:boolean = false }`, and have a Follow button
  return `<Update following={!following}/>` instead of an action.
- Thread design tokens (see the next tutorial) into `tone` and button styling so the component respects theming.
