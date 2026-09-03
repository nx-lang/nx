# drawnui-fiddle Specification

## Purpose
A browser-based fiddle where a visitor edits NX source describing a DrawnUI interface and sees it
drawn live beside the editor, demonstrating the NX compile-to-IR-to-render pipeline end to end.

## Requirements

### Requirement: Gallery presents the DrawnUI example set
The application SHALL open on a gallery that mirrors the top-level structure of the DrawnUI React
demo site, listing its examples so that a visitor can see what the control set can do before writing
any NX.

#### Scenario: Gallery lists the examples
- **WHEN** a visitor opens the application
- **THEN** the gallery SHALL list an entry for each example carried over from the DrawnUI demo site
- **AND** each entry SHALL be identified by the same name the demo site gives it

#### Scenario: Each entry draws its example
- **WHEN** a visitor views a gallery entry
- **THEN** that entry's example SHALL be drawn from its NX source through the same pipeline the
  fiddle uses
- **AND** the drawing SHALL NOT require the visitor to open the fiddle first

### Requirement: Every gallery entry opens in the fiddle
Each gallery entry SHALL offer an affordance that opens the fiddle with that entry's NX loaded, and
the fiddle view for an entry SHALL be reachable by its own address so it can be linked to directly.

#### Scenario: Opening an example in the fiddle
- **WHEN** a visitor activates a gallery entry's fiddle affordance
- **THEN** the fiddle SHALL open with that entry's NX in the source pane
- **AND** the output pane SHALL draw that example

#### Scenario: A fiddle view is linkable
- **WHEN** a visitor navigates directly to an example's fiddle address
- **THEN** the fiddle SHALL open with that example loaded, without passing through the gallery

#### Scenario: Editing an example does not alter the gallery
- **WHEN** a visitor edits an example in the fiddle and returns to the gallery
- **THEN** the gallery entry SHALL still show the example as originally authored

#### Scenario: Returning to the gallery
- **WHEN** a visitor is in the fiddle view
- **THEN** there SHALL be a way back to the gallery

### Requirement: Examples declare how completely they cover the original
Each example SHALL declare its coverage as one of three states, so that a gap in NX's expressiveness
is never mistaken for a broken example, and so that an example whose drawing is correct is not
presented as if it were faulty:

- **complete** — nothing the DrawnUI original does is missing;
- **static** — the drawing is correct and complete, but motion or interaction the original has is
  absent;
- **reduced** — the example is scaled down because NX cannot express the mechanism the original
  demonstrates.

#### Scenario: A complete example carries no coverage note
- **WHEN** an example's NX covers everything the DrawnUI original does
- **THEN** it SHALL NOT carry a coverage note or badge

#### Scenario: A static example is distinguished from a reduced one
- **WHEN** an example draws the original correctly but omits its motion or interaction
- **THEN** it SHALL declare itself static rather than reduced

#### Scenario: A reduced example states what the original demonstrates
- **WHEN** an example is scaled down from the original
- **THEN** it SHALL declare itself reduced
- **AND** what the original demonstrates SHALL be stated where the visitor can read it

#### Scenario: Coverage marking is proportionate
- **WHEN** a static or reduced example is presented
- **THEN** it SHALL NOT be presented as an error or failure

#### Scenario: Omitted examples are accounted for
- **WHEN** a DrawnUI demo page has no NX example at all
- **THEN** the application's documentation SHALL name it and say why
- **AND** the gallery SHALL NOT show an entry for it

### Requirement: Missing capabilities are named from a shared vocabulary
An example that is not complete SHALL attribute its gap to one or more named NX capabilities drawn
from a fixed vocabulary shared across all examples, rather than to prose written per example, so
that gaps can be counted, compared, and found again when a capability lands.

#### Scenario: A gap names a capability
- **WHEN** an example declares itself static or reduced
- **THEN** it SHALL name at least one capability from the shared vocabulary as the reason

#### Scenario: Coverage notes derive from the named capabilities
- **WHEN** an example's coverage is shown to a visitor
- **THEN** the wording SHALL derive from the capabilities it names
- **AND** two examples blocked by the same capability SHALL describe it the same way

#### Scenario: Gaps can be surveyed across the example set
- **WHEN** the example set is inspected
- **THEN** it SHALL be possible to determine which examples are blocked by any given capability

### Requirement: Example source marks where dropped behavior belonged
Where an example omits behavior the DrawnUI original has, its NX source SHALL say so at the point
the behavior would have appeared, so that a visitor who opens the fiddle to find out why nothing
happens reads the answer in the code.

#### Scenario: The source explains an omission in place
- **WHEN** a visitor opens a static or reduced example in the fiddle
- **THEN** the source SHALL carry a note at the point the omitted behavior would have been written
- **AND** that note SHALL name the same capability the example's coverage names

### Requirement: Every gallery entry is backed by working NX
Every gallery entry SHALL have NX source that compiles and draws, and its fiddle affordance SHALL
open that source. No entry SHALL be a placeholder standing in for an example that does not exist.

#### Scenario: A reduced example still opens in the fiddle
- **WHEN** a visitor opens a reduced example's fiddle affordance
- **THEN** the fiddle SHALL open with that example's NX
- **AND** the output pane SHALL draw it

#### Scenario: No entry lacks source
- **WHEN** the gallery is inspected
- **THEN** every entry SHALL resolve to NX source that compiles

### Requirement: A static example rests in a sensible state
Where an example omits interaction that would otherwise move a control into a particular position,
the example SHALL be authored so that the state it rests in is a deliberate one.

#### Scenario: A frozen control does not read as broken
- **WHEN** an example draws a control whose original was driven by interaction, such as a carousel or
  a drawer
- **THEN** the control SHALL be drawn at a resting position that looks intentional rather than
  mid-transition

### Requirement: Example NX is authored against the catalog
Every example SHALL be NX source that compiles through the application's own pipeline, rather than a
hand-built value tree or a drawing produced some other way.

#### Scenario: Examples compile
- **WHEN** the application's examples are checked
- **THEN** every example SHALL compile with no diagnostics

#### Scenario: An example is exactly what the fiddle loads
- **WHEN** a visitor opens an example in the fiddle
- **THEN** the source shown SHALL be the same source the gallery drew

### Requirement: Two-pane fiddle renders authored NX
The application SHALL present a source pane and an output pane, and SHALL draw the interface
described by the source pane's NX into the output pane.

#### Scenario: The fiddle always opens on working source
- **WHEN** a visitor opens the fiddle, whether from a gallery entry or directly
- **THEN** the source pane SHALL contain NX using catalog controls
- **AND** the output pane SHALL draw it without any visitor action

#### Scenario: Edits update the drawing
- **WHEN** a visitor edits the source pane so that it still compiles
- **THEN** the output pane SHALL be redrawn from the edited source

#### Scenario: Updates are not issued per keystroke
- **WHEN** a visitor types continuously
- **THEN** the application SHALL coalesce the edits and compile once the visitor pauses, rather than
  compiling on every keystroke

#### Scenario: Last good drawing survives a broken edit
- **WHEN** an edit makes the source fail to compile
- **THEN** the output pane SHALL continue to show the last successfully drawn interface
- **AND** the failure SHALL be reported as a diagnostic rather than by clearing the output

### Requirement: Source pane is an NX-aware editor
The source pane SHALL highlight NX using the syntax definition the repository already publishes for
editors, so that highlighting in the fiddle and in the editor extension cannot drift apart.

#### Scenario: NX is syntax highlighted
- **WHEN** the source pane contains NX
- **THEN** it SHALL be highlighted according to the repository's published NX grammar

#### Scenario: Grammar is not duplicated
- **WHEN** the repository's published NX grammar changes
- **THEN** the fiddle SHALL pick up the change without a separate grammar being edited

### Requirement: Compilation reports diagnostics against authored source
The application SHALL compile the visitor's NX and report every resulting diagnostic, positioned
against the source the visitor actually wrote.

#### Scenario: Diagnostics are shown in the source pane
- **WHEN** compilation reports a diagnostic with a source position
- **THEN** the source pane SHALL mark the reported span
- **AND** the diagnostic message SHALL be readable by the visitor

#### Scenario: Positions are not shifted by catalog injection
- **WHEN** the application compiles the visitor's source together with the control catalog
- **THEN** a diagnostic on the visitor's source SHALL report the line and column the visitor sees in
  the source pane

#### Scenario: Catalog-internal failures are not blamed on the visitor
- **WHEN** a diagnostic's position falls inside the injected catalog rather than the visitor's source
- **THEN** the application SHALL report it as an application fault
- **AND** it SHALL NOT mark a span in the visitor's source

#### Scenario: A position with no width is still a position
- **WHEN** a diagnostic names a point rather than a range, as a missing token's insertion point does
- **THEN** the application SHALL report it against the visitor's source rather than as a fault with
  no position
- **AND** the source pane SHALL mark it visibly

#### Scenario: Compilation errors do not break the session
- **WHEN** compilation fails for any reason, including a transport failure
- **THEN** the application SHALL report the failure and remain editable

### Requirement: Catalog is available to authored source without being shown
The visitor SHALL be able to use catalog controls without declaring or importing them, and the
catalog SHALL NOT appear in the source pane.

How the catalog is injected SHALL NOT constrain the shape of the visitor's file. Any source the
language accepts as a whole file SHALL compile in the fiddle, and SHALL do so with the diagnostics
and positions the visitor would see compiling that same text on its own.

#### Scenario: Controls are used without an import
- **WHEN** a visitor writes an element naming a catalog control
- **THEN** compilation SHALL resolve it with no import statement present

#### Scenario: Catalog is not editable
- **WHEN** a visitor inspects the source pane
- **THEN** the catalog declarations SHALL NOT be present in it

#### Scenario: A file that is a single trailing element compiles
- **WHEN** a visitor's source declares no `root` and consists of one element expression, such as
  `<SkiaLayer VerticalOptions=Fill></SkiaLayer>`
- **THEN** compilation SHALL accept it and return IR
- **AND** it SHALL NOT report a syntax error caused by the injected catalog

### Requirement: Evaluated NX values are translated to drawn controls
The application SHALL evaluate compiled NX to a value tree and translate that tree into DrawnUI
controls, mapping each element to the control its type names and each property to that control's
corresponding input.

#### Scenario: Element types select controls
- **WHEN** the evaluated tree contains an element naming a catalog control
- **THEN** the application SHALL instantiate the corresponding DrawnUI control

#### Scenario: Nested content is drawn as child controls
- **WHEN** an element carries content, whether a single child or several
- **THEN** all of that content SHALL be drawn as children of the containing control in authored order

#### Scenario: Union values are passed as their case name
- **WHEN** a property's evaluated value is a union case
- **THEN** the control SHALL receive the case name as its value

#### Scenario: Record values are reconstructed
- **WHEN** a property's evaluated value is a record such as a thickness or corner radius
- **THEN** the application SHALL reconstruct the value in the form DrawnUI expects, rather than
  passing the raw record

#### Scenario: Unset properties keep DrawnUI defaults
- **WHEN** an evaluated property carries no value
- **THEN** the application SHALL leave the control's own default in place

#### Scenario: Unknown elements are reported, not drawn
- **WHEN** the evaluated tree contains an element that names no known control
- **THEN** the application SHALL report the unknown element to the visitor
- **AND** it SHALL NOT abort drawing the rest of the tree

### Requirement: Interaction is limited to DrawnUI's own behavior
Authored NX SHALL describe appearance and structure only. The application SHALL NOT invoke authored
NX in response to visitor interaction with the drawing.

#### Scenario: Built-in gestures still work
- **WHEN** a visitor interacts with a drawn control that DrawnUI handles on its own, such as
  scrolling a scrollable region
- **THEN** the control SHALL respond as DrawnUI does

#### Scenario: Event properties are unavailable
- **WHEN** a visitor tries to attach behavior to a control event from NX
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property
- **AND** the application's documentation SHALL state that authored interaction is not yet supported

### Requirement: Compilation runs on the server behind a replaceable boundary
The application SHALL compile NX on the server, and SHALL confine the choice of where compilation
happens to a single seam so that moving compilation into the browser later does not disturb editing
or drawing.

#### Scenario: The browser holds no compiler
- **WHEN** the application is loaded
- **THEN** the browser SHALL obtain NX IR by request rather than by compiling NX itself

#### Scenario: The seam is uniform
- **WHEN** compilation is requested
- **THEN** it SHALL be requested through one interface that takes NX source and yields IR and
  diagnostics
- **AND** editing and drawing SHALL depend on that interface rather than on how it is fulfilled

### Requirement: Application is deployable as a single service
The application SHALL be deployable as one service that serves both the client application and
compilation, and its build SHALL be reproducible from the repository.

#### Scenario: One service serves everything
- **WHEN** the application is deployed
- **THEN** a single service SHALL serve the client application and answer compile requests

#### Scenario: A request the service cannot understand does not end it
- **WHEN** a request names a path the URL decoder rejects, or fails anywhere outside a handler's own
  error handling
- **THEN** the service SHALL answer that request with an error status
- **AND** it SHALL still answer the requests that follow

#### Scenario: Build produces its own native dependencies
- **WHEN** the deployment image is built from a clean checkout
- **THEN** the build SHALL produce every native artifact compilation requires
- **AND** it SHALL NOT depend on artifacts built outside the image

### Requirement: Sample app documents how to run and sync it
The application SHALL carry documentation covering how to build and run it locally, what it depends
on, and how to refresh its vendored DrawnUI copy.

#### Scenario: Local run is documented
- **WHEN** a contributor reads the application's documentation
- **THEN** it SHALL describe the prerequisites and the steps to run the application locally

#### Scenario: Vendored source provenance is recorded
- **WHEN** a contributor inspects the vendored DrawnUI copy
- **THEN** the upstream revision it was taken from SHALL be recorded
