## MODIFIED Requirements

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
  `"Taps: " + count` and a button with `onTapped=<State.Update count={count + 1} />`, and draws it
- **AND** the visitor taps the button twice
- **THEN** the label SHALL read `Taps: 2`
- **AND** no compile SHALL have been issued by the taps

#### Scenario: An emitted action reaches the parent's handler
- **WHEN** a child component declares `emits { Chosen { name:string } }` and binds
  `onTapped=<Child.Chosen name="a" />` on a button
- **AND** a parent with state `picked:string = ""` uses it as
  `<Child onChosen=<State.Update picked={action.name} /> />` and draws `picked` in a label
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

### Requirement: The fiddle's catalog declares DrawnUI events as emits
The catalog the fiddle generates SHALL declare every DrawnUI event, an optional function-typed member
whose return type is `void` or includes it, as an emit of the component that stands for the class
declaring it, inherited by every control below. A callback parameter with a primitive NX type SHALL
become a payload field under its own name; a parameter typed as an engine object SHALL be dropped
and recorded with the catalog's other divergences. The generated metadata SHALL record each event's
parameter names in order, so the renderer can build an action from a callback's arguments.
Function-typed members that are neither events nor a templated control's cell factory SHALL stay
omitted.

#### Scenario: A tap is bindable on every control
- **WHEN** a snippet binds `onTapped` on a `SkiaShape` inside a component
- **THEN** it SHALL compile, because `SkiaControl` declares `Tapped` and `SkiaShape` inherits it

#### Scenario: A payload carries the callback's primitive arguments
- **WHEN** a snippet binds `onToggled=<State.Update on={action.value} />` on a `SkiaSwitch`
- **THEN** it SHALL compile, with `action.value` typed `boolean`

#### Scenario: Events leave the omitted list
- **WHEN** the catalog is regenerated
- **THEN** the omitted list SHALL contain no event callback and no templated control's
  `ItemsSource` or `ItemTemplate`
- **AND** it SHALL still list function-typed members that are neither, such as `ProcessJson`, with
  their TypeScript types

#### Scenario: An unknown event is a compile error in the snippet's lines
- **WHEN** a snippet binds `onNoSuchEvent` on a control
- **THEN** the run SHALL fail with an error listed as `L{line}: {message}` in the snippet's own
  line numbers
