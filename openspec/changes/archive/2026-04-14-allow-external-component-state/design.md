## Context

Concrete external components are currently modeled as bodyless contracts only. In
`crates/nx-syntax`, `component_body` requires a rendered expression and syntax validation rejects
any body on an `external component`, so `external component <SearchBox /> = { state { ... } }`
cannot parse as a valid program today.

The underlying semantic model is already close to what this change needs. `crates/nx-hir::Component`
already separates `state: Vec<RecordField>` from `body: Option<ExprId>`, and library interface
artifacts already publish component state fields. The missing pieces are:

- parser and validation support for a state-only external body,
- an explicit rule for how external-component state behaves in NX versus in hosts, and
- code generation support for emitting a host-facing state contract from exported external
  components.

`nxlang generate` currently emits only aliases, enums, and record-like declarations through
`ExportedTypeGraph`. There is no representation for a generated companion type derived from a
component contract, and the existing record emitters always add runtime discriminator behavior that
does not fit an external-component state payload.

## Goals / Non-Goals

**Goals:**
- Allow a concrete external component to omit its body or to declare a state-only body.
- Reject any external-component body that is empty or that contains anything other than `state`.
- Preserve declared external state through lowering and library artifacts without treating it as an
  NX render body.
- Generate host-facing state data types for exported external components that declare state.
- Keep naming and imports for generated external state deterministic across single-file and library
  generation.

**Non-Goals:**
- Allow state bodies on abstract components, including `abstract external component`.
- Add state inheritance for components.
- Generate full external-component prop or emit contracts.
- Change runtime APIs to accept host-provided initial external state or make NX execute external
  state behavior.

## Decisions

### 1. Concrete external components get a distinct body rule: optional body, but state-only when present

The grammar will relax `component_body` so the rendered expression is optional, then syntax
validation will enforce the body matrix:

- concrete non-external component: body required and must contain a rendered expression
- abstract component: body forbidden
- concrete external component: body optional; if present it must contain `state { ... }` and no
  rendered expression
- abstract external component: body forbidden

This keeps the parser permissive enough to preserve the new shape while leaving the semantic rule
in one place. It also makes empty external bodies (`= {}`) and mixed external bodies
(`= { state { ... } <View /> }`) rejectable with targeted diagnostics instead of parser failures.

Alternative considered: introduce a separate grammar production only for external components.
Rejected because the existing `Component` HIR shape already models `state` and `body`
independently, so one shared body node plus validation is simpler and keeps downstream lowering
unchanged.

### 2. Declared external state reuses the existing `Component.state` and interface-artifact fields

No new HIR type is needed. Lowering already captures `state` and `body` separately, and interface
artifacts already serialize component state fields. The implementation should preserve that model:

- an external component with `= { state { ... } }` lowers with populated `component.state`
- the same component lowers with `component.body == None`
- prepared component contract resolution continues to ignore state for inheritance, invocation
  checking, and `on<ActionName>` binding

This keeps the change focused on allowing the declaration and surfacing its state schema to hosts.
State remains local metadata on the declaring component rather than part of the effective inherited
prop/emits contract.

Alternative considered: synthesize a separate record declaration for external state during lowering.
Rejected because it duplicates field ownership, complicates library artifacts, and is unnecessary
for analysis or code generation.

### 3. External-component state is host-owned schema, not NX-managed runtime state

This change will treat declared external state as a contract for hosts and generated bindings, not
as NX-managed local state. Concretely:

- interpreter initialization for external components will continue to return the external component
  record shape and an NX-owned empty state snapshot
- interpreter dispatch for external components will continue to round-trip an empty NX-managed
  state snapshot
- external component evaluation and serialization will continue to expose normalized props and
  action handlers, without evaluating a render body

The declared state exists so hosts can model and transfer it outside NX, which matches the request
that external state is read and written outside NX source. This also avoids inventing new runtime
API inputs for initial external state or default-evaluation rules for non-default external state
fields.

Alternative considered: make external components participate in the normal component-state
initialization path and include declared state in runtime snapshots.
Rejected because current APIs do not accept host-provided initial state, the request's primary
motivation is generated wire contracts, and requiring NX to initialize external state would create
new runtime semantics that are not needed for this phase.

### 4. Code generation emits a companion plain state contract named `<ComponentName>_state`

`nxlang generate` will synthesize one generated declaration for each exported external component
with a non-empty declared state. The generated declaration name will be
`<ComponentName>_state` and will include exactly the declared state fields from that component.
This keeps generated-source naming aligned with the likely future NX projection syntax
`<ComponentName>.state` without making the generated companion a source-level NX declaration.

Implementation-wise, `ExportedTypeGraph` will gain a new declaration kind for external-component
state so emitters can render it as a plain field bag instead of a runtime-discriminated NX record:

- TypeScript: `export interface SearchBox_state { ... }`
- C#: `[MessagePackObject] public sealed class SearchBox_state { ... }`

The new kind will participate in the existing module-ownership and cross-module import logic so
state fields referencing exported types continue to resolve across generated files. Generated
external state declarations will be omitted when the external component is not exported or when it
declares no state.

Alternative considered: emit the companion type as an ordinary generated record with `$type`.
Rejected because external component state is not a standalone NX runtime declaration and should not
imply discriminator semantics.

### 5. Generated companion names warn and skip on collisions

Because `<ComponentName>_state` is synthesized rather than source-authored, code generation must
detect collisions with existing exported declarations or with another generated companion name in
the same output graph. When a collision occurs, generation should keep the explicit source-authored
declaration when present, emit a warning, and skip the generated companion instead of silently
overwriting one type with another.

Alternative considered: auto-suffix collisions with unstable names such as `<ComponentName>_state2`.
Rejected because generated wire contracts must remain stable and predictable across runs, and
warning + skip preserves explicit user-authored declarations without inventing fallback names.

## Risks / Trade-offs

- [Relaxing the grammar makes more invalid external-component bodies parse] -> Mitigation: add
  explicit validation errors for empty external bodies, mixed state-plus-render bodies, and any
  abstract component body.
- [Keeping external state out of NX runtime snapshots may feel narrower than a fully stateful
  external lifecycle] -> Mitigation: document that this change is schema/codegen-focused and keep
  the runtime API surface unchanged.
- [Generated companion type names can collide with user-authored exports] -> Mitigation: perform a
  graph-wide collision check before emission, warn on conflicts, and skip only the synthesized
  companion declaration that would collide.
- [A new codegen declaration kind touches both emitters and import analysis] -> Mitigation: model
  it centrally in `ExportedTypeGraph` so TypeScript and C# share one ownership and dependency view.

## Migration Plan

1. Relax component-body parsing and update syntax validation/messages for the new external-body
   rules.
2. Add syntax and lowering coverage for valid state-only external bodies and invalid external-body
   combinations.
3. Preserve declared external state through analysis/artifact tests without changing component
   inheritance semantics.
4. Extend codegen collection and emitters with generated external-state companion declarations plus
   warning-driven collision handling.
5. Update external-component and code-generation specs/tests to lock in the host-facing contract.

Rollback is code-only: revert the parser/validation/codegen changes and external components become
bodyless again.

## Open Questions

None. This change intentionally keeps external-component state host-owned and codegen-visible only.
