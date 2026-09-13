# component-runtime-bindings Specification

## Purpose
Defines the public runtime binding contract for initializing stateful component instances,
dispatching host-provided action batches, and round-tripping opaque component state snapshots.
## Requirements
### Requirement: Runtime bindings initialize components as stateful instances
The public NX runtime bindings SHALL allow a host to initialize a named component instance from a
`ProgramArtifact` using input prop values. Initialization SHALL resolve the component through the
embedded `ResolvedProgram` and SHALL return both the rendered element output and an opaque,
module-aware component-state snapshot for that instance.

#### Scenario: Initialization returns rendered output and initial state snapshot
- **WHEN** a host initializes `SearchBox` from a `ProgramArtifact` containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` without passing an explicit `placeholder`
- **THEN** initialization SHALL return a rendered `TextInput` element whose `value` and
  `placeholder` are both `"Find docs"`
- **AND** initialization SHALL return a component-state snapshot for that `SearchBox` instance

#### Scenario: Component without local state still initializes successfully
- **WHEN** a host initializes `Button` from a `ProgramArtifact` containing `component <Button text:string /> = { <button>{text}</button> }` with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** initialization SHALL return a component-state snapshot that can be used for later dispatch
  calls

### Requirement: Runtime bindings dispatch host-provided action batches
The public NX runtime bindings SHALL allow a host to dispatch an ordered batch against a previously
returned component-state snapshot for the same `ProgramArtifact`. Each batch entry SHALL be either
an action the component emits, which runs the handler its parent bound on the component's props, or
a handler invocation naming a handler token from the snapshot's most recent rendered output together
with the action to feed it. Dispatch SHALL process entries in the order provided, SHALL apply every
update record for the snapshot's component to its state in order, SHALL collect every other returned
value as an effect, SHALL re-render the component body against the next state once after the whole
batch, and SHALL return the rendered output, the ordered effect list, and the next component-state
snapshot.

#### Scenario: Dispatch preserves host-provided action order
- **WHEN** a host dispatches `[<SearchSubmitted searchString="docs" />, <SearchSubmitted searchString="guides" />]` against a previously returned `SearchBox` state snapshot
- **THEN** dispatch SHALL process the `"docs"` action before the `"guides"` action
- **AND** dispatch SHALL return an effect action list and a next component-state snapshot for the
  same component instance

#### Scenario: Dispatch carries state forward in this phase
- **WHEN** a host dispatches a batch whose handlers return no update record for the snapshot's
  component
- **THEN** dispatch SHALL return a next component-state snapshot representing the same component
  state values as the prior snapshot
- **AND** every returned value SHALL appear only in the effect list

#### Scenario: Handler invocation patches the component's state
- **WHEN** a host initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`, reads the handler token from the rendered `Button`'s `onTapped`, and dispatches one invocation of that token with `<Button.Tapped />`
- **THEN** dispatch SHALL return a next snapshot whose `count` is `1`
- **AND** the returned rendered output SHALL reflect `count` equal to `1`
- **AND** the effect list SHALL be empty

#### Scenario: Non-update results of a handler invocation are effects
- **WHEN** a host dispatches an invocation of a handler bound as `onTapped=[<Update count=0 />, <Saved />]` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the snapshot's state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a host dispatches `<SearchSubmitted searchString="docs" />` against a `SearchBox` instance whose parent bound `onSearchSubmitted=<Update query={action.searchString} />` inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list for the host to apply

#### Scenario: Dispatch returns rendered output in every case
- **WHEN** a host dispatches a batch that produces only effects and no state change
- **THEN** dispatch SHALL still return the rendered output for the unchanged state

### Requirement: Runtime bindings rely on host-owned state snapshots
The public NX runtime bindings SHALL remain stateless between component lifecycle calls. Hosts SHALL
supply the component-state snapshot returned by initialization or an earlier dispatch when
dispatching later actions, and that snapshot SHALL only be valid for the same `ProgramArtifact`
revision that produced it.

#### Scenario: Saved snapshot can be reused with a fresh runtime instance
- **WHEN** a host initializes a component, stores the returned state snapshot, and later dispatches
  actions through a fresh runtime instance using that saved snapshot and the same `ProgramArtifact`
  revision
- **THEN** dispatch SHALL succeed without requiring any interpreter-held component instance from the
  initialization call
- **AND** SHALL treat the stored snapshot as the full prior component state

#### Scenario: Snapshot from a different program revision is rejected
- **WHEN** a host dispatches actions with a malformed snapshot or with a snapshot produced by a
  different `ProgramArtifact` revision
- **THEN** dispatch SHALL fail with a runtime or binding error rather than silently accepting the
  snapshot

### Requirement: Source-based component lifecycle calls gate on shared analysis
Source-based component lifecycle entry points SHALL run the shared source-analysis pipeline before
component-specific validation or interpreter execution. If source analysis returns any error
diagnostics, initialization and dispatch SHALL return those diagnostics, SHALL NOT build a
`ProgramArtifact` or `ResolvedProgram`, and SHALL NOT produce component lifecycle results.

#### Scenario: Component initialization returns aggregated static diagnostics
- **WHEN** `initialize_component_source` is called with source that contains both a lowering error
  and a component state type error
- **THEN** the call SHALL return both static diagnostics from the shared analysis phase
- **AND** the call SHALL not return rendered output
- **AND** the call SHALL not return a component-state snapshot

#### Scenario: Component dispatch rejects static errors before snapshot processing
- **WHEN** `dispatch_component_actions_source` is called with source that contains both a lowering
  error and a type error and the host also supplies an invalid snapshot
- **THEN** the call SHALL return the shared source-analysis diagnostics for the source
- **AND** the call SHALL not attempt to interpret the component dispatch
- **AND** the call SHALL not return an invalid-snapshot runtime diagnostic for that request

### Requirement: Native component lifecycle bindings are artifact-first
The native C ABI SHALL expose component initialization and dispatch only for previously built
`ProgramArtifact` handles. Hosts that need imported-library resolution SHALL first build a
transient `ProgramArtifact` against a caller-supplied `ProgramBuildContext` backed by a
`LibraryRegistry`, then execute component lifecycle calls against that artifact.

#### Scenario: Native component initialization uses a preloaded library registry through a program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from source file `app/main.nx` that imports `../question-flow`
- **AND** initializes a component through the native program-artifact component API
- **THEN** initialization SHALL succeed without reloading `../question-flow` during that call

#### Scenario: Native component dispatch reuses the same program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from component source that imports `../question-flow`
- **AND** dispatches actions through the native program-artifact dispatch API
- **THEN** dispatch SHALL reuse the already built artifact and its selected loaded library
  snapshots

### Requirement: Runtime bindings support caller-selected component result formats
The public NX component lifecycle bindings SHALL allow a host to request either MessagePack or JSON
for initialization and dispatch results on a per-call basis. Output-format selection SHALL affect
only the returned payload. Hosts SHALL continue to supply props and dispatch batches in MessagePack,
and saved component snapshots SHALL continue to be passed back as opaque raw bytes. Props, dispatch
batches, rendered values, and effect values that travel through the raw `NxValue` contract SHALL
represent enum values as the bare authored member string in both host input and runtime output;
the runtime SHALL resolve those strings against the declared NX type for each prop, action
argument, rendered field, or effect field.

#### Scenario: Component initialization returns JSON with an opaque snapshot
- **WHEN** a host initializes `SearchBox` and requests JSON output
- **THEN** initialization SHALL return a UTF-8 JSON object containing `rendered` and
  `state_snapshot`
- **AND** `state_snapshot` SHALL be a base64 string that preserves the opaque snapshot bytes

#### Scenario: Component dispatch returns JSON while consuming the saved raw snapshot
- **WHEN** a host dispatches a batch using raw snapshot bytes returned from an earlier
  initialization call and requests JSON output
- **THEN** dispatch SHALL accept the raw snapshot bytes and the MessagePack batch
- **AND** SHALL return a UTF-8 JSON object containing `rendered`, `effects` and `state_snapshot`
- **AND** the returned `state_snapshot` SHALL be a base64 string for the next opaque snapshot

#### Scenario: Component MessagePack output remains available
- **WHEN** a host initializes or dispatches a component and requests MessagePack output
- **THEN** the runtime SHALL return the existing MessagePack result payload for that call, with
  dispatch results carrying `rendered` alongside `effects` and `state_snapshot`

#### Scenario: Raw enum values in component props and results are bare authored member strings
- **WHEN** a host initializes or dispatches a component whose props, rendered output, actions, or
  effects contain an enum value such as `ThemeMode.dark`
- **THEN** the raw MessagePack host input SHALL carry that enum as the bare string `"dark"` in the
  slot whose declared NX type is `ThemeMode`
- **AND** any returned raw JSON or MessagePack payload that contains that enum SHALL carry the
  bare string `"dark"` in the corresponding slot
- **AND** the payloads SHALL NOT wrap the enum value in a `"$enum"` / `"$member"` object

#### Scenario: Unknown enum member in component input is rejected
- **WHEN** a host supplies a bare string in a prop slot whose declared NX type is `ThemeMode` and
  the string does not match any authored member of `ThemeMode`
- **THEN** the binding SHALL reject the call with a type-mismatch error
- **AND** SHALL NOT silently treat the unknown member as a plain string value

### Requirement: Runtime bindings evaluate components with explicit state
The public NX runtime bindings SHALL allow a host to evaluate a named concrete component from a
`ProgramArtifact` using caller-provided prop values and caller-provided current state values.
Evaluation SHALL bind props through the component's effective prop contract, bind state through the
component's declared state fields, evaluate the component body, and return the rendered value.
Evaluation SHALL NOT create or consume an opaque component-state snapshot, SHALL NOT dispatch
actions, SHALL NOT invoke action handlers, and SHALL NOT return effects.

#### Scenario: Explicit state controls rendered output
- **WHEN** a host evaluates `SearchBox` from a `ProgramArtifact` containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string } <TextInput value={query} placeholder={placeholder} /> }` with no explicit `placeholder` prop and with state `{ query: "docs" }`
- **THEN** evaluation SHALL return a rendered `TextInput` element whose `value` is `"docs"`
- **AND** the rendered `TextInput` element's `placeholder` SHALL be `"Find docs"`

#### Scenario: Evaluation does not produce lifecycle state or effects
- **WHEN** a host evaluates a component that declares emitted actions and handler props
- **THEN** evaluation SHALL return only the rendered component body value
- **AND** evaluation SHALL NOT return a component-state snapshot
- **AND** evaluation SHALL NOT return an effect action list
- **AND** evaluation SHALL NOT invoke any bound action handler

#### Scenario: State input is normalized against declared state fields
- **WHEN** a host evaluates a component whose state declaration contains a field `query:string`
- **AND** the supplied state value does not provide `query`
- **THEN** evaluation SHALL fail with a type or missing-field diagnostic rather than silently using
  an absent state value

#### Scenario: Stateless component evaluation accepts empty state
- **WHEN** a host evaluates `Button` from a `ProgramArtifact` containing `component <Button text:string /> = { <button>{text}</button> }` with props `{ text: "Save" }` and an empty state value
- **THEN** evaluation SHALL return a rendered `button` element containing `"Save"`

### Requirement: Component evaluation uses artifact-first runtime entry points
Native component evaluation bindings SHALL be artifact-first. Hosts that need imported-library
resolution SHALL build a `ProgramArtifact` against the desired `ProgramBuildContext`, then evaluate
the component through that artifact. Source-based convenience APIs MAY exist above this layer, but
they SHALL build a transient `ProgramArtifact` before invoking the artifact-first evaluation path.

#### Scenario: Native component evaluation uses preloaded libraries through a program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from source file `app/main.nx` that imports `../question-flow`
- **AND** evaluates a component through the program-artifact component evaluation API
- **THEN** evaluation SHALL succeed without reloading `../question-flow` during that call

#### Scenario: Source-based evaluation rejects static diagnostics before execution
- **WHEN** a source-based component evaluation helper is called with source that contains static
  analysis diagnostics
- **THEN** the helper SHALL return those diagnostics
- **AND** SHALL NOT evaluate the component body

### Requirement: Component evaluation supports caller-selected result formats
The component evaluation runtime operation SHALL support caller-selected MessagePack or JSON output
formats. Output-format selection SHALL affect only the returned rendered value. Host-supplied props
and state SHALL continue to use the MessagePack host-input contract, and rendered values SHALL use
the same raw `NxValue` wire rules as root evaluation and component initialization.

#### Scenario: Component evaluation returns JSON rendered value
- **WHEN** a host evaluates `SearchBox` and requests JSON output
- **THEN** evaluation SHALL return UTF-8 JSON for the rendered component body value
- **AND** the JSON payload SHALL NOT wrap the rendered value in a lifecycle result object

#### Scenario: Component evaluation returns MessagePack rendered value
- **WHEN** a host evaluates `SearchBox` and requests MessagePack output
- **THEN** evaluation SHALL return MessagePack bytes for the rendered component body value
- **AND** the MessagePack payload SHALL NOT include component-state snapshot or effect fields

#### Scenario: Component evaluation preserves raw enum wire shape
- **WHEN** a host evaluates a component whose props, state, or rendered output contain an enum value
  such as `ThemeMode.dark`
- **THEN** raw host input and runtime output SHALL represent that enum as the bare authored member
  string in the slot whose declared NX type is `ThemeMode`
- **AND** unknown enum members in prop or state input SHALL be rejected through the existing
  type-mismatch path

### Requirement: Lifecycle rendered output identifies bound handlers by token
Rendered output returned by component initialization and by dispatch SHALL represent every bound
handler as a record with `$type` `ActionHandler` carrying the public name of the action it accepts
and an opaque string token. A token SHALL be valid only for dispatch against the snapshot returned
by the same call. Rendered output returned by pure component evaluation SHALL represent handlers
the same way but SHALL NOT carry a token, since there is no snapshot to dispatch against.

#### Scenario: Initialization output carries handler tokens
- **WHEN** a host initializes a component whose body renders `<Button onTapped=<Update count={count + 1} /> />`
- **THEN** the rendered `Button` element's `onTapped` SHALL be a record with `$type` `ActionHandler`, `action` `Button.Tapped`, and a non-empty `token`

#### Scenario: A token from a stale snapshot is rejected
- **WHEN** a host dispatches a handler invocation using a token read from an earlier rendered output than the one the supplied snapshot was returned with
- **THEN** dispatch SHALL fail with a diagnostic naming the unknown handler token
- **AND** SHALL NOT change state or produce effects

#### Scenario: An invocation with the wrong action type is rejected
- **WHEN** a host dispatches a token bound for `Button.Tapped` together with a `Slider.EndChanged` action
- **THEN** dispatch SHALL fail with a type-mismatch diagnostic naming the expected action

#### Scenario: Pure evaluation output carries no tokens
- **WHEN** a host evaluates a component through the pure evaluation operation
- **THEN** every handler in the rendered output SHALL be an `ActionHandler` record without a `token`

### Requirement: A dispatch batch is atomic
If any entry in a dispatch batch fails — an unknown token, a handler runtime error, an invalid
update record, or an action the component does not emit — dispatch SHALL fail as a whole, SHALL NOT
return a next snapshot, and the snapshot the host supplied SHALL remain the authoritative state.

#### Scenario: A failing second entry discards the first entry's patch
- **WHEN** a host dispatches a batch whose first entry patches `count` to `5` and whose second entry names an unknown token
- **THEN** dispatch SHALL fail with the unknown-token diagnostic
- **AND** dispatching a valid batch afterwards against the original snapshot SHALL observe `count` at its original value

#### Scenario: Update validation failures abort the batch
- **WHEN** a handler invoked during dispatch returns an update record whose field value does not match the state field's type, with static analysis bypassed
- **THEN** dispatch SHALL fail with a diagnostic naming the field
- **AND** SHALL NOT return a partially patched snapshot

### Requirement: Host-supplied records are constructed at every depth
A record the host supplies through component props, explicit component state, or a dispatch batch
entry SHALL be constructed from its fields against the declaration its type name reaches, at every
nesting depth: inside another record, an update record, a union case payload, a component value,
or an array. A component value SHALL be constructed against the component's props, as an element
tag naming it would be. A dispatch batch entry that is an emitted action SHALL be constructed
whether or not the instance's parent bound a handler for it. That
construction SHALL apply the rules static analysis applies to the same value — an unknown field is
rejected, `null` is accepted only where the field is nullable, a plain record's required fields are
present or defaulted, and an update record's absent fields stay absent — and a violation SHALL fail
the call with a diagnostic naming the offending field. A value the runtime itself produced SHALL NOT
be re-checked.

#### Scenario: An update record in a prop keeps only the fields it carries
- **WHEN** a host initializes `component <Editor pending:User.Update /> = { <Panel pending={pending} /> }` with `pending` set to `{ $type: "User.Update", email: null }`
- **THEN** the rendered `pending` SHALL be a `User.Update` carrying `email` as `null` and no `name`

#### Scenario: An unknown field in a prop's update record is rejected
- **WHEN** a host initializes `Editor` with `pending` set to `{ $type: "User.Update", nick: "a" }`
- **THEN** initialization SHALL fail with a diagnostic naming `nick`

#### Scenario: A null for a non-nullable field inside an action payload is rejected
- **WHEN** a host dispatches an invocation of a handler bound for `Button.Tapped`, where `Button` emits `Tapped { patch:User.Update }`, with `patch` set to `{ $type: "User.Update", name: null }`
- **THEN** dispatch SHALL fail with a diagnostic naming `name`
- **AND** SHALL NOT change state or produce effects

#### Scenario: A record inside a prop array is checked
- **WHEN** a host initializes a component with a prop typed `User.Update[]` whose second element carries a field `User` does not declare
- **THEN** initialization SHALL fail with a diagnostic naming that field

#### Scenario: A plain record nested in a prop is checked
- **WHEN** a host initializes `component <Card user:User />`, where `User = { name:string address:Address }` and `Address = { city:string }`, with an `address` that omits `city`
- **THEN** initialization SHALL fail with a diagnostic naming `city`

#### Scenario: A record under a component-typed prop is checked
- **WHEN** a host initializes `component <Wrap inner:Editor />`, where `component <Editor pending:User.Update />`, with `inner` set to `{ $type: "Editor", pending: { $type: "User.Update", nick: "a" } }`
- **THEN** initialization SHALL fail with a diagnostic naming `nick`

#### Scenario: An action entry with no bound handler is still checked
- **WHEN** a host dispatches `{ $type: "Editor.Apply", patch: { $type: "User.Update", name: null } }`, where `Editor` emits `Apply { patch:User.Update }`, against an instance whose parent bound no `onApply`
- **THEN** dispatch SHALL fail with a diagnostic naming `name`
- **AND** the same entry with a well-formed `patch` SHALL succeed with no effects
