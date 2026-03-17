## Context

NX now has parser support for `component`, `emits`, `state`, top-level `action` declarations, and
lazy `on<ActionName>` handler bindings. The current implementation stops at that groundwork:

- `nx-hir` preserves only lightweight component signature metadata and explicitly defers lowering
  the component body and state execution model.
- `nx-interpreter` can invoke lowered action handlers, but it cannot initialize a component or
  reduce a component instance across dispatched actions.
- `nx-api`, `nx-ffi`, and the .NET binding expose only `eval_source`, which is tied to root-source
  evaluation rather than a reusable component lifecycle.

This change needs a full component runtime path without introducing interpreter-held mutable state.
The Rust side must stay stateless between calls, so everything required for later dispatch has to
be reconstructed from caller-provided inputs. That creates two design constraints:

- component lowering has to preserve enough information to execute the component body and its state
  transitions
- runtime bindings need a stable state payload that the host can store and return later

## Goals / Non-Goals

**Goals:**
- Lower component definitions into executable HIR items that preserve props, emitted actions, state
  fields, and the rendered body expression.
- Add interpreter primitives for component initialization and batched component dispatch.
- Keep the interpreter stateless between calls by returning a serialized component-instance state
  payload that the host owns.
- Reuse existing component syntax rather than inventing new reducer or lifecycle syntax in this
  change.
- Shape dispatch around future action-driven state updates without requiring those update-action
  forms to exist yet.
- Feed dispatched actions through the existing emitted-action and `on<ActionName>` handler model so
  later runtime work builds on the same callback contract.
- Add strong automated test coverage across lowering, interpreter execution, API/FFI boundaries, and
  host bindings for this core NX behavior.

**Non-Goals:**
- Introduce new source syntax for reducers, hooks, lifecycle methods, or general closures.
- Keep live component instances, cached modules, or host callbacks resident inside Rust between API
  calls.
- Change the semantics of `eval_source` or remove the existing root-evaluation path.
- Solve diffing, incremental rendering, or host view reconciliation in this change.
- Expose raw lowered handler closures as part of the public FFI or .NET API.
- Introduce declarative state-update actions in this change.

## Decisions

### 1. Lower executable components as first-class HIR items

The existing lightweight `Component` HIR item needs to become executable rather than lookup-only.
Each lowered component should preserve:

- component name
- prop definitions, including types and optional default expressions
- emitted action metadata
- state field definitions, including types and optional default expressions
- lowered body expression
- source span

For props and state, the HIR should reuse the existing record-field shape rather than the current
function-style `Param` shape used for component props today. Both component props and component state
need optional default expressions, and the existing record-field model already carries `name`, `ty`,
`default`, and `span`.

Lowering should treat component bodies similarly to function bodies:

- prop names are in scope while lowering state defaults and the body
- state field names are in scope while lowering the body
- emitted action metadata remains available for `on<ActionName>` recognition and later dispatch

The existing component predeclaration pass should remain, but it should now seed both executable
component items and the already-established public `Component.Action` records for inline emits.

Alternative considered: keep the current lightweight component item and synthesize hidden companion
functions for init and dispatch. Rejected because it would duplicate lowering logic, split emitted
action metadata across multiple synthetic items, and make later diagnostics harder to explain.

### 2. Keep state defaults init-only and reserve state mutation for future actions

This change should not reinterpret `state { field:type = expr }` defaults as reducers. State
defaults are initialization-only: they are evaluated once when the component is initialized and are
then persisted inside the host-owned snapshot.

Initialization rules:

- normalize the incoming props first
- evaluate each state field in declaration order
- if the field has a default expression, evaluate it with normalized props in scope
- if the field has no default and is nullable, initialize it to `null`
- if the field has no default and is not nullable, fail initialization with a runtime error

Dispatch rules in this change:

- start from the previous state snapshot supplied by the caller
- treat that snapshot as the authoritative current value of every state field
- process incoming actions through the existing emitted-action and handler pipeline
- return the resulting effect actions plus a next-state snapshot

The next-state snapshot is part of the API now because future declarative update actions will mutate
it. Those actions are intentionally deferred to a follow-up change. Until they exist, dispatch in
this change preserves the incoming state snapshot.

This keeps the meaning of defaults intuitive, avoids making initialization syntax double as hidden
update logic, and still establishes the API shape that later state-update actions will plug into.

Alternative considered: reevaluate state defaults on every dispatched action. Rejected because you
clarified that state mutation should be action-driven, and default expressions should only run when
the component is first initialized.

### 3. Store component-instance state as an opaque serialized snapshot

The host contract says Rust is stateless between calls, which means dispatch cannot depend on live
Rust objects from initialization time. The returned state therefore needs to carry everything later
dispatch requires.

The internal component-instance snapshot should include:

- component name
- normalized prop values
- normalized user state values
- any lowered handler metadata needed to produce effect actions on later dispatch

That snapshot should be serialized by Rust and treated as opaque by public bindings. The host only
stores it and passes it back. This avoids trying to force internal runtime details such as captured
handler metadata into the stable `NxValue` surface.

Public binding shape:

- Rust API returns a structured result plus serialized state bytes
- MessagePack bindings return state as a binary field
- JSON bindings encode state as a string payload suitable for round-tripping through the existing
  C and .NET entry points

This keeps the external contract stable while leaving room to refine the internal snapshot layout as
component execution becomes richer.

Alternative considered: expose component state as a plain `NxValue` record. Rejected because the
runtime snapshot may need to retain internal execution metadata that is not faithfully round-trippable
through the current `NxValue` model.

### 4. Dispatch actions sequentially and collect effects through the existing handler model

Component dispatch should accept the prior serialized state plus an ordered list of action values
provided by the host. The interpreter should decode the snapshot, then process the actions one at a
time in that host-provided order.

For each action:

1. Validate that the action matches one of the component's declared emitted actions.
2. Apply any recognized state-update action behavior once that follow-up change exists. In this
   change, no supported action mutates state, so the snapshot is carried forward unchanged.
3. If the instance has a matching bound `on<ActionName>` handler, invoke it through the existing
   `invoke_action_handler` machinery and append the normalized returned actions to the effect list.

Processing actions sequentially still matters because the same pipeline will host future declarative
state-update actions. It also matches the requested "dispatch each of the actions" behavior and
reuses the callback model that just landed for emitted actions.

The dispatch API for this change should return exactly what the request asks for:

- ordered effect actions
- updated serialized state

It should not add a second render payload to the public dispatch result yet. The runtime can keep a
shared internal render path so that later work can expose rerendered output without changing the
underlying state machine, but the initial binding contract stays focused on effects plus state.

Alternative considered: interpret the full action list in one NX expression or return a rerendered
element on every dispatch. Rejected because both broaden the first public contract beyond the stated
requirement and make the initial runtime surface harder to stabilize.

### 5. Add explicit component init/dispatch APIs instead of overloading `eval_source`

`eval_source` is intentionally narrow: it parses source, lowers a module, and evaluates a root
entrypoint. Component lifecycle work needs a different contract.

The new internal Rust interpreter surface should look like:

- `initialize_component(module, component_name, props, limits) -> ComponentInitResult`
- `dispatch_component_actions(module, serialized_state, actions, limits) -> ComponentDispatchResult`

`nx-api` should then add source-based wrappers that mirror the existing binding style:

- parse and lower source
- convert incoming `NxValue` props and actions into interpreter `Value`s
- call the new interpreter APIs
- convert returned element/effects back to `NxValue`

To support that path, `nx-api` needs a reverse conversion helper alongside `to_nx_value`, so props
and actions supplied by bindings can be validated and coerced against the component signature.

The C ABI and .NET surface should add dedicated entry points rather than encoding lifecycle calls as
special root functions. The expected public shape is:

- canonical MessagePack FFI entry points: `nx_eval_source`, `nx_component_init`, and
  `nx_component_dispatch_actions`
- optional Rust-side MessagePack-to-JSON debug helpers for tooling and logging
- matching managed wrapper methods in `NxRuntime`

Alternative considered: model component init and dispatch as conventional hidden functions and keep
only `eval_source` at the API boundary. Rejected because it would hide lifecycle-specific result
types, force fragile naming conventions, and make bindings reverse-engineer component state from
ordinary evaluation payloads.

### 6. Keep test coverage broad and layered because this is core runtime behavior

This change touches core language execution, so testing has to be broader than parser-only fixtures.
Coverage should include:

- HIR lowering tests for component bodies, state defaults, and executable component metadata
- interpreter tests for init success/failure, ordered dispatch, stable state round-tripping, and
  effect output
- API/FFI round-trip tests for serialized state handoff across init and dispatch
- .NET binding tests that prove the managed wrapper can initialize, persist, and redispatch state
- docs/examples updates that exercise the supported runtime contract and replace "deferred follow-up"
  notes

State-update behavior tests are intentionally deferred until the follow-up change that introduces
the declarative actions responsible for mutating component state.

Alternative considered: rely mostly on interpreter unit tests and a small number of smoke tests in
bindings. Rejected because the user explicitly asked for strong coverage and this behavior crosses
multiple repository boundaries.

## Risks / Trade-offs

- Opaque serialized state is harder to inspect manually -> document it as host-owned but opaque, and
  keep stable debug formatting or test helpers on the Rust side for inspection.
- Dispatch returning an unchanged state snapshot in this phase may feel incomplete -> document that
  declarative state-update actions are a follow-up change and keep the state field in the public
  API now to avoid a breaking API revision later.
- Not returning a rerendered element from dispatch limits what hosts can do with one call ->
  keep a shared internal render path and document that the initial public dispatch contract is
  effects plus state only.
- Parsing and lowering source for every API call may be expensive for hot paths -> keep the core
  interpreter APIs module-based so a future compiled-module binding can reuse the same runtime
  design without changing component semantics.
- Handler metadata inside the serialized snapshot couples component runtime to the recently added
  handler model -> acceptable because emitted-action handlers are already the chosen callback
  contract for component actions.

## Migration Plan

1. Extend `nx-hir` lowering so executable component items preserve prop defaults, state fields, and
   body expressions while keeping the existing emitted-action predeclaration flow.
2. Add interpreter helpers for component initialization, state snapshot serialization, and
   sequential dispatch with effect collection, keeping the state-transition hook ready for a later
   action-driven update change.
3. Add bidirectional `NxValue` conversion plus new component lifecycle result types in `nx-api`.
4. Extend `nx-ffi` and the .NET binding with dedicated MessagePack init/dispatch entry points,
   MessagePack-to-JSON debug helpers, and round-trip tests for persisted state.
5. Update documentation and examples to describe the supported lifecycle contract and add the full
   test coverage required for this core NX functionality, excluding state-mutation cases that belong
   to the follow-up update-action change.

Rollback is straightforward because the new lifecycle entry points are additive. If the change needs
to be backed out, remove the component runtime APIs and executable component lowering together so
component syntax returns to its current non-executable state cleanly.

## Open Questions

- Should a follow-up API return the rerendered element on dispatch, or should hosts continue to
  treat dispatch as an effects-and-state-only operation?
- What concrete NX action forms should be reserved for declarative component state updates in the
  follow-up change?
- Should public host bindings eventually accept explicit emitted-action handler bindings at
  initialization time, or is that surface reserved for NX-to-NX component composition?
