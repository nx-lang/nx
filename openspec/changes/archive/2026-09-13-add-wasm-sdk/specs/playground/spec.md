## ADDED Requirements

### Requirement: Compilation and language queries run in the browser behind the same seam
The site SHALL compile NX and answer hover, completion, diagnostics and symbol queries in the
visitor's browser, off the main thread, and SHALL keep the choice of where that work happens
confined to the single compile seam and the language service interface, so editing and drawing do
not change when the implementation does.

#### Scenario: The browser holds the compiler
- **WHEN** the site is loaded and the visitor edits
- **THEN** NX IR and language answers SHALL be produced without any request leaving the browser

#### Scenario: The seam is uniform
- **WHEN** compilation is requested
- **THEN** it SHALL be requested through one interface that takes NX source and yields IR and
  diagnostics
- **AND** editing and drawing SHALL depend on that interface rather than on how it is fulfilled

#### Scenario: The editor stays responsive during a compile
- **WHEN** a compile or language query is in progress
- **THEN** the source pane SHALL keep accepting input and painting

#### Scenario: The same compiler is tested and shipped
- **WHEN** the example check runs
- **THEN** it SHALL compile every example through the same in-browser compiler package the site
  ships, with the same catalog handling

### Requirement: A failed compiler costs one request
A compile or language call that traps or overruns its deadline SHALL cost that request only: the
site SHALL report it as a failure, replace the compiler, and answer the next request normally.

#### Scenario: A trap is reported and recovered from
- **WHEN** the compiler traps while handling a request
- **THEN** that request SHALL be reported as a failure the visitor can read
- **AND** the next compile SHALL succeed without reloading the page

#### Scenario: An overrunning request is cut off
- **WHEN** a request has not answered within the site's deadline
- **THEN** it SHALL be reported as a failure
- **AND** the compiler that was running it SHALL be discarded rather than waited on

#### Scenario: A compiler that has not loaded yet is waited for
- **WHEN** a request is made while the compiler is still being loaded
- **THEN** the deadline SHALL measure the compiler's own work rather than the wait for the load
- **AND** the load SHALL be waited for rather than discarded and started again

#### Scenario: A load that never finishes is reported
- **WHEN** the compiler has not arrived within the site's own budget for loading it
- **THEN** the site SHALL report a failure the visitor can read rather than wait on it indefinitely

#### Scenario: A recovered failure draws without an edit
- **WHEN** a compile fails in a way a replaced compiler can answer, on a view that compiles once and
  is never edited
- **THEN** the site SHALL compile again of its own accord rather than leave the failure standing

### Requirement: Public deployment is fronted by an edge that serves assets
The public deployment SHALL sit behind an edge proxy that terminates TLS and caches according to
the origin's cache headers, so that the heavy assets, the compiler module among them, are served
near the visitor. No rate limit on the service SHALL be required, since no request costs the origin
more than a file.

#### Scenario: The site is reachable at its public address
- **WHEN** a visitor opens `https://nxlang.org/playground`
- **THEN** the gallery SHALL be served over TLS
- **AND** `http://nxlang.org/playground` SHALL redirect to it

#### Scenario: Assets are served from the edge
- **WHEN** a hashed asset, the CanvasKit binary or the NX compiler module is requested a second
  time from the same region
- **THEN** it SHALL be served from the edge cache rather than from the service

## MODIFIED Requirements

### Requirement: Site is served under the playground path
Everything the site serves — the gallery, each example's editor view, the health route, and every
static asset — SHALL live under the `/playground` path prefix, so that the rest of the domain can
later be served by something else without the playground changing.

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
- **WHEN** the hosting platform checks the service's health
- **THEN** the request SHALL go to a path under `/playground/api/`
- **AND** no compile or language route SHALL be served under that path, since the client compiles
  and answers language queries itself

#### Scenario: Assets live under the prefix
- **WHEN** the shell loads its scripts, styles, fonts, images, the CanvasKit binary and the NX
  compiler module
- **THEN** every one of those requests SHALL be for a path under `/playground/`

#### Scenario: Root redirects to the playground
- **WHEN** a visitor opens `/`
- **THEN** the service SHALL answer with a temporary redirect to `/playground`

#### Scenario: Paths outside the prefix are not the shell
- **WHEN** a request names a path that is neither `/` nor under `/playground`
- **THEN** the service SHALL answer not found rather than serving the shell

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
- **WHEN** the compiler that answers language queries cannot be loaded or fails
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
- **WHEN** compilation fails for any reason, including a compiler that crashed or did not load
- **THEN** the site SHALL report the failure and remain editable

### Requirement: Service reports its own health
The service SHALL answer a health request under the API prefix so that the hosting platform can
tell a deployment that serves from one that does not before it switches traffic.

#### Scenario: A healthy service answers
- **WHEN** `GET /playground/api/health` is requested and the process is able to serve requests
- **THEN** the service SHALL answer with a success status and a small JSON body

#### Scenario: A stuck service does not answer
- **WHEN** the process's request thread is blocked and cannot serve files
- **THEN** the health request SHALL NOT be answered, since it is served from that same thread

#### Scenario: A deployment that cannot serve is not switched to
- **WHEN** a new deployment starts and its health request is not answered within the platform's
  deadline
- **THEN** the platform SHALL keep the previous deployment serving

### Requirement: Origin sets cache policy for the edge
The service SHALL send cache headers that let an edge cache hold content-addressed assets for a long
time and never hold the shell or the health answer, so that a deploy is visible on the next page
load while the heavy assets are served from the edge.

#### Scenario: Hashed assets are cacheable for a long time
- **WHEN** a build-output asset whose file name carries a content hash is served, the NX compiler
  module included
- **THEN** the response SHALL declare itself publicly cacheable, immutable, and valid for at least a
  year

#### Scenario: Static files without a hash are revalidated
- **WHEN** a font, image or other static file whose name carries no content hash is served
- **THEN** the response SHALL be cacheable but SHALL require revalidation within a day

#### Scenario: The shell is not cached
- **WHEN** the shell document is served, for any address that resolves to it
- **THEN** the response SHALL require revalidation on every use

#### Scenario: API answers are not cached
- **WHEN** a health request, the only request under the API prefix, is answered
- **THEN** the response SHALL declare itself not storable

### Requirement: Site is deployable as a single service
The site SHALL be deployable as one service that serves the client application, its build SHALL be
reproducible from the repository, and the deployment's own configuration — how the image is built,
where the health check is, how restarts happen — SHALL be committed in the repository rather than
held only in a hosting dashboard.

#### Scenario: One service serves everything
- **WHEN** the site is deployed
- **THEN** a single service SHALL serve the client application, the compiler module it loads, and
  the health route

#### Scenario: A request the service cannot understand does not end it
- **WHEN** a request names a path the URL decoder rejects, or fails anywhere outside a handler's own
  error handling
- **THEN** the service SHALL answer that request with an error status
- **AND** it SHALL still answer the requests that follow

#### Scenario: Build produces its own native dependencies
- **WHEN** the deployment image is built from a clean checkout
- **THEN** the build SHALL produce the NX compiler module the client loads
- **AND** it SHALL NOT depend on artifacts built outside the image
- **AND** the running service SHALL need no Rust toolchain and no native addon

#### Scenario: Deployment configuration is in the repository
- **WHEN** the hosting platform builds and runs the service
- **THEN** the Dockerfile path, the health check path and the restart policy SHALL come from a
  configuration file committed in the repository

#### Scenario: Deploys follow the main branch
- **WHEN** a commit that touches the site or a package it depends on lands on `main`
- **THEN** a new deployment SHALL be built and, once its health check passes, replace the previous
  one

### Requirement: Site documents how to run, sync and deploy it
The site SHALL carry documentation covering how to build and run it locally, what it depends on
including the wasm toolchain, how to refresh its vendored DrawnUI copy, and how it is deployed,
including the one-time edge and hosting setup.

#### Scenario: Local run is documented
- **WHEN** a contributor reads the site's documentation
- **THEN** it SHALL describe the prerequisites, the wasm toolchain among them, and the steps to run
  the site locally under the `/playground` prefix with no compile server

#### Scenario: Vendored source provenance is recorded
- **WHEN** a contributor inspects the vendored DrawnUI copy
- **THEN** the upstream revision it was taken from SHALL be recorded

#### Scenario: Deployment is documented
- **WHEN** a maintainer needs to deploy, roll back, or set the site up on a fresh hosting account
- **THEN** the repository's deployment docs SHALL describe the day-to-day flow and the one-time
  setup, including every edge rule the site depends on
- **AND** they SHALL NOT describe a rate limit the site no longer needs

## REMOVED Requirements

### Requirement: Compilation runs on the server behind a replaceable boundary
**Reason**: Compilation moved into the browser; the seam it was confined to is now fulfilled by the
in-browser compiler.
**Migration**: See "Compilation and language queries run in the browser behind the same seam".

### Requirement: A stuck service ends itself
**Reason**: The service no longer runs compile or language calls, so nothing can block its request
thread; the watchdog and the restart policy it relied on have no work to do.
**Migration**: A stuck compiler in the browser is handled per "A failed compiler costs one
request".

### Requirement: Public deployment is fronted by an edge that limits abuse
**Reason**: The rate limit protected a single-threaded origin that compiled on request. The origin
now serves files only.
**Migration**: See "Public deployment is fronted by an edge that serves assets"; the rate-limit
rule is removed from the zone and from the deployment docs.
