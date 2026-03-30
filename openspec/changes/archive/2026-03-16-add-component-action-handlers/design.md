## Context

Component syntax already lets a declaration advertise emitted actions, and the parser can already
read call sites like `onSearchSubmitted=<DoSearch search={action.searchString} />`. The missing
pieces are semantic: `nx-hir` drops component declarations entirely, element properties are lowered
as ordinary eager expressions, and the interpreter evaluates every property immediately. That means
the current example fails as soon as `action.searchString` is evaluated because no `action` binding
exists yet.

This change needs to add the callback groundwork without taking on full component execution. The
language needs a way to recognize which `on<ActionName>` properties are handler bindings, preserve
those bindings lazily through lowering, and invoke them later with an emitted action payload.

## Goals / Non-Goals

**Goals:**
- Preserve enough component signature metadata in HIR to resolve emitted action handler bindings at
  call sites.
- Assign every inline emitted action a public qualified action type name of the form
  `<Component>.<Action>` and make that name usable elsewhere in NX code.
- Lower matching `on<ActionName>` properties as lazy handler expressions instead of ordinary eager
  property values.
- Bind an implicit `action` identifier inside handler bodies without introducing general lambda
  syntax.
- Add interpreter support for capturing, storing, and invoking lowered handler callbacks.
- Require handler invocation to produce one or more action values so later component runtime work can
  consume a stable callback contract.

**Non-Goals:**
- Execute component initialization, rendering, or dispatch loops in this change.
- Introduce general-purpose closures, anonymous functions, or arrow-function syntax.
- Reserve every `on...` property name globally for components or ordinary elements.
- Define host bindings or FFI contracts for component callbacks before the component runtime API
  exists.

## Decisions

### 1. Add a lightweight component declaration model to HIR

`nx-hir` should preserve component declarations as a first-class item instead of discarding them
after parsing. This change only needs signature-level metadata:

- component name
- declared props
- emitted action entries, preserving whether each emit is an inline definition or a shared action
  reference and, for inline definitions, the public action type name they create
- source span

The lowered component item does not need render-body or state execution semantics yet. Its job in
this change is to give lowering and the interpreter a stable place to resolve emitted action names.

Lowering should collect component signatures before lowering element expressions so handler
recognition does not depend on declaration order within a module. The same prepass should also make
public inline emitted action names available to later record and type lookup.

Alternative considered: keep components out of HIR and infer handler bindings from raw property name
strings in the interpreter. Rejected because handler recognition depends on declared emits, and that
metadata needs to survive lowering for diagnostics and future runtime work.

### 2. Lower matching `on<ActionName>` properties as a dedicated lazy expression

Matching handler bindings should keep the ordinary element property surface syntax, but the lowered
representation should be explicit. The simplest shape is a new expression variant such as:

```text
Expr::ActionHandler {
  component: SearchBox,
  emit: SearchSubmitted,
  body: ExprId,
  span,
}
```

During lowering of a component invocation:

- if the target tag resolves to a component and the property name matches one of its emitted actions
  via the `on<ActionName>` convention, lower the property value as `Expr::ActionHandler`
- temporarily bind `action` in scope while lowering the handler body so expressions like
  `action.searchString` lower successfully
- otherwise, preserve the property as an ordinary prop

This keeps `onClick`-style ordinary props working for components that do not emit `Click`, while
still giving emitted handlers an explicit HIR shape.

Lowering should also reject ambiguous component signatures where a declared prop name collides with a
generated handler property name for an emitted action, such as `onSearchSubmitted` alongside an emit
named `SearchSubmitted`.

Alternative considered: create a separate `action_handlers` collection on `Element` and remove the
matching properties from the normal property list. Rejected because a dedicated handler expression is
smaller, keeps the existing element shape mostly intact, and still gives later runtime work a clear
hook.

### 3. Evaluate handler expressions into closure-like runtime values

The interpreter should treat `Expr::ActionHandler` as a lazy callback. Evaluating the expression
should return a new runtime value variant, for example:

```text
Value::ActionHandler {
  component: SearchBox,
  emit: SearchSubmitted,
  body: ExprId,
  captured: { ...visible variables except action... }
}
```

The handler body must not run when the surrounding component invocation is evaluated. Instead, the
interpreter snapshots the currently visible lexical variables and stores them with the handler value.
This allows call-site expressions like:

```nx
let render(userId:string) =
  <SearchBox onSearchSubmitted=<DoSearch userId={userId} search={action.searchString} /> />
```

to keep `userId` available when the callback is invoked later.

This requires a small `ExecutionContext` addition to expose the visible variables for capture.

Alternative considered: defer capture support and allow handler bodies to reference only the implicit
`action` value. Rejected because the syntax is an arbitrary expression today, and forbidding lexical
captures now would either make the feature surprisingly narrow or force a breaking semantic change
later.

### 4. Add an interpreter helper that invokes handlers and normalizes results

Because full component dispatch is out of scope, the interpreter should expose a focused helper for
invoking a lowered handler value with an emitted action input. Invocation should:

- accept a `Value::ActionHandler` plus an incoming emitted action value
- create a fresh execution context seeded with the captured variables and an implicit `action`
  binding
- evaluate the stored handler body expression once
- normalize the result into a non-empty ordered list of actions

Normalization rules:

- a single action record becomes a one-item list
- an array of action records stays an array in order
- empty arrays and non-action values are runtime errors

For emitted action inputs:

- shared action references should validate against the referenced `action` declaration
- inline component-scoped emits should validate against the synthesized public action record named
  `<Component>.<Action>`

This gives later component runtime work a reusable callback invocation primitive without forcing this
change to define init/render/dispatch.

Alternative considered: wait for the full component runtime change and skip interpreter support now.
Rejected because eager property evaluation already makes the syntax unusable; the groundwork needs a
runtime representation immediately.

### 5. Synthesize public component-scoped action records for inline emits

An inline emitted action such as:

```nx
component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }
```

should introduce a public action record named `SearchBox.ValueChanged`. That qualified name should be
usable anywhere NX already accepts an action or record name, including:

```nx
let makeChange(value:string) = <SearchBox.ValueChanged value={value} />
let log(change:SearchBox.ValueChanged) = change.value
```

Lowering should therefore synthesize an `Item::Record` for each inline emitted action using the
qualified public name and `RecordKind::Action`. The component item should retain the local emit name
(`ValueChanged`) and a pointer to the public name (`SearchBox.ValueChanged`) so both handler binding
and general NX code resolve to the same action identity.

This is implementable with the current representation because `Name` and existing qualified
markup/type syntax already preserve dotted names as raw text. No new identifier model is required.

Alternative considered: keep inline emits structural and postpone public naming to the later
component runtime change. Rejected because the public type name is now a language contract, and
locking it down later would risk incompatible synthetic names such as `SearchBoxValueChanged`.

## Risks / Trade-offs

- Closure capture increases interpreter value complexity -> keep capture scope limited to visible
  lexical variables and add focused tests for shadowing and call-site parameters.
- Public `Component.Action` names must stay aligned with future runtime work -> treat the synthesized
  qualified action record name as the canonical identity for inline emits and reuse it in later
  component runtime design instead of introducing flat synthetic names.
- `on...` handler recognition could conflict with conventional prop names -> only treat `on...`
  properties as handlers when the target component actually emits the matching action, and reject
  direct prop/handler name collisions.
- This change will partially supersede the current companion-function direction in
  `add-component-runtime-support` -> keep this proposal explicit that inline handler bindings become
  the callback model future component runtime work should build on.

## Migration Plan

No source migration is required for existing modules that do not use component action handlers.
Implementation should proceed in this order:

1. Add HIR component metadata, a component-signature prepass, and lowering for `Expr::ActionHandler`
   with diagnostics for ambiguous or unknown handler bindings plus synthesized
   `Component.Action` record items for inline emits.
2. Extend interpreter values and execution context capture support, then add a focused handler
   invocation API with result normalization and runtime validation against shared or synthesized
   inline emitted action records.
3. Update examples, docs, and tests to cover shared actions, inline emits, public
   `Component.Action` names, captured variables, and invalid handler results.

If rollback is required, remove the new handler expression/value variants together so the syntax
falls back to its previous unsupported state cleanly.

## Open Questions

- Should future host bindings surface raw handler values directly, or keep handler invocation fully
  behind component runtime APIs?
