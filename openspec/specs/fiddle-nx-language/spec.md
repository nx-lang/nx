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
- **WHEN** a snippet leaves a property unset
- **THEN** the control SHALL be created without that property, so the DrawnUI default applies

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
