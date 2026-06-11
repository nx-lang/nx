## ADDED Requirements

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
