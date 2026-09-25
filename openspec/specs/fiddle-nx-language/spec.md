# fiddle-nx-language Specification

## Purpose

NX as a language of the DrawnUI fiddle engine: a visitor picks NX, edits a snippet, sees it drawn
with DrawnUI in the browser, shares it, and embeds it through the player, with the DrawnUI catalog
kept in the fiddle repository and derived from the DrawnUI engine the fiddle ships.

## Requirements

### Requirement: NX is a built-in language of the fiddle engine
The fiddle engine SHALL ship NX as a language with the stable id `nx`, on its React surface, for a
host to register next to C# and TSX. Switching to NX SHALL load NX's starter preset when the buffer
does not hold NX, and SHALL leave the buffer alone when it does.

#### Scenario: NX appears in the language switch
- **WHEN** a visitor opens the editor of a host that has registered NX
- **THEN** the language switch SHALL offer NX after C# and React, and the `fiddle.listPresets()` API
  SHALL list NX presets with `lang` equal to `nx`

#### Scenario: Switching to NX from foreign code loads the starter
- **WHEN** the buffer holds C# or TSX and the visitor switches to NX
- **THEN** the buffer SHALL be replaced by NX's first preset and drawn

#### Scenario: Switching to NX keeps NX
- **WHEN** the buffer holds text that declares a `root` function and the visitor switches to NX
- **THEN** the buffer SHALL be kept as it is

#### Scenario: Presets draw
- **WHEN** any NX preset is selected
- **THEN** it SHALL compile without errors and draw, using only fonts and assets the fiddle host
  serves or fetches by absolute URL

### Requirement: Snippets compile and draw in the browser
Running an NX snippet SHALL compile the snippet together with the DrawnUI catalog through the NX
WebAssembly compiler in the visitor's tab, evaluate the program's `root` function with the NX IR
runtime, and draw the resulting value tree with DrawnUi.React on the engine's React canvas. No
server SHALL take part in compilation or drawing.

#### Scenario: A valid snippet draws
- **WHEN** the visitor runs a snippet whose `root` function returns DrawnUI controls
- **THEN** the canvas SHALL show those controls with the properties the snippet set, and
  `fiddle.getState()` SHALL report success with no errors

#### Scenario: Union cases and records reach DrawnUI in its own shapes
- **WHEN** a snippet sets a property to a union case such as `Center`, or to a record such as
  `<Thickness Left=4.0 />`
- **THEN** the control SHALL receive the bare case name, and the record SHALL arrive as the DrawnUI
  value type of that name, so that DrawnUI's own type checks accept it

#### Scenario: Unset properties keep DrawnUI's defaults
- **WHEN** a snippet leaves a property unset, or binds it to a value that evaluates to the empty
  value `{}` — an `if` with no `else` that took no branch, an optional prop that was not written
- **THEN** the control SHALL be created without that property, so the DrawnUI default applies
- **AND** the renderer SHALL NOT pass `null`, `undefined` or an empty array in its place

#### Scenario: A compile error is reported in the engine's form
- **WHEN** the snippet does not compile
- **THEN** the run SHALL fail, nothing new SHALL be drawn, and each error SHALL be listed under the
  editor as `L{line}: {message}` in the snippet's own line numbers, with the catalog's lines never
  blamed

#### Scenario: An unknown control is drawn as a placeholder
- **WHEN** an evaluated value carries a type the catalog does not declare and the snippet does not
  define
- **THEN** the rest of the tree SHALL still draw, and a visible placeholder naming the type SHALL
  stand in its place

#### Scenario: An authored component draws in its place
- **WHEN** the snippet declares its own component and uses it in `root`
- **THEN** what the component renders SHALL be drawn where the component was used

#### Scenario: A compiler crash costs one run
- **WHEN** the compiler traps during a run
- **THEN** that run SHALL fail with a readable message, and the next run SHALL compile normally
  without reloading the page

### Requirement: The NX runtime loads on first use only
The NX compiler module and runtime bundle SHALL be fetched the first time a visitor runs or plays an
NX snippet, and never for a visitor who uses C# or TSX alone. Once loaded they SHALL be reused for
every later run in that page.

#### Scenario: C# visitors do not pay for NX
- **WHEN** a visitor opens the editor and runs only C# or TSX snippets
- **THEN** no request for the NX module or the NX runtime bundle SHALL be made

#### Scenario: The runtime loads once
- **WHEN** a visitor runs NX snippets repeatedly
- **THEN** the NX module and runtime bundle SHALL be requested once

### Requirement: The DrawnUI catalog is derived from the package the fiddle ships
The fiddle repository SHALL generate the NX catalog of DrawnUI controls from the `drawnui-react`
package version the fiddle pins, by resolving each renderable tag's author-settable properties
through the package's type declarations, and SHALL commit the generated catalog together with the
metadata the renderer needs (which types are unions, which records construct DrawnUI value types,
and which property carries children). The catalog SHALL record the package version it was generated
from.

#### Scenario: Every renderable tag is authorable
- **WHEN** the package exports a React tag for a DrawnUI control
- **THEN** the catalog SHALL declare an external component with the same name, and inherited
  properties SHALL be declared once on abstract bases

#### Scenario: Regeneration is reproducible and reviewable
- **WHEN** the catalog is generated twice from the same package version
- **THEN** the output SHALL be identical, and a generation after a version bump SHALL show its
  differences as an ordinary diff

#### Scenario: A stale catalog is detected
- **WHEN** the pinned package version differs from the version the committed catalog records
- **THEN** the runtime build SHALL fail with a message naming both versions and the regeneration
  command

### Requirement: The editor reports NX diagnostics and answers NX queries
While NX is selected, the editor SHALL highlight NX with the published NX grammar, SHALL mark
diagnostics from the NX language service as squiggles in the snippet's own coordinates, and SHALL
answer hover and completion over the snippet and the catalog. Diagnostics SHALL feed the engine's
run gating, so that the engine's automatic re-run happens only when the snippet has no errors.

#### Scenario: Errors are marked as the visitor types
- **WHEN** the visitor introduces an error and pauses
- **THEN** a marker SHALL appear at the error's range, and the automatic re-run SHALL not fire until
  the error is gone

#### Scenario: Completion offers catalog components
- **WHEN** the visitor requests completion after `<` in an NX snippet
- **THEN** the list SHALL include the DrawnUI components the catalog declares

#### Scenario: Hover describes a catalog property
- **WHEN** the visitor hovers a property of a DrawnUI control
- **THEN** the hover SHALL describe that property as the catalog declares it

#### Scenario: Color literals show swatches
- **WHEN** the snippet contains a `#RRGGBB` or `#RRGGBBAA` string literal
- **THEN** the editor SHALL show a color swatch for it, as it does for C# and TSX color literals

### Requirement: An NX share stores its program and plays without the compiler
Sharing an NX snippet SHALL store the snippet's source with language `nx` and, as the share
artifact, a module carrying the snippet's own NX IR artifact and nothing else: no catalog
declaration, no runtime, no debug section. That artifact SHALL name the catalog and the catalog
version it was compiled against. The `/p/` player SHALL prepare the bundled catalog, link the share's
artifact against it, and draw with the NX runtime bundle and the DrawnUI engine alone, never
fetching the compiler module. Every NX preset's share module SHALL be under 128 KB, half the
backend's limit.

#### Scenario: A share round-trips in NX
- **WHEN** a visitor shares an NX snippet and another visitor opens the share's editor link
- **THEN** the editor SHALL open in NX with the same source

#### Scenario: The player draws without the compiler
- **WHEN** the player opens an NX share
- **THEN** it SHALL draw the same output the editor showed, SHALL post the engine's `ready` message
  after the first drawn frame, and SHALL make no request for the compiler module

#### Scenario: A share names its catalog version
- **WHEN** a visitor shares a snippet compiled against catalog version `9`
- **THEN** the stored module's artifact SHALL list the catalog with version `9` in its module table

#### Scenario: A share survives a regenerated catalog
- **WHEN** a share was compiled against a catalog in which `SkiaLabel` was the tenth component
- **AND** the host's bundle now carries a regenerated catalog of the same version in which a new
  component precedes `SkiaLabel`
- **THEN** the player SHALL draw the share unchanged

#### Scenario: A share from another catalog version is reported
- **WHEN** a share names catalog version `9` and the bundle carries version `10`
- **THEN** the player SHALL attempt to link by name and draw when every referenced declaration exists
- **AND** it SHALL draw the engine's failure label naming both versions when one does not

#### Scenario: Presets fit the budget
- **WHEN** the runtime's tests compile every NX preset
- **THEN** each share module SHALL be under 128 KB

#### Scenario: Screenshots work for NX
- **WHEN** a thumbnail or frame is taken of a running NX snippet
- **THEN** it SHALL show the drawn output, as it does for TSX

#### Scenario: A host without NX shows the source
- **WHEN** an NX share is opened on a host that has not registered NX
- **THEN** the host SHALL show the source and say the language is unknown rather than failing

### Requirement: The engine lets a language prepare its runtime
The engine's React surface SHALL let a registered language declare a preparation step, and SHALL
await it before the language's first compile in the editor and before the player runs a share in
that language. The engine SHALL take color-swatch patterns from the language registration rather
than from a fixed table.

#### Scenario: Preparation precedes the first compile
- **WHEN** a language registered with a preparation step is run for the first time
- **THEN** the engine SHALL await the preparation before calling the language's compile

#### Scenario: Preparation precedes play
- **WHEN** the player opens a share whose language has a preparation step
- **THEN** the engine SHALL await the preparation before evaluating the share's module

#### Scenario: Preparation failure is a run failure
- **WHEN** the preparation rejects
- **THEN** the run SHALL fail with the rejection's message under the editor, and the next run SHALL
  attempt preparation again

#### Scenario: Existing languages are unaffected
- **WHEN** TSX, which declares no preparation, is run or played
- **THEN** its behavior SHALL be unchanged

### Requirement: The NX runtime bundle is built from pinned packages
The fiddle's runtime build SHALL produce the NX runtime bundle and the compiler module for a host
from the `@nx-lang/*` package versions the fiddle pins, in the same step that builds the DrawnUi.React
runtime, and SHALL be able to target another host's web root the way the React runtime build can.

#### Scenario: One command builds both runtimes
- **WHEN** a maintainer runs the runtime build
- **THEN** the host's web root SHALL hold the DrawnUi.React runtime, the NX runtime bundle and the NX
  compiler module, each from its pinned package version

#### Scenario: Another host is targeted
- **WHEN** the runtime build is given another host's output directory
- **THEN** both runtimes SHALL be written there and nowhere else

### Requirement: The catalog artifact ships with the NX runtime and is prepared once
The NX runtime bundle SHALL contain the DrawnUI catalog compiled to its own NX IR artifact, stamped
with the catalog version recorded when the catalog was generated, and SHALL prepare it once per page
the first time an NX snippet is compiled or played. Every compile of a snippet SHALL build it against
the catalog as a separate module, and every mount SHALL link the snippet's artifact against the
prepared catalog. The runtime build SHALL fail when the bundled catalog artifact does not match the
committed catalog source.

#### Scenario: The catalog is prepared once per page
- **WHEN** a visitor runs three NX snippets in one editor session
- **THEN** the catalog artifact SHALL be prepared once
- **AND** each snippet SHALL be linked against that preparation

#### Scenario: The player never receives the catalog in a share
- **WHEN** the player opens an NX share
- **THEN** every catalog declaration it uses SHALL come from the runtime bundle
- **AND** the share's module SHALL contain none of them

#### Scenario: A stale catalog artifact fails the build
- **WHEN** the committed catalog source has changed since the bundled artifact was emitted
- **THEN** `npm run runtime` SHALL fail naming the catalog and the command that regenerates it

### Requirement: The host registers NX
NX SHALL be registered by a host that chooses to offer it, after the engine's own registration,
rather than by the engine's default language set. The engine SHALL still ship NX's language class,
presets, script and runtime build so that a host adds NX with one registration line and a script
tag.

#### Scenario: The dev host offers NX
- **WHEN** a visitor opens the dev host's editor
- **THEN** the language switch SHALL offer NX after C# and React

#### Scenario: A host that does not register NX does not offer it
- **WHEN** a host calls only the engine's default registration
- **THEN** the language switch SHALL NOT offer NX
- **AND** an NX share opened there SHALL show its source and say the language is unknown

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
- **WHEN** a snippet binds `onToggled=<Update on={action.value} />` on a `SkiaSwitch`
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

### Requirement: The fiddle's catalog declares templated controls
The catalog the fiddle generates SHALL declare, for each class that carries an item collection and
a cell factory, a type parameter `TItem`, the property `ItemsSource?: TItem+` and the property
`ItemTemplate?: <function Item:TItem Index:int />: DrawnNode` — an optional function-typed property
marked on the name, which needs no parentheses — on the component standing for the class that
declares them and inherited by every control below. Every other author-settable property SHALL
carry its optionality on the name the same way, `x?: T` for a scalar and `x?: T+` for a DrawnUI
array, never `?` or `*` in the type slot. The generated metadata SHALL record, for each renderable
control carrying `ItemTemplate`, the parameter names the template is called with, in order, so the
renderer can bind the item and its index. Neither property SHALL appear in the omitted list.

#### Scenario: A layout binds a collection and a template
- **WHEN** the catalog is regenerated from the pinned `drawnui-react` package
- **THEN** `SkiaLayout` SHALL declare `TItem:type`, `ItemsSource?:TItem+` and
  `ItemTemplate?:<function Item:TItem Index:int />: DrawnNode` on the class that declares them
- **AND** a snippet binding `TItem=Contact ItemsSource={contacts} ItemTemplate={ContactCell}
  RecyclingTemplate=Enabled` SHALL compile when `ContactCell` is
  `let <ContactCell Item:Contact Index:int />: DrawnNode = …` and `contacts` is a `Contact+`

#### Scenario: A template whose item type disagrees is rejected
- **WHEN** a snippet binds `TItem=Contact` and an `ItemTemplate` whose `Item` parameter is another
  record type
- **THEN** the run SHALL fail with an error in the snippet's own line numbers

#### Scenario: Children are one or more controls, optional on the name
- **WHEN** the catalog is regenerated
- **THEN** every container's content property SHALL be spelled `content Children?: DrawnNode+`
- **AND** a snippet that nests one control in a container SHALL compile and draw that child

#### Scenario: The template's parameter names are recorded
- **WHEN** the generated metadata is inspected
- **THEN** each renderable control carrying `ItemTemplate` SHALL record the template's parameter
  names in order, and the property that carries the collection

### Requirement: A templated control draws its cells
A control bound to an item collection and a template SHALL draw one cell per item, the template
being called with the item and its index at the moment DrawnUI binds a cell, so that only the cells
DrawnUI realizes are ever built and recycling and measuring behave as they do for a control the
engine's own templates drive. A template that fails for one item SHALL leave the rest of the
drawing standing and SHALL be reported in the host's console naming the template and the index. A
templated control drawn inside a cell SHALL template its own cells the same way.

#### Scenario: Only realized cells are built
- **WHEN** a snippet binds a collection far larger than the visible area through `ItemsSource` with
  an `ItemTemplate`
- **THEN** the drawing SHALL show the cells for the visible items
- **AND** the template SHALL be called once per cell DrawnUI binds, not once per item in the
  collection

#### Scenario: A cell is bound to its item and index
- **WHEN** DrawnUI binds a cell to an item
- **THEN** the template SHALL be called with that item and its index, and the cell's content SHALL
  be what the template returned

#### Scenario: A failing template is reported, not fatal
- **WHEN** a template call fails for one item
- **THEN** the rest of the drawing SHALL stand
- **AND** the console SHALL carry one line naming the template and the index that failed

#### Scenario: A nested templated control still templates
- **WHEN** a cell's own content binds `ItemsSource` and `ItemTemplate`
- **THEN** that inner control SHALL draw its cells through its template rather than losing it

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

### Requirement: A preset draws a virtualized list
The NX presets SHALL include at least one that binds a collection of the size the DrawnUI original
demonstrates through `ItemsSource`, with a cell as an element function through `ItemTemplate` and
the original's recycling and measuring settings. Its collection SHALL be built from a range rather
than from a written-out list or a helper that concatenates shifted copies of one, and neither its
source nor its description SHALL say NX cannot express a collection, a template or a count.

#### Scenario: The list preset draws and scrolls
- **WHEN** the virtualized-list preset is drawn
- **THEN** it SHALL show the cells for the visible items, and scrolling SHALL show later ones

#### Scenario: The collection is one loop over a range
- **WHEN** the preset's source is read
- **THEN** its collection SHALL be one `for` over a range whose bound is the item count the
  original demonstrates

#### Scenario: The list preset fits the share budget
- **WHEN** the runtime's tests compile the presets
- **THEN** the virtualized-list preset's share module SHALL be under 128 KB, as every preset's is

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
