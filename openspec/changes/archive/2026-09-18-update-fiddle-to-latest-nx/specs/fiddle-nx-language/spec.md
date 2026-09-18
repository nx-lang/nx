## ADDED Requirements

### Requirement: The fiddle's catalog declares DrawnUI events as emits
The catalog the fiddle generates SHALL declare every DrawnUI event, an optional function-typed member
whose return type is `void` or includes it, as an emit of the component that stands for the class
declaring it, inherited by every control below. A callback parameter with a primitive NX type SHALL
become a payload field under its own name; a parameter typed as an engine object SHALL be dropped
and recorded with the catalog's other divergences. The generated metadata SHALL record each event's
parameter names in order, so the renderer can build an action from a callback's arguments.
Function-typed members that are not events SHALL stay omitted.

#### Scenario: A tap is bindable on every control
- **WHEN** a snippet binds `onTapped` on a `SkiaShape` inside a component
- **THEN** it SHALL compile, because `SkiaControl` declares `Tapped` and `SkiaShape` inherits it

#### Scenario: A payload carries the callback's primitive arguments
- **WHEN** a snippet binds `onToggled=<Update on={action.value} />` on a `SkiaSwitch`
- **THEN** it SHALL compile, with `action.value` typed `boolean`

#### Scenario: Events leave the omitted list
- **WHEN** the catalog is regenerated
- **THEN** the omitted list SHALL contain no event callback
- **AND** it SHALL still list members such as `ItemTemplate` with their TypeScript types

#### Scenario: An unknown event is a compile error in the snippet's lines
- **WHEN** a snippet binds `onNoSuchEvent` on a control
- **THEN** the run SHALL fail with an error listed as `L{line}: {message}` in the snippet's own
  line numbers

### Requirement: Authored handlers run in the editor and in the player
The NX runtime SHALL draw every use of an authored component as a component instance it holds, SHALL
turn each handler an author binds on a drawn control into that control's DrawnUI event, and on the
event SHALL dispatch the handler against the instance whose body bound it and redraw from the
result, without compiling. Results SHALL be routed as the language defines them: an update record
patches the state of the instance that owns the handler; an action a component emits is delivered
to the handler its parent bound for that emit, and the parent's state is patched in turn; anything
else is a host effect, reported with the action and the component that produced it. An event that
arrives for a drawing a dispatch has already replaced SHALL NOT run — its handler was retired with
the instance it was bound to — and SHALL be reported rather than dropped in silence. Every dispatch
SHALL be atomic, and a dispatch that fails SHALL be reported and leave the drawing as it was. The
player SHALL behave identically from a share's image, without the compiler module.

#### Scenario: A tap patches state and redraws
- **WHEN** a snippet declares a component with `state { count:int = 0 }`, a label whose text is
  `"Taps: " + count` and a button with `onTapped=<Update count={count + 1} />`, and draws it
- **AND** the visitor taps the button twice
- **THEN** the label SHALL read `Taps: 2`
- **AND** no compile SHALL have been issued by the taps

#### Scenario: An emitted action reaches the parent's handler
- **WHEN** a child component declares `emits { Chosen { name:string } }` and binds
  `onTapped=<Child.Chosen name="a" />` on a button
- **AND** a parent with state `picked:string = ""` uses it as
  `<Child onChosen=<Update picked={action.name} /> />` and draws `picked` in a label
- **AND** the visitor taps the child's button
- **THEN** the parent's label SHALL redraw as `a`

#### Scenario: Child state survives a parent redraw
- **WHEN** an authored child with its own state is drawn inside a parent whose state changes
- **THEN** after the parent redraws, the child SHALL be drawn from the parent's new props and the
  state the child held before

#### Scenario: A handler outside a component is inert and reported
- **WHEN** a snippet binds `onTapped` on a control of its top-level element rather than inside a
  component
- **THEN** the control SHALL draw without a callback
- **AND** the runtime SHALL say once that handlers run inside a component

#### Scenario: A failed dispatch leaves the drawing
- **WHEN** a handler's dispatch fails with a runtime diagnostic
- **THEN** the report SHALL carry the diagnostic's message
- **AND** the drawing SHALL be the one from before the event

#### Scenario: A second event in one gesture is reported, not dropped
- **WHEN** one gesture fires two events on one drawing, as a radio group does by toggling the
  button that was on and the button that was tapped
- **THEN** the first SHALL dispatch and redraw
- **AND** the second SHALL be reported as having arrived for a drawing already replaced, naming the
  control and the handler property

#### Scenario: Running again resets the instances
- **WHEN** the visitor edits the snippet and it runs again
- **THEN** every instance SHALL start from its initial state

#### Scenario: The player runs handlers without the compiler
- **WHEN** the player opens a share of the tap-counting snippet and the visitor taps the button
- **THEN** the label SHALL redraw with the new count
- **AND** no request for the compiler module SHALL have been made

### Requirement: A snippet prints through an action, in the host's own console
NX has no statement that prints, so the runtime SHALL give a snippet the host's console through the
actions that leave it: an action named `Log` carrying a string `text` that reaches the host SHALL be
written out as that text alone, and every other host effect named in full with the component that
produced it. Every report the runtime makes — host effects, a failed dispatch, an unknown control,
an inert handler — SHALL be written to the host's own console channel where the page offers one, so
that a visitor reading it sees what a snippet said whichever language wrote it, and SHALL be written
to the browser console in every case, which is the only channel the player has.

#### Scenario: A Log action is written as its text
- **WHEN** a snippet declares `action Log = { text:string }`, emits it from a handler as
  `<Log text={"Button clicked " + (taps + 1) + " time(s)"} />`, and nothing handles it
- **AND** the visitor taps the control twice
- **THEN** the host's console SHALL carry `Button clicked 1 time(s)` and `Button clicked 2 time(s)`,
  with nothing added to either line

#### Scenario: Any other host effect is named in full
- **WHEN** an action that is not a `Log` reaches the host
- **THEN** the report SHALL name the action, its fields and the component that produced it

#### Scenario: Reports reach the editor's console pane
- **WHEN** the page offers a console channel of its own and the runtime reports anything
- **THEN** the line SHALL be written to that channel as well as to the browser console

### Requirement: NX presets show interaction
The NX presets SHALL use the event handlers, component state and primitive conversions NX has,
rather than porting an interactive original as a static drawing: the starter SHALL respond to a tap,
and at least one preset SHALL show an action emitted by a child component reaching its parent. No
preset's source or description SHALL say NX lacks a capability it has.

#### Scenario: The starter responds to a tap
- **WHEN** the starter preset is drawn and the visitor taps its button
- **THEN** the drawing SHALL change to show the tap
- **AND** the console SHALL carry every line its C# twin prints for that tap, with the same text

#### Scenario: A preset says in NX what a twin says another way
- **WHEN** a twin reports something through a mechanism NX does not have, such as observing a
  control's property or running an effect when state changes
- **THEN** the NX preset SHALL report the same thing from the handler that caused it
- **AND** its source SHALL say which mechanism the twin used and that NX has none

#### Scenario: The component a preset draws is named as its twins name theirs
- **WHEN** an NX preset declares the component it draws
- **THEN** that component SHALL be named `App`, as the engine's React templates name their own root
  component, and the preset SHALL end in `<App />`
- **AND** a preset that needs no state or handler of its own MAY be one top-level element instead

#### Scenario: Every interactive preset dispatches in the tests
- **WHEN** the runtime's tests compile the presets
- **THEN** for every preset that binds a handler, the tests SHALL dispatch one and find the
  redrawn output changed

#### Scenario: Interactive presets fit the share budget
- **WHEN** the runtime's tests compile every NX preset
- **THEN** each share module SHALL be under 128 KB

### Requirement: The runtime builds and tests against an NX checkout
The fiddle's runtime build and NX tests SHALL be able to take every `@nx-lang/*` package, the
compiler module included, from a built sibling NX checkout named on the command line or in the
environment, in place of the pinned packages, with no change to `package.json` or `node_modules`.
The build SHALL say which source it used.

#### Scenario: A checkout replaces the pinned packages
- **WHEN** a maintainer runs the runtime build naming an NX checkout
- **THEN** the NX runtime bundle and the compiler module SHALL come from that checkout
- **AND** the build's summary line SHALL name the checkout

#### Scenario: Something that is not a checkout is refused
- **WHEN** the named directory is not an NX checkout
- **THEN** the build SHALL fail naming the directory, before building anything

#### Scenario: A catalog image from another compiler is stale
- **WHEN** the committed catalog image was emitted by a compiler whose image layout differs from
  the checkout's
- **THEN** the build SHALL fail naming the catalog and the command that regenerates it
