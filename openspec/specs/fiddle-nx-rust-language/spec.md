# fiddle-nx-rust-language Specification

## Purpose
NX drawn with DrawnUI for Rust as a language of the DrawnUI fiddle engine: a visitor picks it, edits
the same NX they would write for the fiddle's `nx` language, and sees it drawn by a prebuilt DrawnUI
for Rust player that takes each newly compiled program without a server build.

## Requirements

### Requirement: NX on DrawnUI for Rust is a language of the fiddle engine
The fiddle engine SHALL ship a language with the stable id `nx-rust`, for a host to register next
to `nx`. It SHALL take the same source as `nx` and SHALL offer the same presets, diagnostics, hover
and completion. Switching between `nx` and `nx-rust` SHALL keep the buffer.

#### Scenario: The language appears in the language switch
- **WHEN** a visitor opens the editor of a host that has registered `nx-rust`
- **THEN** the language switch SHALL offer it, and `fiddle.listPresets()` SHALL list its presets
  with `lang` equal to `nx-rust`

#### Scenario: Switching between the two NX languages keeps the snippet
- **WHEN** the buffer holds an NX snippet under `nx` and the visitor switches to `nx-rust`
- **THEN** the buffer SHALL be unchanged and the snippet SHALL be drawn by DrawnUI for Rust

#### Scenario: A host that does not register it is unaffected
- **WHEN** a host registers `nx` and not `nx-rust`
- **THEN** no file of the DrawnUI for Rust player SHALL be requested by any page of that host

#### Scenario: The editor answers as it does for nx
- **WHEN** a snippet under `nx-rust` holds an error the NX language service reports
- **THEN** the editor SHALL show the same squiggle, at the same range, as under `nx`

### Requirement: A snippet compiles in the browser and draws without a server build
Running a snippet under `nx-rust` SHALL compile it with the NX compiler in the visitor's tab against
the DrawnUI catalog and SHALL hand the compiled program to the DrawnUI for Rust player, which
evaluates the program's `root` function with the NX Rust runtime and draws the result. No server
SHALL take part in compiling or drawing.

#### Scenario: A valid snippet draws
- **WHEN** the visitor runs a snippet whose `root` function returns DrawnUI controls
- **THEN** the frame SHALL show those controls with the properties the snippet set, and
  `fiddle.getState()` SHALL report success with no errors

#### Scenario: A compile error is reported as under nx
- **WHEN** the snippet does not compile
- **THEN** the run SHALL fail with the errors `nx` reports for the same snippet, and the frame SHALL
  keep what it showed

#### Scenario: An error the player finds reaches the editor
- **WHEN** a compiled program fails to link against the player's catalog or fails while it is
  evaluated
- **THEN** the run SHALL fail with the runtime's diagnostic under the editor, and the frame SHALL
  keep what it showed

#### Scenario: Every preset draws
- **WHEN** any `nx-rust` preset is selected
- **THEN** it SHALL compile and draw with no error

### Requirement: The player stays loaded across runs
The player SHALL be loaded once per page and SHALL take each later program without being reloaded.
A run that changes only property values SHALL update the controls already drawn, so that what the
engine holds for them, such as a scroll position, is kept. A program handed to the player starts
from its initial state, so a run the page makes by itself SHALL hand the player nothing when the
player already holds that program.

#### Scenario: An edit does not reload the player
- **WHEN** the visitor edits a snippet and it is drawn again
- **THEN** the frame's page SHALL be the one that drew the previous run, and the player's
  WebAssembly module SHALL NOT be fetched or instantiated again

#### Scenario: A scroll position survives a value edit
- **WHEN** a snippet draws a scrolled list, and an edit changes a color in it
- **THEN** the list SHALL be drawn with the new color at the same scroll position

#### Scenario: Another shape of drawing replaces the old one
- **WHEN** an edit replaces a control with one of another type
- **THEN** the new control SHALL be drawn in its place and nothing of the old one SHALL remain

#### Scenario: One Run starts the program once
- **WHEN** the visitor runs a snippet, and the page then runs the same code by itself
- **THEN** the player SHALL be handed the program once, and state a handler changed after the Run
  SHALL still be there

#### Scenario: The player loads on first use
- **WHEN** a visitor uses the editor without ever selecting `nx-rust`
- **THEN** the player's page and module SHALL NOT be requested

### Requirement: The catalog's controls and properties reach DrawnUI for Rust
Every control the catalog declares SHALL be drawn by the DrawnUI for Rust control of that name.
Every catalog property SHALL either be applied to that control or be named in a list of unsupported
members that the fiddle repository carries, with a reason for each. The mapping SHALL be derived
from the catalog and from the DrawnUI for Rust version the player links, and the player's build
SHALL fail when the mapping it carries was derived from another version of either.

#### Scenario: Union cases and records arrive in DrawnUI for Rust's own types
- **WHEN** a snippet sets a property to a union case such as `Center`, or to a record such as
  `<Thickness Left=4.0 />`
- **THEN** the control SHALL be drawn as a DrawnUI for Rust app that set the same property to that
  enum variant or value type would draw it

#### Scenario: An unset property keeps the control's default
- **WHEN** a snippet leaves a property unset, or binds it to a value that evaluates to the empty
  value
- **THEN** the control SHALL be drawn with DrawnUI for Rust's default for that property

#### Scenario: A property that is no longer set returns to its default
- **WHEN** a handler changes state so that a control drawn with a property is rendered without it
- **THEN** the control SHALL be drawn with the default for that property, without being replaced

#### Scenario: An unsupported property is named once
- **WHEN** a snippet sets a property listed as unsupported
- **THEN** the rest of the drawing SHALL stand, and the console SHALL carry one line naming the
  control and the property, however many times the snippet sets it

#### Scenario: A value a control refuses draws as no value
- **WHEN** a handler changes a property from a value the control takes to one DrawnUI for Rust has
  no counterpart for
- **THEN** the control SHALL be drawn with the default for that property, as it is when the
  snippet sets the refused value from the start, and the console SHALL name the value once

#### Scenario: A snippet run again is told again
- **WHEN** a snippet that sets an unsupported property, or binds an event the control has no hook
  for, is run a second time over the controls that stand
- **THEN** the console of that run SHALL carry the same lines as the first

#### Scenario: Every catalog member is accounted for
- **WHEN** the mapping is generated
- **THEN** each property and each event of each catalog control SHALL be either mapped or in the
  unsupported list, and a member in neither SHALL fail the generation

#### Scenario: A stale mapping fails the build
- **WHEN** the catalog or the DrawnUI for Rust version has changed since the mapping was generated
- **THEN** the player's build SHALL stop with a message naming the versions that disagree

### Requirement: Authored components and their handlers run
The player SHALL draw every use of an authored component as a component instance held by the NX
Rust runtime's instance tree, SHALL turn each handler an author binds on a drawn control into that
control's DrawnUI for Rust event, and on the event SHALL dispatch the handler against the instance
whose body bound it and draw the result. An event's payload SHALL reach the handler under the field
names the catalog records for that event.

#### Scenario: A tap changes state and the drawing follows
- **WHEN** a snippet's component binds `onTapped` to an action that increments its state, and the
  visitor taps the control
- **THEN** the drawing SHALL show the incremented value

#### Scenario: Components below the root keep their own state
- **WHEN** a snippet draws two uses of a stateful component, and the visitor taps one
- **THEN** only that use SHALL change, and the other SHALL keep its state

#### Scenario: An emitted action reaches the parent
- **WHEN** a child component emits an action its parent bound a handler for
- **THEN** the parent's handler SHALL run and the drawing SHALL follow the parent's new state

#### Scenario: An event carries its payload
- **WHEN** a snippet binds `onToggled` on a switch and the visitor toggles it
- **THEN** the handler SHALL receive an action whose `value` field is the new value

#### Scenario: An event for a drawing that was replaced changes nothing
- **WHEN** an event arrives for a control that a newer render or a newer program has replaced
- **THEN** no handler SHALL run and the drawing SHALL stay as it is

#### Scenario: An unsupported event is named once
- **WHEN** a snippet binds a handler to an event listed as unsupported
- **THEN** the rest of the drawing SHALL stand, and the console SHALL carry one line naming the
  control and the event

### Requirement: A templated control draws its cells
A control bound to an item collection and a template SHALL draw one cell per item DrawnUI for Rust
realizes, the template being called with the item and its index when the cell is bound. A template
that fails for one item SHALL leave the rest of the drawing standing and SHALL be reported in the
console with the template and the index. An authored component inside a cell SHALL keep its state
for as long as its list stays in the drawing with the same collection.

#### Scenario: Only realized cells are built
- **WHEN** a snippet binds a collection far larger than the visible area through `ItemsSource` with
  an `ItemTemplate`
- **THEN** the drawing SHALL show the cells for the visible items, and the template SHALL NOT be
  called for every item of the collection

#### Scenario: A cell is bound to its item and index
- **WHEN** a cell is bound to an item
- **THEN** its content SHALL be what the template returned for that item and its index

#### Scenario: A failing template is reported, not fatal
- **WHEN** a template call fails for one item
- **THEN** the rest of the drawing SHALL stand, and the console SHALL carry one line naming the
  template and the index

#### Scenario: A component in a cell keeps its state while scrolled away
- **WHEN** the visitor changes the state of a component in a cell, scrolls the cell out of view and
  back
- **THEN** the cell SHALL show the changed state

#### Scenario: A handler the template passed in is reported inert
- **WHEN** a template binds a handler on a component it draws, outside any component's body
- **THEN** the cell SHALL be drawn, and the console SHALL carry one line naming the handler

### Requirement: A snippet cannot take the page down
A program that reaches one of the runtime's limits, or nests controls and components deeper than
the player draws, SHALL end with a diagnostic under the editor. Where it is a cell that nests too
deep, the cell SHALL be left empty and named in the console, and the rest of the drawing SHALL
stand. The player SHALL stay loaded and SHALL draw the next program.

#### Scenario: Deep recursion ends with a diagnostic
- **WHEN** a snippet's function calls itself without end
- **THEN** the run SHALL fail with the runtime's resource-limit diagnostic, and the frame's page
  SHALL NOT crash

#### Scenario: A component that draws itself ends with a diagnostic
- **WHEN** a snippet's component draws itself, with or without a control between one use and the
  next
- **THEN** the run SHALL fail with a diagnostic that says the drawing nests too deep: the
  runtime's, naming `maxComponentDepth`, or the player's, where controls and components together
  pass what the player draws
- **AND** the drawing that stood SHALL still answer events

#### Scenario: A template that draws a list of itself ends in its cell
- **WHEN** a snippet's template draws a templated control whose template is itself
- **THEN** the cells SHALL be drawn down to the depth a drawing may have, the cell past it SHALL
  be left empty, and the console SHALL carry one line naming the template and the nesting
- **AND** the player SHALL stay loaded

#### Scenario: The next run draws
- **WHEN** a run has failed at a limit and the visitor fixes the snippet
- **THEN** the fixed snippet SHALL be drawn by the same player

### Requirement: The console and the snapshot work as for other languages
What a snippet prints, and what the player reports about it, SHALL appear in the fiddle's console
panel. The frame's page SHALL provide the picture of its canvas that the engine asks a Frame
language for, so a share of an `nx-rust` snippet has a thumbnail.

#### Scenario: A print reaches the console
- **WHEN** a handler returns the catalog's print action
- **THEN** its text SHALL appear in the console panel, as it does under `nx`

#### Scenario: A share has a picture
- **WHEN** the visitor shares a drawn `nx-rust` snippet
- **THEN** the share SHALL carry a picture of what the frame showed

### Requirement: A share stores the program and plays without the compiler
Sharing an `nx-rust` snippet SHALL store its source, its language id and the snippet's compiled
program, in the form and within the size limit an `nx` share has. The `/p/` player SHALL draw the
share from that stored program with the DrawnUI for Rust player, without fetching the NX compiler.

#### Scenario: The player page draws a share
- **WHEN** a visitor opens the `/p/` link of an `nx-rust` share
- **THEN** the drawing SHALL appear, and the NX compiler module SHALL NOT be requested

#### Scenario: A share opens in the editor in its language
- **WHEN** a visitor opens the editor link of an `nx-rust` share on a host that has registered it
- **THEN** the editor SHALL select `nx-rust` and draw the snippet

#### Scenario: A share made against another catalog version says so
- **WHEN** a share's program names a catalog version the player does not carry
- **THEN** the run SHALL fail with the runtime's version message and nothing SHALL be drawn

### Requirement: The player builds against an NX checkout or against published crates
The player SHALL be built by one command of the fiddle repository into the host's web root, apart
from the command that builds the JavaScript runtimes. Its manifest SHALL name the NX runtime crates
by published version once they are published; until then it SHALL name them by path to a sibling NX
checkout, and that state SHALL NOT be merged into the fiddle's main branch.

#### Scenario: The build produces what the host serves
- **WHEN** the player's build command runs on a machine with the Rust toolchain and Emscripten it
  names
- **THEN** the web root SHALL hold the player's page and module, and the dev host SHALL draw an
  `nx-rust` snippet from them

#### Scenario: A missing toolchain is named
- **WHEN** the command runs without Emscripten or without the WebAssembly target
- **THEN** it SHALL stop with a message naming what is missing, before compiling anything

#### Scenario: The JavaScript runtime build does not need Rust
- **WHEN** `npm run runtime` runs on a machine without a Rust toolchain
- **THEN** it SHALL succeed as it does today

#### Scenario: The player's tests run headless
- **WHEN** the player's tests run
- **THEN** each NX preset SHALL be compiled, drawn on DrawnUI for Rust's CPU canvas and checked for
  errors and for reports beyond the unsupported list, with no browser
