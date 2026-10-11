## ADDED Requirements

### Requirement: A session runs one component of a prepared program
The repository SHALL publish `@nx-lang/previewer`, a package with no UI and no framework dependency
that depends on `@nx-lang/ir-runtime` alone. It SHALL export `createPreviewSession(program,
component, props, options)`, which initializes the named component of an `NxPreparedProgram` with
the given props and returns a session whose first tick holds the result. It SHALL export
`programFromImages(images, entry)`, which prepares and links a list of `{ identity, bytes }` IR
images into a program whose entry module is `entry`. A session's options SHALL include the
runtime's limits, `maxOperations`, `maxInputSize`, `maxCallDepth` and `maxRangeLength`, passed to
every call the session makes, SHALL include whether to report origins, on by default, and SHALL
include `maxTicks`, the most ticks the session keeps, 1,000 by default.

#### Scenario: Starting the question flow
- **WHEN** a host links the question-flow conformance program's images with `programFromImages` and
  creates a session for `Flow` with the props `{ respondent: "friend" }`
- **THEN** the session SHALL have one tick whose rendered output equals the `initial` output in the
  program's `expected/results.json`

#### Scenario: Props the component does not accept
- **WHEN** a host creates a session with props the component's declaration rejects
- **THEN** `createPreviewSession` SHALL fail with the runtime's diagnostics and return no session

### Requirement: Each tick keeps what made it and what it rendered
Every tick SHALL hold its cause (the initial props, a batch, new props, or a reload), the program
that rendered it, its props, the component instance, the state, the rendered output, the effects
the batch's handlers returned, and its parent tick. Ticks SHALL be immutable. A session SHALL have a
current tick, and its `path` SHALL be the ticks from the current tick's root to the current one, in
order. A dispatch or a props change from a tick SHALL run under the program that rendered it. When a
new tick takes the session past `maxTicks`, the session SHALL drop the oldest ticks that are off the
current path and have no children until it is within the limit or no such tick is left.

#### Scenario: A tick after an answer
- **WHEN** a host dispatches the first batch of the question-flow lifecycle
- **THEN** the new tick SHALL hold that batch, a state whose `step` is 1, the rendered output and
  effects for that batch in `expected/results.json`, and the first tick as its parent

### Requirement: Going back runs nothing, and dispatching from the past branches
A session SHALL let a host make any tick the current one without evaluating anything. A batch
dispatched while the current tick already has children SHALL add a new child to it, keeping the
existing children and every tick after them, so the tree holds both continuations. A tick's children
SHALL be kept in the order they were made.

#### Scenario: Trying another role
- **WHEN** a host runs the question flow to the role question, answers "engineer", runs on two more
  steps, goes back to the tick before the role answer, and answers "designer"
- **THEN** the tick before the role answer SHALL have two children, the "engineer" continuation
  SHALL still be in the tree with its later ticks, and the current path SHALL end with the
  "designer" answer

### Requirement: A host can dispatch by handler
A session SHALL dispatch a batch as the runtime takes it, a list of `ActionHandlerInvocation`
entries naming tokens. It SHALL also dispatch by handler: given the JSON pointer (RFC 6901) of an
`ActionHandler` record in the current tick's rendered output and an action record, it SHALL use the
token the record carries, after checking that the action's `$type` is the action the record names.
A pointer that names no `ActionHandler` record, or an action of another type, SHALL fail without
dispatching, and so SHALL an entry by token whose token the current output does not hold. The new
tick SHALL record, for each entry, the pointer of the handler it invoked: the pointer it was given,
or for an entry by token, the pointer of the record that carries the token. An entry that is an
action the component emits invokes no handler in the output and SHALL record none.

#### Scenario: Answering through the step's handler
- **WHEN** the current output's step renders an `ActionHandler` record for `Step.TextAnswered` at
  `/children/1/onTextAnswered` and a host dispatches a `Step.TextAnswered` action by that pointer
- **THEN** the session SHALL dispatch it with the token that record carries, and the new tick SHALL
  record the pointer

#### Scenario: The wrong action for a handler
- **WHEN** a host dispatches a `Step.ChoiceAnswered` action by the pointer of a handler for
  `Step.TextAnswered`
- **THEN** the dispatch SHALL fail naming both actions, and no tick SHALL be added

### Requirement: A failure adds no tick
A dispatch, a props change or a reload that fails SHALL return the runtime's diagnostics, including
a resource limit's, SHALL add no tick, and SHALL leave the current tick as it was.

#### Scenario: A handler that exceeds the operation budget
- **WHEN** a session has a `maxOperations` budget that the next batch's handlers exceed
- **THEN** the dispatch SHALL fail with the runtime's `nx-ir-resource-limit` diagnostic, and the
  current tick SHALL be unchanged

### Requirement: Props can change without losing state
A session SHALL let a host set new props on the current tick. It SHALL initialize the component
with the new props and the current tick's state, and add the result as a child tick whose cause is
the new props.

#### Scenario: A new respondent
- **WHEN** a host is on the first question of the flow and sets the props to
  `{ respondent: "Grace" }`
- **THEN** the new tick SHALL hold the new props and the same state as the tick it came from,
  including the `name` "friend" that the first render took from the old respondent

### Requirement: A run saves and replays as a scenario
A scenario SHALL be a `program.json` lifecycle, with `module`, `component`, `props` and `batches`,
and two optional members: `name`, and `handlers`, which holds for each entry of each batch the
pointer of the handler it invoked or `null`. A session SHALL export its current path as a
scenario, in which a reload tick adds nothing; a path that changes the props SHALL fail to export,
since a lifecycle cannot hold a props change. Replaying a scenario SHALL start a new root from the
scenario's props and dispatch each batch in turn, using the current token at an entry's pointer
when it has one and the recorded entry otherwise. A replay SHALL stop at the first entry whose
pointer names no handler for the recorded action, or whose token the current output does not hold,
and at a batch the runtime fails; it SHALL keep the ticks before it and report the batch, and the
entry when one could not be placed. The package SHALL also export `replayScenario(program,
scenario, options)`, which starts a session by replaying a scenario.

#### Scenario: Replaying the conformance lifecycle
- **WHEN** a host replays the question-flow program's lifecycle from `program.json` as a scenario
- **THEN** the replay SHALL complete all 30 batches, and each tick's rendered output SHALL equal the
  corresponding output in `expected/results.json`

#### Scenario: A scenario round trip
- **WHEN** a host answers five questions by handler, exports the path as a scenario named
  "designer, team of 24", and replays it in a new session of the same program
- **THEN** the replayed path SHALL have the same states and rendered outputs as the original

#### Scenario: A replay that can no longer be placed
- **WHEN** a scenario's third batch was dispatched by a pointer at which the edited program renders
  no handler for that action
- **THEN** the replay SHALL keep the ticks of the first two batches and report that it stopped at
  the third batch's first entry

### Requirement: Hot reload keeps the reviewer where they were
A session SHALL accept a new program for the same component. It SHALL first initialize the
component under the new program with the current tick's props and state, and on success add the
result as a child of the current tick whose cause is a reload. When that fails but the current
props render under the new program, it SHALL replay the current path's batches and props changes
under the new program from a new root, as a scenario replay does, and report how far it got. When
the props do not render under the new program, it SHALL keep the old program and the current tick
and report the failure. The result SHALL say which of these happened, and the old ticks SHALL stay
in the tree in every case.

#### Scenario: A label fixed on question 22
- **WHEN** a host is on question 22 of the flow and reloads with a program that changes only a
  question's label
- **THEN** the session SHALL keep the state, add a reload tick whose output shows the new label, and
  report that the state was kept

#### Scenario: A state field renamed
- **WHEN** a host is on question 10 of the flow and reloads with a program whose `Flow` renames its
  `rating` state field to `score`
- **THEN** the session SHALL replay the path under the new program, complete it, and report that it
  replayed rather than kept the state

#### Scenario: A required prop added
- **WHEN** a host reloads with a program whose `Flow` adds a required prop that the current props do
  not give
- **THEN** the session SHALL keep the old program and the current tick, add no tick, and report the
  failure with the runtime's diagnostics

### Requirement: Each tick knows where its records came from
When origins are on, every call a session makes SHALL pass an origins report, and each tick SHALL
hold the report's entries for its rendered output. A session SHALL give, for a tick and a JSON
pointer into its rendered output, the origin of the record at that pointer, or none.

#### Scenario: The origins of the initial render
- **WHEN** a host creates a session for the question flow with origins on
- **THEN** the first tick's origin entries SHALL equal the `initial` entries in the program's
  `expected/origins.json`

#### Scenario: Finding a step's question
- **WHEN** a host asks for the origin of `/children/1/question` in a tick of the flow
- **THEN** the session SHALL return the module and span of the question's element expression
