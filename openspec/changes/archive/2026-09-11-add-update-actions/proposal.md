## Why

NX components can declare `state`, emit actions, and bind `on<Action>` handlers, but nothing in the
language can change a state field after initialization: a handler must return actions, dispatch
hands them to the host as effects, and the runtime carries the prior state forward unchanged. Every
interactive example in the playground is blocked on this one gap, and it was explicitly deferred by
`add-component-runtime-support` as "declarative state-update actions". This change fills it, and
does so with a value that is useful beyond components: a patch record.

## What Changes

- Add a derived **update record type** `T.Update` for every record-shaped declaration `T` (plain
  records, actions, and a component's `state`). An update record has the same fields as its target,
  every field optional; an absent field means "unchanged" and `null` means "set to null" and is
  legal only where the target field is nullable. Update records are ordinary values: they can be
  constructed with element syntax (`<User.Update name="Ada" />`), stored, passed, returned, and
  serialized, so a client can build the set of changes a server should apply.
- Inside a component body, the bare tag `Update` resolves contextually to that component's own
  `<Component>.Update`, so a handler can write `onTapped=<Update count={count + 1} />`. Outside a
  component body the bare form is a diagnostic that names the qualified spelling.
- Route handler results by type. An update record for the enclosing component patches that
  component's state. An action the enclosing component lists in `emits` is emitted to its parent.
  Any other action is a host effect at the root and a compile error inside a component.
  **BREAKING**: a handler inside a component body may no longer return an action that component
  does not emit.
- Type check handler bodies. Today the checker skips them entirely; after this change the body is
  inferred with the component's props and state in scope and `action` bound to the emitted action
  type, and its result is checked against the routing rule above.
- Make state reads inside a handler live: a state identifier in a handler body denotes the state
  current at dispatch time, so two `<Update x={x + 1} />` patches in one batch move twice. Other
  captured locals keep their snapshot semantics.
- Extend component dispatch. A dispatch batch may name a handler taken from the rendered tree
  together with the action to feed it; the runtime invokes it against the live state, applies
  update records in order with full validation, collects the rest as effects, re-renders once, and
  returns rendered output, effects, and the next snapshot. The whole batch fails atomically.
  Rendered output identifies each bound handler by a token the host passes back.
- Carry update records through NX IR and the TypeScript IR runtime (patch construction and
  application with absent-versus-null semantics), and through `typegen` to TypeScript and C#
  DTOs, where absence must be distinguishable from null.
- Update the language reference, the language tour, `nx-grammar.md`, and `examples/nx/component.nx`.

Out of scope, named here so the follow-ups are known: an `apply(record, patch)` built-in and patch
composition; list operations inside a patch (a list field is replaced whole); encoding handler
values in NX IR and dispatching them from the TypeScript runtime, which is the separate
`ir-action-handlers` work; and any playground work, which builds on this change.

## Capabilities

### New Capabilities
- `update-records`: the derived `T.Update` type: its shape, the absent-versus-null rule,
  construction and validation, the contextual bare `Update` inside a component, and its canonical
  wire encoding.

### Modified Capabilities
- `component-action-handlers`: handler bodies are type checked; results are routed to state,
  parent, or host; state reads inside handlers are live.
- `component-runtime-bindings`: dispatch accepts handler invocations, applies updates, and returns
  rendered output; rendered output carries dispatchable handler tokens; the batch is atomic.
- `component-syntax`: state changes only through the component's update record; `Update` is a
  reserved emit name.
- `nx-ir-format`: IR declares derived update records with their target.
- `typescript-ir-runtime`: update records normalize without defaults and patch host-owned state.
- `cli-code-generation`: generated TypeScript and C# surfaces include update types with distinct
  absence.
- `dotnet-binding`: managed dispatch exposes handler invocations and the rendered result, and
  typed DTOs can express absence.

## Impact

- `crates/nx-hir`: synthesize `T.Update` records beside inline emit records; new record kind;
  contextual `Update` resolution; handler owner metadata.
- `crates/nx-types`: infer handler bodies; update construction rules; result routing check;
  reserved `Update` emit name.
- `crates/nx-interpreter`: update record construction; handler owner and live state at invocation;
  dispatch by handler token; patch application; snapshot handler table; re-render in dispatch.
- `crates/nx-api`, `crates/nx-ffi`, `bindings/c/nx.h`, `bindings/dotnet`: dispatch input and result
  shapes; handler token in rendered values; `NxOptional<T>` for generated C# update DTOs.
- `crates/nx-codegen` and `runtime/typescript`: IR update record declarations; patch normalization.
- `crates/nx-cli/src/typegen`: generated update types for TypeScript and C#.
- `docs/`, `nx-grammar.md`, `examples/nx/component.nx`.
- No grammar change: `Update` and `T.Update` are already valid element names.
