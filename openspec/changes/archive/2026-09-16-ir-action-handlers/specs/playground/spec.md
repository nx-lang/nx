## ADDED Requirements

### Requirement: Authored handlers run in the output pane
The site SHALL draw every use of an authored component as a component instance held by the
renderer, SHALL turn each handler an author binds on a drawn control into that control's DrawnUI
event, and on the event SHALL dispatch the handler against the instance whose body bound it and
redraw from the result. Results SHALL be routed as the language defines them: an update record
patches the state of the instance that owns the handler; an action a component emits is delivered
to the handler its parent bound for that emit, through the token the parent's rendered output
carries, and the parent's state is patched in turn; anything else is a host effect. Every dispatch
SHALL be atomic with respect to the instance it targets, and a dispatch that fails SHALL leave the
drawing as it was.

#### Scenario: A tap patches state and redraws
- **WHEN** a visitor's source declares `component <Counter /> = { state { count:int = 0 } <SkiaStack><SkiaLabel Text={if count > 0 { "tapped" } else { "untapped" }} /><SkiaButton Text="Tap" onTapped=<Update count={count + 1} /> /></SkiaStack> }` and a root of `<Counter />`
- **AND** the visitor taps the drawn button
- **THEN** the label SHALL redraw as `tapped`
- **AND** no compile SHALL be issued

#### Scenario: Live state across taps
- **WHEN** the button above is tapped twice
- **THEN** the second tap's handler SHALL read the state the first tap left, so `count` is `2`

#### Scenario: An emitted action reaches the parent's handler
- **WHEN** a child component declares `emits { Chosen { name:string } }` and its body binds `onTapped=<Child.Chosen name="a" />` on a button
- **AND** a parent component with state `picked:string = ""` uses it as `<Child onChosen=<Update picked={action.name} /> />` and draws `picked` in a label
- **AND** the visitor taps the child's button
- **THEN** the parent's label SHALL redraw as `a`

#### Scenario: A handler in a content child patches its owner
- **WHEN** a parent component's body places a button with `onTapped=<Update ... />` inside the content of an authored component
- **AND** the visitor taps that button
- **THEN** the parent's state SHALL be patched and its drawing updated
- **AND** the content component's instance SHALL keep its own state

#### Scenario: Child state survives a parent redraw
- **WHEN** an authored child with its own state is drawn inside a parent whose state changes
- **THEN** after the parent redraws, the child SHALL be drawn from the parent's new props and the
  state the child held before

#### Scenario: A handler outside a component is inert and reported
- **WHEN** a visitor's source binds `onTapped` on a control in the root function rather than inside a
  component
- **THEN** the control SHALL draw without a callback
- **AND** the site SHALL report, as it reports an unknown control, that handlers run inside a
  component

#### Scenario: A failed dispatch leaves the drawing
- **WHEN** a handler's dispatch fails with a runtime diagnostic
- **THEN** the site SHALL show the diagnostic's message in the diagnostics pane
- **AND** the drawing SHALL be the one from before the event

#### Scenario: Editing resets the instances
- **WHEN** the visitor edits the source and it recompiles
- **THEN** every instance SHALL start again from its initial state

### Requirement: Effects the tree does not handle are shown to the visitor
An action a handler returns that no instance in the tree handles — an emitted action nobody bound,
or an action outside the component's contract, such as `<DoSearch />` from a page component — SHALL
be shown to the visitor as a host effect, naming the action and the instance that produced it, so
that a visitor can see an action leave the tree even though the site has no host to receive it.

#### Scenario: An unbound emit is listed
- **WHEN** a component declares `emits { Saved }`, its body returns `<Saved />` from a handler, and
  its use binds no `onSaved`
- **AND** the handler is dispatched
- **THEN** the diagnostics pane SHALL list a `Saved` effect from that component

#### Scenario: Effects are cleared on recompile
- **WHEN** the visitor edits the source and it recompiles
- **THEN** the listed effects SHALL be cleared

## MODIFIED Requirements

### Requirement: Evaluated NX values are translated to drawn controls
The site SHALL evaluate compiled NX to a value tree and translate that tree into DrawnUI controls,
mapping each element to the control its type names, each property to that control's corresponding
input, and each handler property to that control's corresponding event.

#### Scenario: Element types select controls
- **WHEN** the evaluated tree contains an element naming a catalog control
- **THEN** the site SHALL instantiate the corresponding DrawnUI control

#### Scenario: Nested content is drawn as child controls
- **WHEN** an element carries content, whether a single child or several
- **THEN** all of that content SHALL be drawn as children of the containing control in authored order

#### Scenario: Union values are passed as their case name
- **WHEN** a property's evaluated value is a union case
- **THEN** the control SHALL receive the case name as its value

#### Scenario: Record values are reconstructed
- **WHEN** a property's evaluated value is a record such as a thickness or corner radius
- **THEN** the site SHALL reconstruct the value in the form DrawnUI expects, rather than passing the
  raw record

#### Scenario: Handler properties become event callbacks
- **WHEN** a drawn control carries a handler property `on<Event>` whose value is a handler record
  with a token
- **THEN** the control SHALL receive a callback under DrawnUI's event name `<Event>`
- **AND** the callback SHALL build the emit's action record from the event's arguments, by the
  parameter names the catalog metadata records, and dispatch it under the token

#### Scenario: Authored components are drawn as instances
- **WHEN** the evaluated tree contains a descriptor of a component the author declared
- **THEN** the site SHALL initialize an instance from the descriptor's fields, with the enclosing
  instance as its parent when there is one
- **AND** it SHALL draw what the instance rendered in the descriptor's place

#### Scenario: Unset properties keep DrawnUI defaults
- **WHEN** an evaluated property carries no value
- **THEN** the site SHALL leave the control's own default in place

#### Scenario: Unknown elements are reported, not drawn
- **WHEN** the evaluated tree contains an element that names no known control
- **THEN** the site SHALL report the unknown element to the visitor
- **AND** it SHALL NOT abort drawing the rest of the tree

### Requirement: Missing capabilities are named from a shared vocabulary
An example that is not complete SHALL attribute its gap to one or more named capabilities drawn
from a fixed vocabulary shared across all examples, rather than to prose written per example, so
that gaps can be counted, compared, and found again when a capability lands. The vocabulary SHALL
distinguish a capability NX does not have from one NX has and the port does not use yet, so that
a landed capability is not presented as missing from NX.

#### Scenario: A gap names a capability
- **WHEN** an example declares itself static or reduced
- **THEN** it SHALL name at least one capability from the shared vocabulary as the reason

#### Scenario: Coverage notes derive from the named capabilities
- **WHEN** an example's coverage is shown to a visitor
- **THEN** the wording SHALL derive from the capabilities it names
- **AND** two examples blocked by the same capability SHALL describe it the same way

#### Scenario: A landed capability reads as a porting gap
- **WHEN** an example names event handlers or component state as its gap
- **THEN** the wording SHALL say the port does not use them yet
- **AND** it SHALL NOT say NX lacks them

#### Scenario: Gaps can be surveyed across the example set
- **WHEN** the example set is inspected
- **THEN** it SHALL be possible to determine which examples are blocked by any given capability

### Requirement: Example NX is authored against the catalog
Every example SHALL be NX source that compiles through the site's own pipeline, rather than a
hand-built value tree or a drawing produced some other way, and the example check SHALL expand
every authored component in every example the way the renderer does.

#### Scenario: Examples compile
- **WHEN** the site's examples are checked
- **THEN** every example SHALL compile with no diagnostics

#### Scenario: Examples are expanded as the renderer draws them
- **WHEN** the site's examples are checked
- **THEN** every authored component use in every example SHALL be initialized under the instance
  that encloses it, handler properties resolved through the parent
- **AND** a failure inside any component body SHALL fail the check

#### Scenario: An example is exactly what the editor loads
- **WHEN** a visitor opens an example in the editor view
- **THEN** the source shown SHALL be the same source the gallery drew

## REMOVED Requirements

### Requirement: Interaction is limited to DrawnUI's own behavior
**Reason**: The site now invokes authored NX in response to visitor interaction; the requirement's
statement that it never does, and the scenario that event properties fail to compile, are
inverted by "Authored handlers run in the output pane" and by the catalog's emits. DrawnUI's own
gestures still work and are covered by the first scenario of the new requirement's premise.
**Migration**: The editor's notice and the README paragraph that said authored interaction is
unsupported are replaced by a description of what runs where.
