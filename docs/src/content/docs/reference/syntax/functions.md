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
- Default values use `=` just like standard attributes. A call that leaves out `className` gets
  `"card"`.
- Invocation reuses the same structure but supplies values instead of types.
- An element-style function is called only as an element. `UserCard(currentUser)` is rejected: its
  attributes have no order a caller could rely on.

## Advanced Parameters

```nx
let <DataGrid
  data:object+
  columns:object+
  className?:string /> =
  <table className={className ?? "data-grid"}>
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

- An optional prop is marked on its name (`className?:string`) and reads as `string?` inside the
  body, so it takes a fallback with `??` or a presence test before it is used as one value.
- A default can read the parameters declared before it, and anything its own module can see.
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

## Type parameters

A component that draws a collection through a template needs to say that its items and its
template agree on one item type without fixing what that type is. A **type parameter** does that.
It is declared with the same `name:type` syntax as a prop, using the keyword `type` as the type,
and it must come first in the signature, after `extends` and before every prop:

```nx
type Contact = { name:string }

external component <SkiaLayout
  TItem:type
  itemsSource?:TItem+
  content children?:object+
/>

let contacts:Contact* = {}

<SkiaLayout TItem=Contact itemsSource={contacts} />
```

- Inside the signature and the body, `TItem` is a type like any other: `itemsSource?:TItem+`
  and a state field `first?:TItem` both work. It is distinct from every other type,
  including a same-named type declared outside the component, which it shadows.
- A use site supplies the type by name, as a bare type name: `TItem=Contact`. Any visible type
  qualifies — a record, a union, an alias, a primitive, or a type parameter of the enclosing
  component (`TItem=TItem` forwards it). Braced, quoted, and conditional forms are rejected.
- Leave it out when nothing needs it. A use site that binds only `children` writes
  `<SkiaLayout>...</SkiaLayout>` and no argument. A prop typed by an unspecified parameter
  accepts only the empty value `{}`; binding anything else reports that `TItem` was not
  specified and shows the `TItem=` form to add.
- A type parameter is not a prop. It carries no value, has no default, is never required, is
  not a field of the runtime record, and is not a case of the component's property union.
  Once type checking has consumed a `TItem=Contact` binding, nothing below the checker sees it.
- An emitted action's payload cannot be typed by a type parameter. `emits { pick { item:TItem } }`
  is rejected: the action is a record of its own, usable outside the component, where `TItem` is
  not a type.
- A derived component inherits its abstract base's type parameters open, ahead of its own, and
  cannot redeclare one. `<ContactList extends ItemsBase />` supplies `TItem` exactly as
  `<ItemsBase />` would.
- Name a type parameter `T` or a `T`-prefixed PascalCase name: `TItem`, `TKey`, `TValue`. A
  primitive type name such as `string` or `object`, and the built-in `Element`, are rejected;
  any other name is allowed and shadows a same-named declared type inside the component.

Type parameters are supported on component signatures and on plain `type` record declarations; an
action, state group, function parameter list, function type, alias and union cannot declare one.
A record's parameters differ from a component's in one way that matters: they are **not** erased,
so `<Range T=int/>` and `<Range T=float64/>` are different types. See
[Generic records](/reference/syntax/types#generic-records).

## Updating state

Every record, action, and component with `state` has a derived **update record**, `T.Update`, with
the same fields as `T` (a component's state fields, never its props), every one optional. An update
record is a patch: a field it leaves out means "unchanged", and a field set to the empty value
`{}` means "cleared", which is allowed only where the field is optional in `T` (`email?:string`).
Constructing one applies no defaults and requires nothing.

```nx
type User = { name:string = "anon" email?:string }

let rename = <User.Update name="Ada" />      // only `name`; `email` is untouched
let clearEmail = <User.Update email={} />     // `email` is cleared
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
unknown-field and cleared-field rules as one the type checker saw.

## Property references

Every record, action, and component with `state` also has a derived **property union**,
`T.Property`: a constant union with one case per field of `T` (a component's state fields, never
its props), inherited fields first. A case names a field as a value, so a sort key, a column list,
or a validation rule is typed by the fields that exist rather than by `string`.

```nx
type Contact = { title:string subtitle:string }

external component <Table sortBy:Contact.Property columns:Contact.Property+ />

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
| `changed(update)` | `T.Property*`: the present fields, in declaration order. |

Every intrinsic keeps the absent-versus-empty rule: a present empty field is carried and an absent
field is never invented, so `apply(u, merge(a, b))` equals `apply(apply(u, a), b)` and
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

let touched(patch:Counter.Update): Counter.Property* = {changed(patch)}
```

## Functions as values

A bare identifier that names a visible function — element or paren style, declared in the same
file, a same-library peer, or an import — is a value of that function's [type](/reference/syntax/types#function-types).
Bind it to a function-typed property or value by name:

```nx
abstract external component <DrawnNode />
type Contact = { name:string }
type RowTemplate = <function Item:Contact Index:int />: DrawnNode
external component <List extends DrawnNode
  TItem:type
  ItemsSource?:TItem+
  ItemTemplate?:<function Item:TItem Index:int />: DrawnNode
/>
external component <Label extends DrawnNode Text?:string />

let <ContactRow Item:Contact Index:int />: DrawnNode = <Label Text={"" + Index + " " + Item.name} />
let <Compact Item:Contact />: DrawnNode = <Label Text={Item.name} />

let contacts = { <Contact name="Ada" /> <Contact name="Kai" /> }
let full = <List TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} />
let short = <List TItem=Contact ItemsSource={contacts} ItemTemplate={Compact} />
let rows: RowTemplate+ = { ContactRow Compact }
```

A function value is a reference to its declaration and captures nothing: functions are module-level
and a body declares nothing of its own. A runtime renders one as the record
`{ "$type": "Function", "module": "<module>", "name": "ContactRow" }`, which a host — a list that
recycles its cells, say — calls once per item. A lexical binding of the same name shadows the
function, as it shadows any top-level name.

### Invoking a function-typed value

A parameter, prop or local `let` whose type is a function type is invoked as an element. Arguments
bind to the **type's** parameters by name, every parameter of the type is required, and the call
has the type's result type:

```nx
component <Highlight extends DrawnNode Item:Contact Row:RowTemplate /> = {
  <Row Item={Item} Index=0 />
}
let pinned = <Highlight Item=<Contact name="Zed" /> Row={Compact} />
```

The **subset rule** decides what happens at run time: the arguments reach the function by name, a
parameter the function does not declare is dropped (`Compact` never sees `Index`), and a parameter
it does declare is always present, because the type supplied it. A paren-style call on a
function-typed value, `Row(item, 0)`, is rejected with a diagnostic showing the element form:
positions would depend on the order of parameters the value's own declaration does not share.

## Paren-style Functions

```nx
let clamp(value:int, low:int = 0, high:int = 100): int = {
  if value < low { low } else { if value > high { high } else { value } }
}

let a = { clamp(150) }                   // 100
let b = { clamp(-5, -10) }               // -5
let c = <clamp value=150 high=120 />     // 120: an element call skips `low`
```

Use the paren style for small, general-purpose functions that a program calls in many places and
that read naturally with parentheses: `min`, `floor`, `rgb`, a formatter. Use the element style for
everything else, and always for markup. Neither is enforced; they are what the two styles are for.

- A paren-style function is called by position, `clamp(150, 0)`, or as an element,
  `<clamp value=150 />`, which binds arguments by name. Calling it with parentheses is the usual
  form.
- Parameters a caller may omit — optional ones (`title?:string`) and defaulted ones
  (`low:int = 0`) — come after every required parameter. `let f(a?:int, b:int)` is rejected.
- A positional call supplies every required parameter and may stop before any of the trailing
  ones. To skip one in the middle, call the function as an element and name the ones you pass.
- A parameter a call leaves out takes its default, or `{}` when it is optional.

## Defaults belong to the function

A default is evaluated by the function each time a call leaves its parameter out; it is never
copied into the calling code. When a library changes a default, every existing caller gets the new
one. A default sees the parameters declared before it and the declarations of the function's own
module, including private ones:

```nx
private let separator = " · "

export let join(first:string, second:string, sep:string = { separator }): string = {
  first + sep + second
}
```

A caller in another module writes `join("a", "b")` without being able to name `separator`. A default
cannot read a parameter declared after it, and it is checked against its parameter's type. A
function *type* takes no defaults, since whoever calls a function-typed value supplies every
parameter the type declares.

## See also
- Language Tour: [Functions & Bindings](/language-tour/functions)
- Reference: [Modules](/reference/syntax/modules)
- Grammar: [nx-grammar.md – Functions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#functions)
