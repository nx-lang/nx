## Context

See proposal.md for motivation. The pieces this design builds on, and the constraints they impose:

- Handler bindings already lower to `ast::Expr::ActionHandler` (nx-hir `lower.rs`
  `lower_property_value`) and evaluate to `Value::ActionHandler` with a by-value snapshot of every
  visible variable (`eval_action_handler_expr`, `snapshot_visible_variables`). The type checker
  returns `Type::Error` for the handler expression and skips handler props entirely
  (`infer.rs` around lines 419 and 2165); result validity is a runtime-only check
  (`normalize_handler_result`, `ensure_action_result_value`).
- Inline emitted actions are synthesized as `Item::Record` values named `Component.Action` during
  component predeclaration and appended to the module after the component item
  (`lower.rs` `predeclare_component`, `lower_module`). `RecordKind` has `Plain` and `Action`.
- The interpreter is stateless between calls. A component instance is an opaque MessagePack
  snapshot (`SerializedComponentSnapshot`: component, props, state) and dispatch re-encodes the
  state it was given; `SerializedValue::ActionHandler` already round-trips handler values inside
  snapshots. Rendered output turns handlers into a display-only `ActionHandler` record that
  `from_nx_value` refuses to read back.
- `NxValue::Record` is a `BTreeMap`, so "absent" is already representable on the wire; the loss
  happens in every normalizer (`build_record_value_from_shape`, `normalize_explicit_component_state`,
  TypeScript `normalizeFields`) which fills a missing key from a default or `null`.
- Contextual bare names resolve in one funnel (`resolve_contextual_name_in`) driven by the expected
  type, and resolutions are rewritten into the module before the type-checked snapshot is taken
  (`apply_contextual_name_resolutions`). Element tags are not contextual today.
- NX IR cannot encode a handler expression (`builder.rs` rejects `ActionHandler`), so the
  TypeScript IR runtime cannot dispatch. That is the separate `ir-action-handlers` change.

## Goals / Non-Goals

**Goals:**
- One value, the update record, serves state patches, host effects, and server-bound change sets.
- Handlers stay pure message constructors; state mutation happens only where dispatch applies a
  patch, so an effect log remains replayable.
- No grammar change and no new keyword.
- The Rust runtime remains stateless; everything dispatch needs comes from the snapshot and the
  batch.
- Host boundaries (NxValue, .NET DTOs, TypeScript DTOs, NX IR) all preserve absent-versus-null.

**Non-Goals:**
- `apply(record, patch)` and patch composition as language built-ins.
- Patches with list operations; a list field is replaced whole.
- Handler dispatch in the TypeScript IR runtime.
- Reactivity, diffing, or any change to how hosts orchestrate a tree of instances beyond what the
  new dispatch entry gives them.

## Decisions

### D1. Update records are a third `RecordKind`, synthesized like inline emit records

Add `RecordKind::Update { target: Name }`. During predeclaration, for every record-shaped
declaration (`type` record, `action`, inline emit record, component with state) synthesize a
`RecordDef` named `<Name>.Update` whose properties are the target's declared fields with defaults
stripped, and register it alongside the inline emit records (`predeclared_records`) so element tags
and type annotations resolve it before bodies are lowered. Update records are appended after every
declared item in `lower_module`, not beside their targets: item indices are definition identities,
and appending keeps every authored item's identity — and so the IR of a program that never uses a
patch — unchanged. A component's update record is derived from its state fields only.

Why a kind rather than a flag on `RecordDef`: handler-result routing, "cannot extend", "is not an
action", and the TypeScript/C# emitters all branch on it, and `RecordKind` is already the branch
point every one of those consults.

Alternative: derive `T.Update` lazily in the type checker as a structural type. Rejected because
the interpreter, IR, and typegen all need a declaration to normalize against, and a synthesized
`RecordDef` gives all of them the same source of truth.

Inheritance: effective fields are computed through the existing base-chain resolution
(`records.rs`): an update record's effective shape is its target's effective shape with every field
made optional and default-free, so `User.Update` for `User extends Named` carries `Named`'s fields,
even when `Named` is in another module. Abstract
records get an update record too; it is harmless and avoids a special case.

### D2. Absence is "key not present" end to end

`Value::Record` fields already live in a map; construction of an update record simply does not
insert absent fields. Every normalizer gains an "is update record" branch: no defaults, no
required-field error, unknown field rejected, `null` accepted only where the target field is
nullable. This is one new arm in `build_record_value_from_shape`, one in the TypeScript
`normalizeNominalValue`/`normalizeFields` path (driven by the IR declaration's kind), and no change
to `NxValue` serialization, whose map encoding already omits missing keys in both JSON and
MessagePack.

Host input is the one place a record arrives that no construction rule has seen. The interpreter's
coercion path carries a `ValueOrigin`: a value the interpreter produced keeps the by-name record
check, so ordinary function calls pay nothing, while a value the host supplied through props,
explicit state, or a batch entry is rebuilt from its fields against its declaration at every depth.
A `User.Update` nested in a prop, an array, a union case payload, a component value (built against
the component's props, as its element tag would be), or an action payload therefore meets the same
unknown-field and non-nullable-`null` rules as one the checker saw, and a host cannot turn
"unchanged" into "set to null" by drifting a DTO. An action entry is constructed before its handler
is looked up, so a malformed one fails even when no parent bound a handler and it would otherwise be
a no-op.

Alternative: a sentinel `Value::Absent`. Rejected: it would leak into every match on `Value`, and
absence is only meaningful at the record-field level, where the map already expresses it.

### D3. The bare `Update` tag is resolved during lowering against the enclosing component

Lowering already knows which component declaration it is inside (it lowers props, state, and
body under that component, and it predeclares every `<Name>.Update` before any body is lowered).
So `lower_element` rewrites a tag spelled exactly `Update` to `<Component>.Update` while a component
is being lowered, before the element-versus-record-literal decision is made. Every later phase —
the checker, the interpreter, codegen, the language service — sees only the qualified name, which
is what "indistinguishable from the qualified form" requires, without a new rewrite pass. A bare
`Update` inside a component that declares no state is rewritten the same way; the qualified name
then resolves to nothing, and the checker's unknown-update-record diagnostic says the component
declares no state. That keeps it to one diagnostic, from the phase that reports every other
`X.Update` with no record.

Outside a component the tag is left as written. A declaration named `Update` then resolves as it
always did, and when nothing does, the checker's unknown-element diagnostic names the qualified
`<Type>.Update` form.

Precedence over a user declaration named `Update` inside a component is deliberate: the
alternative (shadowing by declaration) makes the meaning of a handler depend on what happens to be
declared in the module, which is the kind of action-at-a-distance NX avoids.

The record-literal form resolves the same way because record literals are decided from the
already-rewritten tag.

Alternative considered: resolve in the checker's `infer_element_expression` and record the result
in `resolved_contextual_names` for `apply_contextual_name_resolutions` to rewrite. Rejected during
implementation: element tags are not contextual names, so it would need a second rewrite kind and a
checker-side `current_component`, for the same outcome lowering gets from state it already has.

### D4. Handler bodies are inferred with an owner, and results are routed statically

Replace the `Type::Error` arm for `ActionHandler` with real inference: push a scope, bind `action`
to the emit's action record type, infer the body, pop. The environment at that point already holds
the component's props and state (bound by `infer_component`) and any enclosing `let` or loop
bindings.

Routing is checked at the binding site. Define `owner` as `current_component` (`None` at root).
Normalize the result type to a list of item types (a record, or the element type of a list). Each
item must be: `owner.Update` when `owner` is set; an action in `owner`'s effective emits; or, when
`owner` is `None`, any action or update record. Anything else is a diagnostic whose message names
the fix (`add X to emits`, or `use Form.Update`). An empty-list literal is rejected here as well,
matching the runtime rule.

To make routing available at runtime without re-deriving it, `Expr::ActionHandler` gains
`owner: Option<Name>` filled during lowering (the lowering pass tracks the component it is inside,
which `collect_handler_rewrites_in_item` already walks), and `Value::ActionHandler` carries it.

### D5. State reads in handlers are live; other captures stay snapshots

At handler creation, keep the by-value capture but also record `owner_state: Vec<SmolStr>` (the
owner's state field names, available from the component declaration). At invocation during
dispatch, after seeding `captured`, overwrite each `owner_state` name with the working state
value. Invocation outside dispatch (`invoke_action_handler` called directly by a host or test)
seeds captures only, which preserves today's behavior for existing callers.

This is the smallest change that makes `[<Update x={x + 1} />, <Update x={x + 1} />]` move twice.
Alternative: drop state from the capture and always resolve it from the snapshot. Same outcome for
dispatch, but breaks the direct-invocation path and the "captures snapshot" test without benefit.

### D6. Dispatch gains handler invocations, a handler table in the snapshot, and re-render

Rendered output must let a host say "run this handler". Handler values are self-contained but
large (captures), so the rendered value carries a token and the snapshot carries the handler:

- During `initialize_component` and at the end of dispatch, the render pass walks the rendered
  value, assigns each `Value::ActionHandler` a token, and stores the serialized handler in a new
  `handlers: BTreeMap<String, SerializedValue>` snapshot field. Tokens are `"h<generation>-<n>"`,
  where the snapshot's `render_generation` increments on every dispatch, so a token read from an
  earlier render never names a handler in a later snapshot, and every dispatch — an effect-only or
  empty batch included — retires the tokens of the snapshot it consumed. The token is written onto the rendered
  `Value::ActionHandler` (a `token: Option<SmolStr>` output annotation snapshots do not store), and
  `to_nx_value` encodes such a handler as `{ $type: "ActionHandler", action, token }`. Rust callers
  therefore keep invocable handler values. Pure `evaluate_component` output carries no token and
  encodes as `{ $type: "ActionHandler", action }`.
- A batch entry is either an action record (existing path: the emit's parent-bound handler from
  `props`) or a record `{ $type: "ActionHandlerInvocation", token: string, action: record }`.
  `validate_dispatch_action_input` learns the second shape.
- For an invocation: look up the token (unknown → `UnknownHandlerToken` error), check the action's
  type against the handler's `action_name`, invoke with live state (D5), then route each result:
  `RecordKind::Update` whose target is the snapshot's component → validate and merge into the
  working state; everything else → `effects`. For the existing action-entry path, results are all
  effects, update records included, because the handler's owner is the parent.
- After the batch, re-render the body against the working state through the same path as
  `evaluate_component`, run the token walk, and return `rendered`, `effects`, and the new snapshot.
  Failure anywhere returns `Err` before any snapshot is encoded, which gives atomicity for free.

Alternative considered: embed the whole serialized handler in the rendered output as an opaque
blob and accept it back. Stateless and works for pure evaluate, but every button would carry a
copy of the component's environment, and the rendered JSON is what the playground and .NET hosts
inspect. The token keeps rendered output small and gives stale-token rejection for free.

Alternative considered: keep dispatch returning only effects and state, requiring a second
evaluate call. Rejected: evaluate cannot produce tokens without a snapshot, so the host could not
dispatch again. Returning `rendered` from dispatch is what makes the loop closed.

The snapshot's `version` field is bumped so old snapshots are rejected with the existing
"snapshot from a different revision" diagnostic.

### D7. Result and input shapes across API, FFI, and .NET

- `ComponentDispatchResult` gains `rendered: NxValue`. JSON and MessagePack outputs gain the
  `rendered` key. The C header and `NxNativeMethods` are unchanged in signature; only payloads
  change.
- .NET: `NxComponentDispatchResult<TEffect>` becomes `NxComponentDispatchResult<TRendered, TEffect>`
  with a `Rendered` property (mirrors `NxComponentInitResult<TElement>`), an `NxActionHandlerRef`
  DTO (`Action`, `Token`) for typed rendered elements, an `NxHandlerInvocation<TAction>` helper
  that serializes to the invocation record, and `NxOptional<T>` with a JSON converter and a
  MessagePack formatter for generated update DTOs. There is no backward-compatibility requirement
  in this repository, so the generic arity change is done outright.
- Node: unchanged. Component execution in JavaScript goes through the TypeScript IR runtime.

### D8. NX IR and the TypeScript runtime carry the declaration, not the dispatch

IR record declarations of update records carry `updateTarget`, a reference to the record, action,
or component they patch; the key is omitted on every other record. Only referenced update records
are emitted (a reference is any type annotation, field type, or construction naming it), and a
program that emits one adds `NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1` (`update-records-v1`) to
`required_features`. A record construction op for an update record carries `isUpdate: true`, since
the op's field list alone cannot tell a patch field from a nullable field without a default. The
TypeScript runtime accepts the feature, normalizes `isUpdate` constructions and `updateTarget`
declarations with a patch normalizer (no defaults, no required check, nullable-only `null`), and
the JavaScript program-module emitter writes only the supplied fields of an update construction.
`applyComponentStatePatch` accepts either a plain partial object or a `Component.Update` value and
checks the `$type` target when present.

### D9. typegen companions follow the `_state` precedent

`<Name>_update` is generated for every exported record, action, and stateful component, with the
existing collision warning-and-skip rule. Its fields are the target's effective shape as HIR
resolves it through the prepared module — inherited fields included, imported bases included — since
a companion extends nothing in either language and so has to carry them itself. Typegen therefore
analyzes its input the way the compiler does (`ModuleArtifact::prepared_module`) rather than
flattening bases with a resolver of its own. TypeScript: an interface with optional properties and the
`$type` literal. C#: a dual-annotated class whose properties are `NxOptional<T>`; the converters
omit unset properties and read missing keys as unset.

## Risks / Trade-offs

- [Type checking handler bodies for the first time surfaces latent errors in existing programs and
  tests] → Run the full suite early in the implementation order; fix fixtures rather than
  weakening the check. The routing rule is the intended breaking change and is called out in the
  proposal.
- [Doubling record declarations in HIR for every module could slow analysis and bloat typegen
  output] → Update records are synthesized cheaply from already-resolved field lists; IR emits
  only referenced ones; typegen only for exported declarations. Measure with the existing
  performance tests in nx-types and nx-syntax.
- [Live-state override could mask a bug where a handler was created for a different owner than the
  snapshot] → The token table is per snapshot and the owner is recorded on the handler. Dispatch
  applies the override, and routes update records to state, only for a handler the snapshot's
  component owns. A tokened handler bound elsewhere — at the root, or by a parent whose output this
  instance renders — still runs, from its captures alone, and everything it returns is an effect,
  exactly as the parent-bound handler an action entry runs. So rendered output never advertises a
  token dispatch would refuse.
- [Live-state override could clobber a loop variable or `let` that shadows a state field] → At
  handler creation only the state names whose visible binding is still the component-scope one are
  recorded as live; a shadowed name keeps its captured value, as the checker bound it.
- [Absent-versus-null is easy to lose at one boundary] → Every boundary has a scenario in the specs
  (Rust construction, NxValue JSON, MessagePack, IR, TypeScript runtime, C# DTO, TS DTO), and the
  tasks list one test per boundary. Host input is checked at every nesting depth, not only for the
  top-level props or action record, so a nested update record cannot slip past with a field the
  declaration does not have.
- [Re-rendering on every dispatch costs a body evaluation even for effect-only batches] →
  Accepted; evaluation is cheap relative to a host round trip, and a host that does not need the
  output ignores it.

## Migration Plan

Programs whose handlers inside component bodies return actions the component does not emit must
add those actions to `emits`. Hosts that read `NxComponentDispatchResult<TEffect>` move to the
two-parameter form and may ignore `Rendered`. Snapshots from before this change are rejected by
the version bump and must be re-initialized. No other source migration.

## Open Questions

- Whether the playground should treat the root program as an implicit component so a bare
  `<Update>` at root has somewhere to land. This is a playground design question and does not
  change this change's specs; the root-level rule here is "any action or update record is an
  effect", which the playground can build on either way.
