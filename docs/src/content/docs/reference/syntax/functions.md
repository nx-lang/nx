---
title: 'Functions & Components'
description: 'Use `let` for functions and bindings, and `component` for declarations with emits or persistent state.'
---

NX separates `let` bindings from `component` declarations. Use `let` for values and reusable functions. Use `component` when a declaration needs an `emits` contract or persistent `state`.

Runtime note: `action` declarations parse, lower, and behave like records. Component init returns rendered output plus an opaque host-owned state snapshot. Dispatch consumes that snapshot together with a host-ordered batch, applies the component's own update records to its state, and returns the re-rendered output, the effect actions, and the next snapshot. See [Updating state](#updating-state).

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

## Content-marked Parameters

```nx
let <Dialog title:string  content body:Element /> =
  <section>
    <h2>{title}</h2>
    {body}
  </section>
```

- Prefix one property with `content` to declare where element body content binds.
- The marker is contextual: `content` remains a normal identifier outside property-definition positions.
- The same inline form works for element-style `let`, paren-style `let`, `component` props, emitted-action payloads, and component `state`.

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
    query:string = {placeholder}
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
- State defaults are evaluated once during initialization. After that, state changes only through the component's update record (see [Updating state](#updating-state)).
- Hosts own the serialized snapshot returned by initialization and must pass it back into later dispatch calls.
- Dispatch preserves host action order and returns effect actions in the same order handlers produce them.
- `Update` and `Property` are reserved: a component cannot emit an action named `Update` or `Property`, because `<Component>.Update` is its update record and `<Component>.Property` is its property union.

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

## Updating state

Every record, action, and component with `state` has a derived **update record**, `T.Update`, with
the same fields as `T` (a component's state fields, never its props), every one optional. An update
record is a patch: a field it leaves out means "unchanged", and a field set to `null` means "set
to null", which is allowed only where the field's type is nullable. Constructing one applies no
defaults and requires nothing.

```nx
type User = { name:string = "anon" email:string? }

let rename = <User.Update name="Ada" />      // only `name`; `email` is untouched
let clearEmail = <User.Update email={null} /> // `email` becomes null
let nothing = <User.Update />                 // a valid, empty patch
```

- `T.Update` includes fields `T` inherits, and cannot itself be extended; there is no `T.Update.Update`.
- An update record is an ordinary value: store it, pass it, return it, or send it to a server as the
  set of changes to apply. On the wire it carries `$type: "User.Update"` and only the fields that
  are present.
- A list field is replaced whole.

Inside a component, a bare `Update` tag names that component's own update record, even if the module
declares something else called `Update`. Outside a component, write the qualified form.

```nx
external component <Button label:string emits { Tapped { } } />

action Reset = { }

component <Counter step:int = 1 emits { Reset } /> = {
  state { count:int = 0 }

  <Row>
    <Label text={count} />
    <Button label="Add" onTapped=<Update count={count + step} /> />
    <Button label="Reset" onTapped={<Update count=0 /> <Reset />} />
  </Row>
}
```

A handler's result is routed by type:

- The enclosing component's own update record (`Counter.Update`) patches that component's state.
- An action the enclosing component emits (`Reset`, declared or inherited in `emits`) goes to its
  parent.
- Anything else inside a component is a compile error: add the action to `emits`, or return the
  component's own update record. At the root there is no component to patch or parent to reach, so
  any action or update record is an effect for the host.

A handler may return one value or a list of them, mixed freely; an empty list is an error. Handler
bodies are type checked with everything their binding site can see — the component's props and
state, enclosing `let` bindings and loop variables — plus `action`, typed by the emitted action.

State reads inside a handler are live. When dispatch runs a handler, a state field reads the value
it has at that moment, including patches applied earlier in the same batch, so tapping the counter
twice in one batch moves it by two. Props, `let` bindings, and loop variables keep the values they
had when the handler was created.

Dispatch runs a batch in order, applies each update record for the component to its state, collects
everything else as effects, and re-renders once at the end. Rendered output from initialization and
dispatch identifies each bound handler by a token; a host runs one by dispatching
`{ $type: "ActionHandlerInvocation", token, action }` with the snapshot returned alongside that
output. A token is valid only with that snapshot: every dispatch, even one whose batch is empty or
produces only effects, returns fresh tokens and retires the previous ones. Pure evaluation, which
has no snapshot to dispatch against, renders handlers without tokens. A handler that was bound
outside the component — at the root, or by a parent whose output the component renders — can still
be dispatched by its token, but it reads only what it captured and everything it returns is an
effect. The batch is atomic: if any entry fails — an unknown or stale token, a handler error, an
invalid update — dispatch fails as a whole and the snapshot the host supplied remains the current
state. Everything the host passes in — props, explicit state, and the actions in a batch, whether
or not a handler is bound for them — is checked against its declaration at every depth, so an
update record nested in a prop, a component value, an array, or an action payload meets the same
unknown-field and `null` rules as one the type checker saw.

## Property references

Every record, action, and component with `state` also has a derived **property union**,
`T.Property`: a constant union with one case per field of `T` (a component's state fields, never
its props), inherited fields first. A case names a field as a value, so a sort key, a column list,
or a validation rule is typed by the fields that exist rather than by `string`.

```nx
type Contact = { title:string subtitle:string }

external component <Table sortBy:Contact.Property columns:Contact.Property[] />

let table = <Table sortBy=subtitle columns={ Contact.Property.title Contact.Property.subtitle } />
let key: Contact.Property = {Contact.Property.title}
```

- `T.Property` behaves as any constant union: a bare case resolves at a site typed `T.Property`,
  `if key is { title => "Title" subtitle => "Subtitle" }` is checked for exhaustiveness, and on the
  wire a case is the bare field name, `"subtitle"`.
- `T.Property` includes the fields `T` inherits, cannot be extended, and has no derived declarations
  of its own; nor does `T.Update`. Two property unions are distinct types even when their cases
  share names.
- Inside a component, a bare `Property` names that component's own property union, as a bare
  `Update` names its update record. Outside a component, write the qualified form.

Four intrinsic functions work on update records without naming a record type. Their names are
reserved: they resolve before anything in scope, cannot be shadowed, and a declaration named
`apply`, `merge`, `diff`, or `changed` is rejected.

| Call | Result |
| --- | --- |
| `apply(record, update)` | `T`: the record with each present field of the update replaced. |
| `merge(first, second)` | `T.Update`: every field present in either, the later one winning. |
| `diff(before, after)` | `T.Update`: exactly the fields whose values differ, taken from `after`. |
| `changed(update)` | `T.Property[]`: the present fields, in declaration order. |

Every intrinsic keeps the absent-versus-null rule: a present `null` is carried and an absent field
is never invented, so `apply(u, merge(a, b))` equals `apply(apply(u, a), b)` and
`apply(a, diff(a, b))` equals `b`.

```nx
external component <Button label:string emits { Tapped { } } />

component <Counter step:int = 1 /> = {
  state { count:int = 0 last:Counter.Property = {Property.count} }

  <Row>
    <Label text={count} />
    <Button label="Add" onTapped=<Update count={count + step} last={Property.count} /> />
  </Row>
}

let touched(patch:Counter.Update): Counter.Property[] = {changed(patch)}
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
