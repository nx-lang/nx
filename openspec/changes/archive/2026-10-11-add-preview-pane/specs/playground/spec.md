## ADDED Requirements

### Requirement: The output pane previews a component
The playground's output pane SHALL offer an Output and Preview switch in its title, Output by
default. Preview SHALL run one component of the visitor's source in a preview session of
`@nx-lang/previewer`, inside the playground's worker, and show the session's current output
through `<nx-value>`, spelled by the artifact's `formatValue`. When `root` returns an element of a
component the source declares, Preview SHALL run that component with that element's props.
Otherwise it SHALL run the first non-external component the source declares, with no props given.
A picker SHALL list the non-external components the source declares, in source order, and choosing
one SHALL start a new session for it. Output SHALL keep working as before, and switching between
the two SHALL keep the session.

#### Scenario: A component element as root
- **WHEN** the source declares `component <Counter start:int = 0 />` with a `count` state that
  starts at `start`, ends with `<Counter start=3 />`, and the visitor switches to Preview
- **THEN** the pane SHALL show what `Counter` renders with `start` 3, and its state SHALL hold a
  `count` of 3

#### Scenario: Another component
- **WHEN** the source declares `Counter` and `Stepper` and the visitor picks `Stepper`
- **THEN** a new session SHALL start for `Stepper` with no props given, and the pane SHALL show what
  it renders

#### Scenario: Props the component requires
- **WHEN** the component Preview runs requires a prop that nothing gives
- **THEN** the pane SHALL show the runtime's message, and say that ending the program with the
  component's element, with its props, previews it with them

#### Scenario: No component
- **WHEN** the source declares no component, or only external ones
- **THEN** Preview SHALL say that there is no component to preview, and how to declare one

#### Scenario: Source that does not compile
- **WHEN** the visitor switches to Preview while the source does not compile
- **THEN** Preview SHALL say that the program does not compile yet, as Output does

#### Scenario: Preview on a phone
- **WHEN** a visitor switches to Preview at a viewport 375 pixels wide
- **THEN** the preview, its timeline and its state SHALL fit the pane's width without the page
  scrolling horizontally

### Requirement: Handlers in the preview are buttons
In Preview, every `ActionHandler` record in the output SHALL carry a button named for the action it
handles, through `<nx-value>`'s `actions`. Pressing it SHALL dispatch the action to that handler by
its pointer, and the output, state and timeline SHALL show the new tick. An action with no fields
SHALL dispatch at once. An action with fields SHALL open a form with one input per field, built from
the action's input schema (`typeSchema` with `direction: "input"`): a text field for a string, a
number field for an integer or a `float32` or `float64`, a checkbox for a `boolean`, a list of cases
for a constant union, and a JSON field for any other type, each starting from the field's default
when it has one. A dispatch that fails SHALL add no tick and SHALL show the failure's message beside
the form or button.

#### Scenario: Pressing a button
- **WHEN** a visitor previews the counter, whose `Button` binds `onTapped=<Update count={count +
  1} />`, and presses "Tapped"
- **THEN** the output SHALL show the count as 4, and the timeline SHALL gain a step

#### Scenario: An action with fields
- **WHEN** the output holds a handler for `action Answered = { questionId:string value:int }` and
  the visitor presses its button
- **THEN** a form SHALL open with a text field for `questionId` and a number field for `value`, and
  submitting it SHALL dispatch `Answered` with those values

#### Scenario: A failed dispatch
- **WHEN** a handler's body fails at run time
- **THEN** the pane SHALL show the runtime's message, the timeline SHALL be unchanged, and the
  source pane SHALL mark the failure's span when it has one

### Requirement: The preview keeps a timeline
Preview SHALL show a timeline strip of the ticks on the session's current path, from the first
render to the current tick, each step named by what made it: the start, the action dispatched, new
props, or an edit. Choosing a step SHALL make it current without running anything. A step that has
more than one child SHALL show how many branches leave it and let the visitor choose which to
follow. Back SHALL go to the current step's parent, and Forward SHALL go to the child the visitor
last left, or to the newest child when they have left none. Dispatching from a step that is not the
last SHALL add a branch, keeping the steps that followed it.

#### Scenario: Going back
- **WHEN** a visitor has pressed "Tapped" twice and chooses the first step
- **THEN** the output SHALL show the count as 3 again and the state SHALL hold 3, and nothing SHALL
  have run

#### Scenario: A branch
- **WHEN** the visitor then presses a different button from that first step
- **THEN** the first step SHALL show two branches, and the visitor SHALL be able to return to the
  run with two taps

### Requirement: The preview shows state and what was emitted
Preview SHALL show the current tick's state through `<nx-value>`, one `name=value` line per state
field as the artifact's `formatValue` spells a component's state, and say so when the component
keeps no state. It SHALL list the actions the current tick emitted to its host, the effects no
handler of the component consumed, each spelled as NX text.

#### Scenario: State
- **WHEN** the counter's current tick holds a `count` of 4
- **THEN** the state panel SHALL show `count=4`

#### Scenario: Emitted actions
- **WHEN** a handler returns an action the component emits to its host
- **THEN** the panel SHALL list that action under what the tick emitted

### Requirement: Edits hot reload the preview
While Preview shows a session, each edit that compiles SHALL reload the session with the new
program, after the visitor pauses typing as evaluation does. When the new program accepts the
current state, the output SHALL update in place and the timeline SHALL gain an edit step. When it
does not, the run SHALL be replayed under the new program, and the pane SHALL say so, and where the
replay stopped when it did not complete. When the props no longer render, the pane SHALL keep the
last output, marked out of date, and say why. When an edit changes the props of the element `root`
returns, the session SHALL render the current state with the new props. An edit that does not
compile SHALL leave the session as it was, marked out of date.

#### Scenario: A label fixed
- **WHEN** a visitor previewing the counter changes its label text from `"Count: "` to `"Total: "`
- **THEN** the output SHALL read `Total: 3`, with the state unchanged

#### Scenario: A state field renamed
- **WHEN** the visitor renames the `count` state field to `total`
- **THEN** the pane SHALL say that the run was replayed, and the timeline SHALL show the replayed
  steps

#### Scenario: New props
- **WHEN** the visitor changes the final element to `<Counter start=10 />`
- **THEN** the timeline SHALL gain a props step, and the state SHALL be kept

### Requirement: The preview leads to the source that built it
In Preview, clicking a record's type name SHALL select, in the source pane, the element expression
that built that record, from the session's value origins, and scroll it into view. A record with no
origin SHALL lead to its declaration, as in Output. Hovering SHALL explain a node as it does in
Output.

#### Scenario: From a rendered label to its expression
- **WHEN** a visitor previewing the counter clicks `Label` in the output
- **THEN** the source pane SHALL select `<Label text={"Count: " + count} />`

### Requirement: The preview runs within limits
Every call the preview session makes SHALL run under the runtime's limits, with a call depth of at
most 200 as evaluation allows, so that a handler that runs without end stops with the runtime's
resource-limit message and the page stays responsive. The output shown SHALL be cut at 100,000
characters as Output's is.

#### Scenario: A runaway handler
- **WHEN** a handler calls a function that recurses without end and the visitor presses its button
- **THEN** the pane SHALL show the runtime's resource-limit message, the timeline SHALL be
  unchanged, and the editor SHALL keep responding

### Requirement: An example previews at once
The playground's examples SHALL include "A counter to click", under the Components topic: a
`Counter` component with a `count` state and buttons that add one and reset, whose program ends
with a `Counter` element, so that switching to Preview runs it with no further steps.

#### Scenario: Opening the example
- **WHEN** a visitor opens "A counter to click" and switches to Preview
- **THEN** the pane SHALL show the counter with buttons for adding and resetting, and pressing them
  SHALL change the count
