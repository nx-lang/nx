## Purpose

The public NX Playground at `https://nxlang.org/playground`: a browser site where a visitor picks an
example from a gallery, edits its NX source, and sees the interface it describes drawn live beside the
editor. Today it draws with DrawnUI; the site is shaped so that further targets can join later.

## ADDED Requirements

### Requirement: Site is served under the playground path
Everything the site serves — the gallery, each example's editor view, the API, and every static
asset — SHALL live under the `/playground` path prefix, so that the rest of the domain can later be
served by something else without the playground changing.

#### Scenario: Gallery address
- **WHEN** a visitor opens `/playground` or `/playground/`
- **THEN** the gallery SHALL be shown

#### Scenario: Example editor address
- **WHEN** a visitor opens `/playground/<id>` for a gallery example's id
- **THEN** the editor view SHALL open with that example loaded, without passing through the gallery

#### Scenario: Unknown example
- **WHEN** a visitor opens `/playground/<id>` and no example has that id
- **THEN** the site SHALL show the gallery rather than an error page

#### Scenario: API lives under the prefix
- **WHEN** the client compiles or asks for hover, completion or diagnostics
- **THEN** the request SHALL go to a path under `/playground/api/`

#### Scenario: Assets live under the prefix
- **WHEN** the shell loads its scripts, styles, fonts, images and the CanvasKit binary
- **THEN** every one of those requests SHALL be for a path under `/playground/`

#### Scenario: Root redirects to the playground
- **WHEN** a visitor opens `/`
- **THEN** the service SHALL answer with a temporary redirect to `/playground`

#### Scenario: Paths outside the prefix are not the shell
- **WHEN** a request names a path that is neither `/` nor under `/playground`
- **THEN** the service SHALL answer not found rather than serving the shell

### Requirement: Site is branded as the NX Playground
The site SHALL present itself as the NX Playground — a place to try NX — and SHALL say, secondarily,
that what it draws today is DrawnUI, so that the general purpose and the current scope are both
visible at a glance.

#### Scenario: Gallery heading and subtitle
- **WHEN** a visitor opens the gallery
- **THEN** the primary heading SHALL name the NX Playground
- **AND** a subtitle SHALL name DrawnUI as what the playground currently draws with

#### Scenario: Document titles
- **WHEN** a visitor is on the gallery or on an example's editor view
- **THEN** the browser tab title SHALL name the NX Playground, and on the editor view the example as
  well

#### Scenario: Fiddle is gone from the visitor's view
- **WHEN** the site's text, addresses and document titles are inspected
- **THEN** none of them SHALL use the word "fiddle"

### Requirement: Gallery presents the DrawnUI example set
The site SHALL open on a gallery that mirrors the top-level structure of the DrawnUI React demo
site, listing its examples so that a visitor can see what the control set can do before writing any
NX.

#### Scenario: Gallery lists the examples
- **WHEN** a visitor opens the gallery
- **THEN** the gallery SHALL list an entry for each example carried over from the DrawnUI demo site
- **AND** each entry SHALL be identified by the same name the demo site gives it

#### Scenario: Each entry draws its example
- **WHEN** a visitor views a gallery entry
- **THEN** that entry's example SHALL be drawn from its NX source through the same pipeline the
  editor view uses
- **AND** the drawing SHALL NOT require the visitor to open the editor view first

### Requirement: Every gallery entry opens in the editor view
Each gallery entry SHALL offer an affordance that opens the editor view with that entry's NX loaded,
and the editor view for an entry SHALL be reachable by its own address so it can be linked to
directly.

#### Scenario: Opening an example
- **WHEN** a visitor activates a gallery entry's edit affordance
- **THEN** the editor view SHALL open with that entry's NX in the source pane
- **AND** the output pane SHALL draw that example
- **AND** the address SHALL change to that example's editor address

#### Scenario: Editing an example does not alter the gallery
- **WHEN** a visitor edits an example in the editor view and returns to the gallery
- **THEN** the gallery entry SHALL still show the example as originally authored

#### Scenario: Returning to the gallery
- **WHEN** a visitor is in the editor view
- **THEN** there SHALL be a way back to the gallery
- **AND** taking it SHALL change the address to the gallery address

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
- **AND** its wording SHALL be addressed to a visitor, not to a maintainer surveying gaps

#### Scenario: Omitted examples are accounted for
- **WHEN** a DrawnUI demo page has no NX example at all
- **THEN** the site's documentation SHALL name it and say why
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
the behavior would have appeared, so that a visitor who opens the editor to find out why nothing
happens reads the answer in the code.

#### Scenario: The source explains an omission in place
- **WHEN** a visitor opens a static or reduced example in the editor view
- **THEN** the source SHALL carry a note at the point the omitted behavior would have been written
- **AND** that note SHALL name the same capability the example's coverage names

### Requirement: Every gallery entry is backed by working NX
Every gallery entry SHALL have NX source that compiles and draws, and its edit affordance SHALL open
that source. No entry SHALL be a placeholder standing in for an example that does not exist.

#### Scenario: A reduced example still opens in the editor view
- **WHEN** a visitor opens a reduced example's edit affordance
- **THEN** the editor view SHALL open with that example's NX
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
Every example SHALL be NX source that compiles through the site's own pipeline, rather than a
hand-built value tree or a drawing produced some other way.

#### Scenario: Examples compile
- **WHEN** the site's examples are checked
- **THEN** every example SHALL compile with no diagnostics

#### Scenario: An example is exactly what the editor loads
- **WHEN** a visitor opens an example in the editor view
- **THEN** the source shown SHALL be the same source the gallery drew

### Requirement: Two-pane editor view renders authored NX
The editor view SHALL present a source pane and an output pane, and SHALL draw the interface
described by the source pane's NX into the output pane.

#### Scenario: The editor view always opens on working source
- **WHEN** a visitor opens the editor view, whether from a gallery entry or directly
- **THEN** the source pane SHALL contain NX using catalog controls
- **AND** the output pane SHALL draw it without any visitor action

#### Scenario: Edits update the drawing
- **WHEN** a visitor edits the source pane so that it still compiles
- **THEN** the output pane SHALL be redrawn from the edited source

#### Scenario: Updates are not issued per keystroke
- **WHEN** a visitor types continuously
- **THEN** the site SHALL coalesce the edits and compile once the visitor pauses, rather than
  compiling on every keystroke

#### Scenario: Last good drawing survives a broken edit
- **WHEN** an edit makes the source fail to compile
- **THEN** the output pane SHALL continue to show the last successfully drawn interface
- **AND** the failure SHALL be reported as a diagnostic rather than by clearing the output

### Requirement: Source pane is an NX-aware editor
The source pane SHALL highlight NX using the syntax definition the repository already publishes for
editors, so that highlighting in the playground and in the editor extension cannot drift apart, and
it SHALL obtain that highlighting through the shared Monaco integration rather than a site-local
bridge. The source pane SHALL offer hover and completion answered by the NX language service, with
catalog declarations visible to both, so that a visitor can discover the control set from inside the
editor.

#### Scenario: NX is syntax highlighted
- **WHEN** the source pane contains NX
- **THEN** it SHALL be highlighted according to the repository's published NX grammar

#### Scenario: Grammar is not duplicated
- **WHEN** the repository's published NX grammar changes
- **THEN** the playground SHALL pick up the change without a separate grammar being edited

#### Scenario: Highlighting is not site-specific
- **WHEN** the playground's editor is inspected
- **THEN** it SHALL contain no TextMate bridge, tokenizer, or theme of its own
- **AND** highlighting SHALL come from the shared Monaco integration

#### Scenario: Hover on a catalog component
- **WHEN** a visitor hovers a tag naming a DrawnUI catalog component
- **THEN** the source pane SHALL show that component's signature, including its properties and their
  types, rendered as highlighted NX

#### Scenario: Hover on the visitor's own declarations
- **WHEN** a visitor hovers a name declared in their own source
- **THEN** the source pane SHALL show what the language service reports for it

#### Scenario: Completions inside a catalog tag
- **WHEN** a visitor requests completions inside the opening tag of a catalog component
- **THEN** the source pane SHALL offer that component's properties not yet supplied

#### Scenario: Positions are not shifted by catalog injection
- **WHEN** the visitor hovers or requests completions at a position
- **THEN** the query SHALL be answered for the position the visitor sees, and any returned range
  SHALL be in the visitor's own lines and columns

#### Scenario: Language features degrade without breaking editing
- **WHEN** the language route cannot be reached or fails
- **THEN** hover and completion SHALL silently offer nothing
- **AND** the source pane SHALL remain editable and compilation SHALL continue to work

### Requirement: Compilation reports diagnostics against authored source
The site SHALL compile the visitor's NX and report every resulting diagnostic, positioned against
the source the visitor actually wrote.

#### Scenario: Diagnostics are shown in the source pane
- **WHEN** compilation reports a diagnostic with a source position
- **THEN** the source pane SHALL mark the reported span
- **AND** the diagnostic message SHALL be readable by the visitor

#### Scenario: Positions are not shifted by catalog injection
- **WHEN** the site compiles the visitor's source together with the control catalog
- **THEN** a diagnostic on the visitor's source SHALL report the line and column the visitor sees in
  the source pane

#### Scenario: Catalog-internal failures are not blamed on the visitor
- **WHEN** a diagnostic's position falls inside the injected catalog rather than the visitor's source
- **THEN** the site SHALL report it as an application fault
- **AND** it SHALL NOT mark a span in the visitor's source

#### Scenario: A position with no width is still a position
- **WHEN** a diagnostic names a point rather than a range, as a missing token's insertion point does
- **THEN** the site SHALL report it against the visitor's source rather than as a fault with no
  position
- **AND** the source pane SHALL mark it visibly

#### Scenario: Compilation errors do not break the session
- **WHEN** compilation fails for any reason, including a transport failure
- **THEN** the site SHALL report the failure and remain editable

### Requirement: Catalog is available to authored source without being shown
The visitor SHALL be able to use catalog controls without declaring or importing them, and the
catalog SHALL NOT appear in the source pane.

How the catalog is injected SHALL NOT constrain the shape of the visitor's file. Any source the
language accepts as a whole file SHALL compile in the playground, and SHALL do so with the
diagnostics and positions the visitor would see compiling that same text on its own.

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
The site SHALL evaluate compiled NX to a value tree and translate that tree into DrawnUI controls,
mapping each element to the control its type names and each property to that control's
corresponding input.

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

#### Scenario: Unset properties keep DrawnUI defaults
- **WHEN** an evaluated property carries no value
- **THEN** the site SHALL leave the control's own default in place

#### Scenario: Unknown elements are reported, not drawn
- **WHEN** the evaluated tree contains an element that names no known control
- **THEN** the site SHALL report the unknown element to the visitor
- **AND** it SHALL NOT abort drawing the rest of the tree

### Requirement: Interaction is limited to DrawnUI's own behavior
Authored NX SHALL describe appearance and structure only. The site SHALL NOT invoke authored NX in
response to visitor interaction with the drawing.

#### Scenario: Built-in gestures still work
- **WHEN** a visitor interacts with a drawn control that DrawnUI handles on its own, such as
  scrolling a scrollable region
- **THEN** the control SHALL respond as DrawnUI does

#### Scenario: Event properties are unavailable
- **WHEN** a visitor tries to attach behavior to a control event from NX
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property
- **AND** the site's documentation SHALL state that authored interaction is not yet supported

### Requirement: Compilation runs on the server behind a replaceable boundary
The site SHALL compile NX on the server, and SHALL confine the choice of where compilation happens
to a single seam so that moving compilation into the browser later does not disturb editing or
drawing.

#### Scenario: The browser holds no compiler
- **WHEN** the site is loaded
- **THEN** the browser SHALL obtain NX IR by request rather than by compiling NX itself

#### Scenario: The seam is uniform
- **WHEN** compilation is requested
- **THEN** it SHALL be requested through one interface that takes NX source and yields IR and
  diagnostics
- **AND** editing and drawing SHALL depend on that interface rather than on how it is fulfilled

### Requirement: Service reports its own health
The service SHALL answer a health request under the API prefix, and SHALL answer it from the same
thread that runs compilation, so that a process stuck in a native call cannot report itself
healthy, and so that the hosting platform can tell a deployment that serves from one that does not
before it switches traffic.

#### Scenario: A healthy service answers
- **WHEN** `GET /playground/api/health` is requested and the process is able to serve requests
- **THEN** the service SHALL answer with a success status and a small JSON body

#### Scenario: A stuck service does not answer
- **WHEN** the process is blocked in a compile or language call that has not returned
- **THEN** the health request SHALL NOT be answered until that call returns

#### Scenario: A deployment that cannot serve is not switched to
- **WHEN** a new deployment starts and its health request is not answered within the platform's
  deadline
- **THEN** the platform SHALL keep the previous deployment serving

### Requirement: A stuck service ends itself
Because compile and language calls block the service's only request thread and nothing in the
process can interrupt a native call, the service SHALL detect a main thread that has stopped
answering and SHALL end the process, so that the hosting platform's restart policy replaces it
rather than leaving a process that is up and answers nothing.

#### Scenario: A blocked main thread ends the process
- **WHEN** the main thread has not run for longer than the watchdog's deadline
- **THEN** the service SHALL end the process with a failure the platform's restart policy acts on
- **AND** the platform SHALL be configured to restart a process that ends in failure

#### Scenario: A busy but responsive service is left alone
- **WHEN** the main thread keeps running, however loaded
- **THEN** the watchdog SHALL NOT end the process
- **AND** the watchdog SHALL NOT keep the process alive once the server has stopped

### Requirement: Origin sets cache policy for the edge
The service SHALL send cache headers that let an edge cache hold content-addressed assets for a long
time and never hold the shell or an API answer, so that a deploy is visible on the next page load
while the heavy assets are served from the edge.

#### Scenario: Hashed assets are cacheable for a long time
- **WHEN** a build-output asset whose file name carries a content hash is served
- **THEN** the response SHALL declare itself publicly cacheable, immutable, and valid for at least a
  year

#### Scenario: Static files without a hash are revalidated
- **WHEN** a font, image or other static file whose name carries no content hash is served
- **THEN** the response SHALL be cacheable but SHALL require revalidation within a day

#### Scenario: The shell is not cached
- **WHEN** the shell document is served, for any address that resolves to it
- **THEN** the response SHALL require revalidation on every use

#### Scenario: API answers are not cached
- **WHEN** a compile, language or health request is answered
- **THEN** the response SHALL declare itself not storable

### Requirement: Site is deployable as a single service
The site SHALL be deployable as one service that serves both the client application and
compilation, its build SHALL be reproducible from the repository, and the deployment's own
configuration — how the image is built, where the health check is, how restarts happen — SHALL be
committed in the repository rather than held only in a hosting dashboard.

#### Scenario: One service serves everything
- **WHEN** the site is deployed
- **THEN** a single service SHALL serve the client application and answer compile, language and
  health requests

#### Scenario: A request the service cannot understand does not end it
- **WHEN** a request names a path the URL decoder rejects, or fails anywhere outside a handler's own
  error handling
- **THEN** the service SHALL answer that request with an error status
- **AND** it SHALL still answer the requests that follow

#### Scenario: Build produces its own native dependencies
- **WHEN** the deployment image is built from a clean checkout
- **THEN** the build SHALL produce every native artifact compilation requires
- **AND** it SHALL NOT depend on artifacts built outside the image

#### Scenario: Deployment configuration is in the repository
- **WHEN** the hosting platform builds and runs the service
- **THEN** the Dockerfile path, the health check path and the restart policy SHALL come from a
  configuration file committed in the repository

#### Scenario: Deploys follow the main branch
- **WHEN** a commit that touches the site or a package it depends on lands on `main`
- **THEN** a new deployment SHALL be built and, once its health check passes, replace the previous
  one

### Requirement: Public deployment is fronted by an edge that limits abuse
The public deployment SHALL sit behind an edge proxy that terminates TLS, limits the rate of
requests to the API path, and caches according to the origin's cache headers, because the service
answers compile and language requests one at a time on a single thread.

#### Scenario: The site is reachable at its public address
- **WHEN** a visitor opens `https://nxlang.org/playground`
- **THEN** the gallery SHALL be served over TLS
- **AND** `http://nxlang.org/playground` SHALL redirect to it

#### Scenario: API requests are rate limited
- **WHEN** one client sends requests to the API path faster than the configured limit
- **THEN** the edge SHALL refuse the excess without forwarding it to the service

#### Scenario: Assets are served from the edge
- **WHEN** a hashed asset or the CanvasKit binary is requested a second time from the same region
- **THEN** it SHALL be served from the edge cache rather than from the service

### Requirement: Site documents how to run, sync and deploy it
The site SHALL carry documentation covering how to build and run it locally, what it depends on,
how to refresh its vendored DrawnUI copy, and how it is deployed, including the one-time edge and
hosting setup.

#### Scenario: Local run is documented
- **WHEN** a contributor reads the site's documentation
- **THEN** it SHALL describe the prerequisites and the steps to run the site locally under the
  `/playground` prefix

#### Scenario: Vendored source provenance is recorded
- **WHEN** a contributor inspects the vendored DrawnUI copy
- **THEN** the upstream revision it was taken from SHALL be recorded

#### Scenario: Deployment is documented
- **WHEN** a maintainer needs to deploy, roll back, or set the site up on a fresh hosting account
- **THEN** the repository's deployment docs SHALL describe the day-to-day flow and the one-time
  setup, including every edge rule the site depends on
