## Context

Component declarations are currently always concrete NX implementations. In `crates/nx-syntax`,
the grammar requires `component <... /> = { ... }`, and the syntax validation hints only describe
that concrete form. In `crates/nx-hir`, `Component` stores props, emits, state, and a mandatory
body expression, but it does not preserve inheritance metadata or distinguish abstract/external
contracts from executable components. In `crates/nx-api`, imported component interface metadata
publishes props, emits, and state only, so consumers cannot resolve inherited component contracts
across library boundaries.

The repository already has a proven model for abstract record/action inheritance in
`crates/nx-hir/src/records.rs`: raw lowered items keep declared fields plus `is_abstract` and
`base`, and prepared-module validation computes effective inherited shape across same-library and
imported declarations. Component inheritance should follow that same prepared-module model instead
of flattening contracts opportunistically during parsing.

Runtime behavior is already split in a useful way. Nested `<Component ... />` expressions normalize
props and handlers into an opaque record-like value without evaluating the component body, while
`initialize_component*` is the path that evaluates component state and body. That makes external
components a natural extension of the current runtime model: they need contract validation and host
visible props/emits, but they do not need an NX body.

## Goals / Non-Goals

**Goals:**
- Add `abstract`, `external`, and `extends` support to component syntax and diagnostics.
- Preserve component modifiers and base metadata through HIR and library interface artifacts.
- Resolve effective component contracts across local and imported abstract bases.
- Inherit only public contract surface: props, prop defaults, content props, and emitted actions.
- Keep concrete external components usable from NX and runtime APIs without requiring an NX body.
- Reject invalid combinations early: concrete bodyless components, abstract component
  instantiation, duplicate inherited props/emits, and body/state on abstract or external
  declarations.

**Non-Goals:**
- Multiple inheritance for components.
- State inheritance or implementation inheritance.
- Automatic React or framework-specific binding metadata.
- Overriding inherited props or emitted actions by redeclaring the same local name.

## Decisions

### 1. Component declarations become record-like contracts plus optional implementation

`crates/nx-syntax/grammar.js` will change component declarations from a single concrete form to:

- `[visibility] [abstract] [external] component <Name [extends Base] ... /> = { ... }`
- `[visibility] [abstract] [external] component <Name [extends Base] ... />`

The `extends Base` clause belongs inside the component signature immediately after the component
name, matching the proposed syntax. A body is required only when neither `abstract` nor `external`
is present. Because `state` remains nested inside `component_body`, bodyless declarations cannot
declare state.

`crates/nx-hir::Component` and `crates/nx-hir::InterfaceItemKind::Component` will gain:

- `is_abstract: bool`
- `is_external: bool`
- `base: Option<Name>`
- `body: Option<ExprId>`

`state` remains present as declared state for concrete NX-bodied components only. Abstract and
external components always have an empty declared state vector.

Alternative considered: flatten inherited props/emits directly into raw HIR during lowering.
Rejected because lowering is intentionally file-local and cannot correctly resolve imported bases.
The existing prepared-module model is already the right layer for inheritance.

### 2. Effective component contracts are resolved in prepared-module validation

A new resolver module, parallel to `crates/nx-hir/src/records.rs`, will compute:

- `EffectiveComponentContract { component, props, emits, ancestors }`

Resolution rules:

- `extends Base` is allowed only when `Base` resolves through `PreparedNamespace::Element` to an
  abstract component declaration.
- Base resolution must work for same-library peers, imported libraries, and aliases exposed through
  interface metadata.
- Effective props are the ordered merge of base props followed by local props.
- Duplicate prop names across the base chain are rejected.
- Content props follow the same effective-shape rule as records: at most one effective content prop
  may exist after inheritance.
- Effective emits are the ordered merge of base emits followed by local emits.
- Duplicate emitted action local names are rejected.
- Handler prop collisions (`on<ActionName>`) are validated against the effective emit set, not just
  the locally declared emit set.
- Inheritance cycles are reported the same way record inheritance cycles are reported today.

This resolver is also the source of truth for imported component contracts. Library interface
metadata will publish declared component flags/base plus declared props/emits/state, and consumers
will compute the effective contract through the same prepared-module path instead of relying on
pre-flattened interface payloads.

Alternative considered: publish fully flattened component props/emits in library interfaces.
Rejected because it erases where a contract entry originated, complicates diagnostics, and makes
future inheritance changes harder to reason about.

### 3. Inherited inline emits keep the public action name of the declaring component

For shared emit references, inheritance is straightforward: the derived component inherits the same
referenced action type.

For inline emitted actions, the inherited emit keeps the original `action_name` that was declared
by the component that introduced it. Example: if `SearchBase` declares inline emit `ValueChanged`,
then `NxSearchUi extends SearchBase` inherits the local handler name `onValueChanged`, but the
underlying action type remains `SearchBase.ValueChanged`.

This avoids synthesizing new action record definitions for every descendant component, which would
otherwise require new raw HIR items, imported interface expansion, and additional runtime record
resolution rules. It also keeps one canonical action type per contract origin.

Alternative considered: re-qualify inherited inline emits to the derived component name, e.g.
`NxSearchUi.ValueChanged`. Rejected for this phase because it would force synthetic emitted-action
record generation across raw modules, library interfaces, and runtime validation.

### 4. Body scope, element binding checks, and runtime APIs all use the effective contract

Any logic that currently reads `component.props` or `component.emits` as the full public contract
must switch to the effective component contract helper:

- `crates/nx-hir/src/scope.rs` must seed component body scopes with inherited props plus local
  state, so derived component bodies can reference inherited props without undefined-identifier
  diagnostics.
- `crates/nx-types/src/infer.rs` must validate `<Component ... />` bindings against the effective
  contract for both local and imported derived components.
- `crates/nx-interpreter/src/interpreter.rs` must normalize component props and validate dispatched
  emitted actions against the effective contract, not only locally declared entries.

State is intentionally not inherited. A concrete derived component can have its own local state,
but base state never contributes to the effective contract or runtime snapshot shape.

### 5. External components are concrete host-rendered leaves; abstract components are never instantiated

Runtime behavior splits by modifier:

- Abstract components are declaration-only contracts. They cannot be initialized as entry
  components and cannot be instantiated through markup. Static analysis should report this, and the
  interpreter should add a runtime guard similar to abstract-record instantiation.
- When NX code references a concrete external component in an expression, including inside ordinary
  functions, `<ExternalWidget ... />` evaluates to the same record-shaped value NX already uses for
  component invocations: a `Value::Record` whose type is the component name and whose fields are
  the normalized effective props plus any bound action handlers. The interpreter does not execute an
  NX body for that component because none exists.
- Concrete external components are bodyless but instantiable. `initialize_component*` will:
  1. normalize props and handler bindings against the effective contract,
  2. produce an empty component state snapshot,
  3. return the rendered output as an opaque `Value::Record` whose type is the component name and
     whose fields are the normalized props.
- That record-shaped result is also the wire contract for hosts. When NX evaluation results are
  serialized to JSON or another transport format, external component values are emitted as plain
  typed records, and the client is responsible for interpreting the component name plus serialized
  props to instantiate the corresponding UI component.
- `dispatch_component_actions*` continues to work for external components because it already routes
  host-supplied actions through the component's declared emits and bound handlers; no NX body
  evaluation is required.
- Concrete non-external components keep the current initialization path: normalize props, create
  local state, then evaluate the body.

This keeps the existing API shape intact while making external components usable as program entry
components and nested elements without introducing a new host protocol in this change.

## Risks / Trade-offs

- `[Inherited inline emit names stay base-qualified] -> Mitigation: document this explicitly in the
  specs and add runtime/type-check tests that make the behavior obvious.`
- `[Component contract resolution duplicates record-resolution patterns] -> Mitigation: mirror the
  existing `records.rs` structure and share merge rules where practical instead of building a
  separate ad hoc validator.`
- `[Scope checking, type checking, and runtime all need to switch to effective contracts] ->
  Mitigation: centralize contract lookup behind one helper and update all current direct
  `component.props`/`component.emits` call sites as part of the implementation.`
- `[External entry-component initialization changes runtime semantics] -> Mitigation: keep the
  change local to `initialize_component*`, preserve the snapshot format, and add API/interpreter
  tests for external concrete and abstract cases.`

## Migration Plan

- Update syntax grammar, generated parser artifacts, and syntax validation messages to accept the
  new declaration forms.
- Extend component HIR and interface metadata with modifier/base/body fields.
- Implement prepared-module component contract resolution and validation.
- Update scope checking, element binding checks, interpreter normalization, and component runtime
  initialization to use effective contracts.
- Add syntax, lowering, type, runtime, and API coverage before implementation is considered
  complete.

No data migration is required. Rollback is code-only because the change only affects source
analysis and runtime interpretation of NX programs.

## Open Questions

None for this phase. The major semantic choices needed to implement the feature are fixed above and
should be captured in the follow-on specs.
