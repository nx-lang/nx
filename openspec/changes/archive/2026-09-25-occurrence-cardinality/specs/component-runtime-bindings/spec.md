## MODIFIED Requirements

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
- **WHEN** a host dispatches the batch `{ <SearchSubmitted searchString="docs" /> <SearchSubmitted searchString="guides" /> }` against a previously returned `SearchBox` state snapshot
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
- **WHEN** a host dispatches an invocation of a handler bound as `onTapped={ <Update count=0 /> <Saved /> }` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the snapshot's state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a host dispatches `<SearchSubmitted searchString="docs" />` against a `SearchBox` instance whose parent bound `onSearchSubmitted=<Update query={action.searchString} />` inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list for the host to apply

#### Scenario: Dispatch returns rendered output in every case
- **WHEN** a host dispatches a batch that produces only effects and no state change
- **THEN** dispatch SHALL still return the rendered output for the unchanged state

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

#### Scenario: An optional state field may be omitted from the supplied state
- **WHEN** a host evaluates a component whose state declaration contains a field `query?:string`
- **AND** the supplied state value does not provide `query`, or provides it as `null`
- **THEN** evaluation SHALL bind `query` to the empty value and SHALL render the body

#### Scenario: Stateless component evaluation accepts empty state
- **WHEN** a host evaluates `Button` from a `ProgramArtifact` containing `component <Button text:string /> = { <button>{text}</button> }` with props `{ text: "Save" }` and an empty state value
- **THEN** evaluation SHALL return a rendered `button` element containing `"Save"`

### Requirement: Host-supplied records are constructed at every depth
A record the host supplies through component props, explicit component state, or a dispatch batch
entry SHALL be constructed from its fields against the declaration its type name reaches, at every
nesting depth: inside another record, an update record, a union case payload, a component value,
or a sequence. A component value SHALL be constructed against the component's props, as an element
tag naming it would be. A dispatch batch entry that is an emitted action SHALL be constructed
whether or not the instance's parent bound a handler for it. That
construction SHALL apply the rules static analysis applies to the same value — an unknown field is
rejected, `null`, a missing key or an empty array decodes to the empty value and is accepted only
where the field is optional, an empty array is rejected at a `+` field, a plain record's required
fields are present or defaulted, and an update record's absent fields stay absent while its present
empty fields stay present — as `occurrence-types` and `update-records` define the decoding, and a
violation SHALL fail the call with a diagnostic naming the offending field. A value the runtime
itself produced SHALL NOT be re-checked.

#### Scenario: An update record in a prop keeps only the fields it carries
- **WHEN** a host initializes `component <Editor pending:User.Update /> = { <Panel pending={pending} /> }`, where `User` declares `email?:string`, with `pending` set to `{ $type: "User.Update", email: null }`
- **THEN** the rendered `pending` SHALL be a `User.Update` carrying `email` present and empty and no `name`
- **AND** the returned JSON SHALL encode that `email` as `null`, as `update-records` requires

#### Scenario: An unknown field in a prop's update record is rejected
- **WHEN** a host initializes `Editor` with `pending` set to `{ $type: "User.Update", nick: "a" }`
- **THEN** initialization SHALL fail with a diagnostic naming `nick`

#### Scenario: A null for a non-nullable field inside an action payload is rejected
- **WHEN** a host dispatches an invocation of a handler bound for `Button.Tapped`, where `Button` emits `Tapped { patch:User.Update }` and `User` declares `name:string`, with `patch` set to `{ $type: "User.Update", name: null }`
- **THEN** dispatch SHALL fail with a diagnostic naming `name`
- **AND** SHALL NOT change state or produce effects

#### Scenario: A null and an empty array for an optional field are the empty value
- **WHEN** a host initializes `component <Card user:User />`, where `User = { name:string nickname?:string tags?:string+ }`, with `user` set to `{ name: "Ada", nickname: null, tags: [] }`
- **THEN** initialization SHALL succeed with `nickname` and `tags` both empty
- **AND** the same call with `tags` omitted SHALL produce an equal value

#### Scenario: An empty array for a one-or-more field is rejected
- **WHEN** a host initializes `component <Card user:User />`, where `User = { name:string tags:string+ }`, with `user` set to `{ name: "Ada", tags: [] }`
- **THEN** initialization SHALL fail with a diagnostic naming `tags`

#### Scenario: A record inside a prop array is checked
- **WHEN** a host initializes a component with a prop typed `User.Update+` whose second element carries a field `User` does not declare
- **THEN** initialization SHALL fail with a diagnostic naming that field

#### Scenario: A plain record nested in a prop is checked
- **WHEN** a host initializes `component <Card user:User />`, where `User = { name:string address:Address }` and `Address = { city:string }`, with an `address` that omits `city`
- **THEN** initialization SHALL fail with a diagnostic naming `city`

#### Scenario: A record under a component-typed prop is checked
- **WHEN** a host initializes `component <Wrap inner:Editor />`, where `component <Editor pending:User.Update />`, with `inner` set to `{ $type: "Editor", pending: { $type: "User.Update", nick: "a" } }`
- **THEN** initialization SHALL fail with a diagnostic naming `nick`

#### Scenario: An action entry with no bound handler is still checked
- **WHEN** a host dispatches `{ $type: "Editor.Apply", patch: { $type: "User.Update", name: null } }`, where `Editor` emits `Apply { patch:User.Update }` and `User` declares `name:string`, against an instance whose parent bound no `onApply`
- **THEN** dispatch SHALL fail with a diagnostic naming `name`
- **AND** the same entry with a well-formed `patch` SHALL succeed with no effects
