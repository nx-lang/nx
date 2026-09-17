## Context

See proposal.md for motivation. What the design builds on:

- Lowering already produces `ast::Expr::ActionHandler { component, emit, action_name,
  action_module_identity, owner, body, span }` (`crates/nx-hir/src/components.rs`), and the
  checker infers the body with `action` bound and the owner's props and state in scope. The Rust
  interpreter evaluates it to `Value::ActionHandler` with a by-value capture of visible variables
  plus `owner_state`, the owner's state names that still reach the state binding; at dispatch an
  owned handler's `owner_state` names are overwritten from the working state
  (`invoke_action_handler_in`), tokens are `h<generation>-<n>` assigned by a depth-first walk that
  visits arrays in order and record fields in sorted name order (`RenderedHandlers::assign_tokens`;
  elements and descriptors are records there, so the walk enters them), initialization uses
  generation 1 and each dispatch increments it, and the rendered encoding is
  `{ $type: "ActionHandler", action: "<public action name>", token }` (`nx-api`'s `to_nx_value`).
  The public action name is `Button.Tapped` for an inline emit and `SearchSubmitted` for a shared
  one: in both cases the name of the action record's declaration.
- NX IR is the schema 3 binary image (`docs/nx-ir-format.md`): flat tables of 32-bit cells, node
  kinds `0` to `18`, declaration kinds `0` to `5`, layouts fixed by `crates/nx-codegen/src/ir.rs`
  and checked by the layout table in `ir_image.rs` and by the TypeScript reader. A reader refuses
  a kind number it does not know. Schema 3 is on this branch and untagged; no schema 3 image has
  been published in any encoding, which is the same ground `binary-nx-ir` stood on when it changed
  the encoding without a version bump.
- `crates/nx-codegen` has one expression builder (`builder.rs`) shared by the NX IR target and the
  executable TypeScript/JavaScript targets; `build_expression`'s `ActionHandler` arm is where every
  target fails today. The executable emitter (`emit.rs`) already matches exhaustively over
  `CodegenExpressionKind` and reports unsupported kinds with `codegen-unsupported-construct`.
  Locals are slots: the builder tracks names with `LexicalScope`, `ir.rs` assigns each `let` and
  `for` binding the next integer of the declaration's frame, and the TypeScript runtime binds
  values by slot in a `frame` array, so a shadowed name is a different slot.
- The IR component declaration carries props, state, body, and flags, but not emits. The
  TypeScript runtime's `normalizeFields` rejects any key a contract does not declare, which is how a
  descriptor with `onTapped` would fail even if the builder emitted it. The pruning of derived
  declarations (`referenced_derived_declarations`) walks the module's whole expression arena, so an
  update record named only inside a handler body is already kept.
- The TypeScript runtime already has an internal non-canonical value, `{ $nxKind:
  "functionReference" }`, that never reaches a host, and already applies update records to state
  (`applyComponentStatePatch`) and normalizes update-record construction without defaults.
- `nx-api`'s `from_nx_value` refuses an `ActionHandler` record, so through the FFI and the .NET SDK
  a host cannot hand a parent-bound handler to a child instance; the interpreter's own tests do it
  by passing the parent's rendered `Value` as the child's props. The TypeScript runtime needs a
  public equivalent, or its parent-bound scenarios are unreachable.
- Rust-versus-TypeScript parity runs through the conformance corpus (`specs/ir-conformance`):
  `ir_corpus_tests.rs` records each entrypoint's value through `nx-api`, and
  `runtime/typescript/test/corpus.test.mjs` checks the runtime against it. The corpus's coverage
  check fails when a node or declaration kind is covered by no program. The CLI has no lifecycle
  command, so `emitted-ir.test.mjs` cannot compare dispatch against the native runtime.
- The playground (`sites/playground`) is the browser host. Its DrawnUI catalog (`catalog/skia.nx`)
  is generated from the vendored DrawnUI TypeScript by `scripts/generate-catalog.mjs`, which
  resolves each control's props through the TypeScript checker and omits every function-typed
  member, events included, recording them in `catalog/omitted.json`. The renderer
  (`src/render/DrawnTree.tsx`) walks the value `root()` evaluates to, turns each catalog element
  into a DrawnUI control through the vendored React reconciler, and expands a descriptor of an
  authored component by calling `initializeComponent` once and drawing what it rendered; nothing
  keeps the instance, and nothing dispatches. DrawnUI events are optional function members on the
  controls (`Tapped?: (sender, e) => void`, `Toggled?: (sender, value: boolean) => void`) that the
  reconciler assigns like any prop. The example check (`scripts/check-examples.mjs`) expands
  authored components the same way the renderer does, so an example whose descriptor carries a
  handler property would fail there first.
- NX has string `concat` and no conversion from a number to a string. Most of the readouts the
  static examples dropped ("SelectedIndex=2", a slider's value, a node count) need one, so those
  examples stay static even with handlers and state available.

## Goals / Non-Goals

**Goals:**
- Handlers travel through IR as data the same way every other node does, with no evaluated
  environment in the image.
- The TypeScript dispatch is observably the same as the Rust dispatch: same routing, same live-state
  rule, same atomicity, same tokens. The corpus compares rendered output, tokens included, and
  effects byte for byte.
- An older runtime refuses a module with handlers by feature name. A module without handlers lists
  no new feature.
- The executable TypeScript/JavaScript targets change nothing observable.
- A tap in the playground runs the handler the author bound, patches the state of the component
  that owns it, and redraws, with results that leave the tree shown to the visitor.

**Non-Goals:**
- Reactivity or any orchestration of parent and child instances inside the runtime. The runtime
  dispatches one instance; composing instances is the renderer's job (D10 to D12). The parent
  route in D6 gives a host the values it needs for that; it does not compose anything.
- Serializing a TypeScript instance. The Rust snapshot is bytes because the interpreter is
  stateless across an FFI boundary; a TypeScript host holds the instance object in memory. Nothing
  in the fiddle's share path needs it, since a share embeds IR and rebuilds instances at mount.
- Keeping images byte-identical across the change. Adding `emits` to the component layout changes
  every image that declares a component; the corpus is regenerated and reviewed as its README
  describes. If schema 3 ships before this change lands, the layout change becomes schema 4.
- Invoking a handler from token-less rendered output, and applying a component's own update record
  as a batch entry (see the proposal's out-of-scope list).
- The DrawnUI fiddle's renderer, which lives in its own repository, and porting the static examples
  whose readouts need a conversion NX lacks.

## Decisions

### D1. Node kind 19, `actionHandler`, carrying references and a slot, never values

Layout: `[19, ref, str, ref, slot, ref?, node]` — the component the handler answers, the emit's
name, the action record the handler accepts, the slot of its `action` binding, the owner component
when the binding sits inside a component body, and the body. `kinds::node::ACTION_HANDLER = 19`
joins `NAMES`; `CodegenExpressionKind::ActionHandler` mirrors the arm with `CodegenReference`s for
the component, the action and the owner. The builder's arm pushes a lexical scope, inserts `action`,
builds the body, and pops; `ir.rs` allocates the slot the way it does for `let`, so `action` takes
the next integer of the enclosing frame. The component and action references are resolved from the
handler's `action_module_identity` and the component's declaring module, the same way a
descriptor's `component` reference is built, so a handler for an imported component resolves
without name lookup. The action record's declaration is always emitted: only derived update records
and property unions are pruned, and an inline emit's record (`Button.Tapped`) is an ordinary record.

There is no `action_name` operand. The public name the rendered `ActionHandler` carries is the
action reference's declaration name, which is `Button.Tapped` or `SearchSubmitted` exactly as the
interpreter spells it, so encoding it twice would only let the two disagree.

The owner is a reference, not an "owned" flag, because a handler is not always dispatched by the
component whose body wrote it. Content children are values: `<Stack><Button onTapped=... /></Stack>`
inside `Page` puts a `Page`-owned handler in `Stack`'s rendered output, and a dispatch against the
`Stack` instance must treat it as unowned (captured values only, every result an effect), which is
what the interpreter's `handler_is_owned_by` does by comparing the owner to the instance's
component. A build-time flag could not tell the two apart.

Why references and a slot rather than names: every other node resolves declarations by reference
and locals by slot, and `owner_state` needs no encoding at all. In the interpreter the live-state
rule is "overwrite the captured value for each owner state name that still reaches the state
binding"; in the IR the body already references the state field's slot when it is unshadowed and a
`let` or `for` slot when it is not, so "overwrite the owner's state slots from the working state" is
the whole rule. A corpus scenario whose `for` variable shadows a state field pins this.

The feature is `action-handlers-v1`, listed by `required_features` only when a module contains the
kind, like `update-intrinsics-v1`. The unknown-kind rule would already refuse the module; the
feature names the reason in the diagnostic. The runtime ABI stays `nx-ir-runtime-v2`: nothing
about how an existing kind evaluates changes.

`visit_expressions` in `ir.rs` and every other walk over `CodegenExpressionKind` descend into the
body, so an intrinsic call inside a handler still lists `update-intrinsics-v1`.

Alternative: encode the handler as a synthesized function declaration plus a reference. Rejected:
it would appear in entrypoint tables and declaration counts, and capture semantics would need a
closure construct the IR does not have.

### D2. The executable targets reject handlers in the emitter, not the builder

The `ActionHandler` arm of `build_expression` builds the kind for every target. `emit.rs` gains a
`CodegenExpressionKind::ActionHandler` arm that pushes the existing diagnostic (same code, same
"action-handler codegen is not supported by this non-reactive executable target" message) and
emits nothing. `executable-code-generation`'s "Generated component code rejects action-handler
bindings" requirement keeps its wording; only the phase that enforces it moves. The
`nx-ir-format` scenario that said IR emission fails on a handler keeps its title, since a MODIFIED
requirement cannot drop a scenario, and now says emission succeeds while the executable targets
still refuse. The
.NET and FFI tests that asserted `GenerateNxIr` fails on a handler assert an image instead; their
`GenerateJSProgramModule` cases stay as they are.

### D3. Component declarations carry `emits`

The component layout becomes `[3, str, [field...], [field...], node?, flags, [[str, ref]...]]`:
the emits are appended, each an emit name and a reference to its action record, from the effective
contract (`nx_hir::effective_component_contract`), inherited emits included, in declaration order.
An inherited inline emit's reference names the base's module, which `EffectiveEmit.module_identity`
gives. Appending keeps every existing operand index in the reader, the explainer and the runtime.
The runtime needs it for three things the interpreter does from the contract: validate an action
entry against what the component emits, name the handler property (`on<Emit>`, the interpreter's
`component_handler_prop_name` convention, mirrored rather than encoded per emit), and recognize a
descriptor's handler properties. A component with no emits carries an empty list.

`ir_image.rs`'s layout table and the TypeScript reader (`#fields` beside a new `#emits`) validate
the operand; `ir_explain.rs` prints an `emits` block after `state`, one line per emit as
`<name> = <module>:<action>`, so a corpus diff reads.

### D4. Handler values are internal until the boundary, where they canonicalize

Evaluating the node yields `{ $nxKind: "actionHandler", node, linked, captured: frame.slice(),
owner }`, the `functionReference` pattern. `captured` is a copy of the frame at creation (the
by-value snapshot). Handler values live inside rendered trees and descriptor props during evaluation
and are replaced at every public boundary by `canonicalizeRendered`, a walk that mirrors the
interpreter's `to_nx_value` and `assign_tokens`: arrays in order, object keys in sorted order, each
handler becoming `{ $type: "ActionHandler", action: <action declaration name> }` plus `token` when
a generation is supplied. The walk returns both the canonical tree and the `Map<token, handler>` it
collected. `evaluateFunction`, `constructComponentDescriptor`, and `evaluateComponent` canonicalize
without a generation; `initializeComponent` uses generation 1 and dispatch uses the instance's
generation plus one, matching `RenderedHandlers::new(1)` and `render_generation + 1`.

Sorted-key order at every object is what makes tokens equal across runtimes; the TypeScript
object's insertion order is not used for numbering. Elements, descriptors, records and union-case
payloads are all objects to the walk, as they are all `Value::Record` to the interpreter's.

### D5. Descriptor handler properties bypass normalization and ride on the descriptor

Wherever a descriptor's properties are normalized against a component's props — the descriptor
node in `evalComponentDescriptor`, and the props a host supplies to `initializeComponent`,
`evaluateComponent` and `constructComponentDescriptor` (a component-typed value inside host input
is not normalized by the runtime, so there is no third site) — the properties are split first: one
whose name is `on<Emit>` for an emit in the target's `emits` is a handler property and is set
aside; the rest go through `normalizeFields` unchanged. A handler property that matches no emit
fails with `nx-ir-boundary-field` naming the property and the component, the diagnostic the checker
gives the same mistake at compile time. A handler property whose value is a handler for another
emit or component fails with `nx-ir-type`, as the interpreter's prop initialization does. The
handler value is attached to the descriptor object after normalization, so it canonicalizes with
the rest of the tree.

Alternative: declare handler props in the contract as typed fields. Rejected: they are not props in
the language (a component cannot read `onTapped`), and normalizing them would require a type for
handler values that nothing else uses.

### D6. The instance is an immutable object; a parent instance resolves handler props

```ts
interface NxComponentInstance {  // opaque to hosts
  readonly component: string; readonly reference: NxIrReference;
  readonly props: Record<string, NxCanonicalValue | ActionHandlerValue>;
  readonly state: Record<string, NxCanonicalValue>;
  readonly handlers: ReadonlyMap<string, ActionHandlerValue>;
  readonly generation: number;
}
initializeComponent(program, name, props?, options?)   → { rendered, state, instance }
  // options.parent?: NxComponentInstance
  // options.state?: Record<string, NxCanonicalValue>
dispatchComponentActions(program, instance, batch, options?) → { rendered, effects, state, instance }
```

`ComponentInitResult` keeps `rendered` and `state` so existing callers are untouched.

A host initializing a child from a parent's rendered descriptor passes that descriptor's fields as
`props` and the parent's instance as `options.parent`. Before normalization, every
`{ $type: "ActionHandler", token }` record in the props, at any depth (a handler property, or a
handler inside a content child), is replaced by the handler the parent's table holds under that
token; a token the table lacks fails with `nx-ir-handler-token`, and an `ActionHandler` record
with no `parent` fails with `nx-ir-boundary-field`, since without a table the record names nothing.
The resolved handler then takes the D5 route: a handler property is kept on the instance and, as
in the interpreter, is never bound in the body's scope; a handler inside a content child stays in
the value. This is the TypeScript counterpart of the interpreter's tests passing the parent's
rendered `Value` as the child's props, made explicit because canonical output cannot carry the
handler itself. What the route substitutes is the parent's handler value itself, not a copy: a
host holding both instances can then tell that the handler a child's table holds under one token
is the one the parent's table holds under another, which is how the playground finds the instance
a handler belongs to (D12). A runtime test pins the identity.

`options.state` is the state to use in place of the initial one, validated as a complete state the
way `evaluateComponent` validates its `state` argument, with tokens assigned as for any
initialization. It exists so a host can re-render an instance whose props changed without losing
the state it holds: the runtime has no "update props" operation, and needs none, because
initializing again with the old state and the new props is that operation. A partial state is
refused like any other invalid state; the host holds the complete one.

Dispatch never mutates the instance it is given: state is copied before the first entry, the handler
table for the next render is fresh, and a failure throws `NxIrRuntimeError` before anything is
returned, which is the atomicity the spec asks for. Re-dispatching against the old instance after a
failure works because nothing about it changed. Stale tokens are rejected naturally: a token is
looked up in the supplied instance's table only, and generations never repeat.

Alternative: a mutable instance the runtime updates in place. Rejected by the existing
"pure with respect to runtime-held component instances" requirement and because immutability is
what gives atomicity for free.

### D7. Dispatch mirrors `dispatch_component_actions_with_limits` entry for entry

For each batch entry, in order:

- `{ $type: "ActionHandlerInvocation", token, action }`: look up `token` in `instance.handlers`
  (unknown → `nx-ir-handler-token`), check `action.$type` against the handler's action declaration
  name (mismatch → `nx-ir-type`), normalize the action through the action record's declaration
  (host input, so full construction), then invoke: `env = captured.slice()`; if the handler's
  `owner` resolves to the instance's declaration, for each state field `i`
  `env[props.length + i] = working[field.name]`; `env[actionSlot] = action`; evaluate `body` in
  the handler's linked module. Normalize the result like `normalize_handler_result`: a record
  becomes a one-element list, an empty list is an error, each item must be a record. For an owned
  handler, an item whose `$type` is the owner's update record (`declaration.kind.updateTarget`
  names the component) is applied through the existing `applyComponentStatePatch` logic against
  the working state; every other item is pushed to `effects`. For a handler that is not owned,
  every item is an effect.
- Any other record: it must name an emit of the instance's component (else
  `nx-ir-component-action`), is normalized as host input against that action record, and if
  `instance.props` holds a handler under `on<Emit>` the handler is invoked with captured values
  only and all results are effects; otherwise the entry is a no-op.

After the batch, evaluate the body with props and the working state bound by slot (what
`evaluateComponent` does), canonicalize with generation `instance.generation + 1`, and return the
canonical tree, `effects`, the working state, and a new instance holding the new handler table.

### D8. Parity lives in the conformance corpus

A corpus `program.json` may list `lifecycles`: each names a module and a component, optional
props, and an ordered list of batches, each batch a list of entries as a host would send them,
with literal tokens (`h1-1`), since the token scheme is deterministic and the literal is itself the
parity claim. `ir_corpus_tests.rs` initializes the component through
`initialize_component_program_artifact`, dispatches each batch in turn through
`dispatch_component_actions_program_artifact`, and records under `identity::Component` in
`results.json` the initial rendered output and, per batch, the rendered output and effects, all
canonical values with tokens. State is not recorded because `nx-api` returns it only inside an
opaque snapshot; the rendered output reflects it. `corpus.test.mjs` runs the same lifecycle
through `initializeComponent` and `dispatchComponentActions` and compares each value with
`stableJson`, as it does for function results.

The corpus gains one program, `handlers`, covering node kind 19, `emits` on a declaration, and
these lifecycle scenarios: the `Counter` from `examples/nx/component.nx` (patch plus emit, an
emitted action nobody bound, an empty batch), a two-invocation batch reading live state, a handler
inside a `for` loop whose variable shadows the state field (NX has no `let` expression, so the loop
is the shadowing binding) and one inside a descriptor prop (token order), and a `Page` whose
content children carry its handlers. The corpus manifest's coverage check is what makes the program
mandatory.

Scenarios `nx-api` cannot reach stay in `runtime/typescript/test/runtime.test.ts`: the parent
route (D6), a parent-bound handler returning the parent's update record, and a handler in a
content child dispatched against the child (unowned), all of which need a parent instance, run
against the corpus's `handlers` image; atomicity, stale tokens, the wrong action, an unknown emit,
the no-op entry, an invalid update, and a `$nxKind` never surviving canonicalization run against an
image the test writes. Their expected values are the interpreter's, taken from its own tests
(`update_records_from_a_parent_bound_handler_are_effects` and the `COUNTER_SOURCE` cases).

Alternative: drive Node from `crates/nx-codegen/src/tests.rs` as the executable targets' parity
tests do. Rejected: the corpus already is the cross-runtime contract, its results are reviewable
JSON rather than a script's stdout, and a second runtime starts from it.

### D9. DrawnUI events become catalog emits, declared where DrawnUI declares them

The generator treats an optional function-typed member whose return type is `void`, or a union
containing `void`, as an event, and declares it as an emit on the component that stands for the
class declaring it, abstract bases included: `SkiaControl` carries `Tapped`, `ChildTapped`,
`ContextMenu` and `ConsumeGestures` once, and every control below inherits them through the
catalog's `extends` chain, the same way it inherits props. The emit keeps DrawnUI's name, so the
handler property is `on<Event>` (`onTapped`, `onToggled`), the rule the language already fixes.
Callback parameters after the leading `sender` become the emit's payload: one whose type maps to
`string`, `float64` or `boolean` becomes a field under the parameter's own name (`Toggled { value:
boolean }`, `TextChanged { text:string }`, `SelectedIndexChanged { index:float64 }`); one whose
type has no NX mapping (`ControlTappedEventArgs`, `SkiaGesturesParameters`, `SKPoint`, `Error`) is
dropped and recorded in `omitted.json` as a dropped parameter, with its type as the reason, so
`Tapped { }` carries nothing and `Error { }` loses its message, both on record. A function member
whose return type is something else (`ItemTemplate: () => SkiaControl`, `ProcessJson: (json) =>
string`) is not an event and stays omitted as before.

`catalog-meta.json` records, per registered tag and inherited events included, each event's
parameter names in order (`"events": { "Tapped": [], "Toggled": ["value"] }`), which is what the
renderer needs to build an action record from a callback's arguments. The action's `$type` is not
recorded there: the rendered `ActionHandler` record carries it as `action`, which the runtime takes
from the prepared declaration's `emits`, so the metadata cannot disagree with the image.

`ContextMenu` returns `boolean | void` upstream, "true to suppress the browser menu". An emit has no
return value; the renderer's callback returns `true` whenever a handler is bound, which is what
binding one means in the demos.

Alternative: payload records for the engine argument types. Rejected: a `ControlTappedEventArgs`
holds a control and gesture parameters that have no NX expression, and no example reads a tap's
position.

### D10. The renderer is an instance tree: one node per authored component use, keyed by position

`src/render/instances.ts` holds the tree, plain TypeScript with no React: a map from a position in
the drawn tree (the same key path `drawValue` already threads) to a node holding the runtime
instance, its rendered output, the descriptor it was initialized from, and its parent node. A
drawing pass begins, visits every authored descriptor it meets, and finishes by dropping the nodes
it did not visit. A visit creates the node on first sight, initializing it from the descriptor's
fields with `options.parent` set to the enclosing node's instance; keeps it while the descriptor is
the same object; and, when the parent redrew and handed it a new descriptor, re-initializes it with
the new fields and `options.state` set to the state it holds. Props flow down and state stays, and
an instance is keyed by its position, so a child that moves loses its state as a React component
would. A dispatch redraws the whole value tree from the root through the same pass, without a
compile: only the subtree under the instance that changed sees new descriptors, and the DrawnUI
reconciler applies only the props that differ. Recompiling replaces the program and the tree, so
editing resets every instance; the last good drawing survives a broken edit as it does today.

Why re-initialize on every parent redraw rather than compare the fields: a handler property's
record changes its token every generation and a handler in a content child changes with it, so a
comparison would either reset the child on every parent redraw or keep captures a generation old.
Re-initializing with the kept state is both simpler and right, and its cost is proportional to the
subtree, which a playground can afford.

Why a module of its own rather than a React component per instance holding the runtime instance in
React state: the drawing is built eagerly today, in one place that catches a body's failure and
keeps the last good drawing, and a React component would move initialization into React's render,
where a runtime diagnostic needs an error boundary and a report needs to escape the render. The
tree keeps that structure, is testable without React, and the example check expands through the
same module.

Why not an instance tree inside the runtime: the runtime's requirements keep it pure with respect
to instances, and the fiddle's host may compose differently.

### D11. Handler records become DrawnUI callbacks

`coerceProps` turns a property named `on<Emit>` whose value is an `ActionHandler` record with a
token into the DrawnUI prop `<Emit>`: a function `(sender, ...args)` that builds
`{ $type: <action name>, <field>: args[i] }` from the event's parameter names in the metadata and
the action name the record carries, and dispatches it under the record's token through the
drawing context (D12). Every other object value is coerced as before. An
`ActionHandler` record without a token comes from pure evaluation, which is what `root()` is, and
gets no callback; the renderer reports it once per drawing, as it reports an unknown control, with
the reason: handlers run inside a component, and the root function is evaluated, not instantiated.
Rendering the root as a component (`component <Page /> = { state { ... } ... }` and
`let root() = <Page />`) is the pattern the examples use.

### D12. Routing: to the instance that owns the handler, then up by emit, then to the visitor

A callback for token `t` drawn inside instance `S`'s output looks up the handler value
`h = S.handlers.get(t)`. Handlers only ever flow down, from the body that created them into the
descriptors and content of children, so the instance whose body created `h` is the ancestor of
`S`, `S` included, farthest from `S` whose table holds `h`; D6's identity guarantee is what makes
the lookup possible. The batch `[{ $type: "ActionHandlerInvocation", token: t', action }]` is
dispatched against that instance under its own token `t'`, so the runtime's owner check finds the
handler owned and reads state live, and a `Page`-bound handler inside a `Stack`'s content patches
`Page`, never `Stack`.

The dispatch result's effects are routed one by one. An effect whose `$type` is an emit of the
dispatched instance's component, and whose descriptor bound a handler property for that emit, is
delivered by dispatching `{ $type: "ActionHandlerInvocation", token, action: effect }` against the
instance that owns that handler, found the same way from the parent's table; its effects are routed
in turn. Effects only ever route to strict ancestors of the instance that returned them, so the
pending instance farthest from the root is dispatched first, with every entry pending for it in one
batch: two emits a child returns for handlers its parent bound patch the parent in one dispatch,
the second reading what the first left, and no instance is dispatched twice in a chain, which would
retire the tokens its other pending entries carry. Every effect left over, an emit nobody bound or
an action the component does not emit, is a host effect: the drawing collects them with the name of
the instance that produced them, and the editor lists the most recent ones in the diagnostics pane,
which is how a visitor sees a `DoSearch` leave the tree.

A chain, and the redraw that follows it, is all or nothing in the tree: the tree records every
node's instance before the chain and puts them all back when any dispatch or the redraw throws, and
the failure is reported as a drawing diagnostic with the runtime's message. The drawing stays as it
was, and so do the instances its callbacks name, so the next event still dispatches. A callback
also checks that the instance it was drawn with is still the node's, and ignores the event
otherwise: between a dispatch and React's commit of the new drawing, an event can reach a callback
whose token the dispatch retired, or, after a parent redraw re-initialized the node at generation 1,
one that names a different handler.

The runtime's other entry form, an emitted action dispatched against the child, which invokes the
parent-bound handler with captured values only, is not used by the renderer: routing to the owner
is what makes the parent's state live, and it is what the language's "emitted to the parent" means.

### D13. Examples: the vocabulary splits, two ports become complete, the check draws as the site does

`event-handlers` and `component-state` stay in the vocabulary, since the survey and the search
still need them, and `coverageNote` words them as what the port does not use yet rather than what
NX lacks; `animation`, `list-virtualization` and `code-behind` keep their wording. The example
check's vocabulary is unchanged. Text and Shapes are the examples blocked on event handlers alone
and are ported in full: their pages become components with state, Text's readouts report the last
span tap and the last link tapped (the original's time stamp is dropped, NX has no clock, and its
tap readout becomes "link tapped"), and Shapes' context-menu card reports that the request was taken
in a label rather than a toast, with the browser menu suppressed as in the original. Both become
`complete`. The other examples' in-source notes are reworded so none says NX has no handlers or
no state; their tags stay.

`expand-components.mjs` expands through the same instance module the renderer uses, with each
child initialized under its parent, so the check exercises the parent route on every example and
an example whose descriptor carries a handler property no longer fails as an unknown field.

## Risks / Trade-offs

- [Token numbering diverges from Rust on some tree shape] → the corpus lifecycle scenarios include
  a handler inside a loop, inside nested descriptors, and on a descriptor prop declared out of
  sorted order, and compare tokens exactly.
- [A handler value leaks into canonical output as `$nxKind`] → canonicalization is the only exit
  from every public API, and a test asserts no `$nxKind` key survives in rendered output, effects,
  or descriptors, including inside effects a handler returned.
- [The executable emitter's new arm is missed by some walk] → `CodegenExpressionKind` is matched
  exhaustively in Rust; the compiler flags every match that omits the new variant, and the existing
  negative test for the JavaScript target keeps the diagnostic pinned.
- [Live-state-by-slot differs from the interpreter's name-based rule in some scope] → the rules
  agree exactly when the builder resolves names the way the checker binds them, which is already
  what the IR relies on for every `let`; the shadowing corpus scenario exists to catch a divergence.
- [The component layout change is missed by a reader] → both readers validate the declaration list
  against a layout table, so an image with the old layout is refused as malformed rather than
  misread, and the corpus's damaged-image tests cover the new operand like every other.
- [Schema 3 is tagged before this lands] → the layout change is then a schema change; bump to 4
  in the same three places (`ir.rs`, the TypeScript reader, the document) rather than reading two
  component layouts.
- [IR grows for programs with many handlers] → a handler node is the size of its body plus seven
  cells, and a component with no emits pays one cell; the corpus size budget still holds.
- [The owner lookup in D12 depends on handler identity across instances] → the runtime spec states
  it and a runtime test pins it; the playground's instance test dispatches a content-child handler
  and asserts the owner's state changed.
- [A catalog emit name collides with a prop, or an event's parameter name is not an NX identifier]
  → the catalog is compiled by the example check and by the site build, so a collision fails there
  rather than in a visitor's compile; DrawnUI declares no `on<Event>` props today.
- [`TextChanged` dispatches on every keystroke and the redraw reassigns `Text`] → the reconciler
  assigns only changed props, and the Editor example is not among the ports in this change; the
  behavior is observed in the browser and recorded in FINDINGS if it fights the caret.
- [Host effects accumulate] → the editor keeps the most recent ones and clears them on recompile.

## Migration Plan

No deployment step. The IR schema version is unchanged; the feature list gates older runtimes.
The corpus is regenerated with `NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib ir_corpus` and
the explained-text diff reviewed. The catalog is regenerated with `npm run generate-catalog` in
`sites/playground` and its diff reviewed: added emits, a shorter omitted list, dropped parameters. `@nx-lang/ir-runtime` publishes with the next tag through the
existing package release track. Rollback is reverting the change; images emitted with the feature
are refused by the previous runtime rather than misread.
